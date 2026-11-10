//! egui application: panels and controls only. Every action is a `Command`; every number shown comes from a snapshot.
//!
//! Layout (see DESIGN.md "UI layout and hierarchy"):
//!   top bar      transport + speed on the left, conformance badge on the right (always visible, largest element)
//!   left panel   Scene (preset, what to look at) / Physics / Time stepping / Quadrature / Backend / View
//!   centre       3D view; below it the sky-map panel (toggle with S)
//!   right panel  conformance card + headline tiles, then collapsible detail sections
use crate::export::{default_export_name, export_snapshot, metadata_path};
use crate::protocol::*;
use crate::sim::{self, SimHandle};
use crate::skyview::SkyPanel;
use crate::view3d::{ColorBy, ViewState, COMP_COLORS, COMP_LONG, COMP_NAMES};
use eframe::egui::{self, Color32, RichText};
use shadowgrav_core::skymap::SkyFocus;
use shadowgrav_core::Mode;
use std::collections::VecDeque;
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::time::Duration;

const GREEN: Color32 = Color32::from_rgb(70, 200, 90);
const AMBER: Color32 = Color32::from_rgb(240, 180, 40);
const RED: Color32 = Color32::from_rgb(235, 70, 60);
const ACCENT: Color32 = Color32::from_rgb(120, 190, 255);
const DIM: Color32 = Color32::from_gray(140);

#[derive(Clone)]
struct UiParams {
    kappa0: f64,
    xi: f64,
    v_g: f64,
    alpha_bg: f64,
    h: f64,
    f: f64,
    mode: Mode,
    adaptive: bool,
    quad_tier: QuadTier,
    instrumented: bool,
    leaf_budget: u64,
    thin_fast: bool,
    parallel: bool,
}
impl UiParams {
    fn from(p: &ParamView) -> Self {
        UiParams {
            kappa0: p.kappa0,
            xi: p.xi,
            v_g: p.v_g,
            alpha_bg: p.alpha_bg,
            h: p.h,
            f: p.f,
            mode: p.mode,
            adaptive: p.adaptive,
            quad_tier: p.quad_tier,
            instrumented: p.instrumented,
            leaf_budget: p.leaf_budget,
            thin_fast: p.thin_fast,
            parallel: p.parallel,
        }
    }
}

pub struct App {
    handle: SimHandle,
    snap: Option<Arc<FrameSnapshot>>,
    view: ViewState,
    sky: SkyPanel,
    show_sky: bool, // default on: sky is a first-class panel
    sky_body: usize,
    prev_sky_body: usize,
    sent_sky: Option<(usize, SkyFocus, f64)>,
    sent_sky_on: Option<bool>,
    spec: PresetSpec,
    load_gen: u64,
    last_err: Option<String>,
    p: UiParams,
    rate: f64,
    rate_max: bool,
    hist_de: VecDeque<f64>,
    hist_rp: VecDeque<f64>,
    hist_na: VecDeque<f64>,
    last_diag_seq: u64,
    last_export_msg: Option<String>,
    /// Extra pixels withheld from the right panel when the painted canvas is still under MIN_CANVAS_PX.
    canvas_hold: f32,
}
const HIST: usize = 300;

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        cc.egui_ctx.style_mut(|s| {
            s.spacing.item_spacing = egui::vec2(8.0, 5.0);
            s.spacing.slider_width = 140.0;
        });
        let ctx = cc.egui_ctx.clone();
        let handle = sim::spawn(Box::new(move || ctx.request_repaint()));
        App {
            handle,
            snap: None,
            view: ViewState::new(),
            sky: SkyPanel::new(),
            show_sky: true,
            sky_body: 0,
            prev_sky_body: 0,
            sent_sky: None,
            sent_sky_on: None,
            spec: PresetSpec::default_for(PresetKind::TwoBodyOrbit),
            load_gen: u64::MAX,
            last_err: None,
            p: UiParams {
                kappa0: 2e-10,
                xi: 1.0,
                v_g: 1e3,
                alpha_bg: 0.0,
                h: 1e-3,
                f: 0.15,
                mode: Mode::V0,
                adaptive: false,
                quad_tier: QuadTier::Standard,
                instrumented: true,
                leaf_budget: 0,
                thin_fast: false,
                parallel: true,
            },
            rate: 600.0,
            rate_max: false,
            hist_de: VecDeque::new(),
            hist_rp: VecDeque::new(),
            hist_na: VecDeque::new(),
            last_diag_seq: u64::MAX,
            last_export_msg: None,
            canvas_hold: 0.0,
        }
    }

    fn ingest(&mut self, s: Arc<FrameSnapshot>) {
        if s.load_gen != self.load_gen {
            self.load_gen = s.load_gen;
            self.p = UiParams::from(&s.params);
            if let Some(sp) = s.preset {
                self.spec = sp;
            }
            self.view.fit_and_reset(&s); // data-driven default camera: look straight at a planar scene
            self.view.clear_trails();
            // a body is always selected, so arrows and the sky map are never empty on arrival
            let first = s.bodies.iter().position(|b| !b.is_probe).unwrap_or(0);
            self.view.selected = Some(first);
            self.sky_body = first;
            self.prev_sky_body = first;
            self.sky.reset_focus();
            self.hist_de.clear();
            self.hist_rp.clear();
            self.hist_na.clear();
        }
        if s.error != self.last_err {
            self.p = UiParams::from(&s.params);
            self.last_err = s.error.clone();
        }
        if s.diag.seq != self.last_diag_seq {
            self.last_diag_seq = s.diag.seq;
            let lg = |x: f64| x.max(1e-18).log10();
            let na = s
                .diag
                .targets
                .iter()
                .map(|t| t.delta_na)
                .fold(0.0, f64::max);
            for (h, v) in [
                (&mut self.hist_de, s.diag.de),
                (&mut self.hist_rp, s.diag.rp_total),
                (&mut self.hist_na, na),
            ] {
                if h.len() >= HIST {
                    h.pop_front();
                }
                h.push_back(lg(v));
            }
        }
        self.view.push_trails(&s);
        self.snap = Some(s);
    }
}

// ---------------------------------------------------------------------------------------------- small widgets
fn sci(x: f64) -> String {
    format!("{x:.3e}")
}

fn kv(ui: &mut egui::Ui, k: &str, v: String) {
    ui.label(RichText::new(k).weak());
    ui.monospace(v);
    ui.end_row();
}
fn kv_c(ui: &mut egui::Ui, k: &str, v: String, col: Color32, tip: &str) {
    let r = ui.label(RichText::new(k).weak());
    if !tip.is_empty() {
        r.on_hover_text(tip);
    }
    ui.label(RichText::new(v).monospace().color(col));
    ui.end_row();
}
fn flag(b: bool) -> (String, Color32) {
    (b.to_string(), if b { GREEN } else { DIM })
}

/// Section label in the side panels (small, dim, with a rule) so groups read as groups.
fn header(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(text.to_uppercase())
            .small()
            .strong()
            .color(DIM),
    );
    ui.separator();
}

fn slider(
    ui: &mut egui::Ui,
    v: &mut f64,
    r: RangeInclusive<f64>,
    label: &str,
    log: bool,
    tip: &str,
) -> bool {
    ui.add(egui::Slider::new(v, r).logarithmic(log).text(label))
        .on_hover_text(tip)
        .changed()
}

fn tile(ui: &mut egui::Ui, caption: &str, value: String, col: Color32, tip: &str) {
    egui::Frame::NONE
        .fill(Color32::from_gray(24))
        .corner_radius(4.0)
        .inner_margin(6.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(caption).small().weak());
            ui.label(RichText::new(value).monospace().size(15.0).color(col));
        })
        .response
        .on_hover_text(tip);
}

fn spark(ui: &mut egui::Ui, title: &str, data: &VecDeque<f64>, color: Color32) {
    let w = ui.available_width().min(320.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 46.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 2.0, Color32::from_gray(22));
    let (mut mn, mut mx) = (0.0, 0.0);
    if data.len() >= 2 {
        mn = data.iter().cloned().fold(f64::INFINITY, f64::min);
        mx = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let span = (mx - mn).max(1e-9);
        let n = (data.len() - 1) as f32;
        let pts: Vec<egui::Pos2> = data
            .iter()
            .enumerate()
            .map(|(i, v)| {
                egui::pos2(
                    rect.left() + i as f32 / n * rect.width(),
                    rect.bottom() - 4.0 - ((v - mn) / span) as f32 * (rect.height() - 16.0),
                )
            })
            .collect();
        p.add(egui::Shape::line(pts, egui::Stroke::new(1.3_f32, color)));
    }
    p.text(
        rect.left_top() + egui::vec2(4.0, 2.0),
        egui::Align2::LEFT_TOP,
        format!("{title}  log10 in [{mn:.1}, {mx:.1}]"),
        egui::FontId::monospace(10.0),
        Color32::GRAY,
    );
}

fn badge_style(b: Badge) -> (Color32, &'static str) {
    match b {
        Badge::Green => (GREEN, "CONFORMING"),
        Badge::Amber => (AMBER, "CAVEATS"),
        Badge::Red => (RED, "NON-CONFORMING"),
    }
}

/// The conformance badge as a filled pill: the largest, highest-contrast element on screen.
fn badge_pill(ui: &mut egui::Ui, snap: &FrameSnapshot) {
    let (col, txt) = badge_style(snap.badge);
    let reasons = snap.badge_reasons.clone();
    let r = egui::Frame::NONE
        .fill(col.gamma_multiply(0.22))
        .stroke(egui::Stroke::new(1.5_f32, col))
        .corner_radius(14.0)
        .inner_margin(egui::Margin::symmetric(12, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (dot, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                ui.painter().circle_filled(dot.center(), 6.0, col);
                ui.label(RichText::new(txt).strong().size(15.0).color(col));
            });
        });
    r.response.on_hover_ui(|ui| {
        for t in &reasons {
            ui.label(t);
        }
    });
}

fn next_body(snap: &FrameSnapshot, cur: usize, delta: i32) -> usize {
    let n = snap.bodies.len() as i32;
    if n == 0 {
        return 0;
    }
    let mut i = cur as i32;
    for _ in 0..n {
        i = (i + delta).rem_euclid(n);
        if !snap.bodies[i as usize].is_probe {
            return i as usize;
        }
    }
    cur
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let latest = self.handle.latest.lock().unwrap().clone();
        if let Some(s) = latest {
            if self.snap.as_ref().is_none_or(|c| c.seq != s.seq) {
                self.ingest(s);
            }
        }
        let Some(snap) = self.snap.clone() else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("starting simulation thread...");
            });
            ctx.request_repaint_after(Duration::from_millis(30));
            return;
        };
        let tx = self.handle.tx.clone();
        let send = move |c: Command| {
            let _ = tx.send(c);
        };

        // selection (3D click) drives the sky-map target; a change of target restarts the framing
        if let Some(i) = self.view.selected {
            if snap.bodies.get(i).is_some_and(|b| !b.is_probe) {
                self.sky_body = i;
            }
        }
        if self.sky_body != self.prev_sky_body {
            self.sky.reset_focus();
            self.prev_sky_body = self.sky_body;
        }

        // ------------------------------------------------------------------ keyboard
        if !ctx.wants_keyboard_input() {
            let pressed = |k: egui::Key| ctx.input(|i| i.key_pressed(k));
            if pressed(egui::Key::Space) {
                send(if snap.running {
                    Command::Pause
                } else {
                    Command::Play
                });
            }
            if pressed(egui::Key::Period) {
                send(Command::Step(1));
            }
            if pressed(egui::Key::F) {
                self.view.fit(&snap);
            }
            if pressed(egui::Key::Num1) {
                self.view.set_axis(0);
            }
            if pressed(egui::Key::Num2) {
                self.view.set_axis(1);
            }
            if pressed(egui::Key::Num3) {
                self.view.set_axis(2);
            }
            if pressed(egui::Key::S) {
                self.show_sky = !self.show_sky;
            }
            if pressed(egui::Key::E) {
                let name = default_export_name(&snap);
                match export_snapshot(std::path::Path::new(&name), &snap) {
                    Ok(()) => {
                        self.last_export_msg = Some(format!(
                            "Exported {name} + {}",
                            metadata_path(std::path::Path::new(&name))
                                .file_name()
                                .and_then(|v| v.to_str())
                                .unwrap_or("metadata.json")
                        ))
                    }
                    Err(e) => self.last_export_msg = Some(format!("Export failed: {e}")),
                }
            }
            if pressed(egui::Key::ArrowLeft) {
                self.sky_body = next_body(&snap, self.sky_body, -1);
                self.view.selected = Some(self.sky_body);
            }
            if pressed(egui::Key::ArrowRight) {
                self.sky_body = next_body(&snap, self.sky_body, 1);
                self.view.selected = Some(self.sky_body);
            }
        }

        // ------------------------------------------------------------------ top bar: transport + badge
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                let label = RichText::new(if snap.running { "Pause" } else { "Play" }).strong();
                if ui.add_sized([68.0, 28.0], egui::Button::new(label)).on_hover_text("Space").clicked() {
                    send(if snap.running { Command::Pause } else { Command::Play });
                }
                if ui.add_sized([52.0, 28.0], egui::Button::new("Step")).on_hover_text("One step (.)").clicked() { send(Command::Step(1)); }
                if ui.add_sized([68.0, 28.0], egui::Button::new("Step x100")).clicked() { send(Command::Step(100)); }
                if ui.add_sized([64.0, 28.0], egui::Button::new("Export")).on_hover_text("Rectangular CSV + JSON metadata sidecar (E)").clicked() {
                    let name = default_export_name(&snap);
                    match export_snapshot(std::path::Path::new(&name), &snap) {
                        Ok(()) => self.last_export_msg = Some(format!("Exported {name} + {}", metadata_path(std::path::Path::new(&name)).file_name().and_then(|v| v.to_str()).unwrap_or("metadata.json"))),
                        Err(e) => self.last_export_msg = Some(format!("Export failed: {e}")),
                    }
                }
                ui.separator();
                let mut changed = ui.checkbox(&mut self.rate_max, "max speed")
                    .on_hover_text("Step as fast as an 8 ms budget per frame allows. Otherwise the slider sets a target in steps per wall-clock second.").changed();
                if !self.rate_max {
                    changed |= ui.add(egui::Slider::new(&mut self.rate, 1.0..=1e5).logarithmic(true).text("steps/s")).changed();
                }
                if changed { send(Command::SetRate(if self.rate_max { 0.0 } else { self.rate })); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { badge_pill(ui, &snap); });
            });
            ui.add_space(3.0);
        });

        // ------------------------------------------------------------------ status bar
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            // Impossible-to-miss instrument strip
            if snap.params.thin_fast || snap.badge == Badge::Red {
                let msg = if snap.params.thin_fast {
                    "NON-CONFORMING SESSION — thin-fast preview (Barnes-Hut, no overlap correction). Do not quote forces from this run."
                } else {
                    "NON-CONFORMING / ERROR — see red badge reasons. State may have rolled back the last bad edit."
                };
                egui::Frame::NONE.fill(RED.gamma_multiply(0.25)).inner_margin(6.0).show(ui, |ui| {
                    ui.label(RichText::new(msg).strong().color(RED));
                });
            } else if snap.badge == Badge::Amber {
                egui::Frame::NONE.fill(AMBER.gamma_multiply(0.15)).inner_margin(4.0).show(ui, |ui| {
                    ui.label(RichText::new(format!(
                        "CAVEATS — mode {:?} · quad {} · results usable with stated limitations",
                        snap.params.mode, snap.params.quad_tier.label()
                    )).color(AMBER));
                });
            }
            ui.horizontal_wrapped(|ui| {
                ui.monospace(format!(
                    "t={:.6e}  step {}  h={:.3e}  {:.0}/s  N={}  mode {:?}  quad {}",
                    snap.time, snap.step, snap.params.h, snap.steps_per_sec, snap.bodies.len(),
                    snap.params.mode, snap.params.quad_tier.label()
                ));
                if let Some(e) = &snap.error { ui.colored_label(RED, format!("ERROR: {e}")); }
                if let Some(m) = &self.last_export_msg { ui.colored_label(ACCENT, m); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Space play  . step  E export  F fit  S sky  ←/→ body  1/2/3 views").small().weak());
                });
            });
        });

        // ------------------------------------------------------------------ sky-map panel (below the 3D view)
        let mut sky_step = 0;
        if self.show_sky {
            egui::TopBottomPanel::bottom("sky").resizable(true).default_height(340.0).min_height(200.0).show(ctx, |ui| {
                ui.label(RichText::new("Sky map — optical-depth deficit / non-additive support (selected body)").strong());
                ui.label(RichText::new("Scientific diagnostic view: not a force integrator. Toggle with S.").small().weak());
                sky_step = self.sky.show(ui, &snap, self.sky_body);
            });
        }
        if sky_step != 0 {
            self.sky_body = next_body(&snap, self.sky_body, sky_step);
            self.view.selected = Some(self.sky_body);
        }

        // ------------------------------------------------------------------ left: scene + parameters + view
        let left_w = egui::SidePanel::left("params").default_width(300.0).show(ctx, |ui| {
            // auto_shrink on a vertical-only ScrollArea follows content width, so a long row
            // widens the panel. Both axes with shrink off keep that width inside the panel.
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.set_max_width(ui.available_width());
                header(ui, "Scene");
                egui::ComboBox::from_label("preset").selected_text(self.spec.kind.label()).show_ui(ui, |ui| {
                    for k in PresetKind::ALL {
                        if ui.selectable_value(&mut self.spec.kind, k, k.label()).clicked() { self.spec = PresetSpec::default_for(k); }
                    }
                });
                egui::Frame::NONE.fill(Color32::from_gray(26)).corner_radius(4.0).inner_margin(8.0).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(RichText::new(self.spec.kind.blurb()).small());
                    ui.add_space(2.0);
                    ui.label(RichText::new("Look at").small().strong().color(DIM));
                    for l in self.spec.kind.look_for() { ui.label(RichText::new(format!("- {l}")).small().weak()); }
                });
                if self.spec.kind.uses_tau() {
                    ui.add(egui::Slider::new(&mut self.spec.tau_c, 1e-8..=1e3).logarithmic(true).text("tau_c"))
                        .on_hover_text("Central optical depth tau_c = 2 kappa rho a of every body. tau_c << 1: thin (Newtonian) limit; tau_c ~ 1 and above: saturation (S < 1) and mutual screening.");
                }
                if self.spec.kind.uses_n() {
                    ui.add(egui::Slider::new(&mut self.spec.n, 2..=2000).logarithmic(true).text("N"))
                        .on_hover_text("Number of bodies, placed uniformly at random (fixed seed) with centres at least 2a apart.");
                }
                if self.spec.kind.uses_steps() {
                    ui.add(egui::Slider::new(&mut self.spec.steps_per_orbit, 100..=4000).text("steps/orbit"))
                        .on_hover_text("Sets h = T / steps. The energy error scales as (omega h)^2.");
                }
                let pending = snap.preset.is_none_or(|p| p.kind != self.spec.kind || p.tau_c != self.spec.tau_c
                    || p.n != self.spec.n || p.steps_per_orbit != self.spec.steps_per_orbit);
                let load = egui::Button::new(RichText::new(if pending { "Load scene (changes pending)" } else { "Reload scene" }).strong());
                if ui.add_sized([ui.available_width(), 28.0], load).clicked() { send(Command::Load(self.spec)); }

                header(ui, "Physics");
                ui.horizontal(|ui| {
                    ui.label("mode");
                    for (m, tip) in [
                        (Mode::V0, "V0: static shadow forces only."),
                        (Mode::V1, "V1: adds flux drag F_dr, proportional to (v - W). Needs v_g > 0."),
                        (Mode::V2, "V2: V1 plus first-order aberration of the pair and overlap geometry. Needs alpha_bg = 0 and max |eps| <= 0.1 with eps = (v - W)/v_g. Always carries a caveat."),
                    ] {
                        if ui.selectable_value(&mut self.p.mode, m, format!("{m:?}")).on_hover_text(tip).clicked() { send(Command::SetMode(m)); }
                    }
                });
                if slider(ui, &mut self.p.kappa0, 1e-12..=1e-2, "kappa0", true,
                    "Base flux attenuation coefficient. kappa_k = kappa0 (1 + delta_k) and tau_c = 2 kappa rho a. Pi is re-derived (eq. 19) so the calibrated G_N is preserved.") { send(Command::SetKappa0(self.p.kappa0)); }
                if slider(ui, &mut self.p.xi, 0.5..=2.0, "xi", false,
                    "Coupling factor xi. Under the calibration of eq. 19 it normally cancels against Pi; a visible change in the shadow force points to uncalibrated use.") { send(Command::SetXi(self.p.xi)); }
                if slider(ui, &mut self.p.v_g, 10.0..=1e12, "v_g", true,
                    "Flux propagation speed. Sets the drag rate gamma and the aberration parameter eps = (v - W)/v_g. Ignored in V0.") { send(Command::SetVg(self.p.v_g)); }
                if slider(ui, &mut self.p.alpha_bg, 0.0..=1.0, "alpha_bg", false,
                    "Uniform background absorption rate. 0 = pure shadowing. Above 0 the deficits are attenuated with distance (eq. 17 sweep). V2 requires 0.") { send(Command::SetAlphaBg(self.p.alpha_bg)); }
                ui.label(RichText::new(format!("Pi = {:.4e}  ({})", snap.params.pi_flux, if snap.params.calibrated { "calibrated, eq. 19" } else { "UNCALIBRATED" }))
                    .small().color(if snap.params.calibrated { DIM } else { AMBER }));

                header(ui, "Time stepping");
                if slider(ui, &mut self.p.h, 1e-10..=1.0, "h", true, "Fixed time step. Energy error scales as (omega h)^2; V1/V2 add a fixed-point iteration (eq. 12) whose contraction is about h gamma / 2.") {
                    send(Command::SetFixedH(self.p.h)); self.p.adaptive = false;
                }
                if ui.checkbox(&mut self.p.adaptive, "adaptive h (eq. 13)")
                    .on_hover_text("Adaptive step control. Marks the run non-reversible and non-symplectic: the badge becomes CAVEATS.").changed() { send(Command::SetAdaptive(self.p.adaptive)); }

                header(ui, "Quadrature");
                if slider(ui, &mut self.p.f, 0.05..=0.5, "f", false,
                    "Leaf criterion for the overlap correction: a HEALPix cell is a leaf when rho <= f x theta_min. Smaller f = finer, slower. Standard mode meets |R - 0.8| <= 5e-3 on test (c).") { send(Command::SetF(self.p.f)); }
                ui.horizontal(|ui| {
                    ui.label("Quad tier");
                    egui::ComboBox::from_id_salt("quad_tier")
                        .selected_text(self.p.quad_tier.label())
                        .show_ui(ui, |ui| {
                            for t in QuadTier::ALL {
                                if ui.selectable_value(&mut self.p.quad_tier, t, t.label()).clicked() {
                                    send(Command::SetQuadTier(t));
                                }
                            }
                        })
                        .response
                        .on_hover_text(self.p.quad_tier.detail());
                });
                if self.p.quad_tier == QuadTier::Certified {
                    ui.colored_label(AMBER, "Certified: ~10–20 s per force eval. Prefer single Step, not long Play.");
                }
                ui.horizontal(|ui| {
                    ui.label("leaf budget");
                    if ui.add(egui::DragValue::new(&mut self.p.leaf_budget).speed(1000.0))
                        .on_hover_text("Abort with an error (never silently coarsen) when one target needs more leaves than this. 0 = unlimited.").changed() { send(Command::SetLeafBudget(self.p.leaf_budget)); }
                    ui.label(RichText::new("(0 = off)").small().weak());
                });

                header(ui, "Backend");
                if ui.checkbox(&mut self.p.instrumented, "instrumented")
                    .on_hover_text("Record pair moments and leaf deficits: needed for the G_eff table, heating and the eq. 20 D-bar. Enabled automatically for N <= 16.").changed() { send(Command::SetInstrumented(self.p.instrumented)); }
                if ui.checkbox(&mut self.p.parallel, "parallel over targets")
                    .on_hover_text("rayon over targets. Results are bitwise identical to the serial path.").changed() { send(Command::SetParallel(self.p.parallel)); }
                let thin_fast_colour = if self.p.thin_fast { RED } else { DIM };   // read before the mutable borrow (E0503)
                if ui.checkbox(&mut self.p.thin_fast, RichText::new("thin-fast preview — NON-CONFORMING").color(thin_fast_colour))
                    .on_hover_text("NOT for science quotes. Barnes-Hut monopole, no overlap correction. Gated to V0, alpha_bg=0, all tau_c<1e-2. Forces badge RED and status banner.")
                    .changed() { send(Command::SetThinFast(self.p.thin_fast)); }
                if self.p.thin_fast {
                    ui.colored_label(RED, "Forces shown while thin-fast is on must not be treated as Spec-conforming.");
                }

                header(ui, "View");
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.view.follow, "follow centroid").on_hover_text("Keep the centroid of the bodies at the centre of the view.");
                    ui.checkbox(&mut self.view.trails, "trails");
                    ui.checkbox(&mut self.show_sky, "sky map").on_hover_text("Show the sky-map panel under the 3D view (S).");
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("camera");
                    if ui.small_button("XY").on_hover_text("Look along -z (key 1)").clicked() { self.view.set_axis(0); }
                    if ui.small_button("XZ").on_hover_text("z up (key 2)").clicked() { self.view.set_axis(1); }
                    if ui.small_button("YZ").on_hover_text("z to the right (key 3)").clicked() { self.view.set_axis(2); }
                    if ui.small_button("Fit").on_hover_text("Re-centre and rescale (F, or double-click the view)").clicked() { self.view.fit(&snap); }
                    if ui.small_button("Reset").on_hover_text("Fit and choose the camera from the data").clicked() { self.view.fit_and_reset(&snap); }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("colour");
                    for (c, n) in [(ColorBy::TauC, "tau_c"), (ColorBy::Speed, "speed"), (ColorBy::Force, "force"), (ColorBy::Flat, "flat")] {
                        ui.selectable_value(&mut self.view.color_by, c, n);
                    }
                });
                ui.label(RichText::new("force arrows").small().strong().color(DIM));
                ui.horizontal_wrapped(|ui| {
                    for c in 0..5 {
                        ui.checkbox(&mut self.view.arrows.on[c], RichText::new(COMP_NAMES[c]).color(COMP_COLORS[c])).on_hover_text(COMP_LONG[c]);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.view.arrows.all_bodies, "all bodies").on_hover_text("Off: arrows only on the selected body.");
                    ui.checkbox(&mut self.view.arrows.per_component, "each to own max")
                        .on_hover_text("Off: one common length scale, so components are comparable (F_corr is usually far smaller than F_pair). On: every component is scaled to its own maximum, which shows direction only.");
                });
                ui.add(egui::Slider::new(&mut self.view.arrows.gain, 0.2..=3.0).text("arrow length")).on_hover_text("Length of the longest arrow is 70 px times this.");
                ui.add(egui::Slider::new(&mut self.view.exaggerate, 1.0..=200.0).logarithmic(true).text("radius x"))
                    .on_hover_text("Radii are physical (a ~ 0.01 against separations ~ 1); this exaggerates them for display only.");
                ui.add(egui::Slider::new(&mut self.view.min_px, 1.0..=12.0).text("min px")).on_hover_text("Minimum drawn radius in pixels.");
            });
        }).response.rect.width();

        // ------------------------------------------------------------------ right: conformance + diagnostics
        // Never let the diagnostics column take the canvas below MIN_CANVAS_PX (it grows to fit its widest row otherwise).
        const MIN_CANVAS_PX: f32 = 290.0;
        // canvas_hold is 0 until a frame measures the central view under 290. A previous
        // 960x700 capture was 287 px; the hold is what closes that gap if max_width alone does not.
        let right_max = (ctx.screen_rect().width() - left_w - MIN_CANVAS_PX - self.canvas_hold)
            .clamp(300.0, 560.0);
        egui::SidePanel::right("diag").default_width(370.0).max_width(right_max).show(ctx, |ui| {
            let d = snap.diag.clone();
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.set_max_width(ui.available_width());
                // conformance card: always visible, reasons spelled out instead of hidden in a tooltip
                let (col, txt) = badge_style(snap.badge);
                egui::Frame::NONE.fill(col.gamma_multiply(0.12)).stroke(egui::Stroke::new(1.0_f32, col.gamma_multiply(0.6)))
                    .corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label(RichText::new(txt).strong().size(14.0).color(col));
                        for (i, r) in snap.badge_reasons.iter().enumerate() {
                            let c = if i < snap.badge_n_red { RED } else if snap.badge == Badge::Green { GREEN } else { AMBER };
                            ui.label(RichText::new(r).small().color(c));
                        }
                        for l in &d.labels { ui.label(RichText::new(l).small().color(AMBER)); }
                        ui.add_space(4.0);
                        ui.label(RichText::new(format!(
                            "mode {:?} · {} · thin-fast {}",
                            snap.params.mode, snap.params.quad_tier.label(),
                            if snap.params.thin_fast { "ON (NON-CONFORMING)" } else { "off" }
                        )).small().color(if snap.params.thin_fast { RED } else { DIM }));
                    });
                ui.add_space(6.0);

                // headline tiles
                let max_na = d.targets.iter().map(|t| t.delta_na).fold(0.0, f64::max);
                ui.columns(2, |c| {
                    tile(&mut c[0], "dE/E0 (max so far)", sci(d.max_de), if d.energy_nonconservative { AMBER } else { Color32::WHITE },
                        "Largest relative energy change since the ledger started. Amber when E is only a diagnostic (overlap, V1/V2, alpha_bg > 0).");
                    tile(&mut c[1], "max D^NA", sci(max_na), if max_na > 0.0 { ACCENT } else { DIM },
                        "Largest |F_corr| / |F_pair + F_corr| over bodies: how much of the shadow force is non-additive.");
                });
                ui.columns(2, |c| {
                    tile(&mut c[0], "R_P (shadow only)", sci(d.rp_shadow), Color32::WHITE,
                        "|sum F| / sum |F| over the shadow forces: the momentum-conservation check of the quadrature.");
                    tile(&mut c[1], "cap hits / leaves", format!("{} / {}", d.cap_hits, d.leaf_count), if d.cap_hits > 0 { RED } else { Color32::WHITE },
                        "HEALPix refinement-cap hits (any hit turns the badge red) and total leaves in the last force evaluation.");
                });

                if !d.extras.is_empty() {
                    header(ui, "Preset read-outs");
                    egui::Grid::new("extras").num_columns(2).spacing([10.0, 3.0]).show(ui, |ui| { for (k, v) in &d.extras { kv(ui, k, v.clone()); } });
                }

                egui::CollapsingHeader::new(RichText::new("Conservation").strong()).default_open(true).show(ui, |ui| {
                    egui::Grid::new("cons").num_columns(2).spacing([10.0, 3.0]).show(ui, |ui| {
                        kv_c(ui, "E", format!("{}{}", sci(d.e), if d.energy_nonconservative { "  (diagnostic)" } else { "" }),
                            if d.energy_nonconservative { AMBER } else { Color32::WHITE }, "KE + PE. A conserved quantity only for reciprocal pair forces (V0, alpha_bg = 0, no overlap).");
                        kv(ui, "dE/E0  now / max", format!("{} / {}", sci(d.de), sci(d.max_de)));
                        kv(ui, "dP  now / max", format!("{} / {}", sci(d.dp), sci(d.max_dp)));
                        kv(ui, "dL/L0  now / max", format!("{} / {}", sci(d.dl), sci(d.max_dl)));
                        kv(ui, "R_P total / max", format!("{} / {}", sci(d.rp_total), sci(d.max_rp_total)));
                    });
                    spark(ui, "dE/E0", &self.hist_de, ACCENT);
                    spark(ui, "R_P", &self.hist_rp, Color32::from_rgb(255, 170, 90));
                });

                egui::CollapsingHeader::new(RichText::new("Force decomposition").strong()).default_open(true).show(ui, |ui| {
                    egui::Grid::new("targets").striped(true).spacing([10.0, 3.0]).show(ui, |ui| {
                        for (h, tip) in [("k", "body index (the selected one is highlighted)"), ("D^NA", "|F_corr| / |F_pair + F_corr|"),
                            ("D^sat", "relative difference of F_pair from the uniform-sphere thin-field force"),
                            ("R^N", "relative difference of F_total from the unsaturated Newtonian force"),
                            ("R^thin", "relative difference of F_total from the thin-field force"), ("gamma", "drag rate |F_dr| / (m |v - W|) (V1/V2)")] {
                            ui.label(RichText::new(h).strong()).on_hover_text(tip);
                        }
                        ui.end_row();
                        for t in d.targets.iter().take(12) {
                            let sel = t.k == self.sky_body;
                            let c = if sel { ACCENT } else { Color32::LIGHT_GRAY };
                            let m = |s: String| RichText::new(s).monospace().color(c);
                            ui.label(m(t.k.to_string())); ui.label(m(sci(t.delta_na))); ui.label(m(sci(t.delta_sat)));
                            ui.label(m(sci(t.r_newton))); ui.label(m(sci(t.r_thin)));
                            ui.label(m(t.gamma.map_or("-".to_string(), sci))); ui.end_row();
                        }
                    });
                    if d.targets.len() > 12 { ui.label(RichText::new(format!("... {} more targets", d.targets.len() - 12)).small().weak()); }
                    spark(ui, "max D^NA", &self.hist_na, Color32::from_rgb(190, 140, 255));
                });

                if !d.pairs.is_empty() {
                    egui::CollapsingHeader::new(RichText::new("Effective G (pairs)").strong()).default_open(true).show(ui, |ui| {
                        egui::Grid::new("pairs").striped(true).spacing([10.0, 3.0]).show(ui, |ui| {
                            for h in ["k-j", "G_eff", "expected", "rel diff"] { ui.label(RichText::new(h).strong()); }
                            ui.end_row();
                            for p in d.pairs.iter().take(12) {
                                ui.monospace(format!("{}-{}", p.k, p.j)); ui.monospace(sci(p.g_eff));
                                ui.monospace(p.expected.map_or("-".to_string(), sci));
                                ui.monospace(p.expected.map_or("-".to_string(), |e| sci(p.g_eff / e - 1.0))); ui.end_row();
                            }
                        });
                    });
                } else if snap.bodies.len() > 16 {
                    ui.label(RichText::new("Effective-G table off: instrumented evaluation is automatic only for N <= 16.").small().weak());
                }

                egui::CollapsingHeader::new(RichText::new("Quadrature / integrator").strong()).default_open(false).show(ui, |ui| {
                    egui::Grid::new("quad").num_columns(2).spacing([10.0, 3.0]).show(ui, |ui| {
                        kv_c(ui, "cap hits", d.cap_hits.to_string(), if d.cap_hits > 0 { RED } else { Color32::LIGHT_GRAY }, "Refinement cap reached: the result is below contract quality.");
                        kv(ui, "leaves (last eval)", d.leaf_count.to_string());
                        kv(ui, "max |F_corr|/|F_sh|", sci(d.max_corr_over_f));
                        kv(ui, "eq.12 iters / resid", format!("{} / {}", d.fp_iters, sci(d.fp_residual)));
                        kv_c(ui, "eq.12 converged", d.fp_converged.to_string(), if d.fp_converged { GREEN } else { RED },
                            "Fixed point of the velocity-dependent step. false turns the badge red.");
                        kv(ui, "force evals / step", d.force_evals.to_string());
                    });
                });

                egui::CollapsingHeader::new(RichText::new("Optical depth / validity").strong()).default_open(false).show(ui, |ui| {
                    egui::Grid::new("tau").num_columns(2).spacing([10.0, 3.0]).show(ui, |ui| {
                        kv(ui, "tau_c min/mean/max", format!("{} / {} / {}", sci(d.tau_min), sci(d.tau_mean), sci(d.tau_max)));
                        kv(ui, "S min/mean", format!("{:.6} / {:.6}", d.s_min, d.s_mean));
                        for (k, b) in [("thin-valid (tau<0.1)", d.thin_valid), ("V2-valid (|eps|<=0.1)", d.v2_valid),
                                       ("overlap present", d.overlap_present), ("interior target", d.interior_present)] {
                            let (t, c) = flag(b);
                            kv_c(ui, k, t, c, "");
                        }
                    });
                });

                if let Some(i) = self.view.selected {
                    if let Some(b) = snap.bodies.get(i) {
                        egui::CollapsingHeader::new(RichText::new(format!("Selected body {i}")).strong()).default_open(true).show(ui, |ui| {
                            egui::Grid::new("sel").num_columns(2).spacing([10.0, 3.0]).show(ui, |ui| {
                                kv(ui, "x", format!("[{:.5e}, {:.5e}, {:.5e}]", b.x[0], b.x[1], b.x[2]));
                                kv(ui, "v", format!("[{:.5e}, {:.5e}, {:.5e}]", b.v[0], b.v[1], b.v[2]));
                                kv(ui, "radius a", sci(b.radius)); kv(ui, "tau_c", sci(b.tau_c)); kv(ui, "S", format!("{:.9}", b.s));
                                let n = |v: &[f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                                for (c, v) in [(0usize, &b.f_total), (1, &b.f_pair), (2, &b.f_corr), (3, &b.f_dr), (4, &b.f_ab)] {
                                    ui.label(RichText::new(format!("|F| {}", COMP_NAMES[c])).color(COMP_COLORS[c])).on_hover_text(COMP_LONG[c]);
                                    ui.monospace(sci(n(v))); ui.end_row();
                                }
                            });
                        });
                    }
                }
            });
        });

        // ------------------------------------------------------------------ 3D view
        let mut canvas_w = 0.0_f32;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                canvas_w = ui.available_width();
                self.view.show(ui, &snap);
            });
        let deficit = MIN_CANVAS_PX - canvas_w;
        if deficit > 0.5 {
            self.canvas_hold = (self.canvas_hold + deficit).min(32.0);
        } else if self.canvas_hold > 0.0 && deficit < -1.0 {
            self.canvas_hold = (self.canvas_hold + deficit).max(0.0);
        }

        // ------------------------------------------------------------------ sky-map commands (diagnostic only)
        let want = (self.sky_body, self.sky.focus, self.sky.fov);
        if self.sent_sky != Some(want) {
            send(Command::SetSkyTarget(want.0));
            send(Command::SetSkyFocus(want.1, want.2));
            self.sent_sky = Some(want);
        }
        if self.sent_sky_on != Some(self.show_sky) {
            send(Command::SetSkyEnabled(self.show_sky));
            self.sent_sky_on = Some(self.show_sky);
        }
        ctx.request_repaint_after(Duration::from_millis(100)); // fallback; sim thread wakes us on every publish
    }
}

//! Sky-map panel: the deficit D_k(n) (and the non-additive part D^NA) seen by the selected body.
//! Reads `FrameSnapshot` only. All sky geometry (projection, inverse projection, pixel lookup) is done by
//! `shadowgrav_core::skymap`; this module colours pixels, draws overlays and formats numbers.
use crate::protocol::FrameSnapshot;
use crate::view3d::{colormap, colormap_magma, COMP_COLORS};
use eframe::egui::{self, Align2, Color32, ColorImage, FontId, Painter, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, TextureOptions, Vec2};
use shadowgrav_core::geom::{cross, normalize, norm, psi, V3};
use shadowgrav_core::skymap::{SkyFocus, SkyMap, SkyProjection};
use shadowgrav_core::Mode;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SkyMode { Deficit, NonAdditive }

/// Below this angular radius (rad) an all-sky map cannot resolve the largest disc, so the panel zooms on it once per load.
const AUTO_ZOOM_BELOW: f64 = 0.15;
const LOG_DECADES: f64 = 6.0;

pub struct SkyPanel {
    pub mode: SkyMode, pub log: bool, pub discs: bool, pub force_dirs: bool,
    pub focus: SkyFocus, pub fov: f64,
    auto_pending: bool,
    tex: Option<TextureHandle>,
    key: Option<(u64, u8, bool)>,
    vmax: f64,
}

fn sci(x: f64) -> String { format!("{:.3e}", x) }
fn sci_or_dash(x: f64) -> String { if x.is_finite() { sci(x) } else { "-".to_string() } }

impl SkyPanel {
    pub fn new() -> Self {
        SkyPanel { mode: SkyMode::Deficit, log: false, discs: true, force_dirs: true, focus: SkyFocus::All, fov: 4.0,
                   auto_pending: true, tex: None, key: None, vmax: 0.0 }
    }
    /// New scene or new target body: back to all-sky, and allow one automatic zoom when the discs are too small to see.
    pub fn reset_focus(&mut self) { self.focus = SkyFocus::All; self.auto_pending = true; }

    /// Draws the panel. Returns -1 / +1 when the person asked for the previous / next body.
    pub fn show(&mut self, ui: &mut egui::Ui, snap: &FrameSnapshot, body: usize) -> i32 {
        let mut step = 0;
        // ---------------------------------------------------------------- controls
        let focus_before = self.focus;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Sky map").strong());
            if ui.small_button("<").on_hover_text("previous body (Left arrow)").clicked() { step = -1; }
            ui.monospace(format!("body {body}"));
            if ui.small_button(">").on_hover_text("next body (Right arrow)").clicked() { step = 1; }
            ui.separator();
            ui.selectable_value(&mut self.mode, SkyMode::Deficit, "D(n)")
                .on_hover_text("Total shadow deficit along each direction n: the fraction of the flux from n that is blocked, all other bodies included.");
            ui.selectable_value(&mut self.mode, SkyMode::NonAdditive, "D^NA")
                .on_hover_text("Non-additive part: D(n) - sum_j D_kj(n). Zero where at most one disc covers n; negative where discs overlap. This is what the overlap correction (eq. 4, second sum) integrates.");
            ui.checkbox(&mut self.log, "log")
                .on_hover_text("Logarithmic colour scale over 6 decades below the maximum. Exact zeros stay dark.");
            ui.separator();
            ui.checkbox(&mut self.discs, "discs").on_hover_text("Outline each occluder's silhouette; the number is the body index.");
            ui.checkbox(&mut self.force_dirs, "force dir")
                .on_hover_text("Direction of F_pair (blue ring), F_total (white cross) and F_corr (pink plus) on this body, drawn on the map.");
        });
        if self.focus == SkyFocus::All && focus_before != SkyFocus::All { self.auto_pending = false; }

        let Some(map) = snap.diag.sky.as_ref() else {
            let note = snap.diag.sky_note.clone().unwrap_or_else(|| "waiting for the first diagnostics refresh...".to_string());
            ui.add_space(8.0);
            ui.label(RichText::new(note).weak());
            return step;
        };

        // zoom automatically once when the discs are far below all-sky resolution
        if self.auto_pending && map.k == body {
            if map.focus.is_none() && self.focus == SkyFocus::All {
                let best = map.discs.iter().filter(|d| !d.interior).max_by(|a, b| a.theta.total_cmp(&b.theta));
                if let Some(d) = best { if d.theta < AUTO_ZOOM_BELOW { self.focus = SkyFocus::Occluder(d.j); } }
            }
            self.auto_pending = false;
        }

        // framing row
        ui.horizontal_wrapped(|ui| {
            let sel = match self.focus { SkyFocus::All => "all sky".to_string(), SkyFocus::Occluder(j) => format!("zoom on disc {j}") };
            egui::ComboBox::from_label("view").selected_text(sel).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.focus, SkyFocus::All, "all sky (Mollweide)");
                for d in map.discs.iter().take(32) {
                    ui.selectable_value(&mut self.focus, SkyFocus::Occluder(d.j), format!("zoom on disc {}  (theta {:.3} deg)", d.j, d.theta.to_degrees()));
                }
            });
            if self.focus != SkyFocus::All {
                ui.add(egui::Slider::new(&mut self.fov, 1.5..=50.0).logarithmic(true).text("half FoV / theta"))
                    .on_hover_text("Half field of view as a multiple of the zoomed disc's angular radius.");
            }
        });

        // ---------------------------------------------------------------- texture
        let key = (snap.diag.seq, self.mode as u8, self.log);
        if self.key != Some(key) || self.tex.is_none() {
            let (img, vmax) = build_image(map, self.mode, self.log);
            self.vmax = vmax;
            match self.tex.as_mut() {
                Some(t) => t.set(img, TextureOptions::NEAREST),
                None => self.tex = Some(ui.ctx().load_texture("skymap", img, TextureOptions::NEAREST)),
            }
            self.key = Some(key);
        }

        // ---------------------------------------------------------------- map + stats
        let stats_w = 310.0;
        let map_w_avail = (ui.available_width() - stats_w - 14.0).max(240.0);
        let h_avail = (ui.available_height() - 50.0).max(110.0);
        let aspect = map.aspect();
        let (mw, mh) = if map_w_avail / aspect <= h_avail { (map_w_avail, map_w_avail / aspect) } else { (h_avail * aspect, h_avail) };
        let mut new_focus: Option<SkyFocus> = None;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                let (resp, painter) = ui.allocate_painter(Vec2::new(mw, mh), Sense::hover());
                let rect = resp.rect;
                let to_screen = |x: f64, y: f64| Pos2::new(rect.center().x + x as f32 * rect.width() * 0.5, rect.center().y - y as f32 * rect.height() * 0.5);
                if let Some(t) = &self.tex {
                    painter.image(t.id(), rect, Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
                self.draw_overlays(&painter, map, snap, &to_screen);
                // hover read-out (fixed-height line below the map so the layout does not jump)
                let mut readout = "hover the map for direction and values".to_string();
                if let Some(hp) = resp.hover_pos() {
                    let x = ((hp.x - rect.center().x) / (rect.width() * 0.5)) as f64;
                    let y = ((rect.center().y - hp.y) / (rect.height() * 0.5)) as f64;
                    if let (Some(n), Some(i)) = (map.unproject(x, y), map.pixel_at(x, y)) {
                        let lon = n[1].atan2(n[0]).to_degrees();
                        let lat = n[2].clamp(-1.0, 1.0).asin().to_degrees();
                        let off = match map.proj {
                            SkyProjection::Zoom { c, .. } => format!("  offset {:.4} deg", psi(&n, &c).to_degrees()),
                            SkyProjection::Mollweide => String::new(),
                        };
                        readout = format!("lon {:8.3}  lat {:8.3}{}   D {}   D^NA {}", lon, lat, off, sci(map.d_total[i] as f64), sci(map.d_na[i] as f64));
                        painter.circle_stroke(hp, 4.0, Stroke::new(1.0, Color32::from_white_alpha(200)));
                    }
                }
                self.draw_colorbar(ui, mw);
                ui.monospace(readout);
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.set_width(stats_w);
                new_focus = self.stats(ui, map, snap);
            });
        });
        if let Some(f) = new_focus { self.focus = f; self.auto_pending = false; }
        step
    }

    fn draw_overlays(&self, painter: &Painter, map: &SkyMap, snap: &FrameSnapshot, to_screen: &dyn Fn(f64, f64) -> Pos2) {
        let grid = Stroke::new(1.0, Color32::from_white_alpha(28));
        let rim = Stroke::new(1.2, Color32::from_white_alpha(90));
        // footprint boundary: the unit circle in normalised map coordinates (an ellipse on screen for Mollweide)
        let ring: Vec<Pos2> = (0..=128).map(|i| { let a = i as f64 / 128.0 * std::f64::consts::TAU; to_screen(a.cos(), a.sin()) }).collect();
        painter.add(egui::Shape::line(ring, rim));
        match map.proj {
            SkyProjection::Mollweide => {
                for lat in [-60.0f64, -30.0, 0.0, 30.0, 60.0] {
                    let pts = (0..=72).map(|i| lonlat(-180.0 + 5.0 * i as f64, lat));
                    polyline(painter, map, to_screen, pts, grid);
                }
                for lon in [-120.0f64, -60.0, 0.0, 60.0, 120.0] {
                    let pts = (0..=36).map(|i| lonlat(lon, -90.0 + 5.0 * i as f64));
                    polyline(painter, map, to_screen, pts, grid);
                }
            }
            SkyProjection::Zoom { c, e1, e2, half_fov } => {
                for f in [0.25f64, 0.5, 0.75] {
                    let r = half_fov * f;
                    let pts = (0..=90).map(|i| { let p = i as f64 / 90.0 * std::f64::consts::TAU; ring_dir(&c, &e1, &e2, r, p) });
                    polyline(painter, map, to_screen, pts, grid);
                    painter.text(to_screen(0.0, f), Align2::CENTER_BOTTOM, format!("{:.3} deg", r.to_degrees()), FontId::monospace(9.0), Color32::from_white_alpha(80));
                }
                painter.line_segment([to_screen(-1.0, 0.0), to_screen(1.0, 0.0)], grid);
                painter.line_segment([to_screen(0.0, -1.0), to_screen(0.0, 1.0)], grid);
            }
        }
        if self.discs {
            for d in &map.discs {
                if d.interior || d.theta >= std::f64::consts::PI - 1e-9 { continue; }
                let hot = map.focus == Some(d.j);
                let col = if hot { Color32::from_rgb(255, 225, 90) } else { Color32::from_white_alpha(150) };
                let (u, v) = tangent_basis(&d.centre);
                let pts = (0..=120).map(|i| { let p = i as f64 / 120.0 * std::f64::consts::TAU; ring_dir(&d.centre, &u, &v, d.theta, p) });
                polyline(painter, map, to_screen, pts, Stroke::new(if hot { 1.4 } else { 1.0 }, col));
                if let Some((x, y)) = map.project(&d.centre) {
                    painter.text(to_screen(x, y) + Vec2::new(5.0, -5.0), Align2::LEFT_BOTTOM, d.j.to_string(), FontId::monospace(10.0), col);
                }
            }
        }
        if self.force_dirs {
            if let Some(b) = snap.bodies.get(map.k) {
                let mark = |f: &V3, kind: usize| {
                    let m = norm(f);
                    if !(m > 0.0) { return; }
                    if let Some((x, y)) = map.project(&normalize(f)) {
                        let p = to_screen(x, y);
                        let col = COMP_COLORS[if kind == 0 { 1 } else if kind == 1 { 0 } else { 2 }];
                        let st = Stroke::new(1.6, col);
                        match kind {
                            0 => { painter.circle_stroke(p, 6.0, st); }
                            1 => {
                                painter.line_segment([p + Vec2::new(-5.0, -5.0), p + Vec2::new(5.0, 5.0)], st);
                                painter.line_segment([p + Vec2::new(-5.0, 5.0), p + Vec2::new(5.0, -5.0)], st);
                            }
                            _ => {
                                painter.line_segment([p + Vec2::new(-6.0, 0.0), p + Vec2::new(6.0, 0.0)], st);
                                painter.line_segment([p + Vec2::new(0.0, -6.0), p + Vec2::new(0.0, 6.0)], st);
                            }
                        }
                    }
                };
                mark(&b.f_pair, 0);
                mark(&b.f_total, 1);
                mark(&b.f_corr, 2);
            }
        }
    }

    fn draw_colorbar(&self, ui: &mut egui::Ui, width: f32) {
        let (resp, p) = ui.allocate_painter(Vec2::new(width, 26.0), Sense::hover());
        let r = resp.rect;
        let bar = Rect::from_min_size(r.min, Vec2::new(width, 9.0));
        let n = 96;
        for i in 0..n {
            let t = i as f32 / (n - 1) as f32;
            let col = match self.mode { SkyMode::Deficit => colormap(t), SkyMode::NonAdditive => colormap_magma(t) };
            let x0 = bar.left() + width * i as f32 / n as f32;
            let x1 = bar.left() + width * (i + 1) as f32 / n as f32 + 0.5;
            p.rect_filled(Rect::from_min_max(Pos2::new(x0, bar.top()), Pos2::new(x1, bar.bottom())), 0.0, col);
        }
        let f = FontId::monospace(10.0);
        let (lo, hi, cap) = if self.vmax <= 0.0 {
            ("".to_string(), "".to_string(), match self.mode {
                SkyMode::Deficit => "no signal: D = 0 everywhere in this view".to_string(),
                SkyMode::NonAdditive => "no overlap in this view: D^NA = 0 everywhere".to_string() })
        } else {
            let lo = if self.log { format!("{:.1e}", self.vmax * 10f64.powf(-LOG_DECADES)) } else { "0".to_string() };
            let cap = match (self.mode, self.log) {
                (SkyMode::Deficit, false) => "D (linear)", (SkyMode::Deficit, true) => "D (log, 6 decades)",
                (SkyMode::NonAdditive, false) => "|D^NA|, D^NA <= 0 (linear)", (SkyMode::NonAdditive, true) => "|D^NA|, D^NA <= 0 (log, 6 decades)",
            };
            (lo, format!("{:.3e}", self.vmax), cap.to_string())
        };
        let y = bar.bottom() + 2.0;
        p.text(Pos2::new(r.left(), y), Align2::LEFT_TOP, lo, f.clone(), Color32::GRAY);
        p.text(Pos2::new(r.right(), y), Align2::RIGHT_TOP, hi, f.clone(), Color32::GRAY);
        p.text(Pos2::new(r.center().x, y), Align2::CENTER_TOP, cap, f, Color32::from_gray(190));
    }

    /// Statistics column. Returns a new framing if a disc number was clicked.
    fn stats(&self, ui: &mut egui::Ui, map: &SkyMap, snap: &FrameSnapshot) -> Option<SkyFocus> {
        let mut out = None;
        ui.label(RichText::new(format!("body {} sees {} occluder{}", map.k, map.n_occluders, if map.n_occluders == 1 { "" } else { "s" })).strong());
        egui::Grid::new("sky_stats").num_columns(2).spacing([10.0, 2.0]).show(ui, |ui| {
            let row = |ui: &mut egui::Ui, k: &str, v: String| { ui.label(RichText::new(k).weak()); ui.monospace(v); ui.end_row(); };
            let view = match map.proj {
                SkyProjection::Mollweide => "all sky, Mollweide (equal area)".to_string(),
                SkyProjection::Zoom { half_fov, .. } => format!("zoom on disc {}, half FoV {:.4} deg", map.focus.unwrap_or(0), half_fov.to_degrees()),
            };
            row(ui, "view", view);
            row(ui, "grid", format!("{} x {}", map.nlon, map.nlat));
            let dir = { let n = map.d_max_dir; format!("lon {:.2}, lat {:.2} deg", n[1].atan2(n[0]).to_degrees(), n[2].clamp(-1.0, 1.0).asin().to_degrees()) };
            row(ui, "D max (in view)", format!("{}  at {}", sci(map.d_max), dir));
            row(ui, "sky covered (D>0)", if map.covered.is_finite() { format!("{:.3} %", 100.0 * map.covered) } else { "- (all-sky only)".to_string() });
            row(ui, "<D> map mean", sci_or_dash(map.d_bar));
            ui.label(RichText::new("<D> eq. 20").weak()).on_hover_text("D-bar from the instrumented sinks (exact HEALPix/quadrature route). The map mean agrees only to grid precision, and poorly when discs span few pixels: zoom or compare orders of magnitude.");
            ui.monospace(snap.diag.d_bar_eq20.map_or("- (needs instrumented)".to_string(), sci));
            ui.end_row();
            row(ui, "<D^NA> map mean", sci_or_dash(map.na_mean));
            row(ui, "min D^NA (in view)", sci(map.na_min));
        });
        ui.add_space(4.0);
        if !map.discs.is_empty() {
            ui.label(RichText::new("occluders (largest first; click a number to zoom)").small().weak());
            let mut idx: Vec<usize> = (0..map.discs.len()).collect();
            idx.sort_by(|&a, &b| map.discs[b].theta.total_cmp(&map.discs[a].theta));
            egui::Grid::new("sky_discs").striped(true).num_columns(3).spacing([12.0, 2.0]).show(ui, |ui| {
                for h in ["j", "theta (deg)", "distance"] { ui.label(RichText::new(h).strong().small()); }
                ui.end_row();
                for &i in idx.iter().take(6) {
                    let d = &map.discs[i];
                    if ui.small_button(d.j.to_string()).clicked() { out = Some(SkyFocus::Occluder(d.j)); }
                    ui.monospace(if d.interior { "interior".to_string() } else { format!("{:.4}", d.theta.to_degrees()) });
                    ui.monospace(sci(d.dist));
                    ui.end_row();
                }
            });
            if idx.len() > 6 { ui.label(RichText::new(format!("... {} more", idx.len() - 6)).small().weak()); }
        }
        ui.add_space(4.0);
        let mut notes = vec!["lon = atan2(y, x), lat = asin z, +x at the centre of the all-sky map; the zoom keeps lon increasing to the right.".to_string()];
        if snap.params.mode == Mode::V2 { notes.push("V2: static geometry shown; shifted (aberrated) discs are not drawn.".to_string()); }
        if snap.params.thin_fast {
            notes.push("NON-CONFORMING: thin-fast is active — plotted deficit is NOT the force model in use.".to_string());
        }
        notes.push(format!("Quad tier: {}.", snap.params.quad_tier.label()));
        notes.push("Diagnostic map only: live forces come from the HEALPix quadrature (or thin-fast tree), never from this grid.".to_string());
        notes.push("Modes: Deficit D_k(n) vs non-additive support. Zoom a disc to inspect overlap geometry.".to_string());
        for n in notes { ui.label(RichText::new(n).small().weak()); }
        out
    }
}

fn lonlat(lon_deg: f64, lat_deg: f64) -> V3 {
    let (lo, la) = (lon_deg.to_radians(), lat_deg.to_radians());
    [la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()]
}

/// Point at angular distance `r` from unit `c`, azimuth `phi` in the (e1, e2) tangent frame.
fn ring_dir(c: &V3, e1: &V3, e2: &V3, r: f64, phi: f64) -> V3 {
    let (cr, sr) = (r.cos(), r.sin());
    let (cp, sp) = (phi.cos(), phi.sin());
    [cr * c[0] + sr * (cp * e1[0] + sp * e2[0]), cr * c[1] + sr * (cp * e1[1] + sp * e2[1]), cr * c[2] + sr * (cp * e1[2] + sp * e2[2])]
}

fn tangent_basis(c: &V3) -> (V3, V3) {
    let up = if c[2].abs() > 0.999 { [0.0, 1.0, 0.0] } else { [0.0, 0.0, 1.0] };
    let u = normalize(&cross(&up, c));
    let v = cross(c, &u);
    (u, v)
}

fn flush(painter: &Painter, cur: &mut Vec<Pos2>, stroke: Stroke) {
    if cur.len() >= 2 { painter.add(egui::Shape::line(std::mem::take(cur), stroke)); } else { cur.clear(); }
}

/// Draw a curve given as directions; breaks the line where it leaves the footprint or (all-sky) wraps in longitude.
fn polyline(painter: &Painter, map: &SkyMap, to_screen: &dyn Fn(f64, f64) -> Pos2, pts: impl Iterator<Item = V3>, stroke: Stroke) {
    let mollweide = matches!(map.proj, SkyProjection::Mollweide);
    let mut cur: Vec<Pos2> = Vec::new();
    let mut last_lon: Option<f64> = None;
    for n in pts {
        match map.project(&n) {
            Some((x, y)) => {
                if mollweide {
                    let lon = n[1].atan2(n[0]);
                    if let Some(l) = last_lon { if (lon - l).abs() > std::f64::consts::PI { flush(painter, &mut cur, stroke); } }
                    last_lon = Some(lon);
                }
                cur.push(to_screen(x, y));
            }
            None => { flush(painter, &mut cur, stroke); last_lon = None; }
        }
    }
    flush(painter, &mut cur, stroke);
}

/// Colour image for the chosen quantity; returns (image, vmax = max |value|). NaN (outside the footprint) stays transparent,
/// exact zeros are a neutral dark so "no deficit" is distinguishable from "lowest colour".
fn build_image(map: &SkyMap, mode: SkyMode, log: bool) -> (ColorImage, f64) {
    let data = match mode { SkyMode::Deficit => &map.d_total, SkyMode::NonAdditive => &map.d_na };
    let mut vmax = 0.0f64;
    for &v in data.iter() { if v.is_finite() { vmax = vmax.max((v as f64).abs()); } }
    let mut img = ColorImage::new([map.nlon, map.nlat], Color32::TRANSPARENT);
    let bg = Color32::from_rgb(16, 18, 26);
    for (i, &v) in data.iter().enumerate() {
        if !v.is_finite() { continue; }
        let a = (v as f64).abs();
        if vmax <= 0.0 || a <= 0.0 { img.pixels[i] = bg; continue; }
        let t = if log { ((a / vmax).log10() / LOG_DECADES + 1.0).clamp(0.0, 1.0) } else { (a / vmax).clamp(0.0, 1.0) };
        img.pixels[i] = match mode { SkyMode::Deficit => colormap(t as f32), SkyMode::NonAdditive => colormap_magma(t as f32) };
    }
    (img, vmax)
}

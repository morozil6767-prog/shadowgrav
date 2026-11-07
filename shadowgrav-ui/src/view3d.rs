//! 3D view: CPU perspective projection + egui Painter (no GL code, so the prototype has the smallest possible
//! unverified API surface; a wgpu instanced-sphere renderer can replace `show` later without touching anything else).
//! This module reads `FrameSnapshot` only. It contains no physics: it rotates, projects and colours numbers.
use crate::protocol::{Badge, BodyView, FrameSnapshot};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Sense, Stroke, Vec2};
use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum ColorBy { Flat, TauC, Speed, Force }

/// Force components that can be drawn as arrows (indices into `ArrowOpts::on`).
pub const COMP_NAMES: [&str; 5] = ["total", "pair", "corr", "drag", "aberr"];
pub const COMP_LONG: [&str; 5] = [
    "F_total: everything the integrator uses",
    "F_pair: first sum of eq. 4 (independent pair shadows)",
    "F_corr: overlap correction, the non-additive part of the shadow force",
    "F_dr: flux drag (V1/V2 only)",
    "F_ab: aberration terms (V2 only)",
];
pub const COMP_COLORS: [Color32; 5] = [
    Color32::from_rgb(235, 235, 240), Color32::from_rgb(110, 190, 255), Color32::from_rgb(255, 120, 200),
    Color32::from_rgb(255, 170, 60), Color32::from_rgb(150, 235, 120),
];
const COMP_WIDTH: [f32; 5] = [3.0, 1.8, 1.8, 1.8, 1.8];

fn comp(b: &BodyView, c: usize) -> [f64; 3] {
    match c { 0 => b.f_total, 1 => b.f_pair, 2 => b.f_corr, 3 => b.f_dr, _ => b.f_ab }
}

#[derive(Clone, Debug)]
pub struct ArrowOpts {
    pub on: [bool; 5],
    /// false: one common length scale (components are comparable); true: each component scaled to its own maximum.
    pub per_component: bool,
    /// false: only the selected body.
    pub all_bodies: bool,
    pub gain: f32,
}
impl ArrowOpts {
    pub fn new() -> Self { ArrowOpts { on: [true, true, true, false, false], per_component: true, all_bodies: false, gain: 1.0 } }
}

pub struct ViewState {
    pub yaw: f32, pub pitch: f32, pub zoom: f32,
    pub follow: bool, pub center: [f64; 3], pub extent: f64,
    pub color_by: ColorBy, pub arrows: ArrowOpts,
    pub exaggerate: f32, pub min_px: f32, pub trails: bool,
    pub selected: Option<usize>,
    trail_buf: Vec<VecDeque<[f64; 3]>>, last_step: u64,
}
const TRAIL_LEN: usize = 800;
const ARROW_PX: f32 = 70.0;

impl ViewState {
    pub fn new() -> Self {
        ViewState { yaw: 0.0, pitch: 0.0, zoom: 1.0, follow: true, center: [0.0; 3], extent: 1.0,
                    color_by: ColorBy::TauC, arrows: ArrowOpts::new(), exaggerate: 1.0, min_px: 4.0, trails: true,
                    selected: None, trail_buf: Vec::new(), last_step: u64::MAX }
    }
    /// Re-centre on the bodies and choose a half-width that contains them. Does not touch the camera angles.
    pub fn fit(&mut self, snap: &FrameSnapshot) {
        let n = snap.bodies.len().max(1) as f64;
        let mut c = [0.0; 3];
        for b in &snap.bodies { for i in 0..3 { c[i] += b.x[i] / n; } }
        let (mut rmax, mut amax) = (0.0f64, 0.0f64);
        for b in &snap.bodies {
            let d = ((b.x[0] - c[0]).powi(2) + (b.x[1] - c[1]).powi(2) + (b.x[2] - c[2]).powi(2)).sqrt();
            rmax = rmax.max(d); amax = amax.max(b.radius);
        }
        self.center = c; self.zoom = 1.0;
        self.extent = (1.3 * rmax).max(5.0 * amax).max(1e-12);
    }
    /// Camera from the data: if the bodies lie in a coordinate plane look straight at it, otherwise use an oblique view.
    /// (Checks z first, so a collinear set on the x axis gets the XY view.)
    pub fn reset_camera(&mut self, snap: &FrameSnapshot) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for b in &snap.bodies { for i in 0..3 { lo[i] = lo[i].min(b.x[i]); hi[i] = hi[i].max(b.x[i]); } }
        if snap.bodies.is_empty() { self.set_axis(0); return; }
        let tol = 1e-6 * self.extent.max(1e-300);
        let flat = |i: usize| hi[i] - lo[i] <= tol;
        if flat(2) { self.set_axis(0); } else if flat(1) { self.set_axis(1); } else if flat(0) { self.set_axis(2); }
        else { self.yaw = 0.5; self.pitch = 0.45; }
    }
    pub fn fit_and_reset(&mut self, snap: &FrameSnapshot) { self.fit(snap); self.reset_camera(snap); }
    /// 0: XY (looking down -z), 1: XZ (z up), 2: YZ (z right).
    pub fn set_axis(&mut self, a: usize) {
        match a { 0 => { self.yaw = 0.0; self.pitch = 0.0; } 1 => { self.yaw = 0.0; self.pitch = -1.55; } _ => { self.yaw = 1.5707; self.pitch = 0.0; } }
    }
    pub fn clear_trails(&mut self) { self.trail_buf.clear(); self.last_step = u64::MAX; self.selected = None; }
    pub fn push_trails(&mut self, snap: &FrameSnapshot) {
        if snap.step == self.last_step { return; }
        self.last_step = snap.step;
        if self.trail_buf.len() != snap.bodies.len() { self.trail_buf = vec![VecDeque::new(); snap.bodies.len()]; }
        for (b, t) in snap.bodies.iter().zip(self.trail_buf.iter_mut()) {
            if t.len() >= TRAIL_LEN { t.pop_front(); }
            t.push_back(b.x);
        }
    }

    fn rot(&self, p: [f64; 3]) -> [f64; 3] {
        let (sy, cy) = (self.yaw as f64).sin_cos();
        let (sp, cp) = (self.pitch as f64).sin_cos();
        let x1 = cy * p[0] + sy * p[2];
        let z1 = -sy * p[0] + cy * p[2];
        [x1, cp * p[1] - sp * z1, sp * p[1] + cp * z1]
    }

    pub fn show(&mut self, ui: &mut egui::Ui, snap: &FrameSnapshot) {
        let size = ui.available_size();
        let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        let rect = resp.rect;
        painter.rect_filled(rect, 0.0, Color32::from_rgb(10, 12, 18));

        if resp.dragged() {
            let d = resp.drag_delta();
            self.yaw += d.x * 0.01;
            self.pitch = (self.pitch + d.y * 0.01).clamp(-1.55, 1.55);
        }
        if resp.hovered() {
            let sc = ui.input(|i| i.raw_scroll_delta.y);
            if sc != 0.0 { self.zoom = (self.zoom * (1.0 - sc * 0.002)).clamp(1e-4, 1e4); }
        }
        if resp.double_clicked() { self.fit(snap); }
        if self.follow && !snap.bodies.is_empty() {
            let n = snap.bodies.len() as f64;
            let mut c = [0.0; 3];
            for b in &snap.bodies { for i in 0..3 { c[i] += b.x[i] / n; } }
            self.center = c;
        }

        // ---- projection ----
        let half = self.extent * self.zoom as f64;
        let ppu = (rect.width().min(rect.height()) as f64 * 0.5) / half;      // pixels per code unit at the focal plane
        let cam_d = 6.0 * half;
        let c0 = rect.center();
        let project = |s: &ViewState, p: [f64; 3]| -> Option<(Pos2, f64, f64)> {
            let q = s.rot([p[0] - s.center[0], p[1] - s.center[1], p[2] - s.center[2]]);
            let den = cam_d - q[2];
            if den <= 0.05 * cam_d { return None; }
            let persp = cam_d / den;
            Some((Pos2::new(c0.x + (q[0] * ppu * persp) as f32, c0.y - (q[1] * ppu * persp) as f32), persp, q[2]))
        };

        // ---- trails ----
        if self.trails {
            for (i, t) in self.trail_buf.iter().enumerate() {
                let col = self.body_color(snap, i, 1.0, 1.0).gamma_multiply(0.35);
                // Do not join points across the camera plane: a filtered polyline would
                // invent a trail segment between two visible runs separated by a hidden one.
                let mut segment = Vec::new();
                for point in t {
                    if let Some((screen, _, _)) = project(self, *point) {
                        segment.push(screen);
                    } else {
                        if segment.len() >= 2 {
                            painter.add(egui::Shape::line(std::mem::take(&mut segment), Stroke::new(1.0, col)));
                        } else {
                            segment.clear();
                        }
                    }
                }
                if segment.len() >= 2 { painter.add(egui::Shape::line(segment, Stroke::new(1.0, col))); }
            }
        }

        // ---- bodies, far to near ----
        let vmax = snap.bodies.iter().map(|b| norm3(&b.v)).fold(0.0, f64::max);
        let fmax = snap.bodies.iter().map(|b| norm3(&b.f_total)).fold(0.0, f64::max);
        struct P { i: usize, pos: Pos2, r: f32, z: f64, view: [f64; 3] }
        let mut list: Vec<P> = Vec::with_capacity(snap.bodies.len());
        for (i, b) in snap.bodies.iter().enumerate() {
            if let Some((pos, persp, z)) = project(self, b.x) {
                let r_true = (b.radius * ppu * persp * self.exaggerate as f64) as f32;
                let r = if b.is_probe { 2.5 } else { r_true.max(self.min_px) };
                let view = self.rot([b.x[0] - self.center[0], b.x[1] - self.center[1], b.x[2] - self.center[2]]);
                list.push(P { i, pos, r, z, view });
            }
        }
        list.sort_by(|a, b| a.z.total_cmp(&b.z));
        for p in &list {
            let b = &snap.bodies[p.i];
            let t_speed = if vmax > 0.0 { norm3(&b.v) / vmax } else { 0.0 };
            let t_force = if fmax > 0.0 { norm3(&b.f_total) / fmax } else { 0.0 };
            let col = self.body_color(snap, p.i, t_speed, t_force);
            painter.circle_filled(p.pos, p.r, col);
            painter.circle_filled(Pos2::new(p.pos.x - 0.3 * p.r, p.pos.y - 0.3 * p.r), 0.35 * p.r, Color32::from_white_alpha(70));
            painter.circle_stroke(p.pos, p.r, Stroke::new(1.0, Color32::from_white_alpha(90)));
            if self.selected == Some(p.i) { painter.circle_stroke(p.pos, p.r + 4.0, Stroke::new(1.5, Color32::YELLOW)); }
            if snap.bodies.len() <= 12 && !b.is_probe {
                painter.text(p.pos + Vec2::new(p.r + 3.0, -p.r - 3.0), Align2::LEFT_BOTTOM, p.i.to_string(),
                             FontId::monospace(10.0), Color32::from_gray(150));
            }
        }

        // ---- force arrows (any subset of components, selected body or all) ----
        let any_arrow = self.arrows.on.iter().any(|&o| o);
        if any_arrow {
            let drawn: Vec<&P> = list.iter().filter(|p| self.arrows.all_bodies || self.selected == Some(p.i)).collect();
            let mut mags = [0.0f64; 5];
            for p in &drawn {
                for c in 0..5 { if self.arrows.on[c] { mags[c] = mags[c].max(norm3(&comp(&snap.bodies[p.i], c))); } }
            }
            let common = mags.iter().cloned().fold(0.0, f64::max);
            let mut legend: Vec<(usize, f64)> = Vec::new();
            for c in 0..5 {
                if !self.arrows.on[c] { continue; }
                let m = if self.arrows.per_component { mags[c] } else { common };
                legend.push((c, mags[c]));
                if !(m > 0.0) { continue; }
                let scale = ARROW_PX * self.arrows.gain / m as f32;
                let col = COMP_COLORS[c];
                for p in &drawn {
                    let f = comp(&snap.bodies[p.i], c);
                    let mag = norm3(&f) as f32;
                    if mag <= 0.0 { continue; }
                    let q = self.rot(f);
                    // Differential of the same perspective projection used for positions.
                    // It preserves direction/foreshortening even for bodies far from the focal plane.
                    let den = cam_d - p.view[2];
                    let persp = cam_d / den;
                    let inv_den = cam_d / (den * den);
                    let dx = persp * q[0] + p.view[0] * inv_den * q[2];
                    let dy = -(persp * q[1] + p.view[1] * inv_den * q[2]);
                    let v = Vec2::new(dx as f32, dy as f32) * scale;
                    if v.length() >= 3.0 {
                        painter.arrow(p.pos, v, Stroke::new(COMP_WIDTH[c], col));
                    } else if mag * scale >= 3.0 {
                        // (almost) along the line of sight: dot = towards the viewer, cross = away
                        painter.circle_stroke(p.pos, 5.0, Stroke::new(1.5, col));
                        if q[2] > 0.0 { painter.circle_filled(p.pos, 1.8, col); }
                        else {
                            painter.line_segment([p.pos + Vec2::new(-3.5, -3.5), p.pos + Vec2::new(3.5, 3.5)], Stroke::new(1.3, col));
                            painter.line_segment([p.pos + Vec2::new(-3.5, 3.5), p.pos + Vec2::new(3.5, -3.5)], Stroke::new(1.3, col));
                        }
                    }
                }
            }
            // legend, top right
            let mut y = rect.top() + 8.0;
            let x = rect.right() - 10.0;
            let head = if self.arrows.per_component { "arrows: each component to its own max".to_string() }
                       else { format!("arrows: common scale, {:.0} px = {:.3e}", ARROW_PX * self.arrows.gain, common) };
            painter.text(Pos2::new(x, y), Align2::RIGHT_TOP, head, FontId::monospace(10.5), Color32::GRAY);
            y += 15.0;
            if !self.arrows.all_bodies && self.selected.is_none() {
                painter.text(Pos2::new(x, y), Align2::RIGHT_TOP, "click a body to show its arrows", FontId::monospace(10.5), Color32::from_gray(190));
                y += 15.0;
            }
            for (c, mg) in legend {
                let txt = if mg > 0.0 { format!("{}  max {:.3e}", COMP_NAMES[c], mg) } else { format!("{}  = 0", COMP_NAMES[c]) };
                let r = painter.text(Pos2::new(x, y), Align2::RIGHT_TOP, txt, FontId::monospace(10.5), COMP_COLORS[c]);
                painter.line_segment([Pos2::new(r.left() - 22.0, y + 6.0), Pos2::new(r.left() - 6.0, y + 6.0)], Stroke::new(COMP_WIDTH[c], COMP_COLORS[c]));
                y += 14.0;
            }
        }

        // ---- picking + hover read-out ----
        let nearest = |m: Pos2| -> Option<(f32, usize)> {
            let mut best: Option<(f32, usize)> = None;
            for p in &list {
                let d = (p.pos - m).length();
                if d <= p.r.max(10.0) + 4.0 && best.map_or(true, |(bd, _)| d < bd) { best = Some((d, p.i)); }
            }
            best
        };
        if resp.clicked() {
            if let Some(m) = resp.interact_pointer_pos() { self.selected = nearest(m).map(|b| b.1); }
        }
        if let Some(m) = resp.hover_pos() {
            if !resp.dragged() {
                if let Some((_, i)) = nearest(m) {
                    let b = &snap.bodies[i];
                    let txt = format!(
                        "body {}{}\ntau_c {:.3e}   S {:.6}\n|v| {:.3e}\n|F| total {:.3e}\n  pair {:.3e}   corr {:.3e}\n  drag {:.3e}   aberr {:.3e}",
                        i, if b.is_probe { " (probe)" } else { "" }, b.tau_c, b.s, norm3(&b.v), norm3(&b.f_total),
                        norm3(&b.f_pair), norm3(&b.f_corr), norm3(&b.f_dr), norm3(&b.f_ab));
                    let galley = painter.layout_no_wrap(txt, FontId::monospace(10.5), Color32::from_gray(225));
                    let mut pos = m + Vec2::new(14.0, 14.0);
                    if pos.x + galley.rect.width() + 10.0 > rect.right() { pos.x = m.x - galley.rect.width() - 14.0; }
                    if pos.y + galley.rect.height() + 10.0 > rect.bottom() { pos.y = m.y - galley.rect.height() - 14.0; }
                    let bg = egui::Rect::from_min_size(pos, galley.rect.size()).expand(5.0);
                    painter.rect_filled(bg, 4.0, Color32::from_rgba_unmultiplied(20, 24, 34, 235));
                    painter.rect_stroke(bg, 4.0, Stroke::new(1.0, Color32::from_gray(70)));
                    painter.galley(pos, galley, Color32::from_gray(225));
                }
            }
        }

        // ---- overlays: axes triad, scale bar, legend, conformance stamp ----
        let o = Pos2::new(rect.left() + 48.0, rect.bottom() - 48.0);
        for (k, (lab, col)) in [("x", Color32::from_rgb(230, 90, 90)), ("y", Color32::from_rgb(90, 220, 110)), ("z", Color32::from_rgb(100, 140, 255))].iter().enumerate() {
            let mut u = [0.0; 3]; u[k] = 1.0;
            let q = self.rot(u);
            let e = o + Vec2::new(q[0] as f32, -q[1] as f32) * 32.0;
            painter.line_segment([o, e], Stroke::new(1.5, *col));
            painter.text(e, Align2::CENTER_CENTER, *lab, FontId::monospace(11.0), *col);
        }
        let target = half * 0.5;
        let dec = 10f64.powf(target.log10().floor());
        let len = [1.0, 2.0, 5.0].iter().map(|m| m * dec).filter(|l| *l <= target).fold(dec, f64::max);
        let x0 = rect.right() - 30.0 - (len * ppu) as f32;
        let y0 = rect.bottom() - 24.0;
        painter.line_segment([Pos2::new(x0, y0), Pos2::new(x0 + (len * ppu) as f32, y0)], Stroke::new(2.0, Color32::LIGHT_GRAY));
        painter.text(Pos2::new(x0, y0 - 4.0), Align2::LEFT_BOTTOM, format!("{:.0e} length units", len), FontId::monospace(11.0), Color32::LIGHT_GRAY);
        let legend = match self.color_by {
            ColorBy::TauC => "colour: log10 tau_c, -8 .. 2", ColorBy::Speed => "colour: |v| / max|v|",
            ColorBy::Force => "colour: |F| / max|F|", ColorBy::Flat => "colour: flat",
        };
        painter.text(rect.left_top() + Vec2::new(8.0, 6.0), Align2::LEFT_TOP, legend, FontId::monospace(11.0), Color32::GRAY);
        painter.text(rect.left_top() + Vec2::new(8.0, 20.0), Align2::LEFT_TOP,
            "drag: rotate   wheel: zoom   dbl-click: fit   click: select   keys: 1 2 3 views, F fit",
            FontId::monospace(10.0), Color32::from_gray(90));
        // conformance stamp: anyone looking at a screenshot of this view can see the status
        if snap.badge != Badge::Green {
            let (col, txt) = match snap.badge {
                Badge::Red => (Color32::from_rgb(235, 70, 60), "NON-CONFORMING"),
                _ => (Color32::from_rgb(240, 180, 40), "CAVEATS APPLY"),
            };
            painter.text(Pos2::new(rect.center().x, rect.top() + 8.0), Align2::CENTER_TOP, txt, FontId::monospace(12.0), col);
        }
    }

    fn body_color(&self, snap: &FrameSnapshot, i: usize, t_speed: f64, t_force: f64) -> Color32 {
        let b = &snap.bodies[i];
        if b.is_probe { return Color32::from_rgb(255, 170, 60); }
        let t = match self.color_by {
            ColorBy::Flat => return Color32::from_rgb(120, 170, 255),
            ColorBy::TauC => ((b.tau_c.max(1e-300).log10() + 8.0) / 10.0).clamp(0.0, 1.0),
            ColorBy::Speed => t_speed.clamp(0.0, 1.0),
            ColorBy::Force => t_force.clamp(0.0, 1.0),
        };
        colormap(t as f32)
    }
}

fn norm3(v: &[f64; 3]) -> f64 { (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt() }

/// Five-stop viridis-like gradient.
pub fn colormap(t: f32) -> Color32 {
    const S: [[f32; 3]; 5] = [[68.0, 1.0, 84.0], [59.0, 82.0, 139.0], [33.0, 145.0, 140.0], [94.0, 201.0, 98.0], [253.0, 231.0, 37.0]];
    let x = t.clamp(0.0, 1.0) * 4.0;
    let i = (x.floor() as usize).min(3);
    let f = x - i as f32;
    let c = |k: usize| S[i][k] + (S[i + 1][k] - S[i][k]) * f;
    Color32::from_rgb(c(0) as u8, c(1) as u8, c(2) as u8)
}

/// Five-stop magma-like gradient (used for the non-additive map so it cannot be mistaken for the deficit map).
pub fn colormap_magma(t: f32) -> Color32 {
    const S: [[f32; 3]; 5] = [[0.0, 0.0, 4.0], [81.0, 18.0, 124.0], [183.0, 55.0, 121.0], [252.0, 137.0, 97.0], [252.0, 253.0, 191.0]];
    let x = t.clamp(0.0, 1.0) * 4.0;
    let i = (x.floor() as usize).min(3);
    let f = x - i as f32;
    let c = |k: usize| S[i][k] + (S[i + 1][k] - S[i][k]) * f;
    Color32::from_rgb(c(0) as u8, c(1) as u8, c(2) as u8)
}

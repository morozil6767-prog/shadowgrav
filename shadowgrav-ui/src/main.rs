//! shadowgrav-ui: native research front end for shadowgrav-core.
//!   main.rs      window + app wiring
//!   protocol.rs  Command / FrameSnapshot (the only types crossing the thread boundary)
//!   sim.rs       simulation thread (owns World; applies commands between steps; publishes snapshots)
//!   presets.rs   standard test problems (a)-(d)
//!   view3d.rs    projection + painting (read-only on snapshots)
//!   skyview.rs   sky-map panel: deficit D_k(n) and non-additive part for the selected body (read-only on snapshots)
//!   app.rs       panels, controls, diagnostics read-outs, conformance badge display
mod app;
mod export;
mod presets;
mod protocol;
mod sim;
mod skyview;
mod view3d;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1500.0, 900.0])
            .with_title("shadowgrav — non-additive shadow-gravity research instrument"),
        ..Default::default()
    };
    eframe::run_native(
        "shadowgrav",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}

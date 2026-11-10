pub mod deficit;
pub mod derive;
pub mod force;
pub mod geom;
pub mod gl;
pub mod healpix;
pub mod mathx;
pub mod overlap;
pub mod pair;
pub mod reference;
pub mod skymap;
pub mod sum;
pub mod thin_fast;
pub mod types;
pub mod velocity;
// ---- Prompt 2.3 layer (calls the force core, never modifies it) ----
pub mod diagnostics;
pub mod driver;
pub mod integrator;
pub mod scenario;
pub mod state;

pub use derive::derive;
pub use force::evaluate_forces;
pub use integrator::{advance, advance_by, StepStats};
pub use state::{RunFlags, SimConfig, SimError, StepControl, World};
pub use types::*;

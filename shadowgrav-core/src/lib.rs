pub mod mathx;
pub mod sum; pub mod geom; pub mod gl; pub mod healpix; pub mod types;
pub mod derive; pub mod deficit; pub mod pair; pub mod velocity;
pub mod overlap; pub mod force;
pub mod thin_fast;
pub mod skymap;
pub mod reference;
// ---- Prompt 2.3 layer (calls the force core, never modifies it) ----
pub mod state; pub mod integrator; pub mod diagnostics; pub mod driver;
pub mod scenario;

pub use force::evaluate_forces; pub use derive::derive; pub use types::*;
pub use state::{World, SimConfig, StepControl, SimError, RunFlags};
pub use integrator::{advance, advance_by, StepStats};

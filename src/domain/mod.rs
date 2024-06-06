mod outcome;
mod refresh;
mod state;
mod trigger;

pub use outcome::*;
pub use refresh::*;
pub use state::*;
pub use trigger::*;

use futures::Future;
use std::pin::Pin;

pub type Unit = ();
pub type Task = Pin<Box<dyn Future<Output = Unit> + Send + Sync>>;

mod outcome;
mod refresh;
mod trigger;

pub use outcome::*;
pub use refresh::*;
pub use trigger::*;

use futures::Future;
use std::pin::Pin;

pub type Unit = ();
pub type Task = Pin<Box<dyn Future<Output = Unit> + Send + Sync>>;

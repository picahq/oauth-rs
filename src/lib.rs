mod algebra;
mod domain;
mod service;

pub mod prelude {
    pub use super::algebra::*;
    pub use super::domain::*;
    pub use super::service::*;

    pub type Unit = ();
}

pub mod types;
pub mod verboser;
pub mod browser;
pub mod scrapper;
pub mod generator;
pub mod db;
pub mod engine;
pub mod vpn;

pub mod prelude{
    pub use crate::engine::run as run;
    pub use crate::types::*;
}
pub mod types;
pub mod verboser;
pub mod browser;
pub mod scraper;
pub mod generator;
pub mod db;
pub mod engine;
pub mod vpn;
pub mod storage;

pub mod prelude{
    pub use crate::engine::run_dispatch;
    pub use crate::types::*;
    pub use crate::storage::Persistence;
}

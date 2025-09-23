pub mod config;
pub mod context;
pub mod engine;
pub mod error;
pub mod file_entry;
pub mod interpolation;
pub mod lifecycle;
pub mod linking;
pub mod logging;
pub mod store;
pub mod utils;
pub mod validation;

pub use error::{Result, StowawayError};

use std::path::Path;

pub struct Stowaway {
    engine: engine::StowawayEngine,
}

impl Stowaway {
    pub fn new() -> Self {
        Self {
            engine: engine::StowawayEngine::new(),
        }
    }

    pub fn run<P: AsRef<Path>>(&self, source: P, target: P, dry_run: bool) -> Result<()> {
        self.engine
            .execute(source.as_ref(), target.as_ref(), dry_run)
    }

    pub fn rollback(&self, hash: &str) -> Result<()> {
        self.engine.rollback(hash)
    }

    pub fn list_generations(&self) -> Result<()> {
        self.engine.list_generations()
    }

    pub fn unstow(&self, dry_run: bool) -> Result<()> {
        self.engine.unstow(dry_run)
    }
}

impl Default for Stowaway {
    fn default() -> Self {
        Self::new()
    }
}

use crate::context::StowawayContext;
use crate::error::{Result};

pub mod interpolate;
pub mod link;
pub mod scan;
pub mod validate;

pub trait LifecyclePhase {
    fn execute(&self, context: &mut StowawayContext) -> Result<()>;
}

use dyn_clone::DynClone;
use std::fmt;

use crate::core::Block;

/// A trait to allow other elements to push
/// values into them.
pub trait Entry: fmt::Debug + DynClone + Send {
    /// Pushes a block into this element on port `port`
    fn push(&mut self, block: Block, port: usize);
}
dyn_clone::clone_trait_object!(Entry);

use std::marker::PhantomData;
use std::num::NonZeroU32;

pub use destack_language_arena::{StringId, StringPool};

/// Unique identifier for nodes in an arena, parameterized by node type.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct NodeId<T> {
    idx: NonZeroU32,
    _ty: PhantomData<fn() -> T>,
}

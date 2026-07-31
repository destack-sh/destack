use destack_core::{Optional, SectionEntry};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Program identities assigned to one native object-local function.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Function {
    /// Program function id.
    pub id: u32,
    /// First Program counter id owned by this function.
    pub counter: u32,
    /// First Program sampler id owned by this function.
    pub sampler: u32,
}

impl Function {
    /// Create one linked native function identity.
    pub const fn new(id: u32, counter: u32, sampler: u32) -> Self {
        Self {
            id,
            counter,
            sampler,
        }
    }
}

/// Process-local mappings from native object-local indices to Program identities.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Module {
    /// Program function identities keyed by native object-local function index.
    pub functions: *const Function,
    /// Optional Program type ids keyed by object-local type index.
    pub types: *const Optional<u32>,
    /// Optional Program layout ids keyed by object-local type index.
    pub layouts: *const Optional<u32>,
    /// Program global ids keyed by object-local global index.
    pub globals: *const u32,
    /// Program dynamic-table ids keyed by object-local dispatch-table index.
    pub dynamics: *const u32,
    /// First Program allocation-site id owned by this module.
    pub allocation: u32,
    /// First native frame-map id owned by this module.
    pub frame_map: u32,
}

use destack_source::{ModuleId, ModuleVersion};
use indexmap::IndexMap;
use {destack_dir as dir, destack_machine as machine, destack_mir as mir};

use crate::{ProfileId, ProfileVersion};

/// Output produced by executing a comptime slot.
#[derive(Debug, Clone, PartialEq)]
pub struct ComptimeOutput {
    /// Machine value produced during execution.
    pub machine: Option<machine::Value>,
    /// Static expression produced for DIR patching.
    pub dir: Option<dir::StaticExpression>,
    /// MIR constant derived from the comptime result.
    pub mir: Option<mir::Constant>,
}

impl ComptimeOutput {
    /// Create an empty comptime output with deliberately void data.
    pub fn empty() -> Self {
        Self {
            machine: None,
            dir: None,
            mir: None,
        }
    }
}

/// Comptime results for a module/profile pair.
#[derive(Debug, Clone)]
pub struct ModuleComptime {
    /// The module id.
    pub module_id: ModuleId,
    /// The profile id.
    pub profile_id: ProfileId,
    /// The module version for this comptime result.
    pub module_version: ModuleVersion,
    /// The profile version for this comptime result.
    pub profile_version: ProfileVersion,
    /// Results keyed by comptime expression node id.
    pub results: IndexMap<dir::LocalNodeIdAny, Option<ComptimeOutput>>,
}

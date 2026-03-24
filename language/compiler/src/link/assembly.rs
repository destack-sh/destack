use destack_artifact::{OutputFile, PackageAssembly};

use super::binary::BinaryLinkPlan;
use super::script::ScriptLinkPlan;

/// One assembled target before final package output grouping.
#[derive(Debug, Clone)]
pub(crate) enum TargetAssembly {
    /// One script target assembly.
    Script(ScriptTargetAssembly),
    /// One binary target assembly.
    Binary(BinaryTargetAssembly),
}

/// One assembled script target.
#[derive(Debug, Clone)]
pub(crate) struct ScriptTargetAssembly {
    /// The package assembly mode for this target.
    pub(crate) assembly: PackageAssembly,
    /// The assembled output files for this target.
    pub(crate) output_files: Vec<OutputFile>,
    /// The optional script link plan metadata for this target.
    pub(crate) script_link_plan: Option<ScriptLinkPlan>,
}

/// One assembled binary target.
#[derive(Debug, Clone)]
pub(crate) struct BinaryTargetAssembly {
    /// The package assembly mode for this target.
    pub(crate) assembly: PackageAssembly,
    /// The assembled output files for this target.
    pub(crate) output_files: Vec<OutputFile>,
    /// The optional binary link plan metadata for this target.
    pub(crate) binary_link_plan: Option<BinaryLinkPlan>,
}

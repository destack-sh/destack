use std::path::PathBuf;

use serde::{Deserialize, Serialize};
pub use tspp_artifact::DiagnosticPolicy;

use crate::config::target::{EsTarget, JsModuleFormat};

/// Normalized TS++ compiler options.
///
/// `.tspp` semantics are always strict; these options only describe project,
/// build, interop, and compile-time policy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct CompilerOptions {
    // module & target
    /// JavaScript module format for output.
    pub module: JsModuleFormat,
    /// ECMAScript target version.
    #[serde(rename = "target")]
    pub es_target: EsTarget,
    /// Default profile.
    pub profile: Option<String>,
    /// Default active source graph modes.
    pub modes: Vec<String>,
    /// Default active source graph roles.
    pub roles: Vec<String>,
    /// Default active source graph features.
    pub features: Vec<String>,
    /// Default active source graph tags.
    pub tags: Vec<String>,
    /// Const evaluation environment whitelist (if omitted, all env keys are visible).
    pub const_env: Option<Vec<String>>,
    /// Default tree tag builder provider.
    pub tree: Option<String>,
    /// Global provider modules added to every target profile.
    pub globals: Vec<PathBuf>,
    /// Well-known derives automatically considered for nominal declarations.
    pub derive: Vec<Derive>,

    /// Static semantic restrictions.
    pub restrictions: CompilerRestrictions,

    // emit
    /// Root directory of source files (controls output directory structure, not module resolution).
    pub root_dir: Option<PathBuf>,
    /// Output directory for compiled files.
    pub out_dir: Option<PathBuf>,
    /// Do not emit output files.
    pub no_emit: bool,
}

/// Well-known interface selected for automatic derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "PascalCase")]
pub enum Derive {
    /// Derive `Compare`.
    Compare,
    /// Derive `Copy`.
    Copy,
    /// Derive `Clone`.
    Clone,
    /// Derive `Debug`.
    Debug,
    /// Derive `Default`.
    Default,
    /// Derive `Deserialize`.
    Deserialize,
    /// Derive `Equal`.
    Equal,
    /// Derive `Hash`.
    Hash,
    /// Derive `PartialCompare`.
    PartialCompare,
    /// Derive `PartialEqual`.
    PartialEqual,
    /// Derive `Serialize`.
    Serialize,
}

impl Derive {
    /// Return the canonical profile key spelling.
    pub fn key(self) -> &'static str {
        match self {
            Self::Compare => "Compare",
            Self::Copy => "Copy",
            Self::Clone => "Clone",
            Self::Debug => "Debug",
            Self::Default => "Default",
            Self::Deserialize => "Deserialize",
            Self::Equal => "Equal",
            Self::Hash => "Hash",
            Self::PartialCompare => "PartialCompare",
            Self::PartialEqual => "PartialEqual",
            Self::Serialize => "Serialize",
        }
    }
}

impl CompilerOptions {
    /// Enable heap-free restrictions.
    pub fn apply_no_heap_restrictions(&mut self) {
        self.restrictions.no_managed = self.restrictions.no_managed.max(self.restrictions.no_heap);
    }

    /// Enable runtime-free restrictions for compile-time only targets.
    pub fn apply_no_runtime_restrictions(&mut self) {
        self.restrictions.no_runtime = DiagnosticPolicy::Deny;
        self.restrictions.no_heap = DiagnosticPolicy::Deny;
        self.restrictions.no_managed = DiagnosticPolicy::Deny;
        self.restrictions.no_dynamic_dispatch = DiagnosticPolicy::Deny;
    }
}

/// Static semantic restrictions enforced by the compiler.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct CompilerRestrictions {
    /// Policy for managed values and managed allocation.
    pub no_managed: DiagnosticPolicy,
    /// Policy for all heap allocation.
    pub no_heap: DiagnosticPolicy,
    /// Policy for runtime-dependent language features.
    pub no_runtime: DiagnosticPolicy,
    /// Policy for unsafe operations.
    pub no_unsafe: DiagnosticPolicy,
    /// Policy for calls that cannot be statically resolved.
    pub no_dynamic_dispatch: DiagnosticPolicy,
    /// Policy for runtime reflection and RTTI usage.
    pub no_reflection: DiagnosticPolicy,
    /// Policy for unwinding.
    pub no_unwind: DiagnosticPolicy,
    /// Policy banning aliasing mutable borrows.
    pub no_aliasing_mutable_borrows: DiagnosticPolicy,
    /// Policy banning implicit method receivers.
    pub no_implicit_receivers: DiagnosticPolicy,
}

impl CompilerRestrictions {
    /// Tighten this set with stricter policies from another set.
    pub fn tighten_with(&mut self, other: &Self) {
        self.no_managed = self.no_managed.max(other.no_managed);
        self.no_heap = self.no_heap.max(other.no_heap);
        self.no_runtime = self.no_runtime.max(other.no_runtime);
        self.no_unsafe = self.no_unsafe.max(other.no_unsafe);
        self.no_dynamic_dispatch = self.no_dynamic_dispatch.max(other.no_dynamic_dispatch);
        self.no_reflection = self.no_reflection.max(other.no_reflection);
        self.no_unwind = self.no_unwind.max(other.no_unwind);
        self.no_aliasing_mutable_borrows = self
            .no_aliasing_mutable_borrows
            .max(other.no_aliasing_mutable_borrows);
        self.no_implicit_receivers = self.no_implicit_receivers.max(other.no_implicit_receivers);
    }
}

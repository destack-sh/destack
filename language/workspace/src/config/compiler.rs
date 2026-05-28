use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::target::{EsTarget, JsModuleFormat};

/// Normalized Destack compiler options.
///
/// `.ds` semantics are always strict; these options only describe project,
/// build, interop, and compile-time policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Comptime environment whitelist (if omitted, all env keys are visible).
    pub comptime_env: Option<Vec<String>>,
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
    /// Output directory for declaration files. Defaults to out_dir.
    pub declaration_dir: Option<PathBuf>,
    /// Generate declaration maps for `.d.ts` output.
    pub declaration_map: bool,
    /// Do not emit output files.
    pub no_emit: bool,
    /// Emit checked type annotation sidecars.
    pub emit_checked_types: bool,
}

/// Well-known compiler-owned derive provider.
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
    /// Derive `Tagged`.
    Tagged,
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
            Self::Tagged => "Tagged",
        }
    }
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            module: JsModuleFormat::default(),
            es_target: EsTarget::default(),
            profile: None,
            modes: Vec::new(),
            roles: Vec::new(),
            features: Vec::new(),
            tags: Vec::new(),
            comptime_env: None,
            tree: None,
            globals: Vec::new(),
            derive: Vec::new(),

            restrictions: CompilerRestrictions::default(),

            // emit
            root_dir: None,
            out_dir: None,
            declaration_dir: None,
            declaration_map: false,
            no_emit: false,
            emit_checked_types: false,
        }
    }
}

impl CompilerOptions {
    /// Enable native-only restrictions for native and wasm targets.
    pub fn apply_native_restrictions(&mut self) {
        self.restrictions.no_managed = DiagnosticPolicy::Deny;
    }

    /// Enable heap-free restrictions.
    pub fn apply_no_heap_restrictions(&mut self) {
        if self
            .restrictions
            .no_heap
            .is_stricter_than(self.restrictions.no_managed)
        {
            self.restrictions.no_managed = self.restrictions.no_heap;
        }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// Policy requiring mutable borrows to be exclusive.
    pub exclusive_mutable_borrows: DiagnosticPolicy,
}

impl Default for CompilerRestrictions {
    fn default() -> Self {
        Self {
            no_managed: DiagnosticPolicy::Allow,
            no_heap: DiagnosticPolicy::Allow,
            no_runtime: DiagnosticPolicy::Allow,
            no_unsafe: DiagnosticPolicy::Allow,
            no_dynamic_dispatch: DiagnosticPolicy::Allow,
            no_reflection: DiagnosticPolicy::Allow,
            no_unwind: DiagnosticPolicy::Allow,
            exclusive_mutable_borrows: DiagnosticPolicy::Allow,
        }
    }
}

impl CompilerRestrictions {
    /// Tighten this set with stricter policies from another set.
    pub fn tighten_with(&mut self, other: &Self) {
        self.no_managed = stricter_policy(self.no_managed, other.no_managed);
        self.no_heap = stricter_policy(self.no_heap, other.no_heap);
        self.no_runtime = stricter_policy(self.no_runtime, other.no_runtime);
        self.no_unsafe = stricter_policy(self.no_unsafe, other.no_unsafe);
        self.no_dynamic_dispatch =
            stricter_policy(self.no_dynamic_dispatch, other.no_dynamic_dispatch);
        self.no_reflection = stricter_policy(self.no_reflection, other.no_reflection);
        self.no_unwind = stricter_policy(self.no_unwind, other.no_unwind);
        self.exclusive_mutable_borrows = stricter_policy(
            self.exclusive_mutable_borrows,
            other.exclusive_mutable_borrows,
        );
    }
}

/// Diagnostic policy for allow/warn/deny enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticPolicy {
    /// Allow without diagnostics.
    Allow,
    /// Allow with a warning.
    Warn,
    /// Forbid with an error.
    Deny,
}

impl DiagnosticPolicy {
    /// Return whether this policy is stricter than another.
    pub fn is_stricter_than(self, other: Self) -> bool {
        self.rank() > other.rank()
    }

    /// Return whether this policy is Allow.
    pub fn is_allow(self) -> bool {
        matches!(self, DiagnosticPolicy::Allow)
    }

    /// Return whether this policy is Warn.
    pub fn is_warn(self) -> bool {
        matches!(self, DiagnosticPolicy::Warn)
    }

    /// Return whether this policy is Deny.
    pub fn is_deny(self) -> bool {
        matches!(self, DiagnosticPolicy::Deny)
    }

    /// Return a stable numeric rank for ordering.
    pub fn rank(self) -> u8 {
        match self {
            DiagnosticPolicy::Allow => 0,
            DiagnosticPolicy::Warn => 1,
            DiagnosticPolicy::Deny => 2,
        }
    }
}

/// Return the stricter diagnostic policy.
fn stricter_policy(left: DiagnosticPolicy, right: DiagnosticPolicy) -> DiagnosticPolicy {
    if right.is_stricter_than(left) {
        right
    } else {
        left
    }
}

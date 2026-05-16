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
    /// Default environment.
    pub environment: Option<String>,
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
    /// Derive providers automatically considered for nominal declarations.
    pub derive: Vec<String>,

    // static restrictions
    /// Policy for GC-managed defaults and allocations.
    pub no_managed: DiagnosticPolicy,
    /// Policy for all heap allocation.
    pub no_heap: DiagnosticPolicy,
    /// Policy for runtime usage (no managed memory, no Promise, ...).
    pub no_runtime: DiagnosticPolicy,
    /// Policy for low level internal protocol imports (`platform:`).
    pub no_internal_import: DiagnosticPolicy,
    /// Policy for overloads that are not statically resolvable.
    pub no_implicit_dynamic_dispatch: DiagnosticPolicy,

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
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            module: JsModuleFormat::default(),
            es_target: EsTarget::default(),
            environment: None,
            profile: None,
            modes: Vec::new(),
            roles: Vec::new(),
            features: Vec::new(),
            tags: Vec::new(),
            comptime_env: None,
            tree: None,
            globals: Vec::new(),
            derive: Vec::new(),

            // static restrictions
            no_managed: DiagnosticPolicy::Allow,
            no_heap: DiagnosticPolicy::Allow,
            no_runtime: DiagnosticPolicy::Allow,
            no_internal_import: DiagnosticPolicy::Allow,
            no_implicit_dynamic_dispatch: DiagnosticPolicy::Allow,

            // emit
            root_dir: None,
            out_dir: None,
            declaration_dir: None,
            declaration_map: false,
            no_emit: false,
        }
    }
}

impl CompilerOptions {
    /// Enable native-only restrictions for native and wasm targets.
    pub fn apply_native_restrictions(&mut self) {
        self.no_managed = DiagnosticPolicy::Deny;
    }

    /// Enable heap-free restrictions.
    pub fn apply_no_heap_restrictions(&mut self) {
        if self.no_heap.is_stricter_than(self.no_managed) {
            self.no_managed = self.no_heap;
        }
    }

    /// Enable runtime-free restrictions for compile-time only targets.
    pub fn apply_no_runtime_restrictions(&mut self) {
        // force runtime control flags on when runtime is disabled
        self.no_runtime = DiagnosticPolicy::Deny;
        self.no_heap = DiagnosticPolicy::Deny;
        self.no_managed = DiagnosticPolicy::Deny;
        self.no_implicit_dynamic_dispatch = DiagnosticPolicy::Deny;
    }
}

/// Diagnostic policy for allow/warn/deny enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

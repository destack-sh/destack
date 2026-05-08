use std::path::PathBuf;

use serde::Deserialize;

use crate::config::target::{EsTarget, JsModuleFormat};

/// Normalized Destack compiler options.
///
/// `.ds` semantics are always strict; these options only describe project,
/// build, interop, and capability policy.
#[derive(Debug, Clone)]
pub struct CompilerOptions {
    // module & target
    /// JavaScript module format for output.
    pub module: JsModuleFormat,
    /// ECMAScript target version.
    pub es_target: EsTarget,
    /// Default environment for IDEs and CLI usage.
    pub environment: Option<String>,
    /// Default profile for IDEs and CLI usage.
    pub profile: Option<String>,
    /// Default active source graph modes for IDEs and CLI usage.
    pub modes: Vec<String>,
    /// Comptime environment whitelist (if omitted, all env keys are visible).
    pub comptime_env: Option<Vec<String>>,
    /// Default tree tag builder provider.
    pub tree: Option<String>,
    /// Global provider modules added to every target profile.
    pub globals: Vec<PathBuf>,
    /// Derive providers automatically considered for nominal declarations.
    pub derive: Vec<String>,

    // capability restrictions
    /// Policy for GC-managed defaults and allocations.
    pub no_managed: DiagnosticPolicy,
    /// Policy for all heap allocation.
    pub no_heap: DiagnosticPolicy,
    /// Policy for runtime usage (no managed memory, no Promise, no exceptions, ...).
    pub no_runtime: DiagnosticPolicy,
    /// Policy for low level internal protocol imports (`platform:`).
    pub no_internal_import: DiagnosticPolicy,
    /// Policy for overloads that are not statically resolvable.
    pub no_implicit_dynamic_dispatch: DiagnosticPolicy,
    /// Policy for `throw`.
    pub no_throw: DiagnosticPolicy,

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
            comptime_env: None,
            tree: None,
            globals: Vec::new(),
            derive: Vec::new(),

            // capability restrictions
            no_managed: DiagnosticPolicy::Allow,
            no_heap: DiagnosticPolicy::Allow,
            no_runtime: DiagnosticPolicy::Allow,
            no_internal_import: DiagnosticPolicy::Allow,
            no_implicit_dynamic_dispatch: DiagnosticPolicy::Allow,
            no_throw: DiagnosticPolicy::Allow,

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
        self.no_throw = DiagnosticPolicy::Deny;
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
        self.no_throw = DiagnosticPolicy::Deny;
        self.no_implicit_dynamic_dispatch = DiagnosticPolicy::Deny;
    }
}

/// Diagnostic policy for allow/warn/deny enforcement.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticPolicyValueJson {
    Allow,
    Warn,
    #[serde(alias = "error")]
    Deny,
}

/// Diagnostic policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum DiagnosticPolicyJson {
    Bool(bool),
    Value(DiagnosticPolicyValueJson),
}

impl From<DiagnosticPolicyJson> for DiagnosticPolicy {
    fn from(value: DiagnosticPolicyJson) -> Self {
        match value {
            DiagnosticPolicyJson::Bool(true) => DiagnosticPolicy::Deny,
            DiagnosticPolicyJson::Bool(false) => DiagnosticPolicy::Allow,
            DiagnosticPolicyJson::Value(DiagnosticPolicyValueJson::Allow) => {
                DiagnosticPolicy::Allow
            }
            DiagnosticPolicyJson::Value(DiagnosticPolicyValueJson::Warn) => DiagnosticPolicy::Warn,
            DiagnosticPolicyJson::Value(DiagnosticPolicyValueJson::Deny) => DiagnosticPolicy::Deny,
        }
    }
}

impl DiagnosticPolicyJson {
    /// Convert allow-style policies where `true` means allow.
    pub fn into_allow_policy(self) -> DiagnosticPolicy {
        match self {
            DiagnosticPolicyJson::Bool(true) => DiagnosticPolicy::Allow,
            DiagnosticPolicyJson::Bool(false) => DiagnosticPolicy::Deny,
            DiagnosticPolicyJson::Value(DiagnosticPolicyValueJson::Allow) => {
                DiagnosticPolicy::Allow
            }
            DiagnosticPolicyJson::Value(DiagnosticPolicyValueJson::Warn) => DiagnosticPolicy::Warn,
            DiagnosticPolicyJson::Value(DiagnosticPolicyValueJson::Deny) => DiagnosticPolicy::Deny,
        }
    }
}

/// Diagnostic policy for allow/warn/deny enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Destack configuration compiler options.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CompilerOptionsJson {
    // module & target
    /// Module format for output (e.g., "esnext").
    pub module: Option<String>,
    /// ECMAScript target version (e.g., "es2022", "esnext").
    pub target: Option<String>,
    /// Default environment for IDEs and CLI usage.
    pub environment: Option<String>,
    /// Default profile for IDEs and CLI usage.
    pub profile: Option<String>,
    /// Default active source graph modes for IDEs and CLI usage.
    pub modes: Option<Vec<String>>,
    /// Comptime environment whitelist (if omitted, all env keys are visible).
    pub comptime_env: Option<Vec<String>>,
    /// Default tree tag builder provider.
    pub tree: Option<String>,
    /// Global provider modules added to every target profile.
    pub globals: Option<Vec<String>>,
    /// Derive providers automatically considered for nominal declarations.
    pub derive: Option<Vec<String>>,

    // capability restrictions
    /// Policy for GC-managed defaults and allocations.
    pub no_managed: Option<DiagnosticPolicyJson>,
    /// Policy for all heap allocation.
    pub no_heap: Option<DiagnosticPolicyJson>,
    /// Policy for runtime usage (no managed memory, no Promise, no exceptions, ...).
    pub no_runtime: Option<DiagnosticPolicyJson>,
    /// Policy for low level internal protocol imports (`platform:`).
    pub no_internal_import: Option<DiagnosticPolicyJson>,
    /// Policy for overloads that are not statically resolvable.
    pub no_implicit_dynamic_dispatch: Option<DiagnosticPolicyJson>,
    /// Policy for `throw`.
    pub no_throw: Option<DiagnosticPolicyJson>,

    // emit
    /// Root directory of source files (controls output directory structure, not module resolution).
    pub root_dir: Option<String>,
    /// Output directory for compiled files.
    pub out_dir: Option<String>,
    /// Output directory for declaration files (.d.ts). Defaults to outDir.
    pub declaration_dir: Option<String>,
    /// Generate declaration maps for `.d.ts` output.
    pub declaration_map: Option<bool>,
    /// Do not emit output files.
    pub no_emit: Option<bool>,
}

impl From<&CompilerOptionsJson> for CompilerOptions {
    fn from(json: &CompilerOptionsJson) -> Self {
        let mut options = Self {
            module: json
                .module
                .as_deref()
                .and_then(JsModuleFormat::parse)
                .unwrap_or_default(),
            es_target: json
                .target
                .as_deref()
                .and_then(EsTarget::parse)
                .unwrap_or_default(),
            environment: json.environment.clone(),
            profile: json.profile.clone(),
            modes: json.modes.clone().unwrap_or_default(),
            comptime_env: json.comptime_env.clone(),
            tree: json.tree.clone(),
            globals: json
                .globals
                .as_ref()
                .map(|globals| globals.iter().map(PathBuf::from).collect())
                .unwrap_or_default(),
            derive: json.derive.clone().unwrap_or_default(),

            // capability restrictions
            no_managed: json
                .no_managed
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_heap: json
                .no_heap
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_runtime: json
                .no_runtime
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_internal_import: json
                .no_internal_import
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_implicit_dynamic_dispatch: json
                .no_implicit_dynamic_dispatch
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_throw: json
                .no_throw
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),

            // emit
            root_dir: json.root_dir.as_ref().map(PathBuf::from),
            out_dir: json.out_dir.as_ref().map(PathBuf::from),
            declaration_dir: json.declaration_dir.as_ref().map(PathBuf::from),
            declaration_map: json.declaration_map.unwrap_or(false),
            no_emit: json.no_emit.unwrap_or(false),
        };

        // apply heap-free restrictions when requested
        options.apply_no_heap_restrictions();

        // apply runtime-free restrictions when requested
        if options.no_runtime.is_deny() {
            options.apply_no_runtime_restrictions();
        }

        options
    }
}

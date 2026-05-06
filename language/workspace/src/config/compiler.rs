use std::path::PathBuf;

use indexmap::IndexMap;
use serde::Deserialize;

use crate::config::{EsTarget, ModuleDetection, ModuleResolution, ModuleTarget, TsConfigOptions};

/// Path alias mapping (resolved from Destack config paths).
pub type DsPathAliases = IndexMap<String, Vec<String>>;

/// Normalized Destack compiler options.
///
/// `.ds` semantics are always strict; these options only describe project,
/// build, interop, and capability policy.
#[derive(Debug, Clone)]
pub struct CompilerOptions {
    // module resolution
    /// Base URL for resolving non-relative module names.
    pub base_url: Option<PathBuf>,
    /// Path alias mappings (resolved relative to baseUrl).
    pub paths: Option<DsPathAliases>,

    // module & target
    /// Module format for output.
    pub module: ModuleTarget,
    /// ECMAScript target version.
    pub es_target: EsTarget,
    /// Module resolution strategy.
    pub module_resolution: ModuleResolution,
    /// Use package.json exports field during module resolution.
    pub resolve_package_json_exports: bool,
    /// Use package.json imports field during module resolution.
    pub resolve_package_json_imports: bool,
    /// Custom package export conditions for module resolution.
    pub custom_conditions: Vec<String>,
    /// Package linker strategy for bare module resolution.
    pub node_linker: NodeLinker,
    /// How to detect modules versus scripts.
    pub module_detection: ModuleDetection,
    /// Default environment for IDEs and CLI usage.
    pub environment: Option<String>,
    /// Default profile for IDEs and CLI usage.
    pub profile: Option<String>,
    /// Default mode for IDEs and CLI usage.
    pub mode: Option<String>,
    /// Comptime environment whitelist (if omitted, all env keys are visible).
    pub comptime_env: Option<Vec<String>>,

    // capability restrictions
    /// Policy for GC-managed defaults and allocations.
    pub no_managed: DiagnosticPolicy,
    /// Policy for runtime usage (no managed memory, no Promise, no exceptions, ...).
    pub no_runtime: DiagnosticPolicy,
    /// Policy for referential equality.
    pub no_referential_equality: DiagnosticPolicy,
    /// Policy for `eval()` and `Function` constructor.
    pub no_dynamic_evaluation: DiagnosticPolicy,
    /// Policy for `globalThis` access.
    pub no_global_this: DiagnosticPolicy,
    /// Policy for dynamic `import()` and `require()` expressions.
    pub no_dynamic_import: DiagnosticPolicy,
    /// Policy for low level internal protocol imports (`platform:`).
    pub no_internal_import: DiagnosticPolicy,
    /// Policy for defineProperty, prototype mutation, delete, and declaration expressions.
    pub no_dynamic_shapes: DiagnosticPolicy,
    /// Policy for computed property access `obj[expr]` where expr isn't constant.
    pub no_computed_property_access: DiagnosticPolicy,
    /// Policy for `Proxy`.
    pub no_proxy: DiagnosticPolicy,
    /// Policy for overloads that are not statically resolvable.
    pub no_implicit_dynamic_dispatch: DiagnosticPolicy,
    /// Policy for `throw` and `try`/`catch` (use Result types instead).
    pub no_exceptions: DiagnosticPolicy,

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

    // interop
    /// Path to tsconfig.json to inherit settings from.
    pub tsconfig: Option<PathBuf>,
    /// Allow arbitrary file extensions in import specifiers.
    pub allow_arbitrary_extensions: bool,
    /// Allow TypeScript file extensions in import specifiers.
    pub allow_importing_ts_extensions: bool,
    /// Skip type checking of declaration files.
    pub skip_lib_check: bool,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            base_url: None,
            paths: None,
            module_resolution: ModuleResolution::default(),
            resolve_package_json_exports: true,
            resolve_package_json_imports: true,
            custom_conditions: Vec::new(),
            node_linker: NodeLinker::default(),
            module: ModuleTarget::default(),
            es_target: EsTarget::default(),
            module_detection: ModuleDetection::default(),
            environment: None,
            profile: None,
            mode: None,
            comptime_env: None,

            // capability restrictions
            no_managed: DiagnosticPolicy::Allow,
            no_runtime: DiagnosticPolicy::Allow,
            no_referential_equality: DiagnosticPolicy::Allow,
            no_dynamic_evaluation: DiagnosticPolicy::Allow,
            no_global_this: DiagnosticPolicy::Allow,
            no_dynamic_import: DiagnosticPolicy::Allow,
            no_internal_import: DiagnosticPolicy::Allow,
            no_dynamic_shapes: DiagnosticPolicy::Allow,
            no_computed_property_access: DiagnosticPolicy::Allow,
            no_proxy: DiagnosticPolicy::Allow,
            no_implicit_dynamic_dispatch: DiagnosticPolicy::Allow,
            no_exceptions: DiagnosticPolicy::Allow,

            // emit
            root_dir: None,
            out_dir: None,
            declaration_dir: None,
            declaration_map: false,
            no_emit: false,

            // interop
            tsconfig: None,
            allow_arbitrary_extensions: false,
            allow_importing_ts_extensions: false,
            skip_lib_check: false,
        }
    }
}

impl CompilerOptions {
    /// Apply compiler options from one normalized TypeScript configuration.
    pub fn apply_tsconfig_options(&mut self, tsconfig_options: &TsConfigOptions) {
        let compiler = &tsconfig_options.compiler;

        // module resolution
        self.base_url = compiler.base_url.clone();
        self.paths = compiler.paths.as_ref().map(|paths| {
            let mut mapped_paths = DsPathAliases::default();

            for (key, values) in paths {
                mapped_paths.insert(key.clone(), values.clone());
            }

            mapped_paths
        });

        // language and module semantics
        self.module = compiler.module;
        self.es_target = compiler.es_target;
        self.skip_lib_check = compiler.skip_lib_check;
    }

    /// Enable native-only restrictions for native and wasm targets.
    pub fn apply_native_restrictions(&mut self) {
        self.no_managed = DiagnosticPolicy::Deny;

        // disable runtime features that native backends cannot support
        self.no_dynamic_evaluation = DiagnosticPolicy::Deny;
        self.no_dynamic_import = DiagnosticPolicy::Deny;
        self.no_proxy = DiagnosticPolicy::Deny;
        self.no_dynamic_shapes = DiagnosticPolicy::Deny;
        self.no_exceptions = DiagnosticPolicy::Deny;
        self.no_global_this = DiagnosticPolicy::Deny;
    }

    /// Enable runtime-free restrictions for compile-time only targets.
    pub fn apply_no_runtime_restrictions(&mut self) {
        // force runtime control flags on when runtime is disabled
        self.no_runtime = DiagnosticPolicy::Deny;
        self.no_managed = DiagnosticPolicy::Deny;
        self.no_exceptions = DiagnosticPolicy::Deny;
        self.no_dynamic_evaluation = DiagnosticPolicy::Deny;
        self.no_dynamic_import = DiagnosticPolicy::Deny;
        self.no_dynamic_shapes = DiagnosticPolicy::Deny;
        self.no_computed_property_access = DiagnosticPolicy::Deny;
        self.no_proxy = DiagnosticPolicy::Deny;
        self.no_global_this = DiagnosticPolicy::Deny;
        self.no_implicit_dynamic_dispatch = DiagnosticPolicy::Deny;
    }
}

/// Node package linker mode for module resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeLinker {
    /// Detect linker mode automatically from workspace files.
    #[default]
    Auto,
    /// Resolve packages through node_modules directory traversal.
    NodeModules,
    /// Resolve packages through Yarn Plug'n'Play manifests.
    Pnp,
}

impl NodeLinker {
    /// Parse one node linker value from config text.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "node-modules" | "node_modules" | "nodeModules" => Some(Self::NodeModules),
            "pnp" => Some(Self::Pnp),
            _ => None,
        }
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
    // module resolution
    /// Base URL for resolving non-relative module names.
    pub base_url: Option<String>,
    /// Path alias mappings (resolved relative to baseUrl, like tsconfig paths).
    pub paths: Option<IndexMap<String, Vec<String>>>,

    // module & target
    /// Module format for output (e.g., "esnext", "commonjs").
    pub module: Option<String>,
    /// ECMAScript target version (e.g., "es2022", "esnext").
    pub target: Option<String>,
    /// Module resolution strategy (e.g., "bundler", "node16", "nodenext").
    pub module_resolution: Option<String>,
    /// Use package.json exports field during module resolution.
    pub resolve_package_json_exports: Option<bool>,
    /// Use package.json imports field during module resolution.
    pub resolve_package_json_imports: Option<bool>,
    /// Custom package export conditions for module resolution.
    pub custom_conditions: Option<Vec<String>>,
    /// Package linker strategy for bare module resolution (`auto`, `node-modules`, `pnp`).
    pub node_linker: Option<String>,
    /// How to detect modules versus scripts (e.g., "auto", "force", "legacy").
    pub module_detection: Option<String>,
    /// Default environment for IDEs and CLI usage.
    pub environment: Option<String>,
    /// Default profile for IDEs and CLI usage.
    pub profile: Option<String>,
    /// Default mode for IDEs and CLI usage.
    pub mode: Option<String>,
    /// Comptime environment whitelist (if omitted, all env keys are visible).
    pub comptime_env: Option<Vec<String>>,

    // capability restrictions
    /// Policy for GC-managed defaults and allocations.
    pub no_managed: Option<DiagnosticPolicyJson>,
    /// Policy for runtime usage (no managed memory, no Promise, no exceptions, ...).
    pub no_runtime: Option<DiagnosticPolicyJson>,
    /// Policy for referential equality.
    pub no_referential_equality: Option<DiagnosticPolicyJson>,
    /// Policy for `eval()` and `Function` constructor.
    pub no_dynamic_evaluation: Option<DiagnosticPolicyJson>,
    /// Policy for `globalThis` access.
    pub no_global_this: Option<DiagnosticPolicyJson>,
    /// Policy for dynamic `import()` and `require()` expressions.
    pub no_dynamic_import: Option<DiagnosticPolicyJson>,
    /// Policy for low level internal protocol imports (`platform:`).
    pub no_internal_import: Option<DiagnosticPolicyJson>,
    /// Policy for defineProperty, prototype mutation, delete, and declaration expressions.
    pub no_dynamic_shapes: Option<DiagnosticPolicyJson>,
    /// Policy for computed property access `obj[expr]` where expr isn't constant.
    pub no_computed_property_access: Option<DiagnosticPolicyJson>,
    /// Policy for `Proxy`.
    pub no_proxy: Option<DiagnosticPolicyJson>,
    /// Policy for overloads that are not statically resolvable.
    pub no_implicit_dynamic_dispatch: Option<DiagnosticPolicyJson>,
    /// Policy for `throw` and `try`/`catch` (use Result types instead).
    pub no_exceptions: Option<DiagnosticPolicyJson>,

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

    // interop
    /// Path to tsconfig.json to inherit settings from.
    pub tsconfig: Option<String>,
    /// Allow arbitrary file extensions in import specifiers.
    pub allow_arbitrary_extensions: Option<bool>,
    /// Allow TypeScript file extensions in import specifiers.
    pub allow_importing_ts_extensions: Option<bool>,
    /// Skip type checking of declaration files (.d.ts, .d.ds).
    pub skip_lib_check: Option<bool>,
}

impl From<&CompilerOptionsJson> for CompilerOptions {
    fn from(json: &CompilerOptionsJson) -> Self {
        let module_resolution = json
            .module_resolution
            .as_deref()
            .and_then(ModuleResolution::parse)
            .unwrap_or_default();
        let resolve_package_json_default = matches!(
            module_resolution,
            ModuleResolution::Node16 | ModuleResolution::NodeNext | ModuleResolution::Bundler
        );
        let mut options = Self {
            base_url: json.base_url.as_ref().map(PathBuf::from),
            paths: json.paths.clone(),
            module_resolution,
            allow_arbitrary_extensions: json.allow_arbitrary_extensions.unwrap_or(false),
            allow_importing_ts_extensions: json.allow_importing_ts_extensions.unwrap_or(false),
            resolve_package_json_exports: json
                .resolve_package_json_exports
                .unwrap_or(resolve_package_json_default),
            resolve_package_json_imports: json
                .resolve_package_json_imports
                .unwrap_or(resolve_package_json_default),
            custom_conditions: json.custom_conditions.clone().unwrap_or_default(),
            node_linker: json
                .node_linker
                .as_deref()
                .and_then(NodeLinker::parse)
                .unwrap_or_default(),
            module: json
                .module
                .as_deref()
                .and_then(ModuleTarget::parse)
                .unwrap_or_default(),
            es_target: json
                .target
                .as_deref()
                .and_then(EsTarget::parse)
                .unwrap_or_default(),
            module_detection: json
                .module_detection
                .as_deref()
                .and_then(ModuleDetection::parse)
                .unwrap_or_default(),
            environment: json.environment.clone(),
            profile: json.profile.clone(),
            mode: json.mode.clone(),
            comptime_env: json.comptime_env.clone(),

            // capability restrictions
            no_managed: json
                .no_managed
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_runtime: json
                .no_runtime
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_referential_equality: json
                .no_referential_equality
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_dynamic_evaluation: json
                .no_dynamic_evaluation
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_global_this: json
                .no_global_this
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_dynamic_import: json
                .no_dynamic_import
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_internal_import: json
                .no_internal_import
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_dynamic_shapes: json
                .no_dynamic_shapes
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_computed_property_access: json
                .no_computed_property_access
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_proxy: json
                .no_proxy
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_implicit_dynamic_dispatch: json
                .no_implicit_dynamic_dispatch
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),
            no_exceptions: json
                .no_exceptions
                .map(DiagnosticPolicy::from)
                .unwrap_or(DiagnosticPolicy::Allow),

            // emit
            root_dir: json.root_dir.as_ref().map(PathBuf::from),
            out_dir: json.out_dir.as_ref().map(PathBuf::from),
            declaration_dir: json.declaration_dir.as_ref().map(PathBuf::from),
            declaration_map: json.declaration_map.unwrap_or(false),
            no_emit: json.no_emit.unwrap_or(false),

            // interop
            tsconfig: json.tsconfig.as_ref().map(PathBuf::from),
            skip_lib_check: json.skip_lib_check.unwrap_or(false),
        };

        // apply runtime-free restrictions when requested
        if options.no_runtime.is_deny() {
            options.apply_no_runtime_restrictions();
        }

        options
    }
}

use indexmap::IndexMap;
use serde_json::Value;

use crate::config::{
    AccountOptions, AssetOptions, CompilerOptions, ConfigOptions, DaemonOptions, DestackJson,
    EnvironmentOptions, FeatureOptions, FormatterOptions, LinterOptions, ProfileConfig,
    RuntimeOptions, StackOptions, TargetOptions, TaskOptions, TelemetryOptions, WatchOptions,
    account_options_from_json, asset_options_from_json, config_options_from_json,
    environment_options_from_json, feature_options_from_json, runtime_options_from_json,
    telemetry_options_from_json,
};

/// Effective normalized package options for one revision scoped package view.
#[derive(Debug, Clone, Default)]
pub struct PackageOptions {
    /// Package name.
    pub name: Option<String>,
    /// Package version.
    pub version: Option<String>,
    /// Whether the package is private.
    pub is_private: Option<bool>,
    /// Package description.
    pub description: Option<String>,
    /// Package license identifier.
    pub license: Option<String>,
    /// Package repository metadata.
    pub repository: Option<Value>,
    /// Package homepage.
    pub homepage: Option<String>,
    /// Package keywords.
    pub keywords: Vec<String>,
    /// Preferred package manager string.
    pub package_manager: Option<String>,
    /// Package module type.
    pub module_type: Option<String>,
    /// Package engines.
    pub engines: IndexMap<String, String>,
    /// Package exports map.
    pub exports: Option<Value>,
    /// Package imports map.
    pub imports: IndexMap<String, Value>,
    /// Runtime dependencies.
    pub dependencies: IndexMap<String, String>,
    /// Development dependencies.
    pub dev_dependencies: IndexMap<String, String>,
    /// Peer dependencies.
    pub peer_dependencies: IndexMap<String, String>,
    /// Optional dependencies.
    pub optional_dependencies: IndexMap<String, String>,
    /// Named local workflow tasks.
    pub tasks: IndexMap<String, TaskOptions>,
    /// Specific files to include in the project.
    pub files: Vec<String>,
    /// Glob patterns for files to include.
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,
    /// Compiler options.
    pub compiler: CompilerOptions,
    /// Runtime options.
    pub runtime: RuntimeOptions,
    /// Formatter options.
    pub formatter: FormatterOptions,
    /// Linter options.
    pub linter: LinterOptions,
    /// Watch options.
    pub watch: WatchOptions,
    /// Daemon options.
    pub daemon: DaemonOptions,
    /// Build targets.
    pub targets: IndexMap<String, TargetOptions>,
    /// Deployment stack definitions.
    pub stacks: IndexMap<String, StackOptions>,
    /// Named control plane accounts.
    pub accounts: IndexMap<String, AccountOptions>,
    /// Named reusable environment overlays.
    pub environments: IndexMap<String, EnvironmentOptions>,
    /// Named reusable config bindings.
    pub configs: IndexMap<String, ConfigOptions>,
    /// Named reusable secret bindings.
    pub secrets: IndexMap<String, crate::config::SecretOptions>,
    /// Named reusable asset collections.
    pub assets: IndexMap<String, AssetOptions>,
    /// Named runtime feature definitions.
    pub features: IndexMap<String, FeatureOptions>,
    /// Named telemetry definitions.
    pub telemetry: IndexMap<String, TelemetryOptions>,
    /// Named profiles for semantic configuration.
    pub profiles: IndexMap<String, ProfileConfig>,
    /// Default target for the package.
    pub default_target: Option<String>,
}

impl From<&DestackJson> for PackageOptions {
    fn from(json: &DestackJson) -> Self {
        let compiler = CompilerOptions::from(&json.compiler);
        let runtime = runtime_options_from_json(Some(&json.runtime));

        let targets = json
            .targets
            .as_ref()
            .map(|target_map| {
                target_map
                    .iter()
                    .map(|(name, target_json)| {
                        (
                            name.clone(),
                            TargetOptions::from_json_with_runtime(target_json, &runtime),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut formatter = FormatterOptions::default();
        json.formatter.apply(&mut formatter);

        let mut linter = LinterOptions::default();
        json.linter.apply(&mut linter);

        let watch = WatchOptions::from(&json.watch);
        let daemon = DaemonOptions::from(&json.daemon);

        Self {
            name: json.name.clone(),
            version: json.version.clone(),
            is_private: json.r#private,
            description: json.description.clone(),
            license: json.license.clone(),
            repository: json.repository.clone(),
            homepage: json.homepage.clone(),
            keywords: json.keywords.clone().unwrap_or_default(),
            package_manager: json.package_manager.clone(),
            module_type: json.module_type.clone(),
            engines: json.engines.clone().unwrap_or_default(),
            exports: json.exports.clone(),
            imports: json.imports.clone().unwrap_or_default(),
            dependencies: json.dependencies.clone().unwrap_or_default(),
            dev_dependencies: json.dev_dependencies.clone().unwrap_or_default(),
            peer_dependencies: json.peer_dependencies.clone().unwrap_or_default(),
            optional_dependencies: json.optional_dependencies.clone().unwrap_or_default(),
            tasks: json
                .tasks
                .as_ref()
                .map(|tasks| {
                    tasks
                        .iter()
                        .map(|(name, task)| (name.clone(), TaskOptions::from(task)))
                        .collect()
                })
                .unwrap_or_default(),
            files: json.files.clone().unwrap_or_default(),
            include: json.include.clone().unwrap_or_default(),
            exclude: json.exclude.clone().unwrap_or_default(),
            compiler,
            runtime,
            formatter,
            linter,
            watch,
            daemon,
            accounts: account_options_from_json(&json.accounts),
            environments: environment_options_from_json(&json.environments),
            configs: config_options_from_json(&json.configs),
            secrets: crate::config::secret_options_from_json(&json.secrets),
            assets: asset_options_from_json(&json.assets),
            features: feature_options_from_json(&json.features),
            telemetry: telemetry_options_from_json(&json.telemetry),
            targets,
            stacks: json
                .stacks
                .as_ref()
                .map(|stack_map| {
                    stack_map
                        .iter()
                        .map(|(name, stack_json)| (name.clone(), StackOptions::from(stack_json)))
                        .collect()
                })
                .unwrap_or_default(),
            profiles: json
                .profiles
                .as_ref()
                .map(|profile_map| {
                    profile_map
                        .iter()
                        .map(|(name, profile_json)| {
                            (name.clone(), ProfileConfig::from_json(profile_json))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            default_target: json.default_target.clone(),
        }
    }
}

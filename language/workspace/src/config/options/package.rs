use indexmap::IndexMap;
use serde_json::Value;

use crate::config::{
    CompilerOptions, DaemonOptions, DestackJson, EnvironmentOptions, FormatterOptions,
    LinterOptions, ModeOptions, ProfileOptions, RuntimeOptions, TargetOptions, WatchOptions,
    environment_options_from_json, runtime_options_from_json,
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
    /// Named reusable toolchain and runtime environments.
    pub environments: IndexMap<String, EnvironmentOptions>,
    /// Named profiles for semantic configuration.
    pub profiles: IndexMap<String, ProfileOptions>,
    /// Named modes for emitted output policy.
    pub modes: IndexMap<String, ModeOptions>,
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
            files: json.files.clone().unwrap_or_default(),
            include: json.include.clone().unwrap_or_default(),
            exclude: json.exclude.clone().unwrap_or_default(),
            compiler,
            runtime,
            formatter,
            linter,
            watch,
            daemon,
            environments: environment_options_from_json(&json.environments),
            targets,
            profiles: json
                .profiles
                .as_ref()
                .map(|profile_map| {
                    profile_map
                        .iter()
                        .map(|(name, profile_json)| {
                            (name.clone(), ProfileOptions::from_json(profile_json))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            modes: json
                .modes
                .as_ref()
                .map(|mode_map| {
                    mode_map
                        .iter()
                        .map(|(name, mode_json)| (name.clone(), ModeOptions::from_json(mode_json)))
                        .collect()
                })
                .unwrap_or_default(),
            default_target: json.default_target.clone(),
        }
    }
}

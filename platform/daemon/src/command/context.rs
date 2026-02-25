use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::Compiler;
use destack_resolver::{CachePolicy, ResolveOptions, Resolver};
use destack_source::{
    Diagnostic, DiagnosticCollection, DiagnosticOptions, DiagnosticStoreUpdate, FileId,
    FileVersion, ModuleId, Uri, glob,
};
use destack_workspace::{
    DsConfig, OptimizeLevel, Program, Target, TargetDiscovery, TargetId, Workspace,
};

use crate::Daemon;

use super::CommandOutputBuffer;
use super::common::{CommandInput, CommandTargetOverrides, CommonCommandOptions};

/// Per-request command context.
#[derive(Debug)]
pub(super) struct CommandContext<'a> {
    /// Active daemon instance.
    pub(super) daemon: &'a Daemon,
    /// Program for the command.
    pub(super) program: Arc<Program>,
    /// Compiler for the command.
    pub(super) compiler: Arc<Compiler>,
    /// Common command options.
    pub(super) common: &'a CommonCommandOptions,
    /// Diagnostic options resolved for this request.
    pub(super) diagnostic_options: DiagnosticOptions,
    /// Output buffer for command streaming.
    pub(super) output: &'a mut CommandOutputBuffer,
}

impl<'a> CommandContext<'a> {
    /// Create a command context for a request.
    pub(super) fn new(
        daemon: &'a Daemon,
        program: Arc<Program>,
        compiler: Arc<Compiler>,
        common: &'a CommonCommandOptions,
        output: &'a mut CommandOutputBuffer,
    ) -> Self {
        let diagnostic_options = common.diagnostic.clone().unwrap_or_default();
        Self {
            daemon,
            program,
            compiler,
            common,
            diagnostic_options,
            output,
        }
    }

    /// Resolve command inputs, falling back to dsconfig when allowed.
    pub(super) fn resolve_command_inputs(&self) -> super::CommandResult<Vec<CommandInput>> {
        if !self.common.inputs.is_empty() {
            return Ok(self.common.inputs.clone());
        }

        if !self.common.allow_dsconfig_fallback {
            return Err("no input files provided".to_string().into());
        }

        let dsconfig_path = self.resolve_dsconfig_path(self.common.config_path.as_deref())?;
        let dsconfig = self.load_dsconfig(&dsconfig_path)?;
        let inputs = collect_sources_from_dsconfig(&dsconfig, self.common.target.as_deref());

        if inputs.is_empty() {
            return Err("no input files provided".to_string().into());
        }

        Ok(inputs
            .into_iter()
            .map(|path| CommandInput::File { path })
            .collect())
    }

    /// Resolve command inputs into module ids.
    pub(super) fn resolve_modules(
        &self,
        inputs: &[CommandInput],
    ) -> super::CommandResult<Vec<ModuleId>> {
        let mut seen = HashSet::new();
        let mut modules = Vec::new();

        for input in inputs {
            let module_id = match input {
                CommandInput::File { path } => self
                    .compiler
                    .resolve_path_to_module(&path.to_path_buf())
                    .map_err(|error| format!("failed to resolve {}: {error}", path.display()))?,
                CommandInput::Inline {
                    name,
                    content,
                    file_type,
                } => {
                    let uri = Uri::from_string(name);
                    self.program
                        .register_inline_module(uri, content.clone(), *file_type)
                }
                CommandInput::Stdin {
                    name,
                    content,
                    file_type,
                } => {
                    let uri = Uri::from_string(name);
                    self.program
                        .register_inline_module(uri, content.clone(), *file_type)
                }
            };

            if seen.insert(module_id) {
                modules.push(module_id);
            }
        }

        if modules.is_empty() {
            return Err("no input files provided".to_string().into());
        }

        Ok(modules)
    }

    /// Clear the diagnostic collector before re-compiling.
    pub(super) fn reset_diagnostics(&self) {
        let _ = self.program.diagnostics.drain();
    }

    /// Collect diagnostics for the request.
    pub(super) fn collect_diagnostics(&self) -> DiagnosticCollection {
        self.program
            .diagnostics
            .collect()
            .map(&self.diagnostic_options)
    }

    /// Collect diagnostics without applying options.
    pub(super) fn collect_raw_diagnostics(&self) -> DiagnosticCollection {
        self.program.diagnostics.collect()
    }

    /// Commit diagnostics to the program store for module files.
    pub(super) fn commit_diagnostics_for_modules(
        &self,
        modules: &[ModuleId],
        diagnostics: &DiagnosticCollection,
    ) -> super::CommandResult<()> {
        // group diagnostics by file id
        let mut diagnostics_by_file: HashMap<FileId, Vec<Diagnostic>> = HashMap::new();
        for diagnostic in diagnostics.iter() {
            diagnostics_by_file
                .entry(diagnostic.file_id)
                .or_default()
                .push(diagnostic);
        }

        // collect file versions from modules
        let mut file_versions: HashMap<FileId, FileVersion> = HashMap::new();
        for module_id in modules {
            let module = self.program.modules.get(*module_id);
            let module = module.read();
            file_versions.insert(module.file_id, module.source_version);
        }

        // extend file versions for diagnostics outside modules
        for file_id in diagnostics_by_file.keys().copied() {
            if file_versions.contains_key(&file_id) {
                continue;
            }

            let file = self
                .program
                .files
                .get_maybe(file_id)
                .ok_or_else(|| format!("file metadata missing for {file_id:?}"))?;
            file_versions.insert(file_id, file.version);
        }

        // build store updates for the files
        let mut updates = Vec::new();
        for (file_id, file_version) in file_versions {
            let diagnostics = diagnostics_by_file.remove(&file_id).unwrap_or_default();
            updates.push(DiagnosticStoreUpdate::new(
                file_id,
                file_version,
                diagnostics,
            ));
        }

        // commit diagnostics to the store
        self.program.diagnostic_store.apply_updates(updates);

        Ok(())
    }

    /// Resolve or create a target for a module.
    pub(super) fn ensure_target_for_module(
        &self,
        module_id: ModuleId,
        target_name: &str,
        overrides: Option<&CommandTargetOverrides>,
    ) -> super::CommandResult<TargetId> {
        let module = self.program.modules.get(module_id);
        let package_id = module.read().package_id;
        let target_id = TargetId::new(package_id, target_name);

        let package = self.program.packages.get(package_id);
        let mut package = package.write();
        let existing_target = package.targets.get(&target_id).cloned();

        if let Some(overrides) = overrides
            && !overrides.is_empty()
            && existing_target.is_some()
        {
            return Err(
                "ad-hoc target overrides are not supported for named targets"
                    .to_string()
                    .into(),
            );
        }

        if existing_target.is_none() {
            let mut target = Target::implicit_for_name(target_name)
                .ok_or_else(|| format!("unknown target '{target_name}'"))?;
            if let Some(overrides) = overrides {
                overrides.apply_to_target(&mut target);
            }
            package.targets.insert(target_id.clone(), target);
        }

        Ok(target_id)
    }

    /// Resolve target by id.
    pub(super) fn target_for_id(&self, target_id: &TargetId) -> Option<Target> {
        let package = self.program.packages.get(target_id.package_id);
        let package = package.read();
        package.targets.get(target_id).cloned()
    }

    /// Resolve or infer a target for a module based on command and config defaults.
    pub(super) fn resolve_target_for_module(
        &self,
        module_id: ModuleId,
        overrides: Option<&CommandTargetOverrides>,
    ) -> super::CommandResult<TargetId> {
        // honor explicit target override first
        if let Some(target_name) = self.common.target.as_deref() {
            return self.ensure_target_for_module(module_id, target_name, overrides);
        }

        // derive from package defaults and configured targets
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let package = self.program.packages.get(module.package_id);
        let package = package.read();

        // use dsconfig default target when available
        if let Some(dsconfig) = package.dsconfig.as_ref()
            && let Some(target_name) = dsconfig.options.default_target.as_deref()
        {
            return self.ensure_target_for_module(module_id, target_name, overrides);
        }

        // select single configured target when unambiguous
        let mut target_names: Vec<String> = package
            .targets
            .values()
            .filter(|target| !target.synthetic)
            .map(|target| target.name.clone())
            .collect();
        target_names.sort();
        if target_names.len() == 1 {
            return self.ensure_target_for_module(module_id, &target_names[0], overrides);
        }

        // infer a fallback target when no explicit configuration exists
        if target_names.is_empty() {
            let target_name = if module.language_type.is_destack() {
                "native"
            } else {
                "js"
            };
            return self.ensure_target_for_module(module_id, target_name, overrides);
        }

        // require an explicit target when multiple are available
        Err(format!(
            "multiple targets configured ({}): specify --target",
            target_names.join(", ")
        )
        .into())
    }

    /// Decide whether optimization should run for a target.
    pub(super) fn should_optimize(&self, target: &Target) -> bool {
        if target.optimize {
            return true;
        }

        !matches!(target.optimize_level, OptimizeLevel::O0)
    }

    /// Build a resolver for the current program.
    pub(super) fn resolver(&self) -> Resolver {
        let workspace_config = self.daemon.session.workspace_config();

        Resolver::from_program(
            &self.program,
            ResolveOptions::default_for_workspace(
                self.program.cwd.clone(),
                workspace_config.as_deref(),
            ),
        )
    }

    /// Resolve a dsconfig.json path for the current program.
    pub(super) fn resolve_dsconfig_path(
        &self,
        override_path: Option<&Path>,
    ) -> super::CommandResult<PathBuf> {
        resolve_dsconfig_path(&self.resolver(), self.program.cwd.as_path(), override_path)
    }

    /// Load dsconfig.json for a path.
    pub(super) fn load_dsconfig(&self, path: &Path) -> super::CommandResult<DsConfig> {
        load_dsconfig(&self.resolver(), path)
    }

    /// Find dsconfig.json for a directory.
    pub(super) fn find_dsconfig(&self, cwd: &Path) -> Option<PathBuf> {
        find_dsconfig(&self.resolver(), cwd)
    }

    /// Load workspace dsconfig.json files for all packages.
    pub(super) fn load_workspace_dsconfigs(
        &self,
        workspace: &Workspace,
    ) -> super::CommandResult<Vec<DsConfig>> {
        load_workspace_dsconfigs(&self.resolver(), workspace)
    }
}

fn resolve_dsconfig_path(
    resolver: &Resolver,
    cwd: &Path,
    override_path: Option<&Path>,
) -> super::CommandResult<PathBuf> {
    let dsconfig_path = if let Some(config_path) = override_path {
        let resolved = if config_path.is_absolute() {
            config_path.to_path_buf()
        } else {
            cwd.join(config_path)
        };
        if let Ok(metadata) = resolver.fs.metadata(&resolved) {
            if metadata.is_directory {
                find_dsconfig(resolver, &resolved)
                    .ok_or_else(|| "dsconfig.json not found".to_string())?
            } else {
                resolved
            }
        } else {
            return Err("dsconfig.json not found".to_string().into());
        }
    } else {
        find_dsconfig(resolver, cwd).ok_or_else(|| "dsconfig.json not found".to_string())?
    };

    Ok(dsconfig_path)
}

fn load_dsconfig(resolver: &Resolver, path: &Path) -> super::CommandResult<DsConfig> {
    Ok(resolver
        .load_dsconfig(path, CachePolicy::UseCache)
        .map_err(|error| error.to_string())?)
}

fn find_dsconfig(resolver: &Resolver, cwd: &Path) -> Option<PathBuf> {
    let mut current = cwd.to_path_buf();
    loop {
        let candidate = current.join("dsconfig.json");
        if resolver
            .fs
            .metadata(&candidate)
            .is_ok_and(|metadata| metadata.is_file)
        {
            return Some(candidate);
        }

        let parent = current.parent()?;
        if parent == current {
            return None;
        }
        current = parent.to_path_buf();
    }
}

fn load_workspace_dsconfigs(
    resolver: &Resolver,
    workspace: &Workspace,
) -> super::CommandResult<Vec<DsConfig>> {
    let mut configs = BTreeMap::new();
    for package_path in &workspace.package_paths {
        if let Some(path) = find_dsconfig(resolver, package_path.as_path()) {
            configs.entry(path).or_insert_with(|| package_path.clone());
        }
    }

    let mut resolved = Vec::new();
    for (path, _) in configs {
        resolved.push(load_dsconfig(resolver, &path)?);
    }

    Ok(resolved)
}

fn collect_sources_from_dsconfig(dsconfig: &DsConfig, target_name: Option<&str>) -> Vec<PathBuf> {
    let selected_target = target_name
        .map(str::to_string)
        .or_else(|| dsconfig.options.default_target.clone());
    let target_options = selected_target
        .as_deref()
        .and_then(|name| dsconfig.options.targets.get(name));

    let (entries, includes, excludes, discovery) = if let Some(target) = target_options {
        let includes = if target.include.is_empty() {
            dsconfig.options.include.clone()
        } else {
            target.include.clone()
        };
        let mut excludes = dsconfig.options.exclude.clone();
        excludes.extend(target.exclude.iter().cloned());
        (target.entry.clone(), includes, excludes, target.discovery)
    } else {
        (
            Vec::new(),
            dsconfig.options.include.clone(),
            dsconfig.options.exclude.clone(),
            TargetDiscovery::Include,
        )
    };

    let base_dir = dsconfig.directory.clone();
    let mut paths = BTreeMap::new();

    if discovery == TargetDiscovery::Entry && !entries.is_empty() {
        for entry in entries {
            let path = if entry.is_absolute() {
                entry
            } else {
                base_dir.join(entry)
            };
            paths.insert(path, ());
        }
    }

    if paths.is_empty() && !dsconfig.options.files.is_empty() {
        for file in &dsconfig.options.files {
            let path = base_dir.join(file);
            paths.insert(path, ());
        }
    }

    if paths.is_empty() {
        let patterns = if includes.is_empty() {
            vec![
                "**/*.ds".to_string(),
                "**/*.ts".to_string(),
                "**/*.tsx".to_string(),
            ]
        } else {
            includes
        };

        let include_paths = expand_patterns(&base_dir, &patterns);
        let exclude_paths = expand_patterns(&base_dir, &excludes);
        let excludes: HashSet<PathBuf> = exclude_paths.into_iter().collect();
        for path in include_paths {
            if !excludes.contains(&path) {
                paths.insert(path, ());
            }
        }
    }

    paths.into_keys().collect()
}

fn expand_patterns(base_dir: &Path, patterns: &[String]) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for pattern in patterns {
        let pattern_path = if PathBuf::from(pattern).is_absolute() {
            PathBuf::from(pattern)
        } else {
            base_dir.join(pattern)
        };

        let entries = glob(pattern_path.to_string_lossy().as_ref());
        paths.extend(entries);
    }

    paths
}

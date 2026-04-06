use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::Compiler;
use destack_resolver::{CachePolicy, ResolveOptions, Resolver};
use destack_source::{DiagnosticCollection, DiagnosticOptions, FileType, ModuleId, TargetId, glob};
use destack_workspace::{
    Change, DestackDeclaration, Edit, OptimizeLevel, Ref, Repository, RepositorySnapshot, Revision,
    Target, TargetDiscovery,
};

use crate::Daemon;

use super::CommandOutputBuffer;
use super::common::{CommandInput, CommandTargetOverrides, CommonCommandOptions};

/// Per-request command context.
#[derive(Debug)]
pub(super) struct CommandContext<'a> {
    /// Active daemon instance.
    pub(super) daemon: &'a Daemon,
    /// Workspace root for this command.
    pub(super) root: PathBuf,
    /// Repository for the command.
    pub(super) repository: Arc<Repository>,
    /// Compiler for the command.
    pub(super) compiler: Arc<Compiler>,
    /// Common command options.
    pub(super) common: &'a CommonCommandOptions,
    /// Diagnostic options resolved for this request.
    pub(super) diagnostic_options: DiagnosticOptions,
    /// The pinned command local snapshot when inputs materialize extra source.
    active_snapshot: RefCell<Option<RepositorySnapshot>>,
    /// Output buffer for command streaming.
    pub(super) output: &'a mut CommandOutputBuffer,
}

/// One resolved command target.
#[derive(Debug, Clone)]
pub(super) struct ResolvedTarget {
    /// The resolved target id.
    pub id: TargetId,
    /// The resolved target configuration.
    pub target: Target,
}

impl<'a> CommandContext<'a> {
    /// Create a command context for a request.
    pub(super) fn new(
        daemon: &'a Daemon,
        root: PathBuf,
        repository: Arc<Repository>,
        compiler: Arc<Compiler>,
        common: &'a CommonCommandOptions,
        output: &'a mut CommandOutputBuffer,
    ) -> Self {
        let diagnostic_options = common.diagnostic.clone().unwrap_or_default();
        Self {
            daemon,
            root,
            repository,
            compiler,
            common,
            diagnostic_options,
            active_snapshot: RefCell::new(None),
            output,
        }
    }

    /// Resolve command inputs, falling back to the Destack config when allowed.
    pub(super) fn resolve_command_inputs(&self) -> super::CommandResult<Vec<CommandInput>> {
        if !self.common.inputs.is_empty() {
            return Ok(self.common.inputs.clone());
        }

        if !self.common.allow_destack_config_fallback {
            return Err("no input files provided".to_string().into());
        }

        let config_path = self.resolve_destack_config_path(self.common.config_path.as_deref())?;
        let declaration = self.load_destack_declaration(&config_path)?;
        let inputs =
            collect_sources_from_destack_declaration(&declaration, self.common.target.as_deref());

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
        let revision = self.revision()?;
        let mut seen = HashSet::new();
        let mut modules = Vec::new();

        for input in inputs {
            let module_id = match input {
                CommandInput::File { path } => self
                    .compiler
                    .resolve_path_to_module(revision, &path.to_path_buf())
                    .map_err(|error| format!("failed to resolve {}: {error}", path.display()))?,
                CommandInput::Inline {
                    name,
                    content,
                    file_type,
                } => self.materialize_inline_module("inline", name, content, *file_type)?,
                CommandInput::Stdin {
                    name,
                    content,
                    file_type,
                } => self.materialize_inline_module("stdin", name, content, *file_type)?,
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

    /// Return the active workspace revision for this command.
    pub(super) fn revision(&self) -> super::CommandResult<Revision> {
        if let Some(snapshot) = self.active_snapshot.borrow().as_ref() {
            return Ok(snapshot.revision());
        }

        let reference = Ref::for_workspace_root(&self.root);
        let revision = self
            .repository
            .current(&reference)
            .map_err(|error| format!("missing current workspace revision: {error}"))?;
        let snapshot = self
            .repository
            .snapshot(revision)
            .map_err(|error| format!("failed to pin workspace revision: {error}"))?;

        *self.active_snapshot.borrow_mut() = Some(snapshot.clone());

        Ok(snapshot.revision())
    }

    /// Materialize one inline command input into one command local revision.
    fn materialize_inline_module(
        &self,
        kind: &str,
        name: &str,
        content: &str,
        file_type: FileType,
    ) -> super::CommandResult<ModuleId> {
        let base_revision = self.revision()?;
        let logical_path = command_input_logical_path(kind, name, file_type);
        let change = Change::single(Edit::set_text(&logical_path, content));
        let revision = self
            .repository
            .apply_to_revision(base_revision, change)
            .map_err(|error| format!("failed to materialize command input {name}: {error}"))?;
        let path = self.root.join(&logical_path);
        let module_id = self
            .repository
            .module_id_for_path(revision, &path)
            .map_err(|error| format!("failed to resolve command input module {name}: {error}"))?
            .ok_or_else(|| format!("missing command input module for {name}"))?;

        let snapshot = self
            .repository
            .snapshot(revision)
            .map_err(|error| format!("failed to pin command revision {name}: {error}"))?;
        *self.active_snapshot.borrow_mut() = Some(snapshot);

        Ok(module_id)
    }

    /// Return the visible module count for this command revision.
    pub(super) fn module_count(&self, revision: Revision) -> super::CommandResult<usize> {
        let modules = self
            .repository
            .workspace_module_ids(revision)
            .map_err(|error| format!("failed to collect workspace modules: {error}"))?;

        Ok(modules.len())
    }

    /// Return the unique default profile count for the provided modules.
    pub(super) fn default_profile_count(
        &self,
        revision: Revision,
        modules: &[ModuleId],
    ) -> super::CommandResult<usize> {
        let mut profiles = HashSet::new();

        for module_id in modules {
            let profile_id = self
                .repository
                .default_profile_id_for_module(revision, *module_id)
                .map_err(|error| format!("failed to resolve default profile: {error}"))?;
            profiles.insert(profile_id);
        }

        Ok(profiles.len())
    }

    /// Commit diagnostics to the repository store for module files.
    pub(super) fn commit_diagnostics_for_modules(
        &self,
        modules: &[ModuleId],
        diagnostics: &DiagnosticCollection,
    ) -> super::CommandResult<()> {
        let _ = modules;
        let _ = diagnostics;

        Ok(())
    }

    /// Resolve or create a target for a module.
    pub(super) fn ensure_target_for_module(
        &self,
        module_id: ModuleId,
        target_name: &str,
        overrides: Option<&CommandTargetOverrides>,
    ) -> super::CommandResult<ResolvedTarget> {
        let revision = self.revision()?;
        let module = self
            .repository
            .module(revision, module_id)
            .map_err(|error| format!("failed to read module snapshot: {error}"))?
            .ok_or_else(|| format!("missing module snapshot for {module_id:?}"))?;
        let package_id = module.package_id;
        let target_id = self.repository.intern_target_id(package_id, target_name);

        let package = self
            .repository
            .package(revision, package_id)
            .map_err(|error| format!("failed to read package snapshot: {error}"))?
            .ok_or_else(|| format!("missing package snapshot for {package_id:?}"))?;
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

        let target = if let Some(target) = existing_target {
            target
        } else {
            let mut target = Target::implicit_for_name(target_name)
                .ok_or_else(|| format!("unknown target '{target_name}'"))?;
            if let Some(overrides) = overrides {
                overrides.apply_to_target(&mut target);
            }
            target
        };

        Ok(ResolvedTarget {
            id: target_id,
            target,
        })
    }

    /// Resolve or infer a target for a module based on command and config defaults.
    pub(super) fn resolve_target_for_module(
        &self,
        module_id: ModuleId,
        overrides: Option<&CommandTargetOverrides>,
    ) -> super::CommandResult<ResolvedTarget> {
        let revision = self.revision()?;

        // honor explicit target override first
        if let Some(target_name) = self.common.target.as_deref() {
            return self.ensure_target_for_module(module_id, target_name, overrides);
        }

        // derive from package defaults and configured targets
        let module = self
            .repository
            .module(revision, module_id)
            .map_err(|error| format!("failed to read module snapshot: {error}"))?
            .ok_or_else(|| format!("missing module snapshot for {module_id:?}"))?;
        let package = self
            .repository
            .package(revision, module.package_id)
            .map_err(|error| format!("failed to read package snapshot: {error}"))?
            .ok_or_else(|| format!("missing package snapshot for {:?}", module.package_id))?;

        // use the package config default target when available
        if let Some(package_options) = self
            .repository
            .package_options(revision, package.id)
            .map_err(|error| format!("failed to read package options: {error}"))?
            && let Some(target_name) = package_options.default_target.as_deref()
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

    /// Build a resolver for the current repository.
    pub(super) fn resolver(&self) -> Resolver {
        let workspace_options = self
            .revision()
            .ok()
            .and_then(|revision| self.daemon.repository.workspace_options(revision).ok())
            .flatten();

        Resolver::from_repository(
            &self.repository,
            ResolveOptions::default_for_workspace(self.root.clone(), workspace_options.as_ref()),
        )
    }

    /// Resolve a destack.json path for the current repository.
    pub(super) fn resolve_destack_config_path(
        &self,
        override_path: Option<&Path>,
    ) -> super::CommandResult<PathBuf> {
        resolve_destack_config_path(&self.resolver(), self.root.as_path(), override_path)
    }

    /// Load one `destack.json` declaration for a path.
    pub(super) fn load_destack_declaration(
        &self,
        path: &Path,
    ) -> super::CommandResult<DestackDeclaration> {
        load_destack_declaration(&self.resolver(), path)
    }

    /// Find destack.json for a directory.
    pub(super) fn find_destack_config(&self, cwd: &Path) -> Option<PathBuf> {
        find_destack_config(&self.resolver(), cwd)
    }

    /// Load all visible workspace `destack.json` declarations.
    pub(super) fn load_workspace_declarations(
        &self,
        revision: Revision,
    ) -> super::CommandResult<Vec<DestackDeclaration>> {
        load_workspace_declarations(&self.resolver(), &self.repository, revision)
    }
}

/// Build one stable logical path for one command local source input.
fn command_input_logical_path(kind: &str, name: &str, file_type: FileType) -> String {
    let extension = file_type.extension().unwrap_or("txt");
    let sanitized_name = sanitize_command_input_name(name);

    format!(".destack/command/{kind}/{sanitized_name}.{extension}")
}

/// Sanitize one command input label for use in a logical path.
fn sanitize_command_input_name(name: &str) -> String {
    let mut sanitized = String::new();

    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            sanitized.push(character.to_ascii_lowercase());
        } else if matches!(character, '/' | '\\' | '-' | '_' | '.') {
            sanitized.push('_');
        }
    }

    if sanitized.is_empty() {
        sanitized.push_str("input");
    }

    sanitized
}

fn resolve_destack_config_path(
    resolver: &Resolver,
    cwd: &Path,
    override_path: Option<&Path>,
) -> super::CommandResult<PathBuf> {
    let destack_config_path = if let Some(config_path) = override_path {
        let resolved = if config_path.is_absolute() {
            config_path.to_path_buf()
        } else {
            cwd.join(config_path)
        };
        if let Ok(metadata) = resolver.fs.metadata(&resolved) {
            if metadata.is_directory {
                find_destack_config(resolver, &resolved)
                    .ok_or_else(|| "destack.json not found".to_string())?
            } else {
                resolved
            }
        } else {
            return Err("destack.json not found".to_string().into());
        }
    } else {
        find_destack_config(resolver, cwd).ok_or_else(|| "destack.json not found".to_string())?
    };

    Ok(destack_config_path)
}

fn load_destack_declaration(
    resolver: &Resolver,
    path: &Path,
) -> super::CommandResult<DestackDeclaration> {
    Ok(resolver
        .read_destack_config(path, CachePolicy::UseCache)
        .map_err(|error| error.to_string())?)
}

fn find_destack_config(resolver: &Resolver, cwd: &Path) -> Option<PathBuf> {
    let mut current = cwd.to_path_buf();
    loop {
        let candidate = current.join("destack.json");
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

fn load_workspace_declarations(
    resolver: &Resolver,
    repository: &Repository,
    revision: Revision,
) -> super::CommandResult<Vec<DestackDeclaration>> {
    let mut configs = BTreeMap::new();
    for package_path in repository
        .workspace_package_paths(revision)
        .map_err(|error| error.to_string())?
    {
        if let Some(path) = find_destack_config(resolver, package_path.as_path()) {
            configs.entry(path).or_insert_with(|| package_path.clone());
        }
    }

    let mut resolved = Vec::new();
    for (path, _) in configs {
        resolved.push(load_destack_declaration(resolver, &path)?);
    }

    Ok(resolved)
}

fn collect_sources_from_destack_declaration(
    declaration: &DestackDeclaration,
    target_name: Option<&str>,
) -> Vec<PathBuf> {
    let options = declaration.package_options();
    let selected_target = target_name
        .map(str::to_string)
        .or_else(|| options.default_target.clone());
    let target_options = selected_target
        .as_deref()
        .and_then(|name| options.targets.get(name));

    let (entries, includes, excludes, discovery) = if let Some(target) = target_options {
        let includes = if target.include.is_empty() {
            options.include.clone()
        } else {
            target.include.clone()
        };
        let mut excludes = options.exclude.clone();
        excludes.extend(target.exclude.iter().cloned());
        (target.entry.clone(), includes, excludes, target.discovery)
    } else {
        (
            Vec::new(),
            options.include.clone(),
            options.exclude.clone(),
            TargetDiscovery::Include,
        )
    };

    let base_dir = declaration.directory.clone();
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

    if paths.is_empty() && !options.files.is_empty() {
        for file in &options.files {
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

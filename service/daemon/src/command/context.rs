use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_query::Query;
use destack_resolver::{CachePolicy, Resolver, ResolverContext, ResolverOptions};
use destack_session::{FileChange, Session};
use destack_source::{FileType, ModuleId, ProfileId, TargetId, glob};
use destack_workspace::{
    DestackDeclaration, OptimizeLevel, Ref, Repository, Revision, Target, TargetDiscovery,
};

use crate::Daemon;

use super::common::{CommandInput, CommandTargetOverrides, CommonCommandOptions};
use super::{CommandOutputBuffer, CommandResult, DaemonCommandError};

/// Per-request command context.
#[derive(Debug)]
pub(super) struct CommandContext<'a> {
    /// Active daemon instance.
    pub(super) daemon: &'a Daemon,
    /// Workspace root for this command.
    pub(super) root: PathBuf,
    /// Repository for the command.
    pub(super) repository: Arc<Repository>,
    /// Private command session over the active revision.
    pub(super) session: Session,
    /// Common command options.
    pub(super) common: &'a CommonCommandOptions,
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
    ) -> CommandResult<Self> {
        // root revision
        let reference = Ref::for_workspace_root(&root);
        let revision = repository.current(&reference).map_err(|error| {
            DaemonCommandError::internal(format!(
                "command workspace revision is missing for {}: {error}",
                root.display()
            ))
        })?;

        // private command session
        let linter = Arc::new(Linter::new(repository.clone()));
        let query = Arc::new(Query::new(repository.clone()));
        let cwd = common.cwd.clone().unwrap_or_else(|| root.clone());
        let head = daemon.next_command_ref(&root);
        let session = Session::fork(
            root.clone(),
            cwd,
            repository.clone(),
            head,
            revision,
            compiler.clone(),
            linter,
            query,
            daemon.worker_limit,
            None,
        )
        .map_err(|error| {
            DaemonCommandError::internal(format!("failed to initialize command session: {error}"))
        })?;
        session
            .apply_workspace_config_overrides(session.head(), &common.overrides)
            .map_err(|error| {
                DaemonCommandError::internal(format!(
                    "failed to apply command config overrides: {error}"
                ))
            })?;

        Ok(Self {
            daemon,
            root,
            repository,
            session,
            common,
            output,
        })
    }

    /// Resolve command inputs, falling back to the Destack config when allowed.
    pub(super) fn resolve_command_inputs(&self) -> CommandResult<Vec<CommandInput>> {
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
    pub(super) fn resolve_modules(&self, inputs: &[CommandInput]) -> CommandResult<Vec<ModuleId>> {
        let mut seen = HashSet::new();
        let mut modules = Vec::new();

        for input in inputs {
            let module_id = match input {
                CommandInput::File { path } => self
                    .session
                    .load_module_from_fs(self.session.head(), path)
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
    pub(super) fn revision(&self) -> CommandResult<Revision> {
        self.session
            .revision(self.session.head())
            .map_err(|error| DaemonCommandError::internal(error.to_string()))
    }

    /// Materialize one inline command input into one command local revision.
    fn materialize_inline_module(
        &self,
        kind: &str,
        name: &str,
        content: &str,
        file_type: FileType,
    ) -> CommandResult<ModuleId> {
        let logical_path = command_input_logical_path(kind, name, file_type);
        let path = self.root.join(&logical_path);

        // publish the new command-local file text
        self.session
            .apply_file(
                self.session.head(),
                path.as_path(),
                FileChange::Text {
                    content: content.to_string(),
                },
            )
            .map_err(|error| format!("failed to materialize command input {name}: {error}"))?;

        let path = self.root.join(&logical_path);
        let module_id = self
            .session
            .load_module_from_fs(self.session.head(), path.as_path())
            .map_err(|error| format!("failed to resolve command input module {name}: {error}"))?;

        Ok(module_id)
    }

    /// Return the unique default profile count for the provided modules.
    pub(super) fn default_profile_count(
        &self,
        revision: Revision,
        modules: &[ModuleId],
    ) -> CommandResult<usize> {
        let mut profiles = HashSet::new();

        for module_id in modules {
            let profile_id = self.module_profile_id(revision, *module_id)?;
            profiles.insert(profile_id);
        }

        Ok(profiles.len())
    }

    /// Return the default profile id for one module.
    pub(super) fn module_profile_id(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> CommandResult<ProfileId> {
        let profile = self
            .repository
            .module_profile(revision, module_id)
            .map_err(|error| format!("failed to resolve module profile: {error}"))?;

        Ok(profile.id())
    }

    /// Return the profile id selected for one module target.
    pub(super) fn target_profile_id(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: TargetId,
    ) -> CommandResult<ProfileId> {
        let profile = self
            .repository
            .module_target_profile(revision, module_id, target_id)
            .map_err(|error| format!("failed to resolve target profile: {error}"))?;
        let profile = if let Some(profile) = profile {
            profile
        } else {
            self.repository
                .module_profile(revision, module_id)
                .map_err(|error| format!("failed to resolve module profile: {error}"))?
        };

        Ok(profile.id())
    }

    /// Resolve a named target for a module.
    pub(super) fn resolve_named_target_for_module(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_name: &str,
        overrides: Option<&CommandTargetOverrides>,
    ) -> CommandResult<ResolvedTarget> {
        // load the owning package for the module
        let module = self
            .repository
            .module(revision, module_id)
            .map_err(|error| format!("failed to read module snapshot: {error}"))?
            .ok_or_else(|| format!("missing module snapshot for {module_id:?}"))?;
        let package_id = module.package_id;
        let target_id = TargetId::new(package_id, target_name);

        // distinguish explicit targets from implicit target fallback
        let existing_target = self
            .repository
            .target(revision, target_id)
            .map_err(|error| format!("failed to read target snapshot: {error}"))?;
        let is_explicit_target = existing_target.is_some();
        let effective_target = self
            .repository
            .effective_target(revision, target_id)
            .map_err(|error| format!("failed to read target snapshot: {error}"))?;

        // named targets are already complete configuration entries
        if let Some(overrides) = overrides
            && !overrides.is_empty()
            && is_explicit_target
        {
            return Err(
                "ad-hoc target overrides are not supported for named targets"
                    .to_string()
                    .into(),
            );
        }

        // resolve repository target or build a known built in target
        let target = if let Some(target) = effective_target {
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
        revision: Revision,
        module_id: ModuleId,
        overrides: Option<&CommandTargetOverrides>,
    ) -> CommandResult<ResolvedTarget> {
        // honor explicit target override first
        if let Some(target_name) = self.common.target.as_deref() {
            return self.resolve_named_target_for_module(
                revision,
                module_id,
                target_name,
                overrides,
            );
        }

        // derive from package defaults and configured targets
        let module = self
            .repository
            .module(revision, module_id)
            .map_err(|error| format!("failed to read module snapshot: {error}"))?
            .ok_or_else(|| format!("missing module snapshot for {module_id:?}"))?;
        // use the package default target when it is unambiguous
        if let Some((target_id, target)) = self
            .repository
            .package_default_target(revision, module.package_id)
            .map_err(|error| format!("failed to read target snapshot: {error}"))?
        {
            if let Some(overrides) = overrides
                && !overrides.is_empty()
            {
                return Err(
                    "ad-hoc target overrides are not supported for named targets"
                        .to_string()
                        .into(),
                );
            }

            return Ok(ResolvedTarget {
                id: target_id,
                target,
            });
        }

        // infer a fallback target when no explicit configuration exists
        let target_name = if module.is_destack() { "native" } else { "js" };

        self.resolve_named_target_for_module(revision, module_id, target_name, overrides)
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
            self.repository.clone(),
            ResolverOptions::workspace_defaults(self.root.clone(), workspace_options.as_ref()),
        )
    }

    /// Resolve a destack.json path for the current repository.
    pub(super) fn resolve_destack_config_path(
        &self,
        override_path: Option<&Path>,
    ) -> CommandResult<PathBuf> {
        resolve_destack_config_path(&self.resolver(), self.root.as_path(), override_path)
    }

    /// Load one `destack.json` declaration for a path.
    pub(super) fn load_destack_declaration(
        &self,
        path: &Path,
    ) -> CommandResult<DestackDeclaration> {
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
    ) -> CommandResult<Vec<DestackDeclaration>> {
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
) -> CommandResult<PathBuf> {
    let revision = resolver_revision(resolver)?;
    let repository = resolver.repository();

    let destack_config_path = if let Some(config_path) = override_path {
        let resolved = if config_path.is_absolute() {
            config_path.to_path_buf()
        } else {
            cwd.join(config_path)
        };
        let metadata = repository
            .file_metadata(revision, &resolved)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "destack.json not found".to_string())?;

        if metadata.is_directory {
            find_destack_config(resolver, &resolved)
                .ok_or_else(|| "destack.json not found".to_string())?
        } else {
            resolved
        }
    } else {
        find_destack_config(resolver, cwd).ok_or_else(|| "destack.json not found".to_string())?
    };

    Ok(destack_config_path)
}

fn load_destack_declaration(resolver: &Resolver, path: &Path) -> CommandResult<DestackDeclaration> {
    let revision = resolver_revision(resolver)?;
    let mut context = ResolverContext::new(revision);

    Ok(resolver
        .read_destack(path, &mut context, CachePolicy::UseCache)
        .map_err(|error| error.to_string())?)
}

fn find_destack_config(resolver: &Resolver, cwd: &Path) -> Option<PathBuf> {
    let revision = resolver_revision(resolver).ok()?;
    let repository = resolver.repository();

    let mut directory = if repository
        .file_metadata(revision, cwd)
        .ok()
        .flatten()
        .is_some_and(|metadata| metadata.is_directory)
    {
        cwd.to_path_buf()
    } else {
        cwd.parent()?.to_path_buf()
    };

    loop {
        let candidate = directory.join("destack.json");
        match repository.destack_declaration_for_path(revision, &candidate) {
            Ok(Some(_)) => return Some(candidate),
            Ok(None) => {}
            Err(_) => return None,
        }

        if !directory.pop() {
            return None;
        }
    }
}

fn resolver_revision(resolver: &Resolver) -> CommandResult<Revision> {
    let repository = resolver.repository();
    let reference = Ref::for_workspace_root(repository.workspace_root());

    repository
        .current(&reference)
        .map_err(|error| error.to_string().into())
}

fn load_workspace_declarations(
    resolver: &Resolver,
    repository: &Repository,
    revision: Revision,
) -> CommandResult<Vec<DestackDeclaration>> {
    let mut configs = BTreeMap::new();
    for package_path in repository
        .package_roots(revision)
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

use std::collections::{BTreeMap, HashSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Map, Value};
use tspp_artifact::{ArtifactKey, EnvironmentBound, ModuleGraph};
use tspp_repository as repository;
use tspp_repository::{
    ArtifactReader, DestackFile, Repository, Revision, RevisionPin, Target, TargetRoot, Trace,
    TraceLevel, TraceSnapshot, TraceView, apply_manifest_overrides_to_json, parse_jsonc_text,
};
use tspp_session::{ArtifactPriority, Session, SessionEventHandler};
use tspp_source::{
    DiagnosticCollection, File, FileId, FileType, ModuleId, ProfileId, TargetId, Uri, glob,
};

use crate::{FileImage, Workspace};

use super::common::{
    CommandInput, CommandMessagePayload, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride,
};
use super::outcome::CommandOutcome;
use super::output::OutputBuffer;
use super::{CommandError, CommandResult};

/// Per-request command context.
pub(crate) struct CommandContext<'a> {
    /// Active local workspace.
    pub(super) workspace: &'a Workspace,
    /// Workspace root for this command.
    pub(super) root: PathBuf,
    /// Working directory for this command.
    pub(super) cwd: PathBuf,
    /// Repository for the command.
    pub(crate) repository: Arc<Repository>,
    /// Root workspace revision from which this command forked.
    pub(super) base: Revision,
    /// Active private command revision.
    revision: RevisionPin,
    /// Artifact computation session for this command.
    pub(super) session: Arc<Session>,
    /// Trace spanning the complete command operation.
    trace: Arc<Trace>,
    /// Optional progress event handler for command artifact runs.
    event_handler: Option<SessionEventHandler>,
    /// Command files retained for diagnostics without entering the module index.
    files: BTreeMap<FileId, Arc<File>>,
    /// Common command options.
    pub(super) common: &'a CommandOptions,
    /// Output buffer for command streaming.
    pub(super) output: &'a mut OutputBuffer,
}

impl std::fmt::Debug for CommandContext<'_> {
    /// Format the visible command context.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CommandContext")
            .field("root", &self.root)
            .field("cwd", &self.cwd)
            .field("repository", &self.repository)
            .field("base", &self.base)
            .field("revision", &self.revision)
            .field("session", &self.session)
            .field("trace", &self.trace)
            .field("event_handler", &self.event_handler.is_some())
            .field("files", &self.files)
            .field("common", &self.common)
            .finish_non_exhaustive()
    }
}

/// One selected command target.
#[derive(Debug, Clone)]
pub(super) struct SelectedTarget {
    /// The selected target id.
    pub id: TargetId,
    /// The selected target configuration.
    pub target: Target,
}

impl<'a> CommandContext<'a> {
    /// Create a command context for a request.
    pub(crate) fn new(
        workspace: &'a Workspace,
        repository: Arc<Repository>,
        common: &'a CommandOptions,
        revision: CommandRevision,
        output: &'a mut OutputBuffer,
        event_handler: Option<SessionEventHandler>,
    ) -> CommandResult<Self> {
        let root = workspace.root().to_path_buf();

        // select and privately configure the command revision
        let revision = Self::resolve_command_revision(workspace, revision)?;
        let base = revision.revision();
        let revision = Self::apply_overrides(&root, &repository, revision, &common.overrides)?;

        // bind the command to the shared workspace executor
        let cwd = common.cwd.clone().unwrap_or_else(|| root.clone());
        let session = workspace.session();

        // open the trace after command source preparation
        let trace_level = match common.trace {
            Some(_) => TraceLevel::Timings,
            None => TraceLevel::Disabled,
        };
        let trace = session.start_trace(trace_level);

        Ok(Self {
            workspace,
            root,
            cwd,
            repository,
            base,
            revision,
            session,
            trace,
            event_handler,
            files: BTreeMap::new(),
            common,
            output,
        })
    }

    /// Finish and snapshot this command trace.
    pub(crate) fn command_trace(
        &self,
        revision: Revision,
        view: TraceView,
    ) -> CommandResult<TraceSnapshot> {
        self.trace.finish();
        let report = self.trace.snapshot(
            view,
            |key| {
                self.repository
                    .artifact_display(revision, *key)
                    .map_err(|error| CommandError::internal(error.to_string()))
            },
            |target| {
                self.repository
                    .target_display(revision, target)
                    .map_err(|error| CommandError::internal(error.to_string()))
            },
        )?;

        Ok(report)
    }

    /// Return the trace spanning this command.
    pub(crate) fn trace(&self) -> Arc<Trace> {
        self.trace.clone()
    }

    /// Walk the imports of the roots and implicit globals and return the modules and read keys.
    pub(super) async fn import_closure(
        &self,
        revision: Revision,
        profile: ProfileId,
        roots: &[ModuleId],
    ) -> CommandResult<(Vec<ModuleId>, Vec<ArtifactKey>)> {
        // add the implicit globals to the roots
        let environment_key = ArtifactKey::environment_bound(profile);
        self.provide(revision, &[environment_key]).await?;
        let mut walk_roots = roots.to_vec();
        walk_roots.extend(
            ArtifactReader::new(self.repository.as_ref(), revision)
                .read::<EnvironmentBound>(profile)
                .map_err(|error| CommandError::internal(error.to_string()))?
                .implicit_modules(),
        );

        // collect the packages each root sees
        let mut packages = Vec::new();
        for root in &walk_roots {
            let closure = self
                .repository
                .package_closure(revision, root.package_id)
                .map_err(|error| CommandError::internal(error.to_string()))?;
            for package in closure {
                if !packages.contains(&package) {
                    packages.push(package);
                }
            }
        }

        // provide the module graph of each package
        let mut keys = packages
            .iter()
            .map(|package| ArtifactKey::module_graph(*package, profile))
            .collect::<Vec<_>>();
        self.provide(revision, &keys).await?;
        keys.push(environment_key);

        // walk the import edges
        let artifacts = ArtifactReader::new(self.repository.as_ref(), revision);
        let graphs = packages
            .iter()
            .map(|package| artifacts.read::<ModuleGraph>((*package, profile)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| CommandError::internal(error.to_string()))?;
        let modules = ModuleGraph::reachable_across(&graphs, &walk_roots).map_err(|module| {
            CommandError::internal(format!("no module graph holds module '{module}'"))
        })?;

        Ok((modules, keys))
    }

    /// Provide artifacts while recording into the command trace.
    pub(super) async fn provide(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> CommandResult<()> {
        let run = self.session.provide_traced(
            revision,
            artifact_keys,
            ArtifactPriority::Foreground,
            self.trace.clone(),
            self.event_handler.clone(),
        );

        run.wait().await.map_err(|error| error.to_string().into())
    }

    /// Complete artifacts through terminal outcomes while recording into the command trace.
    pub(super) async fn complete(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> CommandResult<()> {
        let run = self.session.provide_traced(
            revision,
            artifact_keys,
            ArtifactPriority::Foreground,
            self.trace.clone(),
            self.event_handler.clone(),
        );

        run.complete()
            .await
            .map_err(|error| error.to_string().into())
    }

    /// Return diagnostics emitted by the requested artifact roots.
    pub(super) fn command_diagnostics(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> CommandResult<DiagnosticCollection> {
        // include everything the requested roots were built from
        self.repository
            .diagnostics_for_keys(revision, artifact_keys)
            .map_err(|error| error.to_string().into())
    }

    /// Resolve the base revision for one command.
    fn resolve_command_revision(
        workspace: &Workspace,
        revision: CommandRevision,
    ) -> CommandResult<RevisionPin> {
        let revision = match revision {
            CommandRevision::Current => workspace.pin_physical(),
            CommandRevision::Exact(revision) => workspace.pin(revision),
        }
        .map_err(|error| {
            CommandError::internal(format!(
                "failed to select command revision for {}: {error}",
                workspace.root().display()
            ))
        })?;

        Ok(revision.into_revision())
    }

    /// Apply manifest overrides to one private command revision.
    fn apply_overrides(
        root: &Path,
        repository: &Arc<Repository>,
        revision: RevisionPin,
        overrides: &[ManifestOverride],
    ) -> CommandResult<RevisionPin> {
        if overrides.is_empty() {
            return Ok(revision);
        }

        // convert protocol-safe values into repository JSON overrides
        let overrides = overrides
            .iter()
            .map(ManifestOverride::to_repository)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| CommandError::config(format!("invalid manifest override: {error}")))?;

        // collect package manifests visible to this command
        let before = revision.revision();
        let manifests = Self::manifests(root, repository, before)?;
        let mut edits = Vec::with_capacity(manifests.len());

        // apply overrides to each manifest image
        for path in manifests {
            let mut manifest = Self::load_manifest_json(repository, before, &path)?;
            apply_manifest_overrides_to_json(&mut manifest, &overrides).map_err(|detail| {
                CommandError::config(format!("failed to update {}: {detail}", path.display()))
            })?;

            let content = serde_json::to_string_pretty(&manifest).map_err(|error| {
                CommandError::config(format!("failed to serialize {}: {error}", path.display()))
            })?;
            let content = format!("{content}\n");
            let logical_path = repository.logical_path(&path);
            let blob = repository
                .retain_blob(content.as_bytes())
                .map_err(|error| {
                    CommandError::internal(format!("failed to store {}: {error}", path.display()))
                })?;

            edits.push(repository::Edit::set_file(logical_path, blob));
        }

        let after = repository
            .edit(before, edits)
            .map_err(|error| {
                CommandError::internal(format!("failed to apply command config edits: {error}"))
            })?
            .after;

        repository.pin(after).map_err(|error| {
            CommandError::internal(format!("failed to pin command revision: {error}"))
        })
    }

    /// Return manifest paths visible to one command revision.
    fn manifests(
        root: &Path,
        repository: &Repository,
        revision: Revision,
    ) -> CommandResult<Vec<PathBuf>> {
        let mut paths = vec![root.join("destack.json")];

        // include package configs in monorepos
        for package_path in repository.package_roots(revision).map_err(|error| {
            CommandError::internal(format!("failed to read package roots: {error}"))
        })? {
            paths.push(package_path.join("destack.json"));
        }

        paths.sort();
        paths.dedup();

        Ok(paths)
    }

    /// Load one manifest JSON value from the revision or from the file system.
    fn load_manifest_json(
        repository: &Repository,
        revision: Revision,
        path: &Path,
    ) -> CommandResult<Value> {
        let file_id = repository.file_id(path);

        // prefer the file the revision binds
        if let Some(file) = repository.file(revision, file_id).map_err(|error| {
            CommandError::internal(format!("failed to read {}: {error}", path.display()))
        })? {
            return parse_jsonc_text(file.text()).map_err(|error| {
                CommandError::config(format!("failed to parse {}: {error}", path.display()))
            });
        }

        // read physical source when the config file is not tracked yet
        let content = match repository.file_system().read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Value::Object(Map::new()));
            }
            Err(error) => {
                return Err(CommandError::config(format!(
                    "failed to read {}: {error}",
                    path.display()
                )));
            }
        };

        parse_jsonc_text(&content).map_err(|error| {
            CommandError::config(format!("failed to parse {}: {error}", path.display()))
        })
    }

    /// Resolve command inputs from explicit values or destack.json.
    pub(super) fn resolve_command_inputs(&self) -> CommandResult<Vec<CommandInput>> {
        if !self.common.inputs.is_empty() {
            return Ok(self.common.inputs.clone());
        }

        if !self.common.config_inputs {
            return Err("no input files provided".to_string().into());
        }

        let config_path = self.resolve_destack_config_path(self.common.manifest.as_deref())?;
        let config = self.load_destack_config(&config_path)?;
        let configs = if config.workspace_packages().is_some() {
            self.workspace_configs(self.revision())?
        } else {
            vec![config]
        };

        // collect source files from each selected package
        let mut inputs = BTreeMap::new();
        for config in configs {
            let sources =
                collect_sources_from_destack_config(&config, self.common.target.as_deref())?;
            for source in sources {
                inputs.insert(source, ());
            }
        }

        if inputs.is_empty() {
            return Err("no input files provided".to_string().into());
        }

        Ok(inputs
            .into_keys()
            .map(|path| CommandInput::File { path })
            .collect())
    }

    /// Resolve command inputs into module ids.
    pub(super) fn resolve_modules(
        &mut self,
        inputs: &[CommandInput],
    ) -> CommandResult<Vec<ModuleId>> {
        let mut seen = HashSet::new();
        let mut modules = Vec::new();

        for input in inputs {
            let module_id = match input {
                CommandInput::File { path } => self.load_module(path)?,
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

    /// Return the active private command revision.
    pub(crate) fn revision(&self) -> Revision {
        self.revision.revision()
    }

    /// Load one filesystem module into this command revision.
    fn load_module(&mut self, path: &Path) -> CommandResult<ModuleId> {
        // resolve the requested path through this workspace
        let path = self.workspace.resolve_path(path).map_err(|error| {
            CommandError::source(format!("failed to resolve {}: {error}", path.display()))
        })?;

        // import the module into this command's private revision
        let (revision, module_id) = self
            .workspace
            .load_module(self.revision.clone(), &path)
            .map_err(|error| format!("failed to resolve {}: {error}", path.display()))?;
        self.revision = revision;

        Ok(module_id)
    }

    /// Materialize one inline command input into one command local revision.
    fn materialize_inline_module(
        &mut self,
        kind: &str,
        name: &str,
        content: &str,
        file_type: FileType,
    ) -> CommandResult<ModuleId> {
        // commit the virtual source into the private revision
        let path = PathBuf::from(command_input_logical_path(kind, name, file_type));
        let before = self.revision.revision();
        let logical_path = path.to_string_lossy();
        let blob = self
            .repository
            .retain_blob(content.as_bytes())
            .map_err(|error| format!("failed to store command input {name}: {error}"))?;
        let edit = repository::Edit::set_file(logical_path, blob);
        let after = self
            .repository
            .edit(before, vec![edit])
            .map_err(|error| format!("failed to materialize command input {name}: {error}"))?
            .after;

        // retain the new private revision
        let revision = self
            .repository
            .pin(after)
            .map_err(|error| format!("failed to pin command input {name}: {error}"))?;

        // require the virtual source to produce a module
        let module_id = self
            .repository
            .module_id_for_path(after, &path)
            .map_err(|error| format!("failed to resolve command input module {name}: {error}"))?
            .ok_or_else(|| {
                CommandError::source(format!(
                    "command input did not produce a module: {}",
                    path.display()
                ))
            })?;

        // publish the revision inside this command only
        self.revision = revision;

        Ok(module_id)
    }

    /// Add one in-memory command file retained for diagnostics.
    pub(super) fn add_memory_file(
        &mut self,
        path: &str,
        content: &str,
    ) -> CommandResult<Arc<File>> {
        let uri = Uri::memory(path);
        let file_id = FileId::from_logical_str(uri.as_ref());
        let path = Path::new(path);
        let name_and_type = path.file_name().zip(FileType::from_path(path));
        let Some((name, file_type)) = name_and_type else {
            return Err(CommandError::internal(format!(
                "command file path has no file name: {}",
                path.display()
            )));
        };
        let name = name.to_string_lossy().into_owned();
        let file = File::from_text(file_id, name, uri, None, file_type, content.to_string())
            .map_err(|error| CommandError::internal(error.to_string()))?;
        let file = Arc::new(file);
        self.files.insert(file_id, file.clone());

        Ok(file)
    }

    /// Return one command or repository file.
    pub(crate) fn file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> CommandResult<Option<Arc<File>>> {
        if let Some(file) = self.files.get(&file_id) {
            return Ok(Some(file.clone()));
        }

        self.repository
            .file(revision, file_id)
            .map_err(|error| CommandError::internal(error.to_string()))
    }

    /// Return file images referenced by one command result.
    pub(crate) fn file_images(
        &self,
        revision: Revision,
        diagnostics: &DiagnosticCollection,
        sources: &[Arc<File>],
    ) -> CommandResult<Vec<FileImage>> {
        let mut seen = HashSet::new();
        let mut files = Vec::new();

        // retain files referenced by command data
        for file in sources {
            if seen.insert(file.id) {
                files.push(FileImage::from(file.as_ref()));
            }
        }

        // collect every file referenced by labels and suggestion patches
        for diagnostic in diagnostics.iter() {
            let mut file_ids = vec![diagnostic.primary_label().target.file()];
            file_ids.extend(diagnostic.labels().map(|label| label.target.file()));
            file_ids.extend(
                diagnostic
                    .suggestions
                    .iter()
                    .flat_map(|suggestion| &suggestion.patches.files)
                    .map(|patch| patch.file),
            );

            for file_id in file_ids {
                if !seen.insert(file_id) {
                    continue;
                }

                let file = self.file(revision, file_id)?.ok_or_else(|| {
                    CommandError::internal(format!(
                        "diagnostic references missing source file {file_id:?}"
                    ))
                })?;
                files.push(FileImage::from(file.as_ref()));
            }
        }

        Ok(files)
    }

    /// Return the unique selected profile count for the provided modules.
    pub(super) fn selected_profile_count(
        &self,
        revision: Revision,
        modules: &[ModuleId],
    ) -> CommandResult<usize> {
        let mut profiles = HashSet::new();

        for module_id in modules {
            let profile_id = self.selected_profile_id(revision, *module_id)?;
            profiles.insert(profile_id);
        }

        Ok(profiles.len())
    }

    /// Return the profile id selected for one module.
    pub(super) fn selected_profile_id(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> CommandResult<ProfileId> {
        let target = self.resolve_target_for_module(revision, module_id, None)?;

        self.target_profile_id(revision, module_id, target.id)
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
            .profile_for_module_target(revision, module_id, target_id)
            .map_err(|error| format!("failed to resolve target profile: {error}"))?;

        Ok(profile.id())
    }

    /// Resolve a named target for a module.
    pub(super) fn resolve_named_target_for_module(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_name: &str,
        overrides: Option<&CommandTargetOverrides>,
    ) -> CommandResult<SelectedTarget> {
        // load the owning package for the module
        let module = self
            .repository
            .module(revision, module_id)
            .map_err(|error| format!("failed to read module snapshot: {error}"))?
            .ok_or_else(|| format!("missing module snapshot for {module_id:?}"))?;
        let package_id = module.package_id;
        let target_id = TargetId::new(package_id, target_name);

        let target_or_builtin = self
            .repository
            .target_or_builtin(revision, target_id)
            .map_err(|error| format!("failed to read target snapshot: {error}"))?;

        // resolve repository target or build a known built-in target
        let mut target = if let Some(target) = target_or_builtin {
            target
        } else {
            Target::builtin_for_name(target_name)
                .ok_or_else(|| format!("unknown target '{target_name}'"))?
        };

        // apply command output redirection
        if let Some(overrides) = overrides {
            overrides.apply_to_target(&mut target);
        }

        Ok(SelectedTarget {
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
    ) -> CommandResult<SelectedTarget> {
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
        if let Some((target_id, mut target)) = self
            .repository
            .package_default_target(revision, module.package_id)
            .map_err(|error| format!("failed to read target snapshot: {error}"))?
        {
            if let Some(overrides) = overrides {
                overrides.apply_to_target(&mut target);
            }

            return Ok(SelectedTarget {
                id: target_id,
                target,
            });
        }

        // infer a default target when no explicit configuration exists
        let target_name = if module.is_code() { "bytecode" } else { "js" };

        self.resolve_named_target_for_module(revision, module_id, target_name, overrides)
    }

    /// Resolve a destack.json path for the current repository.
    pub(super) fn resolve_destack_config_path(
        &self,
        override_path: Option<&Path>,
    ) -> CommandResult<PathBuf> {
        let revision = self.revision();

        // honor explicit config paths first
        if let Some(config_path) = override_path {
            return self.resolve_manifest_override(revision, config_path);
        }

        self.find_destack_config(self.root.as_path())
            .ok_or_else(|| "destack.json not found".to_string().into())
    }

    /// Load one `destack.json` config for a path.
    pub(super) fn load_destack_config(&self, path: &Path) -> CommandResult<DestackFile> {
        let revision = self.revision();

        self.repository
            .inherited_destack_for_path(revision, path)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("destack.json not found: {}", path.display()).into())
    }

    /// Find destack.json for a directory.
    pub(super) fn find_destack_config(&self, cwd: &Path) -> Option<PathBuf> {
        let revision = self.revision();

        self.find_destack_config_in_revision(revision, cwd)
    }

    /// Return all authored package configurations in path order.
    pub(super) fn workspace_configs(&self, revision: Revision) -> CommandResult<Vec<DestackFile>> {
        let package_ids = self
            .repository
            .package_ids(revision)
            .map_err(|error| error.to_string())?;
        let mut configs = Vec::new();

        // retain configurations for authored packages
        for package_id in package_ids {
            let Some(package) = self
                .repository
                .package(revision, package_id)
                .map_err(|error| error.to_string())?
            else {
                return Err(CommandError::internal(format!(
                    "workspace package is missing: {package_id:?}"
                )));
            };
            if package.kind.is_authored()
                && let Some(config) = package.configuration.as_deref()
            {
                configs.push(config.clone());
            }
        }

        configs.sort_by(|left, right| left.path.cmp(&right.path));

        Ok(configs)
    }

    /// Return a standard unimplemented command response.
    pub(super) fn unimplemented_command(
        &mut self,
        message: &str,
    ) -> CommandResult<CommandOutcome<CommandMessagePayload>> {
        self.output.push_stderr(format!("{message}\n").into_bytes());

        Ok(CommandOutcome::unimplemented(message))
    }

    /// Resolve an explicit manifest path override.
    fn resolve_manifest_override(
        &self,
        revision: Revision,
        config_path: &Path,
    ) -> CommandResult<PathBuf> {
        // resolve the manifest inside this workspace
        let resolved = self.workspace.resolve_path(config_path).map_err(|error| {
            CommandError::config(format!(
                "failed to resolve manifest {}: {error}",
                config_path.display()
            ))
        })?;

        // handle directory overrides
        let metadata = self
            .repository
            .file_metadata(revision, &resolved)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "destack.json not found".to_string())?;
        if metadata.is_directory {
            return self
                .find_destack_config_in_revision(revision, &resolved)
                .ok_or_else(|| "destack.json not found".to_string().into());
        }

        Ok(resolved)
    }

    /// Find the nearest `destack.json` at or above a path in one revision.
    fn find_destack_config_in_revision(&self, revision: Revision, cwd: &Path) -> Option<PathBuf> {
        let cwd = self.workspace.resolve_path(cwd).ok()?;
        let mut directory = if self
            .repository
            .file_metadata(revision, &cwd)
            .ok()
            .flatten()
            .is_some_and(|metadata| metadata.is_directory)
        {
            cwd
        } else {
            cwd.parent()?.to_path_buf()
        };

        loop {
            // check the current directory
            let candidate = directory.join("destack.json");
            if self
                .repository
                .file_metadata(revision, &candidate)
                .ok()
                .flatten()
                .is_some_and(|metadata| metadata.is_file)
            {
                return Some(candidate);
            }

            // stop after checking the workspace root
            if directory == self.root {
                return None;
            }

            directory = directory.parent()?.to_path_buf();
        }
    }
}

/// Build one stable logical path for one command local source input.
fn command_input_logical_path(kind: &str, name: &str, file_type: FileType) -> String {
    let extension = file_type.extension().unwrap_or("txt");
    let sanitized_name = sanitize_command_input_name(name);

    format!(".tspp/command/{kind}/{sanitized_name}.{extension}")
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

/// Collect source entries from a command destack.json.
fn collect_sources_from_destack_config(
    config: &DestackFile,
    target_name: Option<&str>,
) -> CommandResult<Vec<PathBuf>> {
    // select target source settings
    let selected_target = target_name
        .map(str::to_string)
        .or_else(|| config.default_target.clone());
    let target_options = selected_target
        .as_deref()
        .and_then(|name| config.targets.get(name));

    // derive entries and include rules
    let (entries, includes, excludes, root) = if let Some(target) = target_options {
        let includes = if target.include.is_empty() {
            config.include.clone()
        } else {
            target.include.clone()
        };
        let mut excludes = config.exclude.clone();
        excludes.extend(target.exclude.iter().cloned());
        (target.entry.clone(), includes, excludes, target.root())
    } else {
        (
            Vec::new(),
            config.include.clone(),
            config.exclude.clone(),
            TargetRoot::Include,
        )
    };

    let base_dir = config.directory.clone();
    let mut paths = BTreeMap::new();

    // honor entry roots first
    if root == TargetRoot::Entry {
        for entry in entries {
            let path = if entry.is_absolute() {
                entry
            } else {
                base_dir.join(entry)
            };
            paths.insert(path, ());
        }
    }

    // fall back to explicit files
    if paths.is_empty() && !config.files.is_empty() {
        for file in &config.files {
            let path = base_dir.join(file);
            paths.insert(path, ());
        }
    }

    // fall back to include patterns
    if paths.is_empty() {
        let patterns = if includes.is_empty() {
            vec!["**/*.tspp".to_string()]
        } else {
            includes
        };

        let include_paths = expand_patterns(&base_dir, &patterns)?;
        let exclude_paths = expand_patterns(&base_dir, &excludes)?;
        let excludes: HashSet<PathBuf> = exclude_paths.into_iter().collect();
        for path in include_paths {
            if !excludes.contains(&path) {
                paths.insert(path, ());
            }
        }
    }

    Ok(paths.into_keys().collect())
}

/// Expand path patterns relative to a base directory.
fn expand_patterns(base_dir: &Path, patterns: &[String]) -> CommandResult<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for pattern in patterns {
        let pattern_path = if PathBuf::from(pattern).is_absolute() {
            PathBuf::from(pattern)
        } else {
            base_dir.join(pattern)
        };

        let entries = glob(pattern_path.to_string_lossy().as_ref()).map_err(|error| {
            CommandError::config(format!(
                "failed to expand source pattern {}: {error}",
                pattern_path.display()
            ))
        })?;
        paths.extend(entries);
    }

    Ok(paths)
}

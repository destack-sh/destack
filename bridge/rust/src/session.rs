use std::error;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact as artifact;
use destack_bridge_language as bridge;
use destack_compiler as compiler;
use destack_formatter as formatter;
use destack_linter as linter;
use destack_query as query;
use destack_repository as repository;
use destack_session as session;
use destack_source::{self as source, File, FileId, FileSystem, Uri};

use crate::{BuildRequest, Document, FormatRequest, LintRequest, Module, Repository, Scope};

/// Result returned by the Rust bridge facade.
pub type Result<T> = std::result::Result<T, Error>;

/// Error returned by the Rust bridge facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// Error message.
    message: String,
}

/// Live language session exposed to Rust clients.
#[derive(Debug, Clone)]
pub struct Session {
    /// Live language session.
    pub(crate) session: Arc<session::Session>,
}

impl Error {
    /// Create one bridge error from a displayable error.
    pub(crate) fn new(error: impl ToString) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

impl Display for Error {
    /// Format this bridge error.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl error::Error for Error {}

impl Session {
    /// Wrap one live language session.
    pub(crate) fn from_session(session: Arc<session::Session>) -> Self {
        Self { session }
    }

    /// Open one session from one source input.
    pub fn open(source: bridge::Source) -> Result<Self> {
        let repository = Repository::open(source)?;

        repository.session()
    }

    /// Return the current session revision.
    pub fn revision(&self) -> Result<repository::Revision> {
        let revision = self
            .session
            .revision(self.session.head())
            .map_err(Error::new)?;

        Ok(revision)
    }

    /// Return editable repository file paths at the current revision.
    pub fn files(&self) -> Result<Vec<bridge::SessionFile>> {
        let repository = self.session.repository();
        let revision = self
            .session
            .revision(self.session.head())
            .map_err(Error::new)?;
        let mut paths = repository
            .editable_file_logical_paths(revision)
            .map_err(Error::new)?
            .into_iter()
            .map(|(_, path)| repository.string_pool().get(path).to_string())
            .collect::<Vec<_>>();

        paths.sort();

        let files = paths.into_iter().map(bridge::SessionFile::new).collect();

        Ok(files)
    }

    /// Edit files through the default session ref.
    pub fn edit(&self, edits: Vec<bridge::Edit>) -> Result<bridge::Commit> {
        let edits = edits
            .into_iter()
            .map(session::Edit::try_from)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::new)?;
        let result = self
            .session
            .edit(self.session.head(), edits)
            .map_err(Error::new)?;

        Ok(bridge::Commit::from_session_commit(&self.session, result))
    }

    /// Edit files when the default session ref still points at one revision.
    pub fn edit_if_current(
        &self,
        revision: repository::Revision,
        edits: Vec<bridge::Edit>,
    ) -> Result<bridge::Commit> {
        let edits = edits
            .into_iter()
            .map(session::Edit::try_from)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::new)?;
        let result = self
            .session
            .edit_if_current(self.session.head(), revision, edits)
            .map_err(Error::new)?;

        Ok(bridge::Commit::from_session_commit(&self.session, result))
    }

    /// Reload tracked files from this session backing source.
    pub fn reload(&self) -> Result<Vec<bridge::Change>> {
        let changes = self
            .session
            .reload_from_fs(self.session.head())
            .map_err(Error::new)?;
        let changes = changes
            .into_iter()
            .map(|change| bridge::Change::from_session_change(&self.session, change))
            .collect();

        Ok(changes)
    }

    /// Return one module path in the default session ref.
    pub fn module(&self, path: impl AsRef<Path>) -> Result<Module> {
        let module = self
            .session
            .load_module_from_fs(self.session.head(), path.as_ref())
            .map_err(Error::new)?;

        Ok(Module::new(module))
    }

    /// Return one named target in one package.
    pub fn target(
        &self,
        revision: repository::Revision,
        package: source::PackageId,
        name: impl AsRef<str>,
    ) -> Result<source::TargetId> {
        let name = name.as_ref();
        let target = source::TargetId::new(package, name);
        let repository = self.session.repository();

        // require the target to exist in config or builtins
        repository
            .target_or_builtin(revision, target)
            .map_err(Error::new)?
            .ok_or_else(|| Error::new(format!("target is missing: {name}")))?;

        Ok(target)
    }

    /// Return the semantic profile selected by one module target name.
    pub fn profile(
        &self,
        revision: repository::Revision,
        module: Module,
        name: impl AsRef<str>,
    ) -> Result<source::ProfileId> {
        let module = module.id;
        let target = source::TargetId::new(module.package_id, name.as_ref());
        let profile = self
            .session
            .repository()
            .profile_for_module_target(revision, module, target)
            .map_err(Error::new)?;

        Ok(profile.id())
    }

    /// Provide root artifacts for one immutable revision.
    pub fn provide(
        &self,
        revision: repository::Revision,
        keys: Vec<artifact::ArtifactKey>,
    ) -> Result<()> {
        self.session.provide(revision, &keys).map_err(Error::new)
    }

    /// Require one root artifact for one immutable revision.
    pub fn require(
        &self,
        revision: repository::Revision,
        key: artifact::ArtifactKey,
    ) -> Result<artifact::ArtifactVersion> {
        let version = self.session.require(revision, key).map_err(Error::new)?;

        Ok(version)
    }

    /// Return one raw artifact record for one immutable revision.
    pub fn artifact_record(
        &self,
        revision: repository::Revision,
        key: artifact::ArtifactKey,
    ) -> Result<bridge::ArtifactRecord> {
        let version = self.session.require(revision, key).map_err(Error::new)?;
        let repository = self.session.repository();
        let record = repository
            .artifact_table()
            .record(&version, repository.string_pool())
            .map_err(Error::new)?
            .ok_or_else(|| Error::new(format!("artifact record is missing for {version:?}")))?;

        Ok(bridge::ArtifactRecord::from_artifact(
            record,
            repository.string_pool(),
        ))
    }

    /// Return the trace report for the latest completed artifact run.
    pub fn trace(
        &self,
        revision: repository::Revision,
        detailed: bool,
    ) -> Result<Option<bridge::TraceReport>> {
        let Some(trace) = self.session.last_trace() else {
            return Ok(None);
        };
        let repository = self.session.repository();

        // label artifact keys through the requested revision
        let report = trace.snapshot(
            detailed,
            |key| {
                key.module_id()
                    .and_then(|module| repository.module_display(revision, module).ok().flatten())
            },
            |target| repository.target_display(revision, target).ok().flatten(),
        );

        Ok(Some(bridge::TraceReport::from_repository(report)))
    }

    /// Build one typed language output for one immutable revision.
    pub fn build(
        &self,
        revision: repository::Revision,
        request: BuildRequest,
    ) -> Result<bridge::BuildOutput> {
        let key = self.build_key(revision, request)?;
        let version = self.session.require(revision, key).map_err(Error::new)?;
        let output = self.build_output(revision, version, key)?;

        Ok(output)
    }

    /// Return one shared content payload by exact content id.
    pub fn content(&self, id: source::ContentId) -> Result<bridge::Content> {
        let content = self
            .session
            .repository()
            .content(id)
            .map_err(Error::new)?
            .payload()
            .clone();

        Ok(content.into())
    }

    /// Return one text content payload by exact content id.
    pub fn text(&self, id: source::ContentId) -> Result<String> {
        let content = self.content(id)?;

        match content {
            bridge::Content::Text { content } => Ok(content),
            bridge::Content::Binary { .. } => Err(Error::new("content payload is binary")),
        }
    }

    /// Return one binary content payload by exact content id.
    pub fn bytes(&self, id: source::ContentId) -> Result<Vec<u8>> {
        let content = self.content(id)?;

        match content {
            bridge::Content::Binary { content } => Ok(content),
            bridge::Content::Text { .. } => Err(Error::new("content payload is text")),
        }
    }

    /// Parse one loaded module.
    pub fn parse(
        &self,
        revision: repository::Revision,
        module: Module,
    ) -> Result<bridge::ParseOutput> {
        let module_id = module.id;

        // require the parsed artifact for this module
        let key = artifact::ArtifactKey::DirParsed { module: module_id };
        let version = self.session.require(revision, key).map_err(Error::new)?;
        let repository = self.session.repository();
        let parsed = repository
            .artifact_table()
            .dir_parsed(&version)
            .ok_or_else(|| Error::new(format!("parsed DIR artifact is missing for {version:?}")))?;
        let parsed = bridge::DirParsed::from_artifact(version, module_id.into(), parsed.as_ref());

        // read diagnostics from the parsed artifact
        let diagnostic_key = artifact::ArtifactKey::DirParsed { module: module_id };
        let diagnostics = repository
            .diagnostics(revision, Some(diagnostic_key))
            .map_err(Error::new)?;
        let diagnostics = diagnostics
            .to_vec()
            .into_iter()
            .map(bridge::Diagnostic::from_source)
            .collect();

        Ok(bridge::ParseOutput {
            parsed,
            diagnostics,
        })
    }

    /// Return the resolved DIR artifact for one loaded module profile.
    pub fn resolve(
        &self,
        revision: repository::Revision,
        module: Module,
        profile: source::ProfileId,
    ) -> Result<bridge::DirResolved> {
        let module_id = module.id;
        let key = artifact::ArtifactKey::DirResolved {
            module: module_id,
            profile,
        };
        let version = self.session.require(revision, key).map_err(Error::new)?;
        let repository = self.session.repository();
        let resolved = repository
            .artifact_table()
            .dir_resolved(&version)
            .ok_or_else(|| {
                Error::new(format!("resolved DIR artifact is missing for {version:?}"))
            })?;

        Ok(bridge::DirResolved::from_artifact(
            version,
            module_id.into(),
            profile.into(),
            resolved.as_ref(),
        ))
    }

    /// Check one loaded module profile.
    pub fn check(
        &self,
        revision: repository::Revision,
        module: Module,
        profile: source::ProfileId,
    ) -> Result<bridge::CheckOutput> {
        let module_id = module.id;

        // require the checked facade artifact for this module
        let key = artifact::ArtifactKey::DirChecked {
            module: module_id,
            profile,
        };
        let version = self.session.require(revision, key).map_err(Error::new)?;
        let repository = self.session.repository();
        let store = repository.artifact_table();
        let checked = store.dir_checked(&version).ok_or_else(|| {
            Error::new(format!("checked DIR artifact is missing for {version:?}"))
        })?;
        let checked_output = bridge::DirChecked::from_artifact(
            version,
            module_id.into(),
            profile.into(),
            checked.as_ref(),
        );

        // read diagnostics from the component artifact that owns the checked module
        let diagnostic_key = artifact::ArtifactKey::DirCheckedComponent {
            entry: checked.entry,
            component: checked.component,
            profile,
        };
        let diagnostics = self
            .session
            .repository()
            .diagnostics(revision, Some(diagnostic_key))
            .map_err(Error::new)?;
        let diagnostics = diagnostics
            .to_vec()
            .into_iter()
            .map(bridge::Diagnostic::from_source)
            .collect();

        Ok(bridge::CheckOutput {
            checked: checked_output,
            diagnostics,
        })
    }

    /// Format one document for one immutable revision.
    pub fn format(
        &self,
        revision: repository::Revision,
        request: FormatRequest,
    ) -> Result<bridge::FormatOutput> {
        let text = self.format_document(revision, request.document)?;

        Ok(bridge::FormatOutput { text })
    }

    /// Lint one scope for one immutable revision.
    pub fn lint(
        &self,
        revision: repository::Revision,
        request: LintRequest,
    ) -> Result<bridge::LintOutput> {
        let key = Self::lint_key(request.scope)?;
        let _version = self.session.require(revision, key).map_err(Error::new)?;
        let diagnostics = self
            .session
            .repository()
            .diagnostics(revision, Some(key))
            .map_err(Error::new)?;
        let diagnostics = diagnostics
            .to_vec()
            .into_iter()
            .map(bridge::Diagnostic::from_source)
            .collect();

        Ok(bridge::LintOutput { diagnostics })
    }

    /// Return diagnostics for one immutable revision.
    pub fn diagnostics(
        &self,
        revision: repository::Revision,
        key: Option<artifact::ArtifactKey>,
    ) -> Result<Vec<bridge::Diagnostic>> {
        let diagnostics = self
            .session
            .repository()
            .diagnostics(revision, key)
            .map_err(Error::new)?;
        let diagnostics = diagnostics
            .to_vec()
            .into_iter()
            .map(bridge::Diagnostic::from_source)
            .collect();

        Ok(diagnostics)
    }

    /// Return sidecars for one artifact key in one immutable revision.
    pub fn sidecars(
        &self,
        revision: repository::Revision,
        key: artifact::ArtifactKey,
    ) -> Result<Vec<bridge::ArtifactSidecar>> {
        let sidecars = self
            .session
            .repository()
            .artifact_sidecars(revision, key)
            .map_err(Error::new)?;
        let sidecars = sidecars
            .iter()
            .cloned()
            .map(bridge::ArtifactSidecar::from_artifact)
            .collect();

        Ok(sidecars)
    }

    /// Resolve one build request to its canonical artifact key.
    fn build_key(
        &self,
        revision: repository::Revision,
        request: BuildRequest,
    ) -> Result<artifact::ArtifactKey> {
        match request {
            BuildRequest::Module {
                module,
                target,
                output,
            } => match output {
                bridge::ModuleBuildKind::Script => {
                    Ok(artifact::ArtifactKey::script(module.id, target))
                }
                bridge::ModuleBuildKind::Object => {
                    Ok(artifact::ArtifactKey::object(module.id, target))
                }
                bridge::ModuleBuildKind::Asset => {
                    Ok(artifact::ArtifactKey::asset(module.id, target))
                }
            },
            BuildRequest::Build { target } => Ok(artifact::ArtifactKey::build(target)),
            BuildRequest::Target { target } => {
                let package = target.package_id();
                let target_config = self
                    .session
                    .repository()
                    .target_or_builtin(revision, target)
                    .map_err(Error::new)?
                    .ok_or(repository::RepositoryError::MissingTarget { target })
                    .map_err(Error::new)?;

                if target_config.emit.is_js_family() {
                    Ok(artifact::ArtifactKey::bundle(package, target))
                } else if target_config.emit.is_native_family() {
                    Ok(artifact::ArtifactKey::program(package, target))
                } else {
                    Err(Error::new(format!(
                        "unsupported target emit format: {}",
                        target_config.emit.canonical_tag()
                    )))
                }
            }
            BuildRequest::Product { product } => {
                let package = product.package_id();

                Ok(artifact::ArtifactKey::product(package, product))
            }
        }
    }

    /// Project one built artifact into one bridge build output.
    fn build_output(
        &self,
        revision: repository::Revision,
        version: artifact::ArtifactVersion,
        key: artifact::ArtifactKey,
    ) -> Result<bridge::BuildOutput> {
        let repository = self.session.repository();
        let artifacts = repository.artifact_reader(revision);

        match key {
            artifact::ArtifactKey::Script { module, target } => {
                let script = artifacts.script(module, target).map_err(Error::new)?;

                Ok(bridge::BuildOutput::Script {
                    version: version.into(),
                    script: script.as_ref().into(),
                })
            }
            artifact::ArtifactKey::Object { module, target } => {
                let object = artifacts.object(module, target).map_err(Error::new)?;

                Ok(bridge::BuildOutput::Object {
                    version: version.into(),
                    object: object.as_ref().into(),
                })
            }
            artifact::ArtifactKey::Asset { module, target } => {
                let asset = artifacts.asset(module, target).map_err(Error::new)?;

                Ok(bridge::BuildOutput::Asset {
                    version: version.into(),
                    asset: asset.as_ref().into(),
                })
            }
            artifact::ArtifactKey::Build { target } => {
                let build = artifacts.build(target).map_err(Error::new)?;

                Ok(bridge::BuildOutput::Build {
                    version: version.into(),
                    build: build.as_ref().into(),
                })
            }
            artifact::ArtifactKey::Bundle { package, target } => {
                let bundle = artifacts.bundle(package, target).map_err(Error::new)?;

                Ok(bridge::BuildOutput::Bundle {
                    version: version.into(),
                    bundle: bundle.as_ref().into(),
                })
            }
            artifact::ArtifactKey::Program { package, target } => {
                let program = artifacts.program(package, target).map_err(Error::new)?;

                Ok(bridge::BuildOutput::Program {
                    version: version.into(),
                    program: program.as_ref().into(),
                })
            }
            artifact::ArtifactKey::Product { package, product } => {
                let product = artifacts.product(package, product).map_err(Error::new)?;

                Ok(bridge::BuildOutput::Product {
                    version: version.into(),
                    product: product.as_ref().into(),
                })
            }
            artifact_key => Err(Error::new(format!(
                "artifact key is not a build output: {artifact_key:?}"
            ))),
        }
    }

    /// Format one bridge document.
    fn format_document(
        &self,
        revision: repository::Revision,
        document: Document,
    ) -> Result<String> {
        match document {
            Document::Module { module } => self.format_module(revision, module),
            Document::Text { path, text } => Self::format_text(path, text),
        }
    }

    /// Format one loaded module from repository source.
    fn format_module(&self, revision: repository::Revision, module: Module) -> Result<String> {
        let module_id = module.id;
        let repository = self.session.repository();
        let module = repository
            .module(revision, module_id)
            .map_err(Error::new)?
            .ok_or_else(|| Error::new(format!("module is missing: {module_id:?}")))?;
        let file = repository
            .file(revision, module.file_id)
            .map_err(Error::new)?
            .ok_or_else(|| Error::new(format!("module file is missing: {:?}", module.file_id)))?;
        let options = repository
            .destack_for_file(revision, module.file_id)
            .map_err(Error::new)?
            .map(|config| config.formatter)
            .unwrap_or_default();

        formatter::format_file_source(file.as_ref(), file.text(), options).map_err(Error::new)
    }

    /// Format one ad hoc text document.
    fn format_text(path: String, text: String) -> Result<String> {
        let path = PathBuf::from(path);
        let file_type = destack_source::FileType::from_path_or_unknown(&path);
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let file = File::from_text(
            FileId::from_logical_path(&path),
            name,
            Uri::from_path(&path),
            Some(path),
            file_type,
            text,
        );

        formatter::format_file_source(&file, file.text(), repository::FormatterOptions::default())
            .map_err(Error::new)
    }

    /// Resolve one lint request to its canonical artifact key.
    fn lint_key(scope: Scope) -> Result<artifact::ArtifactKey> {
        match scope {
            Scope::Module { module, profile } => {
                Ok(artifact::ArtifactKey::module_linted(module.id, profile))
            }
            Scope::Package { package } => Ok(artifact::ArtifactKey::package_linted(package)),
            Scope::Workspace => Ok(artifact::ArtifactKey::workspace_linted()),
        }
    }

    /// Open one Rust session from one prepared repository.
    pub(crate) fn from_repository(repository: Arc<repository::Repository>) -> Result<Self> {
        let root = repository.path().to_path_buf();
        let head = repository::Ref::for_root(&root);
        let compiler = Arc::new(compiler::Compiler::new(Arc::clone(&repository)));
        let linter = Arc::new(linter::Linter::new(Arc::clone(&repository)));
        let query = Arc::new(query::Query::new(Arc::clone(&repository)));
        let worker_count = session::Session::default_worker_count();
        let session = session::Session::new(
            root.clone(),
            root,
            repository,
            head,
            compiler,
            linter,
            query,
            worker_count,
            None,
        )
        .map_err(Error::new)?;
        let session = Arc::new(session);

        Ok(Self { session })
    }

    /// Open one repository from one filesystem source.
    pub(crate) fn open_repository_from_file_system(
        path: PathBuf,
        file_system: Arc<dyn FileSystem>,
    ) -> Result<repository::Repository> {
        session::open_repository_from_fs(
            path,
            file_system,
            repository::Environment::default(),
            repository::Settings::default(),
            repository::DestackLayoutOverride::default(),
        )
        .map_err(Error::new)
    }
}

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
use destack_source::{File, FileId, FileSystem, PhysicalFileSystem, Uri};

/// Result returned by the Rust bridge facade.
pub type Result<T> = std::result::Result<T, Error>;

/// Error returned by the Rust bridge facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// Error message.
    message: String,
}

/// Live language session exposed to Rust clients.
#[derive(Debug)]
pub struct Session {
    /// Live language session.
    session: session::Session,
}

impl Error {
    /// Create one bridge error from a displayable error.
    fn new(error: impl ToString) -> Self {
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
    /// Open one session from one source input.
    pub fn open(source: bridge::Source) -> Result<Self> {
        let repository = match source {
            bridge::Source::FileSystem { path } => Self::open_repository_from_file_system(
                PathBuf::from(path),
                Arc::new(PhysicalFileSystem::new()),
            )?,
            bridge::Source::Memory { root, edits } => {
                let edits = edits
                    .into_iter()
                    .map(session::Edit::try_from)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Error::new)?;

                session::open_repository_from_memory(
                    PathBuf::from(root),
                    edits,
                    repository::Environment::default(),
                    repository::Settings::default(),
                    repository::DestackLayoutOverride::default(),
                )
                .map_err(Error::new)?
            }
        };

        Self::from_repository(repository)
    }

    /// Return the current session revision.
    pub fn revision(&self) -> Result<bridge::Revision> {
        let revision = self
            .session
            .revision(self.session.head())
            .map_err(Error::new)?;

        Ok(bridge::Revision::from_repository(revision))
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
    pub fn edit_at(
        &self,
        revision: bridge::Revision,
        edits: Vec<bridge::Edit>,
    ) -> Result<bridge::Commit> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let edits = edits
            .into_iter()
            .map(session::Edit::try_from)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::new)?;
        let result = self
            .session
            .edit_at(self.session.head(), revision, edits)
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

    /// Load one module path into the default session ref.
    pub fn load_module(&self, path: impl AsRef<Path>) -> Result<bridge::Module> {
        let module = self
            .session
            .load_module_from_fs(self.session.head(), path.as_ref())
            .map_err(Error::new)?;

        Ok(bridge::Module::new(module.into()))
    }

    /// Provide root artifacts for one immutable revision.
    pub fn provide(
        &self,
        revision: bridge::Revision,
        keys: Vec<bridge::ArtifactKey>,
    ) -> Result<()> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let keys = keys
            .into_iter()
            .map(bridge::ArtifactKey::into_artifact)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::new)?;

        self.session.provide(revision, &keys).map_err(Error::new)
    }

    /// Require one root artifact for one immutable revision.
    pub fn require(
        &self,
        revision: bridge::Revision,
        key: bridge::ArtifactKey,
    ) -> Result<bridge::ArtifactVersion> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let key = key.into_artifact().map_err(Error::new)?;
        let version = self.session.require(revision, key).map_err(Error::new)?;

        Ok(bridge::ArtifactVersion::from_artifact(version))
    }

    /// Return one raw artifact record for one immutable revision.
    pub fn artifact_record(
        &self,
        revision: bridge::Revision,
        key: bridge::ArtifactKey,
    ) -> Result<bridge::ArtifactRecord> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let key = key.into_artifact().map_err(Error::new)?;
        let version = self.session.require(revision, key).map_err(Error::new)?;
        let repository = self.session.repository();
        let record = repository
            .artifact_cache()
            .record(&version, repository.string_pool())
            .map_err(Error::new)?
            .ok_or_else(|| Error::new(format!("artifact record is missing for {version:?}")))?;

        Ok(bridge::ArtifactRecord::from_artifact(record))
    }

    /// Build one typed language output for one immutable revision.
    pub fn build(
        &self,
        revision: bridge::Revision,
        request: bridge::BuildRequest,
    ) -> Result<bridge::BuildOutput> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let key = self.build_key(revision, request)?;
        let version = self
            .session
            .require(revision, key.clone())
            .map_err(Error::new)?;
        let output = self.build_output(revision, version, key)?;

        Ok(output)
    }

    /// Return one shared content payload by exact content id.
    pub fn content(&self, id: bridge::ContentId) -> Result<bridge::Content> {
        let id = id.into_source().map_err(Error::new)?;
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
    pub fn text(&self, id: bridge::ContentId) -> Result<String> {
        let content = self.content(id)?;

        match content {
            bridge::Content::Text { content } => Ok(content),
            bridge::Content::Binary { .. } => Err(Error::new("content payload is binary")),
        }
    }

    /// Return one binary content payload by exact content id.
    pub fn bytes(&self, id: bridge::ContentId) -> Result<Vec<u8>> {
        let content = self.content(id)?;

        match content {
            bridge::Content::Binary { content } => Ok(content),
            bridge::Content::Text { .. } => Err(Error::new("content payload is text")),
        }
    }

    /// Return the parsed DIR artifact for one loaded module.
    pub fn parse(
        &self,
        revision: bridge::Revision,
        module: bridge::Module,
    ) -> Result<bridge::DirParsed> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let module_id = module.id.clone();
        let module = module.id.into_source().map_err(Error::new)?;
        let key = bridge::ArtifactKey::DirParsed {
            module: module_id.clone(),
        };
        let key = key.into_artifact().map_err(Error::new)?;
        let version = self.session.require(revision, key).map_err(Error::new)?;
        let repository = self.session.repository();
        let parsed = repository
            .artifact_cache()
            .dir_parsed(&version)
            .ok_or_else(|| Error::new(format!("parsed DIR artifact is missing for {version:?}")))?;

        Ok(bridge::DirParsed::from_artifact(
            version,
            module.into(),
            parsed.as_ref(),
        ))
    }

    /// Return the resolved DIR artifact for one loaded module profile.
    pub fn resolve(
        &self,
        revision: bridge::Revision,
        module: bridge::Module,
        profile: bridge::ProfileId,
    ) -> Result<bridge::DirResolved> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let module_id = module.id.clone();
        let profile_id = profile.clone();
        let module = module.id.into_source().map_err(Error::new)?;
        let profile = profile.into_source().map_err(Error::new)?;
        let key = bridge::ArtifactKey::DirResolved {
            module: module_id,
            profile: profile_id,
        };
        let key = key.into_artifact().map_err(Error::new)?;
        let version = self.session.require(revision, key).map_err(Error::new)?;
        let repository = self.session.repository();
        let resolved = repository
            .artifact_cache()
            .dir_resolved(&version)
            .ok_or_else(|| {
                Error::new(format!("resolved DIR artifact is missing for {version:?}"))
            })?;

        Ok(bridge::DirResolved::from_artifact(
            version,
            module.into(),
            profile.into(),
            resolved.as_ref(),
        ))
    }

    /// Return the checked DIR facade artifact for one loaded module profile.
    pub fn check(
        &self,
        revision: bridge::Revision,
        module: bridge::Module,
        profile: bridge::ProfileId,
    ) -> Result<bridge::DirChecked> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let module_id = module.id.clone();
        let profile_id = profile.clone();
        let module = module.id.into_source().map_err(Error::new)?;
        let profile = profile.into_source().map_err(Error::new)?;
        let key = bridge::ArtifactKey::DirChecked {
            module: module_id,
            profile: profile_id,
        };
        let key = key.into_artifact().map_err(Error::new)?;
        let version = self.session.require(revision, key).map_err(Error::new)?;
        let repository = self.session.repository();
        let store = repository.artifact_cache();
        let checked = store.dir_checked(&version).ok_or_else(|| {
            Error::new(format!("checked DIR artifact is missing for {version:?}"))
        })?;

        Ok(bridge::DirChecked::from_artifact(
            version,
            module.into(),
            profile.into(),
            checked.as_ref(),
        ))
    }

    /// Format one document for one immutable revision.
    pub fn format(
        &self,
        revision: bridge::Revision,
        request: bridge::FormatRequest,
    ) -> Result<bridge::FormatOutput> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let text = self.format_document(revision, request.document)?;

        Ok(bridge::FormatOutput { text })
    }

    /// Lint one scope for one immutable revision.
    pub fn lint(
        &self,
        revision: bridge::Revision,
        request: bridge::LintRequest,
    ) -> Result<bridge::LintOutput> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let key = Self::lint_key(request.scope)?;
        let _version = self
            .session
            .require(revision, key.clone())
            .map_err(Error::new)?;
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
        revision: bridge::Revision,
        key: Option<bridge::ArtifactKey>,
    ) -> Result<Vec<bridge::Diagnostic>> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let key = key
            .map(bridge::ArtifactKey::into_artifact)
            .transpose()
            .map_err(Error::new)?;
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
        revision: bridge::Revision,
        key: bridge::ArtifactKey,
    ) -> Result<Vec<bridge::ArtifactSidecar>> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let key = key.into_artifact().map_err(Error::new)?;
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
        request: bridge::BuildRequest,
    ) -> Result<artifact::ArtifactKey> {
        match request {
            bridge::BuildRequest::Module {
                module,
                target,
                output,
            } => {
                let module = module.id.into_source().map_err(Error::new)?;
                let target = target.into_source().map_err(Error::new)?;

                match output {
                    bridge::ModuleBuildKind::Script => {
                        Ok(artifact::ArtifactKey::script(module, target))
                    }
                    bridge::ModuleBuildKind::Object => {
                        Ok(artifact::ArtifactKey::object(module, target))
                    }
                    bridge::ModuleBuildKind::Asset => {
                        Ok(artifact::ArtifactKey::asset(module, target))
                    }
                }
            }
            bridge::BuildRequest::Build { target } => {
                let target = target.into_source().map_err(Error::new)?;

                Ok(artifact::ArtifactKey::build(target))
            }
            bridge::BuildRequest::Target { target } => {
                let target = target.into_source().map_err(Error::new)?;
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
            bridge::BuildRequest::Product { product } => {
                let product = product.into_source().map_err(Error::new)?;
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
        document: bridge::Document,
    ) -> Result<String> {
        match document {
            bridge::Document::Module { module } => self.format_module(revision, module),
            bridge::Document::Text { path, text } => Self::format_text(path, text),
        }
    }

    /// Format one loaded module from repository source.
    fn format_module(
        &self,
        revision: repository::Revision,
        module: bridge::Module,
    ) -> Result<String> {
        let module_id = module.id.into_source().map_err(Error::new)?;
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
    fn lint_key(scope: bridge::Scope) -> Result<artifact::ArtifactKey> {
        match scope {
            bridge::Scope::Module { module, profile } => {
                let module = module.id.into_source().map_err(Error::new)?;
                let profile = profile.into_source().map_err(Error::new)?;

                Ok(artifact::ArtifactKey::module_linted(module, profile))
            }
            bridge::Scope::Package { package } => {
                let package = package.into_source().map_err(Error::new)?;

                Ok(artifact::ArtifactKey::package_linted(package))
            }
            bridge::Scope::Workspace => Ok(artifact::ArtifactKey::workspace_linted()),
        }
    }

    /// Open one Rust session from one prepared repository.
    fn from_repository(repository: repository::Repository) -> Result<Self> {
        let repository = Arc::new(repository);
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

        Ok(Self { session })
    }

    /// Open one repository from one filesystem source.
    fn open_repository_from_file_system(
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

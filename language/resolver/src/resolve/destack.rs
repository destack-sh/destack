use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{File, FileId, FileType, PathExt, Uri};
use destack_workspace::{DestackDeclaration, Revision};

use crate::{CachePolicy, ResolveContext, ResolveError, Resolver};

impl Resolver {
    /// Read and parse a `destack.json` file recursively, handling extends.
    pub fn read_destack(
        &self,
        revision: Revision,
        path: &Path,
        cache_policy: CachePolicy,
    ) -> Result<DestackDeclaration, ResolveError> {
        let mut ctx = ResolveContext::new(revision);

        self.read_destack_with_context(path, &mut ctx, cache_policy)
    }

    /// Read and parse a `destack.json` file recursively, handling extends.
    pub(crate) fn read_destack_with_context(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
        cache_policy: CachePolicy,
    ) -> Result<DestackDeclaration, ResolveError> {
        // parse the local declaration first
        let mut config = self.parse_destack(path, ctx, cache_policy)?;

        // reject circular extends chains
        if ctx.is_extended_destack_config(&config.path) {
            return Err(ResolveError::DestackCircular {
                paths: ctx.extended_destack_configs_with(config.path.to_path_buf()),
            });
        }

        // resolve every extended config path up front
        let extended_config_paths: Vec<PathBuf> = config
            .extends()
            .map(|specifier| self.get_extended_destack_path(&config.directory, specifier))
            .collect::<Result<Vec<_>, _>>()?;

        // merge parent configs in order
        if !extended_config_paths.is_empty() {
            let config_path = config.path.clone();
            ctx.with_extended_destack_config(config_path, |ctx| {
                for extended_config_path in extended_config_paths {
                    let extended =
                        self.read_destack_with_context(&extended_config_path, ctx, cache_policy)?;
                    config
                        .extend_from(&extended)
                        .map_err(|_| ResolveError::DestackInvalid {
                            path: config.path.clone(),
                        })?;
                }
                Ok(())
            })?;
        }

        Ok(config)
    }

    /// Parse one destack.json file without processing extends.
    fn parse_destack(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
        cache_policy: CachePolicy,
    ) -> Result<DestackDeclaration, ResolveError> {
        // normalize the input into a concrete config path
        let destack_config_path = self.materialize_destack_path(path, ctx)?;

        // reuse the cached declaration when allowed
        if cache_policy.use_cache()
            && let Some(config) = ctx.destack_declaration(&destack_config_path)
        {
            return Ok(config);
        }

        // prefer revision backed declarations inside the workspace
        let (repository, revision) = self.source_world(ctx);
        if destack_config_path.starts_with(repository.workspace_root())
            && let Some(config) = repository
                .destack_declaration_for_path(revision, &destack_config_path)
                .map_err(|error| ResolveError::RepositoryError {
                    path: destack_config_path.to_path_buf(),
                    message: error.to_string(),
                })?
        {
            let config = config.as_ref().clone();
            ctx.remember_destack_declaration(config.clone());
            return Ok(config);
        }

        // read the config file from disk
        let content = match self.read_path_to_string(&destack_config_path, ctx) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(ResolveError::DestackNotFound {
                    path: path.to_path_buf(),
                });
            }
            Err(error) => {
                return Err(ResolveError::IoError {
                    path: destack_config_path.to_path_buf(),
                    kind: error.kind(),
                });
            }
        };

        let (name, uri) = Uri::from_path_with_name(&destack_config_path);
        let file_id = FileId::from_logical_path(&destack_config_path);
        let file = File::from_text(
            file_id,
            name,
            uri,
            Some(destack_config_path.to_path_buf()),
            FileType::Json,
            content,
        );
        let file = Arc::new(file);

        // parse the Destack config from the local file
        let config =
            DestackDeclaration::parse(&file).map_err(|_| ResolveError::DestackInvalid {
                path: destack_config_path.to_path_buf(),
            })?;

        ctx.remember_destack_declaration(config.clone());

        Ok(config)
    }

    /// Materialize one user supplied destack path into the concrete config file path.
    fn materialize_destack_path(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
    ) -> Result<PathBuf, ResolveError> {
        let path = match self.path_metadata(path, ctx)? {
            Some(metadata) if metadata.is_file => path.to_path_buf(),
            Some(metadata) if metadata.is_directory => path.join("destack.json"),
            _ => {
                let mut os_string = path.to_path_buf().into_os_string();
                os_string.push(".json");
                PathBuf::from(os_string)
            }
        };

        Ok(path)
    }
    /// Resolve the path of one extended Destack config file.
    fn get_extended_destack_path(
        &self,
        directory: &Path,
        specifier: &str,
    ) -> Result<PathBuf, ResolveError> {
        match specifier.as_bytes().first() {
            // empty specifier
            None => Err(ResolveError::InvalidSpecifier {
                specifier: specifier.to_string(),
                message: None,
            }),
            // absolute path
            Some(b'/') => Ok(PathBuf::from(specifier)),
            // relative path
            Some(b'.') => Ok(directory.normalize_with(specifier)),

            // bare specifiers are not supported here
            _ => Err(ResolveError::InvalidSpecifier {
                specifier: specifier.to_string(),
                message: Some(
                    "destack config extends must use an absolute or relative path".to_string(),
                ),
            }),
        }
    }
}

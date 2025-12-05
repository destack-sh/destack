use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

use destack_source::{PathExt, SLASH_START};
use destack_workspace::ModuleSpecifier;

use crate::{ResolveContext, ResolveError, Resolver};

#[cfg(not(target_arch = "wasm32"))]
fn resolve_file_protocol(specifier: &str) -> Result<Cow<'_, str>, ResolveError> {
    if specifier.starts_with("file://") {
        url::Url::parse(specifier)
            .map_err(|_| ())
            .and_then(|url| {
                url.to_file_path().map(|path| {
                    let mut result = path.to_string_lossy().to_string();
                    // preserve query and fragment from the url
                    if let Some(query) = url.query() {
                        result.push('?');
                        result.push_str(query);
                    }
                    if let Some(fragment) = url.fragment() {
                        result.push('#');
                        result.push_str(fragment);
                    }
                    Cow::Owned(result)
                })
            })
            .map_err(|()| ResolveError::UnsupportedPath {
                path: PathBuf::from(specifier),
            })
    } else {
        Ok(Cow::Borrowed(specifier))
    }
}

impl Resolver {
    /// Resolve a specifier from a directory, parsing query and fragment.
    #[tracing::instrument(name = "resolver.require", level = "trace", skip(self, ctx))]
    pub(crate) fn require(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<PathBuf, ResolveError> {
        tracing::trace!(?path, ?specifier, "resolver.require");
        ctx.check_depth()?;

        // parse query and fragment identifiers
        let parsed = ModuleSpecifier::parse(specifier);
        if let Some(query) = &parsed.query {
            ctx.query.replace(query.to_string());
        }
        if let Some(fragment) = &parsed.fragment {
            ctx.fragment.replace(fragment.to_string());
        }

        // if there is a fragment but no query, it might be part of the filename
        if ctx.fragment.is_some() && ctx.query.is_none() {
            let base_path = parsed.path();
            let fragment = ctx.fragment.take().unwrap();
            let candidate = format!("{base_path}{fragment}");
            if let Ok(resolved) = self.require_specifier(path, &candidate, ctx) {
                return Ok(resolved);
            }
            ctx.fragment.replace(fragment);
        }

        self.require_specifier(path, parsed.path(), ctx)
    }

    /// Resolve a specifier without parsing query/fragment (already parsed).
    fn require_specifier(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<PathBuf, ResolveError> {
        // check tsconfig paths
        if let Some(resolved) =
            self.load_tsconfig_paths(path, specifier, &mut ResolveContext::default())?
        {
            return Ok(resolved);
        }

        // check alias
        if let Some(resolved) = self.load_alias(path, specifier, &self.options.alias, ctx)? {
            return Ok(resolved);
        }

        // resolve file protocol
        cfg_if::cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                let specifier = resolve_file_protocol(specifier)?;
                let specifier = specifier.as_ref();
            }
        };

        let result = match Path::new(&specifier).components().next() {
            // absolute path
            Some(Component::RootDir | Component::Prefix(_)) => {
                self.require_absolute(path, specifier, ctx)
            }
            // relative path
            Some(Component::CurDir | Component::ParentDir) => {
                self.require_relative(path, specifier, ctx)
            }
            // internal package import
            Some(Component::Normal(_)) if specifier.as_bytes()[0] == b'#' => {
                self.require_hash(path, specifier, ctx)
            }
            // bare specifier (module)
            _ => self.require_bare(path, specifier, ctx),
        };

        result.or_else(|err| {
            if err.is_ignore() {
                return Err(err);
            }
            // check fallback alias
            self.load_alias(path, specifier, &self.options.fallback, ctx)
                .and_then(|value| value.ok_or(err))
        })
    }

    /// Resolve an absolute path specifier (starting with `/` or drive letter).
    #[tracing::instrument(name = "resolver.require.absolute", level = "trace", skip(self, ctx))]
    fn require_absolute(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<PathBuf, ResolveError> {
        tracing::trace!(?path, ?specifier, "resolver.require.absolute");
        debug_assert!(
            Path::new(specifier)
                .components()
                .next()
                .is_some_and(|c| matches!(c, Component::RootDir | Component::Prefix(_)))
        );

        // try to load from package itself or node_modules
        if !self.options.prefer_relative
            && self.options.prefer_absolute
            && let Ok(resolved) = self.load_package_self_or_modules(path, specifier, ctx)
        {
            return Ok(resolved);
        }

        // try to load from roots
        if let Some(resolved) = self.load_roots(path, specifier, ctx) {
            return Ok(resolved);
        }

        // try to load as file or directory
        let specifier_path = Path::new(specifier).to_path_buf();
        if let Some(resolved) = self.load_file_or_directory(&specifier_path, specifier, ctx)? {
            return Ok(resolved);
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Resolve a relative path specifier (starting with `./` or `../`).
    #[tracing::instrument(name = "resolver.require.relative", level = "trace", skip(self, ctx))]
    fn require_relative(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<PathBuf, ResolveError> {
        tracing::trace!(?path, ?specifier, "resolver.require.relative");
        debug_assert!(
            Path::new(specifier)
                .components()
                .next()
                .is_some_and(|c| matches!(
                    c,
                    Component::CurDir | Component::ParentDir | Component::Normal(_)
                ))
        );
        let path_with_specifier = path.normalize_with(specifier);

        // load as file or directory
        if let Some(resolved) = self.load_file_or_directory(
            &path_with_specifier,
            // ensure resolve directory only when specifier is `.`
            if specifier == "." { "./" } else { specifier },
            ctx,
        )? {
            return Ok(resolved);
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Resolve a hash-prefixed specifier against package.json imports.
    #[tracing::instrument(name = "resolver.require.hash", level = "trace", skip(self, ctx))]
    fn require_hash(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<PathBuf, ResolveError> {
        tracing::trace!(?path, ?specifier, "resolver.require.hash");
        debug_assert_eq!(specifier.chars().next(), Some('#'));

        self.load_package_imports(path, specifier, ctx)?
            .ok_or_else(|| ResolveError::NotFound {
                specifier: specifier.to_string(),
            })
    }

    /// Resolve a bare specifier by searching node_modules directories.
    #[tracing::instrument(name = "resolver.require.bare", level = "trace", skip(self, ctx))]
    fn require_bare(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<PathBuf, ResolveError> {
        tracing::trace!(?path, ?specifier, "resolver.require.bare");
        debug_assert!(
            Path::new(specifier)
                .components()
                .next()
                .is_some_and(|c| matches!(c, Component::Normal(_)))
        );

        // prefer relative path
        if self.options.prefer_relative
            && let Ok(resolved) = self.require_relative(path, specifier, ctx)
        {
            Ok(resolved)
        }
        // try package itself or modules
        else {
            self.load_package_self_or_modules(path, specifier, ctx)
        }
    }

    /// Resolve against configured root directories.
    ///
    /// Root directories allow absolute-style imports (starting with `/`) to resolve
    /// relative to project roots rather than the filesystem root.
    pub(crate) fn load_roots(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Option<PathBuf> {
        // bail if no roots configured
        if self.options.roots.is_empty() {
            return None;
        }

        // only handle specifiers starting with `/`
        let relative_specifier = specifier.strip_prefix(SLASH_START)?;

        // bare `/` resolves to the current directory if it's a root
        if relative_specifier.is_empty() {
            let is_root = self.options.roots.iter().any(|root| root.as_path() == path);
            if is_root && let Ok(resolved) = self.require_relative(path, "./", ctx) {
                return Some(resolved);
            }
        }
        // `/path` tries each root directory in order
        else {
            for root in &self.options.roots {
                if let Ok(resolved) = self.require_relative(root, relative_specifier, ctx) {
                    return Some(resolved);
                }
            }
        }

        None
    }
}

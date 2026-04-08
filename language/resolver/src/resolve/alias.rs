use std::borrow::Cow;
use std::path::Path;

use destack_source::{PathExt, SLASH_START};
use destack_workspace::PackageDeclaration;

use crate::{
    Alias, AliasValue, Resolution, ResolveContext, ResolveError, ResolveOrigin, ResolvePath,
    ResolveState, Resolver,
};

/// One compiled alias table.
#[derive(Debug, Clone, Default)]
pub(crate) struct CompiledAliasTable {
    /// The compiled alias entries in matching order.
    entries: Vec<CompiledAliasEntry>,
}

/// One compiled alias entry.
#[derive(Debug, Clone)]
pub(crate) struct CompiledAliasEntry {
    /// The parsed alias pattern.
    pattern: CompiledAliasPattern,
    /// The configured alias values in matching order.
    values: Vec<AliasValue>,
}

/// One parsed alias matching pattern.
#[derive(Debug, Clone)]
enum CompiledAliasPattern {
    /// One exact alias key that previously ended with `$`.
    Exact { key: String },
    /// One wildcard alias containing exactly one `*`.
    Wildcard {
        /// The original alias key.
        key: String,
        /// The literal prefix before the wildcard.
        prefix: String,
        /// The literal suffix after the wildcard.
        suffix: String,
    },
    /// One package style prefix alias.
    Prefix { key: String },
}

impl CompiledAliasTable {
    /// Compile one alias table into match ready entries.
    pub(crate) fn from_aliases(aliases: &Alias) -> Self {
        let entries = aliases
            .iter()
            .map(|(alias_key_raw, values)| {
                let pattern = if let Some(key) = alias_key_raw.strip_suffix('$') {
                    CompiledAliasPattern::Exact {
                        key: key.to_string(),
                    }
                } else if let Some((prefix, suffix)) = alias_key_raw.split_once('*') {
                    CompiledAliasPattern::Wildcard {
                        key: alias_key_raw.clone(),
                        prefix: prefix.to_string(),
                        suffix: suffix.to_string(),
                    }
                } else {
                    CompiledAliasPattern::Prefix {
                        key: alias_key_raw.clone(),
                    }
                };

                CompiledAliasEntry {
                    pattern,
                    values: values.clone(),
                }
            })
            .collect();

        Self { entries }
    }

    /// Return the compiled alias entries in matching order.
    fn entries(&self) -> &[CompiledAliasEntry] {
        &self.entries
    }
}

#[allow(clippy::too_many_arguments)]
impl Resolver {
    /// Derive branch state for one applied mapping.
    fn apply_mapping_state(
        &self,
        active_rewrite: Option<String>,
        state: ResolveState,
    ) -> ResolveState {
        state
            .with_active_rewrite(active_rewrite)
            .with_fully_specified(false)
    }

    /// Find one browser field substitution for a path or request.
    pub(crate) fn find_browser_substitution<'a>(
        &self,
        package_declaration: &'a PackageDeclaration,
        path: &Path,
        request: Option<&str>,
    ) -> Result<Option<&'a str>, ResolveError> {
        let Some(object) = package_declaration
            .manifest
            .browser
            .as_ref()
            .and_then(|v| v.as_object())
        else {
            return Ok(None);
        };

        // match by the raw request string first
        if let Some(request) = request {
            if let Some(value) = object.get(request) {
                return match value {
                    serde_json::Value::String(s) => Ok(Some(s.as_str())),
                    serde_json::Value::Bool(false) => Err(ResolveError::Ignored {
                        path: path.to_path_buf(),
                    }),
                    _ => Ok(None),
                };
            }
        }
        // otherwise match by the resolved path
        else {
            let directory =
                package_declaration
                    .path
                    .parent()
                    .ok_or_else(|| ResolveError::InvalidPackageJson {
                        path: package_declaration.path.clone(),
                    });
            let directory = directory?;

            for (key, value) in object {
                let joined = directory.normalize_with(key.as_str());
                if joined == path {
                    return match value {
                        serde_json::Value::String(s) => Ok(Some(s.as_str())),
                        serde_json::Value::Bool(false) => Err(ResolveError::Ignored {
                            path: path.to_path_buf(),
                        }),
                        _ => Ok(None),
                    };
                }
            }
        }

        Ok(None)
    }

    /// Apply browser field substitution.
    pub(crate) fn apply_browser(
        &self,
        path: &Path,
        module_specifier: Option<&str>,
        package_declaration: &PackageDeclaration,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Option<Resolution>, ResolveError> {
        if state.is_fully_specified() {
            return Ok(None);
        }

        // return early when there is no browser rewrite
        let Some(new_specifier) =
            self.find_browser_substitution(package_declaration, path, module_specifier)?
        else {
            return Ok(None);
        };

        // ignore trivial self rewrites
        if module_specifier.is_some_and(|s| s == new_specifier) {
            return Ok(None);
        }

        // reject recursive rewrite loops
        if state
            .active_rewrite()
            .is_some_and(|active_rewrite| active_rewrite == new_specifier)
        {
            // allow self rewrites like `{"./a.js": "./a.js"}`
            if new_specifier
                .strip_prefix("./")
                .filter(|s| path.ends_with(Path::new(s)))
                .is_some()
            {
                return if self.is_file(path, ctx) {
                    if self.check_restrictions(path) {
                        Ok(Some(Resolution::path_only(path.to_path_buf())))
                    } else {
                        Ok(None)
                    }
                } else {
                    Err(ResolveError::NotFound {
                        specifier: new_specifier.to_string(),
                    })
                };
            }
            return Err(ResolveError::RecursiveDependency {
                depth: state.depth(),
            });
        }

        let package_url =
            package_declaration
                .path
                .parent()
                .ok_or_else(|| ResolveError::InvalidPackageJson {
                    path: package_declaration.path.clone(),
                })?;
        let package_url = package_url.to_path_buf();
        let request = ResolvePath::parse(new_specifier);
        let state = self.apply_mapping_state(Some(new_specifier.to_string()), state);
        self.resolve_request(
            ResolveOrigin::Directory,
            &package_url,
            &package_url,
            &request,
            state,
            ctx,
        )
        .map(Some)
    }

    /// Apply aliases from the primary alias table.
    pub(crate) fn apply_primary_alias(
        &self,
        origin: ResolveOrigin,
        request_directory: &Path,
        lookup_path: &Path,
        specifier: &str,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Option<Resolution>, ResolveError> {
        self.apply_alias(
            origin,
            request_directory,
            lookup_path,
            specifier,
            &self.compiled_alias,
            state,
            ctx,
        )
    }

    /// Apply aliases from the fallback alias table.
    pub(crate) fn apply_fallback_alias(
        &self,
        origin: ResolveOrigin,
        request_directory: &Path,
        lookup_path: &Path,
        specifier: &str,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Option<Resolution>, ResolveError> {
        self.apply_alias(
            origin,
            request_directory,
            lookup_path,
            specifier,
            &self.compiled_fallback,
            state,
            ctx,
        )
    }

    /// Apply one compiled alias table.
    fn apply_alias(
        &self,
        origin: ResolveOrigin,
        request_directory: &Path,
        lookup_path: &Path,
        specifier: &str,
        aliases: &CompiledAliasTable,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Option<Resolution>, ResolveError> {
        for alias in aliases.entries() {
            let (alias_key, alias_key_has_wildcard) = match &alias.pattern {
                CompiledAliasPattern::Exact { key } => {
                    if key != specifier {
                        continue;
                    }
                    (key.as_str(), false)
                }
                CompiledAliasPattern::Wildcard {
                    key,
                    prefix,
                    suffix,
                } => {
                    if !specifier.starts_with(prefix)
                        || !specifier.ends_with(suffix)
                        || specifier.len() < prefix.len() + suffix.len()
                    {
                        continue;
                    }
                    (key.as_str(), true)
                }
                CompiledAliasPattern::Prefix { key } => {
                    if Self::strip_package_name(specifier, key).is_none() {
                        continue;
                    }
                    (key.as_str(), false)
                }
            };

            // stop once every matched alias value failed
            let mut should_stop = false;
            for alias_value in &alias.values {
                match alias_value {
                    AliasValue::Path(alias_path) => {
                        if let Some(resolved) = self.apply_alias_value(
                            origin,
                            request_directory,
                            lookup_path,
                            alias_key,
                            alias_key_has_wildcard,
                            alias_path,
                            specifier,
                            state.clone(),
                            ctx,
                            &mut should_stop,
                        )? {
                            return Ok(Some(resolved));
                        }
                    }
                    AliasValue::Ignore => {
                        let ignored_path = request_directory.normalize_with(alias_key);
                        return Err(ResolveError::Ignored { path: ignored_path });
                    }
                }
            }
            if should_stop {
                return Err(ResolveError::MatchedAliasNotFound {
                    specifier: specifier.to_string(),
                    alias_key: alias_key.to_string(),
                });
            }
        }
        Ok(None)
    }

    /// Apply one alias value by substituting the matched portion.
    fn apply_alias_value(
        &self,
        origin: ResolveOrigin,
        request_directory: &Path,
        lookup_path: &Path,
        alias_key: &str,
        alias_key_has_wildcard: bool,
        alias_value: &str,
        request: &str,
        state: ResolveState,
        ctx: &mut ResolveContext,
        should_stop: &mut bool,
    ) -> Result<Option<Resolution>, ResolveError> {
        // skip exact self aliases and direct subpaths
        if request == alias_value
            || request
                .strip_prefix(alias_value)
                .is_some_and(|suffix| suffix.starts_with('/'))
        {
            return Ok(None);
        }

        // build the substituted specifier
        let new_specifier = if alias_key_has_wildcard {
            // extract the wildcard match
            let Some(matched) = alias_key.split_once('*').and_then(|(prefix, suffix)| {
                request
                    .strip_prefix(prefix)
                    .and_then(|rest| rest.strip_suffix(suffix))
            }) else {
                return Ok(None);
            };

            // substitute the wildcard into the alias value
            if alias_value.contains('*') {
                Cow::Owned(alias_value.replacen('*', matched, 1))
            } else {
                Cow::Borrowed(alias_value)
            }
        }
        // append the unmatched tail for prefix aliases
        else {
            let tail = &request[alias_key.len()..];
            if tail.is_empty() {
                Cow::Borrowed(alias_value)
            } else {
                let alias_path = Path::new(alias_value).normalize();
                // keep explicit file aliases untouched
                if self.is_file(&alias_path, ctx) {
                    return Ok(None);
                }
                // normalize the unmatched tail
                let tail = tail.trim_start_matches(SLASH_START);
                if tail.is_empty() {
                    Cow::Borrowed(alias_value)
                } else {
                    let normalized = alias_path.normalize_with(tail);
                    Cow::Owned(normalized.to_string_lossy().to_string())
                }
            }
        };

        // resolve the substituted specifier
        *should_stop = true;
        let request = ResolvePath::parse(new_specifier.as_ref());
        let state = self.apply_mapping_state(None, state);
        let resolution =
            self.resolve_request(origin, request_directory, lookup_path, &request, state, ctx);

        match resolution {
            Ok(resolved) => Ok(Some(resolved)),
            Err(error) if error.is_alternative_candidate_miss() => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Apply extension alias mappings (e.g., `.js` to `.ts`).
    pub(crate) fn apply_extension_alias(
        &self,
        path: &Path,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Option<Resolution>, ResolveError> {
        // return early when no extension alias applies
        if self.options.extension_alias.is_empty() {
            return Ok(None);
        }
        let Some(path_extension) = path.extension() else {
            return Ok(None);
        };
        let Some(file_name) = path.file_name() else {
            return Ok(None);
        };
        let Some(path_extension_str) = path_extension.to_str() else {
            return Ok(None);
        };

        // find the extension alias mapping
        let extension_key = format!(".{path_extension_str}");
        let Some(extensions) = self.options.extension_alias.get(&extension_key) else {
            return Ok(None);
        };

        let state = state.with_fully_specified(true);
        for extension in extensions {
            // strip the leading dot for `with_extension`
            let extension = extension.strip_prefix('.').unwrap_or(extension);
            let path_with_ext = path.with_extension(extension);
            if let Some(resolved) = self.probe_alias_or_file(&path_with_ext, state.clone(), ctx)? {
                return Ok(Some(resolved));
            }
        }

        // return quietly for unresolved module directory lookups like `ipaddr.js`
        if !self.is_file(path, ctx) || !self.check_restrictions(path) {
            return Ok(None);
        }

        // report the failed alias candidates
        let dir = path
            .parent()
            .map_or_else(|| Path::new(".").to_path_buf(), Path::to_path_buf);
        let filename_without_extension = Path::new(file_name).with_extension("");
        let filename_without_extension = filename_without_extension.to_string_lossy();
        let files = extensions
            .iter()
            .map(|ext| format!("{filename_without_extension}{ext}"))
            .collect::<Vec<_>>()
            .join(",");
        Err(ResolveError::ExtensionAliasNotFound {
            filename: file_name.to_string_lossy().to_string(),
            tried: files,
            dir,
        })
    }

    /// Strip the package name prefix from a specifier if it matches.
    pub(crate) fn strip_package_name<'a>(
        specifier: &'a str,
        package_name: &'a str,
    ) -> Option<&'a str> {
        specifier
            .strip_prefix(package_name)
            .filter(|tail| tail.is_empty() || tail.starts_with(SLASH_START))
    }
}

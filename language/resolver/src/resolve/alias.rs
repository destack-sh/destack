use std::borrow::Cow;
use std::path::{Path, PathBuf};

use dyst_dir::PackageConfig;
use dyst_source::{PathExt, SLASH_START};

use crate::{Alias, AliasValue, ResolveContext, ResolveError, Resolver};

#[allow(clippy::too_many_arguments)]
impl Resolver {
    /// Resolve the browser field value for a path or request.
    pub(crate) fn resolve_browser_field<'a>(
        &self,
        package_config: &'a PackageConfig,
        path: &Path,
        request: Option<&str>,
    ) -> Result<Option<&'a str>, ResolveError> {
        let Some(object) = package_config
            .content
            .browser
            .as_ref()
            .and_then(|v| v.as_object())
        else {
            return Ok(None);
        };

        // find matching key in object by request string
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
        // find matching key by resolved path
        else {
            let directory = package_config.path.parent().unwrap_or_else(|| {
                panic!(
                    "package.json path is not in a directory: {}",
                    package_config.path.display()
                )
            });
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

    /// Resolve via browser field substitution.
    pub(crate) fn load_browser_field(
        &self,
        path: &Path,
        module_specifier: Option<&str>,
        package_config: &PackageConfig,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        if ctx.skip_extension {
            return Ok(None);
        }

        // bail if there is no new browser specifier
        let Some(new_specifier) =
            self.resolve_browser_field(package_config, path, module_specifier)?
        else {
            return Ok(None);
        };

        // abort when resolving recursive module
        if module_specifier.is_some_and(|s| s == new_specifier) {
            return Ok(None);
        }

        // check for recursive alias resolution
        if ctx.alias.as_ref().is_some_and(|s| s == new_specifier) {
            // complete when resolving to self `{"./a.js": "./a.js"}`
            if new_specifier
                .strip_prefix("./")
                .filter(|s| path.ends_with(Path::new(s)))
                .is_some()
            {
                return if self.is_file(path, ctx) {
                    if self.check_restrictions(path) {
                        Ok(Some(path.to_path_buf()))
                    } else {
                        Ok(None)
                    }
                } else {
                    Err(ResolveError::NotFound {
                        specifier: new_specifier.to_string(),
                    })
                };
            }
            return Err(ResolveError::RecursiveDependency { depth: ctx.depth });
        }

        // resolve alias
        ctx.alias = Some(new_specifier.to_string());
        ctx.skip_extension = false;
        let package_url = package_config.path.parent().unwrap().to_path_buf();
        self.require(&package_url, new_specifier, ctx).map(Some)
    }

    /// Resolve aliases and fallbacks from options.
    pub(crate) fn load_alias(
        &self,
        path: &Path,
        specifier: &str,
        aliases: &Alias,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        for (alias_key_raw, specifiers) in aliases {
            let mut alias_key_has_wildcard = false;
            let alias_key = {
                // exact match (key ends with `$`)
                if let Some(alias_key) = alias_key_raw.strip_suffix('$') {
                    if alias_key != specifier {
                        continue;
                    }
                    alias_key
                }
                // wildcard pattern match (key contains `*`)
                else if alias_key_raw.contains('*') {
                    alias_key_has_wildcard = true;
                    alias_key_raw
                }
                // directory pattern match
                else {
                    let strip_package_name = Self::strip_package_name(specifier, alias_key_raw);
                    if strip_package_name.is_none() {
                        continue;
                    }
                    alias_key_raw
                }
            };

            // stop resolving when all tried alias values fail
            let mut should_stop = false;
            for alias_value in specifiers {
                match alias_value {
                    AliasValue::Path(alias_path) => {
                        if let Some(resolved) = self.load_alias_value(
                            path,
                            alias_key,
                            alias_key_has_wildcard,
                            alias_path,
                            specifier,
                            ctx,
                            &mut should_stop,
                        )? {
                            return Ok(Some(resolved));
                        }
                    }
                    AliasValue::Ignore => {
                        let ignored_path = path.normalize_with(alias_key);
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

    /// Resolve an alias value by substituting the matched portion.
    fn load_alias_value(
        &self,
        path: &Path,
        alias_key: &str,
        alias_key_has_wildcard: bool,
        alias_value: &str,
        request: &str,
        ctx: &mut ResolveContext,
        should_stop: &mut bool,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // skip if request matches alias_value exactly or is a subpath of it
        if request == alias_value
            || request
                .strip_prefix(alias_value)
                .is_some_and(|suffix| suffix.starts_with('/'))
        {
            return Ok(None);
        }

        // build the new specifier by substituting the alias
        let new_specifier = if alias_key_has_wildcard {
            // wildcard alias: `@/*` -> `./src/*`
            let Some(matched) = alias_key.split_once('*').and_then(|(prefix, suffix)| {
                request
                    .strip_prefix(prefix)
                    .and_then(|rest| rest.strip_suffix(suffix))
            }) else {
                return Ok(None);
            };

            // substitute wildcard in alias value if present
            if alias_value.contains('*') {
                Cow::Owned(alias_value.replacen('*', matched, 1))
            } else {
                Cow::Borrowed(alias_value)
            }
        }
        // non-wildcard alias: concatenate tail
        else {
            let tail = &request[alias_key.len()..];
            if tail.is_empty() {
                Cow::Borrowed(alias_value)
            } else {
                let alias_path = Path::new(alias_value).normalize();
                // don't append tail if alias_value is already a file
                if self.is_file(&alias_path, ctx) {
                    return Ok(None);
                }
                // strip leading slash and normalize
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
        ctx.skip_extension = false;
        match self.require(path, new_specifier.as_ref(), ctx) {
            Ok(resolved) => Ok(Some(resolved)),
            Err(ResolveError::NotFound { .. } | ResolveError::MatchedAliasNotFound { .. }) => {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    /// Resolve via extension alias (e.g., mapping `.js` to `.ts`).
    pub(crate) fn load_with_extension_alias(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // no extension alias configured or found
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

        // get the extension alias mapping
        let extension_key = format!(".{path_extension_str}");
        let Some(extensions) = self.options.extension_alias.get(&extension_key) else {
            return Ok(None);
        };

        ctx.skip_extension = true;
        for extension in extensions {
            // extension has leading dot (e.g., ".ts"), but with_extension needs without dot
            let extension = extension.strip_prefix('.').unwrap_or(extension);
            let path_with_ext = path.with_extension(extension);
            if let Some(resolved) = self.load_alias_or_file(&path_with_ext, ctx)? {
                ctx.skip_extension = false;
                return Ok(Some(resolved));
            }
        }

        // bail if path is module directory (like `ipaddr.js`)
        if !self.is_file(path, ctx) {
            ctx.skip_extension = false;
            return Ok(None);
        } else if !self.check_restrictions(path) {
            return Ok(None);
        }

        // error: couldn't resolve any extension alias
        ctx.skip_extension = false;
        let dir = path.parent().unwrap().to_path_buf();
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

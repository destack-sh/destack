use std::borrow::Cow;
use std::path::{Component, Path};

use destack_source::PathExt;
use destack_workspace::PackageManifest;

use crate::{CachePolicy, Resolution, ResolveError, ResolveFrame, ResolveRequest, Resolver};

#[allow(clippy::too_many_arguments)]
impl Resolver {
    /// Return true when one exports target path is invalid.
    fn is_path_invalid_exports_target(path: &Path) -> bool {
        path.components().enumerate().any(|(index, c)| match c {
            Component::ParentDir => true,
            Component::CurDir => index > 0,
            Component::Normal(c) => c.eq_ignore_ascii_case("node_modules"),
            _ => false,
        })
    }

    /// Normalize one string target by substituting the matched export pattern.
    fn normalize_string_target<'a>(
        target_key: &'a str,
        target: &'a str,
        pattern_match: Option<&'a str>,
        package_url: &Path,
    ) -> Result<Cow<'a, str>, ResolveError> {
        if let Some(pattern_match) = pattern_match {
            if !target_key.contains('*') && !target.contains('*') {
                // enhanced resolve supports trailing slash patterns here
                if target_key.ends_with('/') && target.ends_with('/') {
                    Ok(Cow::Owned(format!("{target}{pattern_match}")))
                } else {
                    Err(ResolveError::InvalidPackageConfigDirectory {
                        path: package_url.join("package.json"),
                    })
                }
            } else {
                Ok(Cow::Owned(target.replace('*', pattern_match)))
            }
        } else {
            Ok(Cow::Borrowed(target))
        }
    }

    /// Probe one package target and reattach any target query or fragment overrides.
    pub(crate) fn finalize_package_target(
        &self,
        specifier: &str,
        target: Resolution,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<Resolution>, ResolveError> {
        let Resolution {
            path,
            query,
            fragment,
        } = target;

        let probed = self.probe_esm_target(specifier, &path, ctx)?;
        Ok(probed.map(|resolved| resolved.override_parts(query, fragment)))
    }

    /// Resolve one package import request through `package.json#imports`.
    pub(crate) fn rewrite_package_import(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<Resolution>, ResolveError> {
        // return early when package imports are disabled
        if !self.options.resolve_package_json_imports {
            return Ok(None);
        }

        // find the closest package scope
        let Some(package_id) = self.find_nearest_package_scope(path, ctx)? else {
            return Ok(None);
        };
        let package = self.packages.get(package_id);
        let package = package.read();

        // resolve package imports when present
        if let Some(ref config) = package.manifest
            && let Some(resolved) = self.package_imports_resolve(specifier, config, ctx)?
        {
            return self.finalize_package_target(specifier, resolved, ctx);
        }
        Ok(None)
    }

    /// Resolve one package `exports` match from a specific package directory.
    pub(crate) fn resolve_package_exports(
        &self,
        specifier: &str,
        subpath: &str,
        path: &Path,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<Resolution>, ResolveError> {
        // load the package manifest
        let Some(package_id) = self.read_package_manifest(path, ctx, CachePolicy::UseCache)? else {
            return Ok(None);
        };
        let package = self.packages.get(package_id);
        let package = package.read();

        // resolve package exports when present
        if let Some(ref config) = package.manifest
            && let Some(exports) = config.content.exports.as_ref()
            && let Some(resolved) =
                self.package_exports_resolve(path, &format!(".{subpath}"), exports, ctx)?
        {
            return self.finalize_package_target(specifier, resolved, ctx);
        }

        Ok(None)
    }

    /// Try to resolve a self reference.
    pub(crate) fn rewrite_package_self_reference(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<Resolution>, ResolveError> {
        // find the closest package scope
        let Some(package_id) = self.find_nearest_package_scope(path, ctx)? else {
            return Ok(None);
        };
        let package = self.packages.get(package_id);
        let package = package.read();

        // return early when the package has no manifest
        let Some(ref config) = package.manifest else {
            return Ok(None);
        };

        // prefer the package scope browser field when the self package matches
        let mut browser_field_path = path.to_path_buf();

        // resolve package self references by package name
        if let Some(subpath) = config
            .content
            .name
            .as_ref()
            .and_then(|package_name: &String| {
                Self::strip_package_name(specifier, package_name.as_str())
            })
        {
            let package_url = config
                .path
                .parent()
                .unwrap_or_else(|| {
                    panic!(
                        "package.json path is not in a directory: {}",
                        config.path.display()
                    )
                })
                .to_path_buf();

            if let Some(exports) = config.content.exports.as_ref()
                && let Some(resolved) = self.package_exports_resolve(
                    &package_url,
                    &format!(".{subpath}"),
                    exports,
                    ctx,
                )?
            {
                return self.finalize_package_target(specifier, resolved, ctx);
            }

            // resolve the package types entry for type conditions
            if (subpath.is_empty() || subpath == ".")
                && self
                    .options
                    .conditions
                    .iter()
                    .any(|condition| condition == "types")
                && let Some(types_field) = config.content.types.as_deref()
            {
                let types_path = package_url.normalize_with(types_field);
                if self.is_file(&types_path, ctx) && self.check_restrictions(&types_path) {
                    return self.probe_esm_target(specifier, &types_path, ctx);
                }
            }

            browser_field_path = package_url;
        }

        // fall back to the browser field
        self.rewrite_browser_field(&browser_field_path, Some(specifier), config, ctx)
    }

    /// Resolve an ESM match by loading as file or directory.
    pub(crate) fn probe_esm_target(
        &self,
        specifier: &str,
        path: &Path,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<Resolution>, ResolveError> {
        // non compliant esm can still resolve to a directory
        if let Some(resolved) = self.probe_path(path, "", ctx)? {
            Ok(Some(resolved))
        } else {
            Err(ResolveError::NotFound {
                specifier: specifier.to_string(),
            })
        }
    }

    /// Resolve a bare package specifier by searching module directories.
    pub(crate) fn resolve_package_target(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<Resolution>, ResolveError> {
        self.resolve_package_or_modules(path, specifier, ctx)
            .map(Some)
    }

    /// Resolve a subpath against a package's exports field.
    pub(crate) fn package_exports_resolve(
        &self,
        package_url: &Path,
        subpath: &str,
        exports: &serde_json::Value,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<Resolution>, ResolveError> {
        // return early when exports resolution is disabled
        if !self.options.resolve_package_json_exports {
            return Ok(None);
        }

        // capture the active conditions once for nested resolution
        let conditions = &self.options.conditions;

        // validate exports key shape
        if let Some(map) = exports.as_object() {
            let mut has_dot = false;
            let mut without_dot = false;
            for key in map.keys() {
                let starts_with_dot_or_hash = key.starts_with(['.', '#']);
                has_dot = has_dot || starts_with_dot_or_hash;
                without_dot = without_dot || !starts_with_dot_or_hash;
                if has_dot && without_dot {
                    return Err(ResolveError::InvalidPackageJson {
                        path: package_url.join("package.json"),
                    });
                }
            }
        }

        // resolve the root export
        if subpath == "." {
            let main_export = match exports {
                serde_json::Value::String(_) | serde_json::Value::Array(_) => {
                    Some(Cow::Borrowed(exports))
                }
                serde_json::Value::Object(map) => map.get(".").map_or_else(
                    || {
                        if map
                            .keys()
                            .any(|key| key.starts_with("./") || key.starts_with('#'))
                        {
                            None
                        } else {
                            Some(Cow::Borrowed(exports))
                        }
                    },
                    |entry| Some(Cow::Borrowed(entry)),
                ),
                _ => None,
            };
            if let Some(main_export) = main_export {
                let resolved = self.package_target_resolve(
                    package_url,
                    ".",
                    main_export.as_ref(),
                    None,
                    false,
                    conditions,
                    ctx,
                )?;
                if let Some(path) = resolved {
                    return Ok(Some(path));
                }
            }
        }

        // resolve a subpath export
        if let Some(exports) = exports.as_object()
            && let Some(resolved) =
                self.package_match_resolve(subpath, exports, package_url, false, conditions, ctx)?
        {
            return Ok(Some(resolved));
        }

        // report a missing package export
        Err(ResolveError::PackagePathNotExported {
            subpath: subpath.to_string(),
            package_path: package_url.to_path_buf(),
            package_json_path: package_url.join("package.json"),
            conditions: self.options.conditions.clone(),
        })
    }

    /// Resolve an imports specifier against package.json imports field.
    fn package_imports_resolve(
        &self,
        specifier: &str,
        package_config: &PackageManifest,
        ctx: &mut ResolveFrame,
    ) -> Result<Option<Resolution>, ResolveError> {
        debug_assert!(specifier.starts_with('#'), "{specifier}");

        // return early when imports are not configured
        let Some(imports) = package_config.content.imports.as_ref() else {
            return Ok(None);
        };

        // reject invalid `#` specifiers
        if specifier == "#" || specifier.starts_with("#/") {
            return Err(ResolveError::InvalidModuleSpecifier {
                specifier: specifier.to_string(),
                package_path: package_config.path.to_path_buf(),
            });
        }

        // resolve the imports mapping
        if let Some(resolved) = self.package_match_resolve(
            specifier,
            imports,
            &package_config.directory,
            true,
            &self.options.conditions,
            ctx,
        )? {
            Ok(Some(resolved))
        } else {
            Err(ResolveError::PackageImportNotDefined {
                specifier: specifier.to_string(),
                package_path: package_config.path.to_path_buf(),
            })
        }
    }

    /// Resolve a key against an imports or exports mapping object.
    pub(crate) fn package_match_resolve(
        &self,
        match_key: &str,
        match_obj: &serde_json::Map<String, serde_json::Value>,
        package_url: &Path,
        is_imports: bool,
        conditions: &[String],
        ctx: &mut ResolveFrame,
    ) -> Result<Option<Resolution>, ResolveError> {
        // directory style requests never match here
        if match_key.ends_with('/') {
            return Ok(None);
        }

        // try a direct match first
        if !match_key.contains('*')
            && let Some(target) = match_obj.get(match_key)
        {
            return self.package_target_resolve(
                package_url,
                match_key,
                target,
                None,
                is_imports,
                conditions,
                ctx,
            );
        }

        // track the best matching pattern
        let mut best_target = None;
        let mut best_match = "";
        let mut best_key = "";
        for (source_key, target_key) in match_obj.iter() {
            // skip invalid mapping shapes
            if source_key.ends_with('*') && target_key.as_str().is_some_and(|s| !s.contains('*')) {
                // (can't have asterisk in source key but not in target)
                continue;
            }

            if source_key.starts_with("./") || source_key.starts_with('#') {
                // check wildcard patterns
                if let Some((pattern_base, pattern_trailer)) = source_key.split_once('*') {
                    if match_key.starts_with(pattern_base)
                        && !pattern_trailer.contains('*')
                        && (pattern_trailer.is_empty()
                            || (match_key.len() >= source_key.len()
                                && match_key.ends_with(pattern_trailer)))
                        && Self::pattern_key_compare(best_key, source_key).is_gt()
                    {
                        best_target = Some(target_key);
                        best_match =
                            &match_key[pattern_base.len()..match_key.len() - pattern_trailer.len()];
                        best_key = source_key;
                    }
                }
                // check directory patterns
                else if source_key.ends_with('/')
                    && match_key.starts_with(source_key)
                    && Self::pattern_key_compare(best_key, source_key).is_gt()
                {
                    best_target = Some(target_key);
                    best_match = &match_key[source_key.len()..];
                    best_key = source_key;
                }
            }
        }

        // resolve the best matching key
        if let Some(best_target) = best_target {
            return self.package_target_resolve(
                package_url,
                best_key,
                best_target,
                Some(best_match),
                is_imports,
                conditions,
                ctx,
            );
        }

        Ok(None)
    }

    /// Resolve a package target value (string, object, or array) to a path.
    fn package_target_resolve(
        &self,
        package_url: &Path,
        target_key: &str,
        target: &serde_json::Value,
        pattern_match: Option<&str>,
        is_imports: bool,
        conditions: &[String],
        ctx: &mut ResolveFrame,
    ) -> Result<Option<Resolution>, ResolveError> {
        // resolve string targets
        if let Some(target) = target.as_str() {
            // parse query and fragment parts
            let parsed = ResolveRequest::parse(target);
            let target = parsed.path.as_str();

            // handle package style targets
            if !target.starts_with("./") {
                // reject invalid package targets
                if !is_imports || target.starts_with("../") || target.starts_with('/') {
                    return Err(ResolveError::InvalidPackageTarget {
                        target: (*target).to_string(),
                        name: target_key.to_string(),
                        package_path: package_url.join("package.json"),
                    });
                }
                // resolve the target as another package request
                let target =
                    Self::normalize_string_target(target_key, target, pattern_match, package_url)?;
                let resolved = self.resolve_package_target(package_url, &target, ctx)?;
                return Ok(resolved.map(|resolved| {
                    resolved.override_parts(parsed.query.clone(), parsed.fragment.clone())
                }));
            }
            // handle relative package targets
            else {
                let target =
                    Self::normalize_string_target(target_key, target, pattern_match, package_url)?;
                if Self::is_path_invalid_exports_target(Path::new(target.as_ref())) {
                    return Err(ResolveError::InvalidPackageTarget {
                        target: target.to_string(),
                        name: target_key.to_string(),
                        package_path: package_url.join("package.json"),
                    });
                }
                let resolved_path = package_url.normalize_with(target.as_ref());
                return Ok(Some(Resolution::with_parts(
                    resolved_path,
                    parsed.query.clone(),
                    parsed.fragment.clone(),
                )));
            }
        }
        // resolve conditional object targets
        else if let Some(target) = target.as_object() {
            for (key, target_value) in target.iter() {
                if key == "default" || conditions.iter().any(|condition| condition == key) {
                    let resolved = self.package_target_resolve(
                        package_url,
                        target_key,
                        target_value,
                        pattern_match,
                        is_imports,
                        conditions,
                        ctx,
                    );
                    if let Some(path) = resolved? {
                        return Ok(Some(path));
                    }
                }
            }
            return Ok(None);
        }
        // resolve array fallback targets
        else if let Some(targets) = target.as_array() {
            if targets.is_empty() {
                return Err(ResolveError::PackagePathNotExported {
                    subpath: pattern_match.unwrap_or(".").to_string(),
                    package_path: package_url.to_path_buf(),
                    package_json_path: package_url.join("package.json"),
                    conditions: self.options.conditions.clone(),
                });
            }
            for target_value in targets {
                let resolved = self.package_target_resolve(
                    package_url,
                    target_key,
                    target_value,
                    pattern_match,
                    is_imports,
                    conditions,
                    ctx,
                );

                // accept the first concrete target
                match resolved {
                    Ok(Some(path)) => return Ok(Some(path)),
                    Ok(None) => continue,
                    // continue through expected fallback failures
                    Err(error) if error.is_alternative_candidate_miss() => {
                        continue;
                    }
                    // surface all other errors immediately
                    Err(error) => return Err(error),
                }
            }
        }

        Ok(None)
    }
}

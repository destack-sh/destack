use std::borrow::Cow;
use std::path::Path;

use destack_source::{PathExt, SLASH_START};

use crate::{
    Alias, AliasValue, Resolution, Resolver, ResolverBase, ResolverContext, ResolverError,
    ResolverResult, ResolverSearch, ResolverSpecifier,
};

/// One alias table prepared for request matching.
#[derive(Debug, Clone, Default)]
pub(crate) struct AliasTable {
    /// The alias entries in matching order.
    entries: Vec<AliasEntry>,
}

/// One alias entry.
#[derive(Debug, Clone)]
pub(crate) struct AliasEntry {
    /// The alias matching pattern.
    pattern: AliasPattern,
    /// The configured alias values in matching order.
    values: Vec<AliasValue>,
}

/// One parsed alias matching pattern.
#[derive(Debug, Clone)]
enum AliasPattern {
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
    /// One prefix alias.
    Prefix { key: String },
}

impl AliasTable {
    /// Create one alias table from resolver options.
    pub(crate) fn new(aliases: &Alias) -> Self {
        let entries = aliases
            .iter()
            .map(|(alias_key_raw, values)| {
                let pattern = if let Some(key) = alias_key_raw.strip_suffix('$') {
                    AliasPattern::Exact {
                        key: key.to_string(),
                    }
                } else if let Some((prefix, suffix)) = alias_key_raw.split_once('*') {
                    AliasPattern::Wildcard {
                        key: alias_key_raw.clone(),
                        prefix: prefix.to_string(),
                        suffix: suffix.to_string(),
                    }
                } else {
                    AliasPattern::Prefix {
                        key: alias_key_raw.clone(),
                    }
                };

                AliasEntry {
                    pattern,
                    values: values.clone(),
                }
            })
            .collect();

        Self { entries }
    }
}

#[allow(clippy::too_many_arguments)]
impl Resolver {
    /// Resolve through one alias table.
    pub(crate) fn resolve_alias_table(
        &self,
        base: ResolverBase<'_>,
        request_directory: &Path,
        specifier: &str,
        aliases: &AliasTable,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        for alias in &aliases.entries {
            let (alias_key, alias_key_has_wildcard) = match &alias.pattern {
                AliasPattern::Exact { key } => {
                    if key != specifier {
                        continue;
                    }
                    (key.as_str(), false)
                }
                AliasPattern::Wildcard {
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
                AliasPattern::Prefix { key } => {
                    if Self::strip_prefix_alias(specifier, key).is_none() {
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
                        if let Some(resolved) = self.resolve_alias_value(
                            base,
                            request_directory,
                            alias_key,
                            alias_key_has_wildcard,
                            alias_path,
                            specifier,
                            search.clone(),
                            ctx,
                            &mut should_stop,
                        )? {
                            return Ok(Some(resolved));
                        }
                    }
                    AliasValue::Ignore => {
                        let ignored_path = request_directory.normalize_with(alias_key);
                        return Err(ResolverError::Ignored { path: ignored_path });
                    }
                }
            }
            if should_stop {
                return Err(ResolverError::MatchedAliasNotFound {
                    specifier: specifier.to_string(),
                    alias_key: alias_key.to_string(),
                });
            }
        }
        Ok(None)
    }

    /// Resolve through one alias value by substituting the matched portion.
    fn resolve_alias_value(
        &self,
        base: ResolverBase<'_>,
        request_directory: &Path,
        alias_key: &str,
        alias_key_has_wildcard: bool,
        alias_value: &str,
        request: &str,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
        should_stop: &mut bool,
    ) -> ResolverResult<Option<Resolution>> {
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
                if self.is_file(&alias_path, ctx)? {
                    return Ok(None);
                }
                // normalize the unmatched tail
                let tail = tail.trim_start_matches(SLASH_START);
                if tail.is_empty() {
                    Cow::Borrowed(alias_value)
                } else if alias_value.starts_with('.') {
                    let alias_value = alias_value.trim_end_matches(SLASH_START);
                    Cow::Owned(format!("{alias_value}/{tail}"))
                } else {
                    let normalized = alias_path.normalize_with(tail);
                    Cow::Owned(normalized.to_string_lossy().to_string())
                }
            }
        };

        // resolve the substituted specifier
        *should_stop = true;
        if search.has_rewrite(new_specifier.as_ref()) {
            return Err(search.recursive_dependency());
        }

        let request = ResolverSpecifier::parse(new_specifier.as_ref());
        let mut search = search;
        search.enter_rewrite(new_specifier.as_ref());

        let resolution = self.resolve_request(base, request_directory, &request, search, ctx);

        match resolution {
            Ok(resolved) => Ok(Some(resolved)),
            Err(error) if error.is_alternative_candidate_miss() => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Strip one prefix alias from a specifier if it matches.
    pub(crate) fn strip_prefix_alias<'a>(specifier: &'a str, alias: &'a str) -> Option<&'a str> {
        specifier
            .strip_prefix(alias)
            .filter(|tail| tail.is_empty() || tail.starts_with(SLASH_START))
    }
}

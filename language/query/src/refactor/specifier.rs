use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use destack_core::StringId;
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProviderError, ProviderResult, Repository, Revision};
use destack_source::{ModuleId, PathExt, ProfileId, SourceIndex, Span};

use crate::source::{path_text, relative_path, strip_module_extension};
use crate::{QueryError, QueryResult};

/// One authored module specifier with its exact resolved target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedSpecifier {
    /// The authored string literal span.
    pub(crate) span: Span,
    /// The authored specifier text.
    pub(crate) text: String,
    /// The normalized target path.
    pub(crate) target_path: PathBuf,
}

/// Reader for resolved module specifiers.
struct SpecifierReader<'a> {
    /// The repository being queried.
    repository: &'a Repository,
    /// The queried revision.
    revision: Revision,
    /// The source module.
    module_id: ModuleId,
    /// The visible DIR tree.
    view: dir::View<'a>,
    /// The authored source index.
    source_index: &'a SourceIndex,
    /// The visible module edges.
    modules: dir::ModuleTable<'static>,
    /// The collected resolved specifiers.
    specifiers: Vec<ResolvedSpecifier>,
}

impl SpecifierReader<'_> {
    /// Read the resolved module specifiers.
    fn read(mut self) -> ProviderResult<Vec<ResolvedSpecifier>> {
        self.collect()?;

        Ok(self.specifiers)
    }

    /// Collect import and re-export specifiers with resolved module edges.
    fn collect(&mut self) -> ProviderResult<()> {
        // transcribe each expression with one resolved module edge
        for (expression_id, expression) in self.view.iter_nodes_of_type::<dir::Expression>() {
            let Some((text, relation)) = Self::specifier(expression) else {
                continue;
            };

            let source = expression_id.into_global_any(self.module_id);
            let Some(target_module) = self.modules.target_for_source(source, relation) else {
                continue;
            };
            let Some(target_path) = self.target_path(target_module)? else {
                continue;
            };
            let source_id = self.view.get_source(expression_id);

            // generated nodes have no authored specifier occurrence to edit
            if self.source_index.try_get(source_id).is_none() {
                continue;
            }

            let Some(span) = self.source_index.get_main(source_id) else {
                return Err(ProviderError::internal(format!(
                    "resolved module specifier {source:?} has no authored main span"
                ))
                .into());
            };

            self.specifiers.push(ResolvedSpecifier {
                span,
                text: self.repository.string_pool().get(text).to_string(),
                target_path,
            });
        }

        Ok(())
    }

    /// Return one resolved module specifier and its relation.
    fn specifier(expression: &dir::Expression) -> Option<(StringId, dir::ModuleRelation)> {
        match expression {
            dir::Expression::Import { target, .. } => Some((*target, dir::ModuleRelation::Import)),
            dir::Expression::Export {
                target: Some(target),
                ..
            } => Some((*target, dir::ModuleRelation::ReExport)),
            _ => None,
        }
    }

    /// Return the normalized source path for one target module.
    fn target_path(&self, target_module: ModuleId) -> ProviderResult<Option<PathBuf>> {
        let module = self
            .repository
            .module(self.revision, target_module)
            .map_err(|error| {
                ProviderError::internal(format!(
                    "failed to read resolved target module {target_module:?}: {error}"
                ))
            })?
            .ok_or_else(|| {
                ProviderError::internal(format!("missing resolved target module {target_module:?}"))
            })?;

        let Some(path) = module.path.as_ref() else {
            return Ok(None);
        };
        let path = if path.is_absolute() {
            path.normalize()
        } else {
            self.repository.path().join(path).normalize()
        };

        Ok(Some(path))
    }
}

/// Build authored module specifiers from their exact ready artifacts.
pub(crate) fn module_specifiers(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> ProviderResult<Vec<ResolvedSpecifier>> {
    let artifacts = ArtifactReader::new(repository, revision);
    let parsed = artifacts.dir_parsed(module_id)?;
    let imported = artifacts.dir_imported(module_id, profile_id)?;
    let expanded = artifacts.dir_expanded(module_id, profile_id)?;
    let view = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));
    let source_index = &parsed.tree.source_index;
    let modules = expanded.module_table(&imported);
    let reader = SpecifierReader {
        repository,
        revision,
        module_id,
        view,
        source_index,
        modules,
        specifiers: Vec::new(),
    };

    reader.read()
}

/// Return one absolute normalized path relative to the workspace root.
pub(crate) fn workspace_path(workspace_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.normalize()
    } else {
        workspace_root.join(path).normalize()
    }
}

/// Resolve one exact target rewrite from the most specific renamed ancestor.
pub(crate) fn renamed_target_path(
    target_path: &Path,
    renames: &BTreeMap<PathBuf, PathBuf>,
) -> QueryResult<Option<PathBuf>> {
    let Some((old_path, new_path)) = renames
        .iter()
        .filter(|(old_path, _)| target_path.starts_with(old_path))
        .max_by_key(|(old_path, _)| old_path.components().count())
    else {
        return Ok(None);
    };
    let relative = target_path.strip_prefix(old_path).map_err(|_| {
        QueryError::invalid(format!(
            "rename ancestor {} for {}",
            old_path.display(),
            target_path.display()
        ))
    })?;
    let updated = new_path.join(relative).normalize();

    Ok(Some(updated))
}

/// Rewrite one authored specifier after moving its source or target.
pub(crate) fn rename_specifier(
    source_path: Option<&Path>,
    target_path: &Path,
    specifier: &str,
) -> QueryResult<Option<String>> {
    // relative specifiers follow the source and target paths
    if specifier.starts_with("./") || specifier.starts_with("../") {
        let Some(source_path) = source_path else {
            return Ok(None);
        };
        let strip_extension = strip_module_extension(specifier) == specifier;

        relative_specifier(source_path, target_path, strip_extension)
    }
    // package specifiers retain their declared export path
    else {
        Ok(Some(specifier.to_string()))
    }
}

/// Build one relative authored specifier from a source file to a target.
fn relative_specifier(
    source_path: &Path,
    target_path: &Path,
    strip_extension: bool,
) -> QueryResult<Option<String>> {
    let Some(source_directory) = source_path.parent() else {
        return Ok(None);
    };
    let Some(relative) = relative_path(source_directory, target_path) else {
        return Ok(None);
    };
    let mut display = display_path(&relative, strip_extension)?;

    if !display.starts_with("./") && !display.starts_with("../") {
        display = format!("./{display}");
    }

    Ok(Some(display))
}

/// Return one normalized display path with optional module extension removal.
fn display_path(path: &Path, strip_extension: bool) -> QueryResult<String> {
    let display = path_text(path)?;

    Ok(maybe_strip_extension(display, strip_extension))
}

/// Remove one authored module extension when requested.
fn maybe_strip_extension(path: String, strip_extension: bool) -> String {
    if !strip_extension {
        return path;
    }

    strip_module_extension(&path)
}

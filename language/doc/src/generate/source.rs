use std::path::Path;

use tspp_repository::{Repository, Revision};
use tspp_source::Span;

use crate::{DocError, DocResult, SourceReference};

impl SourceReference {
    /// Build one stable source reference.
    pub(crate) fn build(
        span: Span,
        package_path: Option<&Path>,
        repository: &Repository,
        revision: Revision,
    ) -> DocResult<Self> {
        let file = repository.file(revision, span.file)?.ok_or_else(|| {
            DocError::invalid(format!("missing documented source: {:?}", span.file))
        })?;
        let logical_path = repository
            .file_logical_path(revision, span.file)?
            .ok_or_else(|| {
                DocError::invalid(format!("missing documented source path: {:?}", span.file))
            })?;
        let (line, column) = file.get_position(span.start).ok_or_else(|| {
            DocError::invalid(format!("invalid documented source span: {span:?}"))
        })?;
        let end = span.end.saturating_sub(1).max(span.start);
        let (end_line, end_column) = file.get_position(end).ok_or_else(|| {
            DocError::invalid(format!("invalid documented source span: {span:?}"))
        })?;

        Ok(Self {
            path: Some(relative_logical_path(
                Path::new(&logical_path),
                package_path,
                repository,
            )?),
            line: line + 1,
            column: column + 1,
            end_line: end_line + 1,
            end_column: end_column + 1,
        })
    }
}

/// Return one package-relative path using stable forward slashes.
pub(crate) fn relative_path(
    path: Option<&Path>,
    package_path: Option<&Path>,
    repository: &Repository,
) -> DocResult<Option<String>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let path = repository.logical_path(path);

    Ok(Some(relative_logical_path(
        Path::new(&path),
        package_path,
        repository,
    )?))
}

/// Return one package-relative logical path using stable forward slashes.
fn relative_logical_path(
    path: &Path,
    package_path: Option<&Path>,
    repository: &Repository,
) -> DocResult<String> {
    let path = if let Some(package_path) = package_path {
        let package_path = repository.logical_path(package_path);

        path.strip_prefix(&package_path).map_err(|_| {
            DocError::invalid(format!(
                "documented source path {} is outside package {}",
                path.display(),
                package_path
            ))
        })?
    } else {
        path
    };

    Ok(path.to_string_lossy().replace('\\', "/"))
}

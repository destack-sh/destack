use std::path::{Path, PathBuf};
use std::sync::Arc;

use tspp_formatter::{format_source, format_source_range};
use tspp_repository::{FormatterOptions, Revision};
use tspp_source::{DiagnosticSeverity, File, Patch, Span, TextRange};

use crate::Error;

use super::Workspace;

/// One patch over an exact source file.
#[derive(Debug, Clone)]
pub struct FileEdit {
    /// The exact source file interpreted by the patch.
    pub file: Arc<File>,
    /// The source replacement.
    pub patch: Patch,
}

impl Workspace {
    /// Format one source file or selected text range.
    pub fn format_file(
        &self,
        revision: Revision,
        path: PathBuf,
        range: Option<TextRange>,
    ) -> Result<Option<FileEdit>, Error> {
        let path = self.resolve_path(&path)?;

        // read the file and formatter options from one revision
        let session = self.pin(revision)?;
        let Some(file_id) = session.file_id(&path)? else {
            return Ok(None);
        };
        let file = session.file(file_id)?;
        let options = self.formatter_options(session.revision(), &path)?;

        // format the requested source selection
        let patch = if let Some(range) = range {
            self.format_range(&path, file.as_ref(), options, range)?
        } else {
            self.format_full_file(file.as_ref(), options)?
        };
        let Some(patch) = patch else {
            return Ok(None);
        };

        Ok(Some(FileEdit { file, patch }))
    }

    /// Format one entire source file.
    fn format_full_file(
        &self,
        file: &File,
        options: FormatterOptions,
    ) -> Result<Option<Patch>, Error> {
        let formatted =
            format_source(file, file.text(), options).map_err(|error| Error::Internal {
                detail: format!("failed to format source file: {error}"),
            })?;
        if formatted
            .diagnostics
            .has_diagnostics_of_severity(DiagnosticSeverity::Error)
        {
            return Ok(None);
        }

        // replace the complete authored source
        let span = Span::new(file.id, 0, file.len);

        Ok(Some(Patch::replace(span, formatted.text)))
    }

    /// Format one selected source range.
    fn format_range(
        &self,
        path: &Path,
        file: &File,
        options: FormatterOptions,
        range: TextRange,
    ) -> Result<Option<Patch>, Error> {
        let range = range
            .byte_range(file.text())
            .map_err(|error| Error::InvalidTextChange {
                path: path.to_path_buf(),
                detail: error.to_string(),
            })?;
        let formatted = format_source_range(file, file.text(), options, range.start, range.end)
            .map_err(|error| Error::Internal {
                detail: format!("failed to format source range: {error}"),
            })?;
        if formatted
            .diagnostics
            .has_diagnostics_of_severity(DiagnosticSeverity::Error)
        {
            return Ok(None);
        }

        // convert the formatter replacement into a source patch
        let edit = formatted
            .edit
            .map(|edit| Patch::replace(edit.span, edit.text));

        Ok(edit)
    }

    /// Return selected formatter options for one path.
    fn formatter_options(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<FormatterOptions, Error> {
        // prefer package-local formatter options
        let package = self.repository.nearest_package(revision, path)?;
        if let Some(package) = package
            && let Some(config) = self
                .repository
                .destack_for_package_id(revision, package.id)?
        {
            return Ok(config.formatter);
        }

        // use workspace options or formatter defaults
        let config = self.repository.destack_for_workspace(revision)?;
        let options = config
            .map(|config| config.formatter)
            .unwrap_or_else(FormatterOptions::default);

        Ok(options)
    }
}

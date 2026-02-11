use std::path::{Path, PathBuf};
use std::sync::Arc;

use napi::Error;
use napi_derive::napi;
use {destack_workspace as workspace, destack_workspace_service as workspace_service};

use super::{CompilerOptions, Diagnostic, diagnostics_have_errors, diagnostics_have_warnings};

/// Options for check operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct CheckOptions {
    /// The current working directory.
    pub cwd: String,
    /// The workspace roots to open initially.
    pub roots: Vec<String>,
    /// Compiler options for check diagnostics.
    pub compiler: CompilerOptions,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            roots: Vec::new(),
            compiler: CompilerOptions::default(),
        }
    }
}

/// Result payload for check operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Diagnostics emitted by this check.
    pub diagnostics: Vec<Diagnostic>,
    /// Number of diagnostics.
    pub diagnostic_count: u32,
    /// Whether diagnostics include at least one error.
    pub has_errors: bool,
    /// Whether diagnostics include at least one warning.
    pub has_warnings: bool,
}

/// Get the default check options.
#[napi(js_name = "defaultCheckOptions")]
pub fn default_check_options() -> CheckOptions {
    CheckOptions::default()
}

/// Check a file synchronously.
#[napi(js_name = "checkSync")]
pub fn check_sync(
    path: String,
    content: String,
    options: Option<CheckOptions>,
) -> napi::Result<CheckResult> {
    let options = options.unwrap_or_default();
    let path = PathBuf::from(path);

    let cwd = PathBuf::from(&options.cwd);
    let roots = check_roots_from_options(&options, &cwd);
    let session = Arc::new(workspace::Session::new(cwd));
    let service =
        workspace_service::WorkspaceService::with_options(session, roots, options.compiler.into())
            .map_err(napi_error_from_check)?;

    let update_result = service
        .update_virtual_file(&path, content)
        .map_err(napi_error_from_check)?;
    let diagnostics = check_diagnostics_from_workspace_result(update_result);

    Ok(CheckResult {
        diagnostic_count: diagnostics.len() as u32,
        has_errors: diagnostics_have_errors(&diagnostics),
        has_warnings: diagnostics_have_warnings(&diagnostics),
        diagnostics,
    })
}

fn check_roots_from_options(options: &CheckOptions, cwd: &Path) -> Vec<PathBuf> {
    if options.roots.is_empty() {
        vec![cwd.to_path_buf()]
    } else {
        options.roots.iter().map(PathBuf::from).collect()
    }
}

fn check_diagnostics_from_workspace_result(
    result: workspace_service::WorkspaceServiceResult,
) -> Vec<Diagnostic> {
    result
        .updates
        .into_iter()
        .flat_map(|update| update.diagnostics)
        .map(Into::into)
        .collect()
}

fn napi_error_from_check(error: impl std::fmt::Display) -> Error {
    Error::from_reason(error.to_string())
}

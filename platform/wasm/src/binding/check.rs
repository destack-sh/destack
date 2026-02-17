use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use {destack_service as workspace_service, destack_workspace as workspace};

use super::error::{js_error, parse_optional_input, to_js_value};
use super::{CompilerOptions, Diagnostic, diagnostics_have_errors, diagnostics_have_warnings};

/// Options for check operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[wasm_bindgen(js_name = defaultCheckOptions)]
pub fn default_check_options() -> Result<JsValue, JsValue> {
    to_js_value(&CheckOptions::default())
}

/// Check a file synchronously.
#[wasm_bindgen(js_name = checkSync)]
pub fn check_sync(
    path: String,
    content: String,
    options: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    // decode options and normalize input path
    let options: CheckOptions = parse_optional_input(options)?;
    let path = PathBuf::from(path);

    // create a workspace service for this check request
    let cwd = PathBuf::from(&options.cwd);
    let roots = check_roots_from_options(&options, &cwd);
    let session = Arc::new(workspace::Session::new(cwd));
    let service =
        workspace_service::LanguageService::with_options(session, roots, options.compiler.into())
            .map_err(js_error)?;

    // update the virtual file and collect diagnostics
    let update_result = service
        .update_virtual_file(&path, content)
        .map_err(js_error)?;
    let diagnostics = check_diagnostics_from_workspace_result(update_result);

    // build the check result payload
    to_js_value(&CheckResult {
        diagnostic_count: diagnostics.len() as u32,
        has_errors: diagnostics_have_errors(&diagnostics),
        has_warnings: diagnostics_have_warnings(&diagnostics),
        diagnostics,
    })
}

/// Resolve roots for check operations.
fn check_roots_from_options(options: &CheckOptions, cwd: &Path) -> Vec<PathBuf> {
    if options.roots.is_empty() {
        vec![cwd.to_path_buf()]
    } else {
        options.roots.iter().map(PathBuf::from).collect()
    }
}

/// Collect diagnostics from workspace update records.
fn check_diagnostics_from_workspace_result(
    result: workspace_service::LanguageServiceResult,
) -> Vec<Diagnostic> {
    result
        .updates
        .into_iter()
        .flat_map(|update| update.diagnostics)
        .map(Into::into)
        .collect()
}

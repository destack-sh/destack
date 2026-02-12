use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use {
    destack_compiler as compiler, destack_workspace as workspace,
    destack_workspace_service as workspace_service,
};

use super::error::{js_error, parse_optional_input, to_js_value};
use super::{CompilerOptions, Diagnostic, diagnostics_have_errors};

/// Parsed program payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Program {
    /// Source path used to parse the module.
    pub path: String,
    /// Serialized module AST payload.
    pub ast_json: String,
    /// Number of top level roots in the module.
    pub root_count: u32,
    /// Number of semantic tokens in the module.
    pub token_count: u32,
    /// Number of side tokens in the module.
    pub side_token_count: u32,
}

/// Options for parse operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseOptions {
    /// The current working directory.
    pub cwd: String,
    /// The workspace roots to open initially.
    pub roots: Vec<String>,
    /// Compiler options for parse diagnostics.
    pub compiler: CompilerOptions,
}

impl Default for ParseOptions {
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

/// Parse result payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    /// Parsed program payload.
    pub program: Program,
    /// Parse diagnostics captured during update.
    pub diagnostics: Vec<Diagnostic>,
    /// Number of parse diagnostics.
    pub diagnostic_count: u32,
    /// Return whether diagnostics include at least one error.
    pub has_errors: bool,
}

/// Get the default parse options.
#[wasm_bindgen(js_name = defaultParseOptions)]
pub fn default_parse_options() -> Result<JsValue, JsValue> {
    to_js_value(&ParseOptions::default())
}

/// Parse a file synchronously.
#[wasm_bindgen(js_name = parseSync)]
pub fn parse_sync(
    path: String,
    content: String,
    options: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    // decode options and normalize path input
    let options: ParseOptions = parse_optional_input(options)?;
    let path_buf = PathBuf::from(&path);

    // create a workspace service for this parse request
    let cwd = PathBuf::from(&options.cwd);
    let roots = parse_roots_from_options(&options, &cwd);
    let session = Arc::new(workspace::Session::new(cwd));
    let service =
        workspace_service::WorkspaceService::with_options(session, roots, options.compiler.into())
            .map_err(js_error)?;

    // update virtual content to populate diagnostics and module state
    let update_result = service
        .update_virtual_file(&path_buf, content)
        .map_err(js_error)?;

    // collect diagnostics and read the parsed program payload
    let diagnostics = parse_diagnostics_from_result(update_result);
    let mut program = service
        .with_program_for_path(&path_buf, |program, compiler| {
            parse_program_for_path(&path_buf, &program, &compiler)
        })
        .map_err(js_error)?;

    // preserve the caller path string in the response payload
    program.path = path;

    // encode the parse response payload
    to_js_value(&ParseResult {
        diagnostic_count: diagnostics.len() as u32,
        has_errors: diagnostics_have_errors(&diagnostics),
        diagnostics,
        program,
    })
}

/// Resolve roots for parse operations.
fn parse_roots_from_options(options: &ParseOptions, cwd: &Path) -> Vec<PathBuf> {
    if options.roots.is_empty() {
        vec![cwd.to_path_buf()]
    } else {
        options.roots.iter().map(PathBuf::from).collect()
    }
}

/// Build a parsed program payload for a path.
fn parse_program_for_path(
    path: &Path,
    program: &workspace::Program,
    compiler: &compiler::Compiler,
) -> Result<Program, String> {
    // resolve and load the module state from the workspace program
    let module_id = compiler
        .resolve_path_to_module(&path.to_path_buf())
        .map_err(|error| format!("failed to resolve module for '{}': {error}", path.display()))?;

    let module = program.modules.get(module_id);
    let module = module.read();
    let module_ast = match &module.content {
        workspace::ModuleContent::Code(code) => code
            .ast
            .as_ref()
            .ok_or_else(|| format!("module AST is not available for '{}'", path.display()))?,
        workspace::ModuleContent::Data { ast, .. } => ast,
        workspace::ModuleContent::Text { ast, .. } => ast,
        workspace::ModuleContent::Binary { ast, .. } => ast,
        workspace::ModuleContent::Unloaded => {
            return Err(format!(
                "module content is not loaded for '{}'",
                path.display()
            ));
        }
    };

    // serialize key ast collections for the js payload
    let ast_json = serde_json::json!({
        "roots": module_ast.roots,
        "tokens": module_ast.tokens,
        "sideTokens": module_ast.side_tokens,
    })
    .to_string();

    Ok(Program {
        path: path.to_string_lossy().to_string(),
        ast_json,
        root_count: module_ast.roots.len() as u32,
        token_count: module_ast.tokens.len() as u32,
        side_token_count: module_ast.side_tokens.len() as u32,
    })
}

/// Collect diagnostics from workspace update records.
fn parse_diagnostics_from_result(
    result: workspace_service::WorkspaceServiceResult,
) -> Vec<Diagnostic> {
    result
        .updates
        .into_iter()
        .flat_map(|update| update.diagnostics)
        .map(Into::into)
        .collect()
}

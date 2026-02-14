use std::path::{Path, PathBuf};
use std::sync::Arc;

use napi::Error;
use napi_derive::napi;
use {
    destack_compiler as compiler, destack_service as workspace_service,
    destack_workspace as workspace,
};

use super::{CompilerOptions, Diagnostic, diagnostics_have_errors};

/// Parsed program payload.
#[napi(object)]
#[derive(Debug, Clone)]
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
#[napi(object)]
#[derive(Debug, Clone)]
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
#[napi(object)]
#[derive(Debug, Clone)]
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
#[napi(js_name = "defaultParseOptions")]
pub fn default_parse_options() -> ParseOptions {
    ParseOptions::default()
}

/// Parse a file synchronously.
#[napi(js_name = "parseSync")]
pub fn parse_sync(
    path: String,
    content: String,
    options: Option<ParseOptions>,
) -> napi::Result<ParseResult> {
    // normalize inputs
    let options = options.unwrap_or_default();
    let path_buf = PathBuf::from(&path);

    // build workspace service
    let cwd = PathBuf::from(&options.cwd);
    let roots = parse_roots_from_options(&options, &cwd);
    let session = Arc::new(workspace::Session::new(cwd));
    let service =
        workspace_service::LanguageService::with_options(session, roots, options.compiler.into())
            .map_err(napi_error_from_parse)?;

    // apply virtual update and gather diagnostics
    let update_result = service
        .update_virtual_file(&path_buf, content)
        .map_err(napi_error_from_parse)?;

    let diagnostics = parse_diagnostics_from_result(update_result);

    // extract parsed program payload
    let mut program = service
        .with_program_for_path(&path_buf, |program, compiler| {
            parse_program_for_path(&path_buf, &program, &compiler)
        })
        .map_err(Error::from_reason)?;

    // preserve caller path in the payload
    program.path = path;

    // assemble response
    Ok(ParseResult {
        diagnostic_count: diagnostics.len() as u32,
        has_errors: diagnostics_have_errors(&diagnostics),
        diagnostics,
        program,
    })
}

/// Build workspace roots for a parse request.
fn parse_roots_from_options(options: &ParseOptions, cwd: &Path) -> Vec<PathBuf> {
    if options.roots.is_empty() {
        vec![cwd.to_path_buf()]
    } else {
        options.roots.iter().map(PathBuf::from).collect()
    }
}

/// Build a serialized program payload for a path.
fn parse_program_for_path(
    path: &Path,
    program: &workspace::Program,
    compiler: &compiler::Compiler,
) -> Result<Program, String> {
    // resolve the module for this path
    let module_id = compiler
        .resolve_path_to_module(&path.to_path_buf())
        .map_err(|error| format!("failed to resolve module for '{}': {error}", path.display()))?;

    // select the parsed ast from module content
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

    // serialize the ast payload
    let ast_json = serde_json::json!({
        "roots": module_ast.roots,
        "tokens": module_ast.tokens,
        "sideTokens": module_ast.side_tokens,
    })
    .to_string();

    // return parse payload metadata
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
    result: workspace_service::LanguageServiceResult,
) -> Vec<Diagnostic> {
    result
        .updates
        .into_iter()
        .flat_map(|update| update.diagnostics)
        .map(Into::into)
        .collect()
}

/// Convert parse errors into NAPI errors.
fn napi_error_from_parse(error: impl std::fmt::Display) -> Error {
    Error::from_reason(error.to_string())
}

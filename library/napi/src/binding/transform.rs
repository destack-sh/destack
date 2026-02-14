use std::path::{Path, PathBuf};
use std::sync::Arc;

use napi::Error;
use napi_derive::napi;
use {destack_ast as ast, destack_compiler as compiler, destack_formatter as formatter};

use {
    destack_service as workspace_service, destack_source as source, destack_workspace as workspace,
};

use super::CompilerOptions;

/// Transform target format.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformTarget {
    /// Emit TypeScript output.
    TypeScript,
    /// Emit JavaScript output.
    JavaScript,
}

/// Options for transform file operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct TransformOptions {
    /// The current working directory.
    pub cwd: String,
    /// The workspace roots to open initially.
    pub roots: Vec<String>,
    /// Compiler options for analysis and generation.
    pub compiler: CompilerOptions,
    /// Output target format.
    pub target: TransformTarget,
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            roots: Vec::new(),
            compiler: CompilerOptions::default(),
            target: TransformTarget::TypeScript,
        }
    }
}

/// Result payload for transform operations.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct TransformResult {
    /// Emitted output code.
    pub code: String,
}

/// Get the default transform options.
#[napi(js_name = "defaultTransformOptions")]
pub fn default_transform_options() -> TransformOptions {
    TransformOptions::default()
}

/// Transform a file synchronously.
#[napi(js_name = "transformSync")]
pub fn transform_sync(
    path: String,
    content: String,
    options: Option<TransformOptions>,
) -> napi::Result<TransformResult> {
    // normalize inputs
    let options = options.unwrap_or_default();
    let path = PathBuf::from(path);

    // build workspace service
    let cwd = PathBuf::from(&options.cwd);
    let roots = transform_roots_from_options(&options, &cwd);
    let session = Arc::new(workspace::Session::new(cwd));
    let service =
        workspace_service::LanguageService::with_options(session, roots, options.compiler.into())
            .map_err(napi_error_from_transform)?;

    // update and analyze the module
    service
        .update_virtual_file(&path, content)
        .map_err(napi_error_from_transform)?;
    service
        .ensure_analyzed_for_path(&path)
        .map_err(napi_error_from_transform)?;

    // format transformed output
    let code = service
        .with_program_for_path(&path, |program, compiler| {
            transform_module_code(&path, &program, &compiler, options.target)
        })
        .map_err(Error::from_reason)?;

    // assemble response
    Ok(TransformResult { code })
}

/// Build workspace roots for a transform request.
fn transform_roots_from_options(options: &TransformOptions, cwd: &Path) -> Vec<PathBuf> {
    if options.roots.is_empty() {
        vec![cwd.to_path_buf()]
    } else {
        options.roots.iter().map(PathBuf::from).collect()
    }
}

/// Transform a module into formatted output code.
fn transform_module_code(
    path: &Path,
    program: &workspace::Program,
    compiler: &compiler::Compiler,
    target: TransformTarget,
) -> Result<String, String> {
    // resolve and unbind the module
    let module_id = compiler
        .resolve_path_to_module(&path.to_path_buf())
        .map_err(|error| format!("failed to resolve module for '{}': {error}", path.display()))?;
    let profile_id = program.default_profile_id_for_module(module_id);

    let module = program.modules.get(module_id);
    let module = module.read();
    let unbound = compiler.unbind_module(&module, profile_id);

    // build synthetic output file context
    let file_type = transform_target_file_type(target);
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "<transform>".to_string());
    let file = source::File::from_text(
        source::FileId::new(0),
        file_name,
        source::Uri::from_path(path),
        Some(path.to_path_buf()),
        file_type,
        String::new(),
    );

    // create formatter context for unbound trees
    let strings = unbound.strings.into_immutable();
    let empty_tokens = Vec::new();
    let empty_side_tokens = Vec::new();
    let empty_side_span = source::MultiSpan::new(Vec::new());
    let context = formatter::DestackFormatContext::new(
        formatter::DestackFormatOptions::default(),
        formatter::DestackFormatArtifacts {
            file: &file,
            tree: &unbound.tree,
            tokens: &empty_tokens,
            side_tokens: &empty_side_tokens,
            side_span: &empty_side_span,
            strings: &strings,
            parents: ast::NodeParentIndex::from_tree(&unbound.tree),
        },
    );

    // format each root and join chunks
    let mut chunks = Vec::new();
    for root_id in &unbound.roots {
        let formatted = destack_fir::format!(context.clone(), [root_id])
            .map_err(|error| format!("failed to format '{}': {error}", path.display()))?;
        let printed = formatted
            .print()
            .map_err(|error| format!("failed to print '{}': {error}", path.display()))?;
        chunks.push(printed.into_str());
    }

    Ok(chunks.join("\n\n"))
}

/// Map a transform target to the output file type.
fn transform_target_file_type(target: TransformTarget) -> source::FileType {
    match target {
        TransformTarget::TypeScript => source::FileType::TypeScript,
        TransformTarget::JavaScript => source::FileType::JavaScript,
    }
}

/// Convert transform errors into NAPI errors.
fn napi_error_from_transform(error: impl std::fmt::Display) -> Error {
    Error::from_reason(error.to_string())
}

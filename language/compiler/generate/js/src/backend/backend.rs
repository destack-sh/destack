//! JS codegen backend implementation.

use std::sync::Arc;

use destack_artifact::{ArtifactStore, OutputEntry};
use destack_codegen_lib::CodegenBackend;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Program, Target};

use crate::{CodegenJsError, CodegenJsResult};

/// Output from JS code generation.
#[derive(Debug)]
pub struct CodegenJsOutput {
    /// Generated output entries.
    pub entries: Vec<OutputEntry>,
    /// Warnings encountered during generation.
    pub warnings: Vec<crate::CodegenJsWarning>,
    /// Non fatal errors encountered during generation.
    pub errors: Vec<CodegenJsError>,
}

/// JavaScript/TypeScript codegen backend.
#[derive(Debug, Default)]
pub struct JsBackend;

impl CodegenBackend for JsBackend {
    fn name(&self) -> &'static str {
        "js"
    }

    fn supports_target(&self, target: &Target) -> bool {
        target.uses_js_generate_pipeline()
    }
}

/// Generate code for a module.
///
/// This is the main entry point for JS/TS code generation from the compiler.
/// Returns outputs and any warnings encountered during generation.
pub fn generate_module(
    program: Arc<Program>,
    artifacts: Arc<ArtifactStore>,
    module_id: ModuleId,
    target: &Target,
    profile: ProfileId,
) -> CodegenJsResult<CodegenJsOutput> {
    // validate target
    if !target.uses_js_generate_pipeline() {
        return Err(CodegenJsError::UnsupportedTarget {
            format: format!("{:?}", target.emit),
            message: Some("expected JS, TS, or HTML".to_string()),
        });
    }

    // get module
    let module_ref = program.modules.get(module_id);
    let module = module_ref.as_ref();
    let ast = artifacts
        .ast(module_id)
        .unwrap_or_else(|| panic!("missing committed AST artifact for {module_id:?}"));
    let dir = artifacts
        .dir_analyzed(module_id, profile)
        .unwrap_or_else(|| panic!("missing committed dir artifact for {module_id:?}"));
    let dir_tree = &dir.tree;
    let dir_roots = dir.roots.as_ref().clone();
    let symbols = &dir.symbols;
    let types = &dir.types;

    // get package path and root_dir for output path resolution
    let package = program.packages.get(module.package_id);
    let package = package.read();
    let package_dir = package.path.clone().unwrap_or_else(|| program.cwd.clone());
    let root_dir = package
        .config
        .as_ref()
        .and_then(|c| c.options.compiler.root_dir.clone());
    drop(package);

    // plan the requested output shape
    let plan = crate::plan_module_output(&module, target, &package_dir, root_dir.as_deref())?;

    // emit one lowered JavaScript module tree
    let emit = crate::emit_module(&module, &ast, dir_tree, &dir_roots, symbols, types, target)?;

    // assemble target level output shape
    let emit = crate::bundle_module_output(&plan, emit)?;

    // apply post assembly minification policy
    let emit = crate::minify_module_output(&plan, emit)?;

    // print the final output entries
    let (entries, warnings, errors) = crate::print_module_output(target, &plan, emit)?;

    Ok(CodegenJsOutput {
        entries,
        warnings,
        errors,
    })
}

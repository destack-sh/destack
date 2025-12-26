//! JS codegen backend implementation.

use std::sync::Arc;

use destack_codegen_lib::CodegenBackend;
use destack_source::ModuleId;
use destack_workspace::{OutputFormat, ProfileId, Program, Target};

use crate::lower::{CodegenJsOutput, ModuleLowerer};
use crate::{CodegenJsError, CodegenJsResult};

/// JavaScript/TypeScript codegen backend.
#[derive(Debug, Default)]
pub struct JsBackend;

impl CodegenBackend for JsBackend {
    fn name(&self) -> &'static str {
        "js"
    }

    fn supports_target(&self, target: &Target) -> bool {
        matches!(target.output, OutputFormat::Js | OutputFormat::Ts)
    }
}

/// Generate code for a module.
///
/// This is the main entry point for JS/TS code generation from the compiler.
/// Returns artifacts and any warnings encountered during generation.
pub fn generate_module(
    program: Arc<Program>,
    module_id: ModuleId,
    target: &Target,
    profile: ProfileId,
) -> CodegenJsResult<CodegenJsOutput> {
    // validate target
    if !matches!(target.output, OutputFormat::Js | OutputFormat::Ts) {
        return Err(CodegenJsError::UnsupportedTarget {
            format: format!("{:?}", target.output),
            message: Some("expected Js or Ts".to_string()),
        });
    }

    // get module
    let module_ref = program.modules.get(module_id);
    let module = module_ref.read();
    let ast = module.ast();
    let dir = module.dir(profile);
    let dir_tree = dir.tree.read();
    let dir_roots = dir.roots.clone();
    let symbols = dir.symbols.read();
    let types = dir.types.read();

    // get package path and root_dir for output path resolution
    let package = program.packages.get(module.package_id);
    let package = package.read();
    let package_dir = package.path.clone().unwrap_or_else(|| program.cwd.clone());
    let root_dir = package
        .dsconfig
        .as_ref()
        .and_then(|c| c.options.compiler.root_dir.clone());
    drop(package);

    // create lowerer and process
    let mut lowerer = ModuleLowerer::new(
        &module, ast, &dir_tree, &dir_roots, &symbols, &types, target,
    );
    lowerer.lower_module()?;

    // finish and get artifacts + warnings
    let registry_next_id = || program.artifacts.next_id();
    lowerer.finish(registry_next_id, &package_dir, root_dir.as_deref())
}

//! JS codegen backend implementation.

use std::sync::Arc;

use destack_codegen_lib::CodegenBackend;
use destack_source::ModuleId;
use destack_workspace::{OutputFormat, Program, Target};

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
) -> CodegenJsResult<CodegenJsOutput> {
    // validate target
    if !matches!(target.output, OutputFormat::Js | OutputFormat::Ts) {
        return Err(CodegenJsError::UnsupportedTarget {
            format: format!("{:?}", target.output),
            message: Some("expected Js or Ts".to_string()),
        });
    }

    // get module and acquire read locks
    let module_ref = program.modules.get(module_id);
    let module = module_ref.read();
    let dir_tree = module.dir.tree.read();
    let symbols = module.dir.symbols.read();
    let types = module.dir.types.read();

    // get package path for output path resolution
    let package_ref = program.packages.get(module.package_id);
    let package = package_ref.read();
    let package_dir = package.path.clone().unwrap_or_else(|| program.cwd.clone());
    drop(package);

    // create lowerer and process
    let mut lowerer = ModuleLowerer::new(&module, &dir_tree, &symbols, &types, target);
    lowerer.lower_module()?;

    // finish and get artifacts + warnings
    let registry_next_id = || program.artifacts.next_id();
    lowerer.finish(registry_next_id, &package_dir)
}

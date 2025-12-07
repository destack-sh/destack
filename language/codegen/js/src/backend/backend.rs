//! JS codegen backend implementation.

use std::sync::Arc;

use destack_codegen_lib::CodegenBackend;
use destack_source::ModuleId;
use destack_workspace::{Artifact, ArtifactId, OutputFormat, Program, Target};

use crate::CodegenJsResult;
use crate::lower::ModuleLowerer;

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

/// Generate artifacts for a module.
///
/// This is the main entry point for JS/TS code generation from the compiler.
pub fn generate_module(
    program: Arc<Program>,
    module_id: ModuleId,
    target: &Target,
    registry_next_id: impl Fn() -> ArtifactId,
) -> CodegenJsResult<Vec<Artifact>> {
    // validate target
    if !matches!(target.output, OutputFormat::Js | OutputFormat::Ts) {
        return Err(crate::CodegenJsError::UnsupportedConstruct {
            node: destack_dir::GlobalNodeIdAny::new(
                module_id,
                destack_dir::LocalNodeIdAny {
                    id: 0,
                    ty: destack_dir::NodeType::Expression,
                },
            ),
            message: Some(format!("unsupported output format: {:?}", target.output)),
        });
    }

    // get module and acquire read locks
    let module_ref = program.modules.get(module_id);
    let module = module_ref.read();
    let dir_tree = module.dir.tree.read();
    let symbols = module.dir.symbols.read();
    let types = module.dir.types.read();

    // create lowerer and process
    let mut lowerer = ModuleLowerer::new(&program, &module, &dir_tree, &symbols, &types, target);
    lowerer.lower()?;
    lowerer.finish(registry_next_id)
}

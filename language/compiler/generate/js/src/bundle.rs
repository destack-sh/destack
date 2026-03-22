use crate::CodegenJsResult;
use crate::emit::ModuleEmitOutput;
use crate::plan::{JsGenerateMode, ModuleGeneratePlan};

/// Assemble one target level JavaScript output shape.
pub fn bundle_module_output(
    plan: &ModuleGeneratePlan,
    emit: ModuleEmitOutput,
) -> CodegenJsResult<ModuleEmitOutput> {
    // FUGU #Incomplete: assembled target generation still preserves per module output
    if plan.mode == JsGenerateMode::Assembled {
        return Ok(emit);
    }

    Ok(emit)
}

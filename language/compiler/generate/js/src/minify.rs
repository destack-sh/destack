use crate::CodegenJsResult;
use crate::emit::ModuleEmitOutput;
use crate::plan::ModuleGeneratePlan;

/// Minify one assembled JavaScript output shape.
pub fn minify_module_output(
    plan: &ModuleGeneratePlan,
    emit: ModuleEmitOutput,
) -> CodegenJsResult<ModuleEmitOutput> {
    // FUGU #Incomplete: final JS minification is not implemented yet
    if plan.should_minify {
        return Ok(emit);
    }

    Ok(emit)
}

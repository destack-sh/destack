use destack_codegen_lib::CodegenBackend;
use destack_workspace::Target;

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

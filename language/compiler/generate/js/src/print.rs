use destack_artifact::{OutputContent, OutputFile};
use destack_fir::format as fir_format;
use destack_fir::prelude::format_with;
use destack_source::{File, FileType};
use destack_workspace::Target;

use crate::emit::ModuleEmitOutput;
use crate::plan::ModuleGeneratePlan;
use crate::{
    CodegenJsError, CodegenJsFormatContext, CodegenJsFormatOptions, CodegenJsResult, ScriptModule,
    format_statements,
};

/// Print one generated script module to text for a specific file type.
pub fn print_script_module(
    target: &Target,
    file_type: FileType,
    module: &ScriptModule,
) -> CodegenJsResult<String> {
    let strings = module.strings.clone().into_immutable();
    let source_file = File::empty_text(file_type);
    let options = CodegenJsFormatOptions::from_target(target, file_type);
    let context = CodegenJsFormatContext {
        options,
        file: &source_file,
        tree: &module.tree,
        roots: &module.roots,
        strings: &strings,
    };
    let formatted = fir_format!(
        context,
        [format_with(|f| format_statements(f, &module.roots))]
    );
    let formatted = match formatted {
        Ok(formatted) => formatted,
        Err(error) => {
            return Err(CodegenJsError::Internal {
                message: format!("failed to format roots: {error}"),
            });
        }
    };
    let printed = match formatted.print() {
        Ok(printed) => printed,
        Err(error) => {
            return Err(CodegenJsError::Internal {
                message: format!("failed to print formatted: {error}"),
            });
        }
    };

    Ok(printed.as_str().to_string())
}

/// Print one emitted JavaScript module into final output files.
pub fn print_module_output(
    target: &Target,
    plan: &ModuleGeneratePlan,
    emit: ModuleEmitOutput,
) -> CodegenJsResult<(
    Vec<OutputFile>,
    Vec<crate::CodegenJsWarning>,
    Vec<crate::CodegenJsError>,
)> {
    let module = emit.module;
    let warnings = emit.warnings;
    let errors = emit.errors;
    let mut entries = Vec::new();

    // print each planned file with the requested output format
    for file in &plan.files {
        let code = print_script_module(target, file.file_type, &module)?;

        let content = match file.file_type {
            FileType::JavaScript => OutputContent::javascript(code),
            FileType::TypeScript => OutputContent::typescript(code),
            FileType::TypeScriptDeclaration => OutputContent::declaration(code),
            FileType::Html => OutputContent::html(wrap_html_document(&code)),
            other => {
                return Err(CodegenJsError::Internal {
                    message: format!("unsupported file type: {other:?}"),
                });
            }
        };
        entries.push(OutputFile {
            uri: file.uri.clone(),
            content,
            source: None,
        });
    }

    Ok((entries, warnings, errors))
}

/// Wrap one emitted module body in a minimal HTML document shell.
pub fn wrap_html_document(code: &str) -> String {
    format!(
        "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n</head>\n<body>\n<script type=\"module\">\n{code}\n</script>\n</body>\n</html>\n"
    )
}

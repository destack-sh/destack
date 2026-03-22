use destack_fir::format as fir_format;
use destack_fir::prelude::format_with;
use destack_source::{File, FileType};
use destack_workspace::{OutputContent, OutputEntry, Target};

use crate::emit::ModuleEmitOutput;
use crate::plan::ModuleGeneratePlan;
use crate::{
    CodegenJsError, CodegenJsFormatContext, CodegenJsFormatOptions, CodegenJsResult,
    format_statements,
};

/// Print one emitted JavaScript module into final output entries.
pub fn print_module_output(
    target: &Target,
    plan: &ModuleGeneratePlan,
    emit: ModuleEmitOutput,
) -> CodegenJsResult<(
    Vec<OutputEntry>,
    Vec<crate::CodegenJsWarning>,
    Vec<crate::CodegenJsError>,
)> {
    let strings = emit.strings.into_immutable();
    let roots = emit.roots;
    let tree = emit.tree;
    let warnings = emit.warnings;
    let errors = emit.errors;
    let mut entries = Vec::new();

    // print each planned file with the requested output format
    for file in &plan.files {
        let source_file = File::empty_text(file.file_type);
        let options = CodegenJsFormatOptions::from_target(target, file.file_type);
        let context = CodegenJsFormatContext {
            options,
            file: &source_file,
            tree: &tree,
            roots: &roots,
            strings: &strings,
        };
        let formatted = fir_format!(context, [format_with(|f| format_statements(f, &roots))]);
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
        let code = printed.as_str().to_string();

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
        entries.push(OutputEntry {
            uri: file.uri.clone(),
            content,
            source: None,
        });
    }

    Ok((entries, warnings, errors))
}

/// Wrap one emitted module body in a minimal HTML document shell.
fn wrap_html_document(code: &str) -> String {
    format!(
        "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n</head>\n<body>\n<script type=\"module\">\n{code}\n</script>\n</body>\n</html>\n"
    )
}

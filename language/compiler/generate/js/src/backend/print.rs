use destack_artifact::{Ast, DirPatched};
use destack_fir::format as fir_format;
use destack_fir::format::FileMarker;
use destack_fir::prelude::format_with;
use destack_source::{File, FileType};
use destack_workspace::Target;

use crate::{
    CodegenJsError, CodegenJsFormatContext, CodegenJsFormatOptions, CodegenJsResult, ScriptModule,
    format_statements,
};

/// One printed script module with precise source markers.
#[derive(Debug, Clone)]
pub struct PrintedScriptModule {
    /// The printed module text.
    pub code: String,
    /// The precise output to source markers.
    pub markers: Vec<FileMarker>,
}

/// Print one generated script module to text for a specific file type.
pub fn print_script_module(
    target: &Target,
    ast: &Ast,
    dir: &DirPatched,
    source_file: &File,
    file_type: FileType,
    module: &ScriptModule,
) -> CodegenJsResult<PrintedScriptModule> {
    let strings = module.strings.clone().into_immutable();
    let options = CodegenJsFormatOptions::from_target(target, file_type);
    let context = CodegenJsFormatContext {
        options,
        file: source_file,
        ast,
        dir,
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

    Ok(PrintedScriptModule {
        code: printed.as_str().to_string(),
        markers: printed.into_sourcemap(),
    })
}

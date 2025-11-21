use crate::{BindTask, Compiler, CompilerTask, ImportError, ImportResult};

use dyst_dir::{Module, PackageId, Session};
use dyst_parser::Parser;
use dyst_source::{DiagnosticCollector, FileId, StringId};

/// Task to import a file into the compiler.
#[derive(Debug, Clone)]
pub enum ImportTask {
    /// Import module from a (preloaded) file.
    ImportModuleFromFile { file: FileId },
    /// Import module from a specifier.
    ImportModuleFromSpecifier {
        directory: Option<StringId>,
        target: StringId,
    },
}

impl ImportTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            ImportTask::ImportModuleFromFile { .. } => 1,
            ImportTask::ImportModuleFromSpecifier { .. } => 2,
        }
    }

    /// Get a message for the task.
    pub fn message<'a>(&self, session: &'a Session<'a>) -> String {
        match self {
            ImportTask::ImportModuleFromFile { file: file_id } => {
                format!("import file:'{file_id:?}'")
            }
            ImportTask::ImportModuleFromSpecifier { directory, target } => {
                let target_str = session.strings.get(*target).to_string();
                if let Some(directory) = directory {
                    let directory_str = session.strings.get(*directory).to_string();
                    format!("import '{target_str}' from '{directory_str}'")
                } else {
                    format!("import '{target_str}'")
                }
            }
        }
    }
}

impl From<ImportTask> for CompilerTask {
    fn from(task: ImportTask) -> Self {
        CompilerTask::Import(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process an import task.
    pub fn process_import(&self, task: ImportTask) -> ImportResult<()> {
        let file = match task {
            ImportTask::ImportModuleFromFile { file: file_id } => {
                match self.session.files.get(file_id) {
                    Some(file) => file,
                    None => return Err(ImportError::FileIdNotFound { file_id }),
                }
            }
            ImportTask::ImportModuleFromSpecifier { directory, target } => {
                return Err(ImportError::ModuleNotFound {
                    target,
                    directory,
                    error: None,
                });
            }
        };
        let package_id: Option<PackageId> = None; // nocheckin: resolve package for module

        // parse AST from file
        let mut diagnostics = DiagnosticCollector::new();
        let mut parser = Parser::lex_file(file, self.session.language, &mut diagnostics);
        let expressions = parser.parse();
        self.session.diagnostics.merge_from(parser.diagnostics);

        // insert module
        let module_id = self.session.modules.next_id();
        let module = Module::new(
            module_id,
            file.id,
            file.uri.clone(),
            package_id,
            parser.tree,
            expressions,
            parser.strings,
        );
        self.session.modules.insert(module);

        // next task: bind module
        self.enqueue(BindTask::BindModule { module: module_id }.into());

        Ok(())
    }
}

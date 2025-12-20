use crate::{Compiler, ImportError, ImportResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_parser::Parser;
use destack_source::{File, LanguageType, ModuleId};
use destack_workspace::ModuleAst;

/// Task to import (load and parse) a module.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Import)]
pub enum ImportTask {
    /// Import a module by its id (module must already be registered).
    #[task(code = 1, trace = "module={module}")]
    ImportModule { module: ModuleId },
}

impl Compiler {
    /// Process an import task.
    pub fn process_import(&self, task: ImportTask) -> ImportResult<()> {
        match task {
            ImportTask::ImportModule { module } => self.import_module(module),
        }
    }

    /// Import (load and parse) a module.
    fn import_module(&self, module_id: ModuleId) -> ImportResult<()> {
        let module = self.program.modules.get(module_id);

        // check if already parsed
        {
            let module = module.read();
            if module.ast.is_some() {
                return Ok(());
            }
        }

        // get module info
        let (file_id, path, uri) = {
            let module = module.read();
            (module.file_id, module.path.clone(), module.uri.clone())
        };

        // load file if not already loaded
        let file = self.program.files.get(file_id);
        let file =
            if file.is_loaded() {
                // file already has content (e.g., inline string or pre-loaded)
                file
            } else {
                // read from filesystem
                let path = path.ok_or_else(|| ImportError::ModuleNotFound {
                    target: self.program.strings.intern(&uri),
                    error: None,
                })?;
                let content = self.program.fs.read_to_string(&path).map_err(|_| {
                    ImportError::ModuleNotFound {
                        target: self.program.strings.intern(path.to_string_lossy()),
                        error: None,
                    }
                })?;

                // replace the blank file with loaded content
                let loaded_file = File::from_text(
                    file_id,
                    file.name.clone(),
                    uri,
                    Some(path),
                    file.ty,
                    content,
                );
                self.program.files.replace(loaded_file);
                self.program.files.get(file_id)
            };

        // parse
        let language_type = LanguageType::from(file.ty);
        let mut parser = Parser::lex_file(file.clone(), language_type);
        let expressions = parser.parse();
        self.program.diagnostics.merge_from(&parser.diagnostics);

        // update module with AST
        let mut module = module.write();
        module.ast = Some(ModuleAst::from_tree(
            module_id,
            module.version,
            parser.tree,
            expressions,
            parser.strings,
            parser.tokens,
            parser.side_tokens,
        ));
        drop(module);

        tracing::trace!(?module_id, "import.module.parse");
        Ok(())
    }

    /// Ensure a module has been imported (loaded and parsed).
    pub fn require_import_module(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ImportTask::ImportModule { module })
    }
}

use crate::{
    Compiler, ImportError, ImportResult, Task, TaskDebug, TaskDependencyError, TaskOutput,
};

use destack_ast::TokenType;
use destack_parser::Parser;
use destack_source::{File, ModuleId};
use destack_workspace::{Module, ModuleAst, Program, ModuleType};

/// Task to import (load and parse) a module.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ImportTask {
    /// Import a module by its id (module must already be registered).
    ImportModule { module: ModuleId },
}

impl ImportTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            ImportTask::ImportModule { .. } => 1,
        }
    }
}

impl TaskDebug for ImportTask {
    fn name(&self) -> &'static str {
        match self {
            ImportTask::ImportModule { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            ImportTask::ImportModule { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
        }
    }
}

impl From<ImportTask> for Task {
    fn from(task: ImportTask) -> Self {
        Task::Import(task)
    }
}

/// Output of an import task.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportOutput {
    pub module: ModuleId,
}

impl From<ImportOutput> for TaskOutput {
    fn from(output: ImportOutput) -> Self {
        TaskOutput::Import(output)
    }
}

impl Compiler {
    /// Process an import task.
    pub fn process_import(&self, task: ImportTask) -> ImportResult<ImportOutput> {
        match task {
            ImportTask::ImportModule { module } => self.import_module(module),
        }
    }

    /// Import (load and parse) a module.
    fn import_module(&self, module_id: ModuleId) -> ImportResult<ImportOutput> {
        let module = self.program.modules.get(module_id);

        // check if already parsed
        {
            let module = module.read();
            if module.is_parsed() {
                return Ok(ImportOutput { module: module_id });
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
        let mut parser = Parser::lex_file(file.clone(), self.program.language);
        let expressions = parser.parse();
        self.program.diagnostics.merge_from(&parser.diagnostics);

        // update module with AST
        let mut module = module.write();
        self.check_imported_module(&module, module.module_type, &parser);
        module.ast = ModuleAst::from_tree(module_id, parser.tree, expressions, parser.strings);
        drop(module);

        tracing::trace!(?module_id, "import.module.parse");
        Ok(ImportOutput { module: module_id })
    }

    /// Ensure a module has been imported (loaded and parsed).
    pub fn require_import(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.require_task(ImportTask::ImportModule { module })
    }

    /// Check if the module is valid in context.
    /// (Unfortunately we need parser state here to check the actual tokens.)
    fn check_imported_module(&self, _module: &Module, module_type: ModuleType, parser: &Parser) {
        // HTML comments are forbidden in ES modules (ECMAScript Annex B.1.3)
        // Note: HtmlComment is a trivia token, so it lives in side_tokens, not tokens
        if module_type.is_module() {
            for token in &parser.side_tokens {
                if token.token.ty == TokenType::HtmlComment {
                    self.error(ImportError::UnsupportedConstruct {
                        span: token.span,
                        message: "HTML comments are not allowed in ES modules".to_string(),
                    });
                }
            }
        }
    }
}

use crate::{Compiler, ImportError, ImportResult};

use dyst_dir::Module;
use dyst_parser::Parser;
use dyst_source::{DiagnosticCollector, FileId};

/// Task to import a file into the compiler.
#[derive(Debug, Clone)]
pub enum ImportTask {
    /// Feed a file from a file id.
    ImportFileFromId { file_id: FileId },
}

impl<'a> Compiler<'a> {
    /// Process an import task.
    pub fn process_import(&mut self, task: ImportTask) -> ImportResult<()> {
        let file = match task {
            ImportTask::ImportFileFromId { file_id } => match self.session.files.get(file_id) {
                Some(file) => file,
                None => return Err(ImportError::FileIdNotFound { file_id }),
            },
        };

        // parse AST from file
        let mut diagnostics = DiagnosticCollector::new();
        let mut parser = Parser::lex_file(file, self.session.language, &mut diagnostics);
        let expressions = parser.parse();
        self.session.diagnostics.merge_from(parser.diagnostics);

        // lower AST into DIR AST
        let module_id = self.session.modules.next_id();
        let module = Module::from_file(
            module_id,
            file.id,
            file.uri.clone(),
            parser.tree,
            parser.strings,
        );
        self.lower_module(module, &expressions);

        Ok(())
    }
}

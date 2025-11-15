use crate::{Compiler, ImportError, ImportResult};

use dyst_dir::Module;
use dyst_parser::Parser;
use dyst_source::{DiagnosticCollector, FileId, Uri};

/// Task to import a file into the compiler.
#[derive(Debug, Clone)]
pub enum ImportTask {
    /// Feed a file from a file id.
    ImportFileFromId { file_id: FileId },
    /// Import a file from a URI.
    ImportFileFromUri { path: Uri },
}

impl<'a> Compiler<'a> {
    /// Process a import task.
    pub fn process_import(&mut self, task: ImportTask) -> ImportResult<()> {
        let file = match task {
            ImportTask::ImportFileFromId { file_id } => match self.session.files.get(file_id) {
                Some(file) => file,
                None => return Err(ImportError::FileIdNotFound { file_id }),
            },
            ImportTask::ImportFileFromUri { path } => {
                todo!("process_import_from_disk({path:?})")
            }
        };

        // parse
        let mut diagnostics = DiagnosticCollector::new();
        let mut parser = Parser::lex_file(file, self.session.language, &mut diagnostics);
        let expressions = parser.parse();
        self.session.diagnostics.merge_from(parser.diagnostics);

        // lower & insert
        let module_id = self.session.modules.next_id();
        let mut module = Module::from_file(
            module_id,
            file.id,
            file.uri.clone(),
            parser.tree,
            parser.strings,
        );
        let expressions: Vec<_> = expressions
            .into_iter()
            .map(|id| self.lower_expression(&module, id))
            .collect();
        module.expressions.extend(expressions);
        self.session.modules.insert(module);

        Ok(())
    }
}

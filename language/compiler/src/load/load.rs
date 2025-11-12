use crate::{Compiler, LoadError, LoadResult};

use dyst_dir::Module;
use dyst_parser::Parser;
use dyst_source::{DiagnosticCollector, FileId, Uri};

/// Task to load a file into the compiler.
#[derive(Debug, Clone)]
pub enum LoadTask {
    /// Feed a preloaded file.
    LoadFileFromMemory { file_id: FileId },
    /// Load a file from disk and feed it.
    LoadFileFromDisk { path: Uri },
}

impl<'a> Compiler<'a> {
    /// Process a load task.
    pub fn process_load(&mut self, task: LoadTask) -> LoadResult<()> {
        let file = match task {
            LoadTask::LoadFileFromMemory { file_id } => match self.session.files.get(file_id) {
                Some(file) => file,
                None => return Err(LoadError::FileIdNotFound { file_id }),
            },
            LoadTask::LoadFileFromDisk { path } => {
                todo!("process_load_from_disk({path:?})")
            }
        };

        // parse
        let mut diagnostics = DiagnosticCollector::new();
        let mut parser = Parser::lex_file(&file, self.session.language, &mut diagnostics);
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

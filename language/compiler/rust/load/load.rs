use crate::Compiler;

use dyst_dir::Module;
use dyst_parser::Parser;
use dyst_source::{DiagnosticCollector, File, Uri};

/// Task to load a file into the compiler.
#[derive(Debug, Clone)]
pub enum LoadTask {
    /// Feed a preloaded file.
    LoadFileFromMemory { file: File },
    /// Load a file from disk and feed it.
    LoadFileFromDisk { path: Uri },
}

impl<'a> Compiler<'a> {
    /// Process a load task.
    pub fn process_load(&mut self, task: LoadTask) {
        let file = match task {
            LoadTask::LoadFileFromMemory { file } => file,
            LoadTask::LoadFileFromDisk { path } => {
                panic!("process_load_from_disk({path:?})")
            }
        };

        // parse
        let mut diagnostics = DiagnosticCollector::new();
        let mut parser = Parser::lex_file(&file, self.language, &mut diagnostics);
        let expressions = parser.parse();
        self.diagnostics.merge_from(parser.diagnostics);

        // lower & insert
        let (tree, strings) = (parser.tree, parser.strings);
        let mut module = Module::from_file(file, tree, strings);
        let expressions: Vec<_> = expressions
            .into_iter()
            .map(|id| self.lower_expression(&module, id))
            .collect();
        module.expressions.extend(expressions);
        self.modules.insert(module);

        // nocheckin: schedule evaluate/next tasks
    }
}

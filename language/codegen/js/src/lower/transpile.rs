use std::sync::Arc;

use destack_ast::StringPool;
use destack_source::{DiagnosticCollector, FileId, Uri};
use destack_workspace::{ModuleRegistry, Program};

use crate::{
    TranspileOptions, Transpiler, TranspilerMode, TranspilerUnit, TranspilerUnitId, tree as ast,
};

impl Transpiler {
    /// Map the modules to the units.
    fn make_units(
        program: Arc<Program>,
        options: &TranspileOptions,
        modules: &ModuleRegistry,
    ) -> Vec<TranspilerUnit> {
        let mut units: Vec<TranspilerUnit> = Vec::new();
        match options.mode {
            // map every module to an artifact
            TranspilerMode::Retained => {
                for (idx, module) in modules.iter().enumerate() {
                    let module = module.read();
                    let unit_id = TranspilerUnitId::new(idx as u32);
                    let uri = module.uri.without_extension();
                    let unit = TranspilerUnit {
                        id: unit_id,
                        program: program.clone(),
                        options: options.clone(),
                        uri,
                        ast: ast::NodeTree::new(),
                        roots: Vec::new(),
                        strings: StringPool::new(),
                        sources: vec![module.id],
                        pending_diagnostics: DiagnosticCollector::new(),
                        artifacts: Vec::new(),
                    };
                    units.push(unit);
                }
            }
            // map all modules to a single artifact
            TranspilerMode::Combined => {
                let unit = TranspilerUnit {
                    id: TranspilerUnitId::new(0),
                    program: program.clone(),
                    options: options.clone(),
                    uri: Uri::from_string("combined"),
                    ast: ast::NodeTree::new(),
                    roots: Vec::new(),
                    strings: StringPool::new(),
                    sources: modules.iter().map(|module| module.read().id).collect(),
                    pending_diagnostics: DiagnosticCollector::new(),
                    artifacts: Vec::new(),
                };
                units.push(unit);
            }
        }
        units
    }

    /// Lower the compiler's DIR into JS/TS artifacts.
    #[tracing::instrument(name = "transpiler.transpile", level = "debug", skip(self))]
    pub fn transpile(&self) {
        let module_count = self.program.modules.len();
        tracing::debug!(module_count, "transpiler.transpile");

        // transpile each module into AST
        let mut units =
            Transpiler::make_units(self.program.clone(), &self.options, &self.program.modules);
        for unit in units.iter_mut() {
            for source_module_id in unit.sources.clone() {
                let source_module = self.program.modules.get(source_module_id);
                let source_module = source_module.read();
                tracing::trace!(uri = %source_module.uri, "transpiler.module");
                let tree = source_module.tree.read();
                let symbols = source_module.symbols.read();
                let types = source_module.types.read();
                self.lower_module(&source_module, &tree, &symbols, &types, unit);
            }
        }

        // generate artifacts
        for (file_idx, mut unit) in units.into_iter().enumerate() {
            let file_id = FileId::new(file_idx as u32);
            for language in self.options.target.language_targets() {
                let formatting = self.options.formatting.with_language(language);
                match self.generate_artifact(&unit, file_id, formatting, language) {
                    Ok(artifact) => {
                        tracing::trace!(uri = %artifact.file.uri, "transpiler.artifact");
                        unit.artifacts.push(artifact.file.uri.clone());
                        self.artifacts.insert(artifact.file.uri.clone(), artifact);
                    }
                    Err(error) => unit.error(error),
                }
            }
            // add all the diagnostics to the program
            for diagnostic in unit.pending_diagnostics.drain() {
                self.pending_diagnostics.insert(diagnostic);
            }
            self.units.insert(unit.uri.clone(), unit);
        }

        // flush remaining diagnostics
        self.flush_diagnostics();
    }
}

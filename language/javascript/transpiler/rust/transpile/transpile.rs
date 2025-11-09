use dyst_dir::ModuleGraph;
use dyst_javascript_ast as ast;

use crate::{
    LanguageFormatOptions, Transpiler, TranspilerMode, TranspilerOptions, TranspilerUnit,
    TranspilerUnitId,
};

impl<'a> Transpiler<'a> {
    /// Map the modules to the transpiled modules.
    pub(crate) fn make_transpiled_modules(
        options: TranspilerOptions,
        modules: &ModuleGraph,
    ) -> Vec<TranspilerUnit> {
        let mut transpiled_modules: Vec<TranspilerUnit> = Vec::new();
        match options.mode {
            // map every module to an artifact
            TranspilerMode::Retained => {
                for (idx, module) in modules.iter().enumerate() {
                    let unit_id = TranspilerUnitId::new(idx as u32);
                    let transpiled_module = TranspilerUnit {
                        id: unit_id,
                        ast: ast::NodeTree::new(),
                        sources: vec![module.id],
                    };
                    transpiled_modules.push(transpiled_module);
                }
            }
            // map all modules to a single artifact
            TranspilerMode::Combined => {
                let transpiled_module = TranspilerUnit {
                    id: TranspilerUnitId::new(0),
                    ast: ast::NodeTree::new(),
                    sources: modules.iter().map(|module| module.id).collect(),
                };
                transpiled_modules.push(transpiled_module);
            }
        }
        transpiled_modules
    }

    /// Transpile the compiler's DIR into JS/TS/.. artifacts.
    pub fn transpile(&mut self) {
        // transpile each module into AST
        let mut units = Transpiler::make_transpiled_modules(self.options, self.modules);
        for unit in units.iter_mut() {
            for source_module_id in unit.sources.clone() {
                let source_module = self
                    .modules
                    .get(source_module_id)
                    .unwrap_or_else(|| panic!("source module not found: {source_module_id:?}"));
                self.transpile_module(source_module, unit);
            }
        }
        for unit in units.into_iter() {
            self.units.insert(unit.id, unit);
        }

        // print content
        for unit in self.units.values() {
            let artifact = self.print_unit(unit);
            self.artifacts.push(artifact);
        }
    }
}

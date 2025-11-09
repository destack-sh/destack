use dyst_dir::ModuleGraph;
use dyst_javascript_ast as ast;

use crate::{Transpiler, TranspilerMode, TranspilerOptions, TranspilerUnit, TranspilerUnitId};

impl<'a> Transpiler<'a> {
    /// Map the modules to the units.
    pub(crate) fn make_units(
        options: TranspilerOptions,
        modules: &ModuleGraph,
    ) -> Vec<TranspilerUnit> {
        let mut units: Vec<TranspilerUnit> = Vec::new();
        match options.mode {
            // map every module to an artifact
            TranspilerMode::Retained => {
                for (idx, module) in modules.iter().enumerate() {
                    let unit_id = TranspilerUnitId::new(idx as u32);
                    let unit = TranspilerUnit {
                        id: unit_id,
                        ast: ast::NodeTree::new(),
                        sources: vec![module.id],
                    };
                    units.push(unit);
                }
            }
            // map all modules to a single artifact
            TranspilerMode::Combined => {
                let unit = TranspilerUnit {
                    id: TranspilerUnitId::new(0),
                    ast: ast::NodeTree::new(),
                    sources: modules.iter().map(|module| module.id).collect(),
                };
                units.push(unit);
            }
        }
        units
    }

    /// Transpile the compiler's DIR into JS/TS/.. artifacts.
    pub fn transpile(&mut self) {
        // transpile each module into AST
        let mut units = Transpiler::make_units(self.options, self.modules);
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

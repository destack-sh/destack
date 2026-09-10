use std::sync::Arc;

use destack_core::FxIndexMap;
use destack_source::{ModuleId, PackageId};

use crate::{Function, FunctionAnalysis, ProgramEffects, ResolutionTable, Symbol};

use super::TestModule;

/// Parsed MIR modules used by interprocedural analysis fixtures.
pub(crate) struct TestProgram {
    /// The modules in source order.
    pub(crate) modules: Vec<TestModule>,
}

impl TestProgram {
    /// Parse each source as a distinct module.
    pub(crate) fn new(sources: &[&str]) -> Self {
        // assign stable identities in fixture order
        let modules = sources
            .iter()
            .enumerate()
            .map(|(index, source)| {
                let module = ModuleId::new(PackageId::new(0), index as u64);

                TestModule::parse(source, module)
            })
            .collect();

        Self { modules }
    }

    /// Assign the defining function's symbol to an imported declaration.
    pub(crate) fn import(&mut self, declaration: (usize, &str), definition: (usize, &str)) {
        // read the defining symbol from its module
        let module = &self.modules[definition.0];
        let function = module.tree.get(module.function_id_by_name(definition.1));
        assert!(
            function.is_defined(),
            "import requires a function definition"
        );
        assert!(
            function.linkage.is_exported(),
            "import requires an exported function"
        );
        let symbol = function.symbol;

        // connect the declaration to the same persistent symbol
        let module = &mut self.modules[declaration.0];
        let function = module.function_id_by_name(declaration.1);
        let function = module.tree.get_mut(function);
        assert!(
            function.linkage.is_import(),
            "import requires a declaration"
        );
        function.symbol = symbol;
    }

    /// Analyse all modules with optional previous interprocedural results.
    pub(crate) fn analyse_effects(&self, previous: Option<&ProgramEffects>) -> ProgramEffects {
        // extract function effects from each module's MIR
        let mut functions = FxIndexMap::<Symbol, Arc<FunctionAnalysis>>::default();
        for module in &self.modules {
            let resolution = ResolutionTable::analyse(&module.dispatch, None, &module.tree);
            for (id, function) in module.tree.iter_nodes::<Function>() {
                let analysis = FunctionAnalysis::analyse(
                    id,
                    &resolution,
                    &module.accesses,
                    &module.effects,
                    &module.tree,
                )
                .expect("MIR effects should be serializable");

                // require one definition per fixture symbol and consistent external declarations
                if let Some(current) = functions.get(&function.symbol) {
                    assert!(
                        !current.is_defined() || !analysis.is_defined(),
                        "fixture defines the same function twice"
                    );
                    if current.is_defined() {
                        continue;
                    }
                    if !analysis.is_defined() {
                        assert_eq!(**current, analysis, "inconsistent external declarations");
                    }
                }
                functions.insert(function.symbol, Arc::new(analysis));
            }
        }

        ProgramEffects::analyse(functions.into_iter().collect(), previous)
    }
}

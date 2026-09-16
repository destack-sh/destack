use std::sync::Arc;

use destack_source::{ModuleId, PackageId};

use crate::{Function, FunctionEffectBody, ProgramEffectTable, ResolutionTable};

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
    pub(crate) fn analyse_effects(
        &mut self,
        previous: Option<&ProgramEffectTable>,
    ) -> ProgramEffectTable {
        // extract function effects from each module's MIR
        let mut functions = Vec::new();
        for module in &mut self.modules {
            let resolution = ResolutionTable::analyse(&module.dispatch, None, &module.tree);
            let ids = module
                .tree
                .iter_nodes::<Function>()
                .map(|(id, function)| (id, function.symbol))
                .collect::<Vec<_>>();
            for (id, symbol) in ids {
                let analysis =
                    FunctionEffectBody::analyse(id, &resolution, &module.effects, &module.tree)
                        .expect("MIR effects should be serializable");

                functions.push((symbol, Arc::new(analysis)));
            }
        }

        ProgramEffectTable::analyse(functions, previous).expect("valid program functions")
    }
}

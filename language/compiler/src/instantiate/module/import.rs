use std::sync::Arc;

use tspp_artifact::MirDeclared;
use tspp_mir as mir;
use tspp_source::ModuleId;

use crate::instantiate::state::InstantiateState;

impl InstantiateState<'_> {
    /// Return this tree's function for one source function, a foreign one imported on first reach.
    pub(crate) fn import_function(
        &mut self,
        module: ModuleId,
        source: &mir::Tree,
        function: mir::FunctionId,
    ) -> mir::FunctionId {
        if module == self.module {
            return function;
        }
        let symbol = source.get(function).symbol;
        if let Some(existing) = self.functions.get(&symbol) {
            return *existing;
        }
        let stage = self.sources[&module].clone();
        let header =
            self.import_declared(|importer| importer.import_function_header(&stage.tree, function));
        let id = self.tree.insert(header);
        self.functions.insert(symbol, id);

        id
    }

    /// Return this tree's global for one source global, a foreign one imported on first reach.
    pub(crate) fn import_global(
        &mut self,
        module: ModuleId,
        source: &mir::Tree,
        global: mir::GlobalId,
    ) -> mir::GlobalId {
        if module == self.module {
            return global;
        }
        let symbol = source.get(global).symbol;
        if let Some(existing) = self.globals.get(&symbol) {
            return *existing;
        }
        let stage = self.sources[&module].clone();
        let imported = self.import_declared(|importer| importer.import_global(&stage.tree, global));
        let id = self.tree.insert(imported);
        self.globals.insert(symbol, id);

        id
    }

    /// Return this tree's type for one source type under the instance arguments.
    pub(crate) fn import_type(
        &mut self,
        module: ModuleId,
        ty: mir::TypeId,
        arguments: &[mir::GenericArgument],
    ) -> mir::TypeId {
        if module == self.module {
            return self.close_type(ty, arguments);
        }
        let stage = self.sources[&module].clone();
        let imported = self.import_declared(|importer| importer.import_type(&stage.tree, ty));

        self.close_type(imported, arguments)
    }

    /// Run one import with every reachable module's declared tree on hand.
    pub(crate) fn import_declared<T>(
        &mut self,
        import: impl FnOnce(&mut mir::Importer<'_, '_>) -> T,
    ) -> T {
        let Self {
            tree,
            declared,
            blocked,
            artifacts,
            profile,
            target,
            ..
        } = self;
        let mut declared_of = |module: ModuleId| {
            if let Some(stage) = declared.get(&module) {
                return Some(Arc::clone(&stage.tree));
            }
            match artifacts.read::<MirDeclared>((module, *profile, *target)) {
                Ok(stage) => {
                    let tree = Arc::clone(&stage.tree);
                    declared.insert(module, stage);

                    Some(tree)
                }
                // leave the symbols reserved, the finish yielding on the blocked read
                Err(error) => {
                    blocked.get_or_insert(error.into());

                    None
                }
            }
        };

        import(&mut mir::Importer::new(tree, &mut declared_of))
    }
}

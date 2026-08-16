use std::sync::Arc;

use destack_artifact::DirMaterialized;
use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin};

impl CheckState<'_> {
    /// Run the materialize pass: evaluate open computations and close instances.
    pub(in crate::sema) fn run_materialize(&mut self) -> CompilerResult<()> {
        self.import_external_modules()?;

        self.materialize_definitions()?;
        self.materialize_instances()
    }

    /// Evaluate each committed definition's types to their closed forms.
    fn materialize_definitions(&mut self) -> CompilerResult<()> {
        // collect the committed rows once, evaluating outside the module borrow
        let committed: Vec<(dir::GlobalSymbolId, dir::Definition)> = self
            .module
            .iter_definitions()
            .map(|(symbol, definition)| (symbol, definition.clone()))
            .collect();

        // evaluate every embedded type and keep the definitions that settle further
        for (symbol, definition) in committed {
            let Some(source) = self.module.definition_source_maybe(symbol) else {
                continue;
            };
            let origin = Origin::Node(source, None);
            let mut resolved = definition.clone();
            dir::TypeFold::map_types(&mut resolved, &mut |ty| self.evaluate_type(origin, ty))?;
            if resolved != definition {
                self.module
                    .definitions_tail
                    .insert_definition(symbol, source, resolved);
            }
        }

        Ok(())
    }

    /// Convert materialized state into one materialized DIR module.
    pub(in crate::sema) fn into_materialized(mut self) -> CompilerResult<DirMaterialized> {
        self.write_back()?;

        // take the tail segments this pass wrote
        let parsed = Arc::clone(&self.module.parsed);
        let roots = self.module.expanded.roots.clone();
        let module_state = self.module;
        let types = module_state.types_tail.finish();

        Ok(DirMaterialized {
            patch: dir::Patch::new(&parsed.tree, "materialize"),
            types: Arc::new(types),
            generics: Arc::new(module_state.generics_tail),
            definitions: Arc::new(module_state.definitions_tail),
            roots,
        })
    }
}

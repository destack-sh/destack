use destack_artifact::{DiagnosticLike, MirLowered};
use destack_core::{FxIndexMap, StringPool};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    ExternalCallables, FunctionLowerer, GenericInstanceKey, LayoutBuilder, LowerModuleState,
    NominalState,
};
use crate::{CompilerError, CompilerResult, LowerError};

/// Lowering state for one module: the checked DIR read side.
pub(crate) struct ModuleLowerer<'a> {
    /// The module being lowered.
    pub(in crate::lower) module: ModuleId,
    /// The source string pool.
    pub(in crate::lower) strings: &'a StringPool,
    /// The sealed check output of every reachable module.
    pub(in crate::lower) modules: FxIndexMap<ModuleId, LowerModuleState>,
    /// The MIR function declared for each generic instance key.
    pub(in crate::lower) functions: FxIndexMap<GenericInstanceKey, mir::FunctionId>,
    /// The state of each nominal representation being lowered or already lowered.
    pub(in crate::lower) nominals: FxIndexMap<GenericInstanceKey, NominalState>,
    /// The declaring symbol behind each loaded language item, scanned lazily.
    pub(in crate::lower) language_items: FxIndexMap<dir::LanguageItem, dir::GlobalSymbolId>,
    /// The MIR global declared for each module constant.
    pub(in crate::lower) globals: FxIndexMap<dir::GlobalSymbolId, mir::LocalNodeId<mir::Global>>,
}

impl<'a> ModuleLowerer<'a> {
    /// Create the lowering state over one materialized module.
    pub(crate) fn new(
        module: ModuleId,
        strings: &'a StringPool,
        modules: FxIndexMap<ModuleId, LowerModuleState>,
    ) -> Self {
        Self {
            module,
            strings,
            modules,
            functions: FxIndexMap::default(),
            nominals: FxIndexMap::default(),
            language_items: FxIndexMap::default(),
            globals: FxIndexMap::default(),
        }
    }

    /// Lower the module to MIR, returning the artifact and its diagnostics.
    pub(crate) fn lower(
        &mut self,
        pointer_bytes: u8,
    ) -> CompilerResult<(MirLowered, Vec<Box<dyn DiagnosticLike>>)> {
        let mut builder = mir::ModuleBuilder::new();
        builder.set_pointer_bytes(pointer_bytes);
        let mut errors = Vec::new();

        // lower every concrete nominal declaration owned by this module
        self.lower_nominal_declarations(&mut builder)?;

        // declare every callable header so bodies can call in any order
        let mut bodies = Vec::new();
        for index in 0..self.local().roots.len() {
            let root = self.local().roots[index];

            // classify the root form
            let declaration = match *self.local().tree().get(root) {
                dir::Expression::Declaration(declaration) => declaration,
                // imports and exports carry no runtime code
                dir::Expression::Import { .. } | dir::Expression::Export { .. } => continue,
                // module constants declare their evaluated globals
                dir::Expression::Let {
                    mutability,
                    ref declarators,
                    ..
                } => {
                    let declarators = declarators.clone();
                    match self.declare_module_constants(&mut builder, mutability, &declarators) {
                        Ok(()) => {}
                        Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                        Err(error) => return Err(error),
                    }

                    continue;
                }
                ref other => {
                    let error = LowerError::Unsupported {
                        anchor: self.module.into(),
                        construct: format!("a module-level '{}' statement", other.variant_name()),
                    };
                    match CompilerError::from(error) {
                        CompilerError::Diagnostic(diagnostic) => errors.push(diagnostic),
                        error => return Err(error),
                    }

                    continue;
                }
            };
            self.declare_root(&mut builder, declaration, &mut bodies, &mut errors)?;
        }

        // declare every concrete generic instance reachable from a body
        let mut references = ExternalCallables::default();
        match self.declare_reachable_instances(&mut builder, &bodies) {
            Ok((instances, callables)) => {
                bodies.extend(instances);
                references = callables;
            }
            Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
            Err(error) => return Err(error),
        }

        // declare an import for every foreign callable the bodies call
        match self.declare_imported_functions(&mut builder, references.imports) {
            Ok(()) => {}
            Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
            Err(error) => return Err(error),
        }

        // declare a dotted host extern for every sealed binding the bodies call
        for symbol in references.bindings {
            match self.declare_binding_function(&mut builder, symbol) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        // lower every declared body, keeping failures isolated per function
        for body in bodies {
            match FunctionLowerer::lower(self, &mut builder, body) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        // compute layouts for every aggregate type in the module
        let pointer_bytes = builder.pointer_bytes();
        let (tree, layouts) = builder.tree_and_layouts_mut();
        let mut layouts = LayoutBuilder::new(self.module, tree, layouts, pointer_bytes);
        layouts.layout_reachable_types()?;

        // publish the MIR names into the shared pool, string ids are content hashed
        let (tree, target, types, layouts, dispatch, drops, memory, effects, profile, strings) =
            builder.finish();
        self.strings.ensure_all_from(&strings);

        let lowered = MirLowered {
            tree,
            target,
            types,
            layouts,
            dispatch,
            drops,
            memory,
            effects,
            profile,
        };

        Ok((lowered, errors))
    }

}

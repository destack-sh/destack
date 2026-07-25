use destack_artifact::{DiagnosticLike, MirLowered};
use destack_core::{FxIndexMap, FxIndexSet, StringPool};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    DynamicSource, FunctionLowerer, GenericInstanceKey, LayoutBuilder, LowerModuleState,
    NominalState,
};
use crate::{CompilerError, CompilerResult};

/// Lowering state for one module.
pub(crate) struct ModuleLowerer<'a> {
    /// The module being lowered.
    pub(in crate::lower) module: ModuleId,
    /// The source string pool.
    pub(in crate::lower) strings: &'a StringPool,
    /// The state of every reachable module.
    pub(in crate::lower) modules: FxIndexMap<ModuleId, LowerModuleState>,
    /// The function declared for each generic instance key.
    pub(in crate::lower) functions: FxIndexMap<GenericInstanceKey, mir::FunctionId>,
    /// The state of each nominal representation being lowered or already lowered.
    pub(in crate::lower) nominals: FxIndexMap<GenericInstanceKey, NominalState>,
    /// The declaring symbol behind each loaded language item, scanned lazily.
    pub(in crate::lower) language_items: FxIndexMap<dir::LanguageItem, dir::GlobalSymbolId>,
    /// The global declared for each module constant.
    pub(in crate::lower) globals: FxIndexMap<dir::GlobalSymbolId, mir::LocalNodeId<mir::Global>>,
    /// The runtime bindings stored by the module initializer, in order.
    pub(in crate::lower) initializers: Vec<(
        mir::LocalNodeId<mir::Global>,
        dir::LocalNodeId<dir::Expression>,
    )>,
    /// The dispatch shape registered for each lowered constraint.
    pub(in crate::lower) dynamic_shapes: FxIndexMap<mir::LocalNodeId<mir::Type>, mir::DynamicShape>,
    /// The erasure source registered for each concrete row and constraint.
    pub(in crate::lower) dynamic_sources:
        FxIndexMap<(mir::LocalNodeId<mir::Type>, mir::LocalNodeId<mir::Type>), DynamicSource>,
    /// Constraints whose values answer keyed finds by field name.
    pub(in crate::lower) keyed_constraints: FxIndexSet<mir::LocalNodeId<mir::Type>>,
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
            initializers: Vec::new(),
            dynamic_shapes: FxIndexMap::default(),
            dynamic_sources: FxIndexMap::default(),
            keyed_constraints: FxIndexSet::default(),
        }
    }

    /// Lower the module, returning the artifact and its diagnostics.
    pub(crate) fn lower(
        &mut self,
        target_layout: mir::TargetLayout,
    ) -> CompilerResult<(MirLowered, Vec<Box<dyn DiagnosticLike>>)> {
        let mut builder = mir::ModuleBuilder::new();
        builder.set_target_layout(target_layout);

        // declare identities: types, callable headers, globals, imports, instances
        let (bodies, mut errors) = self.declare_module(&mut builder)?;

        // lower every declared body, keeping failures isolated per function
        for body in bodies {
            match FunctionLowerer::lower(self, &mut builder, body) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        // store the runtime bindings from the module initializer
        let mut initializer = None;
        match self.lower_module_initializer(&mut builder) {
            Ok(function) => initializer = function,
            Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
            Err(error) => return Err(error),
        }

        // compute layouts for every represented type in the module
        let target = builder.target_layout();
        let (tree, layouts) = builder.tree_and_layouts_mut();
        let mut layouts = LayoutBuilder::new(self.module, tree, layouts, target);
        layouts.layout_reachable_types()?;

        // publish dynamic dispatch over the laid-out types
        self.publish_dispatch(&mut builder, &mut errors)?;

        // publish the lowered names into the shared pool
        let (tree, target, types, layouts, dispatch, drops, memory, effects, profile, strings) =
            builder.finish();
        self.strings.ensure_all_from(&strings);

        // assemble the lowered module artifact
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
            initializer,
        };

        Ok((lowered, errors))
    }
}

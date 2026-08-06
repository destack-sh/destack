use std::sync::Arc;

use destack_artifact::{DiagnosticLike, MirLowered};
use destack_core::{FxIndexMap, StringId, StringPool};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    FunctionDeclaration, FunctionLowerer, GenericInstanceKey, Implementer, LayoutBuilder,
    LowerModuleState, NominalInstance, NominalState,
};
use crate::{CompilerError, CompilerResult};

/// One resolved type under one instance's bindings.
pub(in crate::lower) type ResolvedTypeKey = (
    dir::GlobalTypeId,
    Vec<(dir::GlobalGenericParameterId, dir::GlobalTypeId)>,
);

/// One lowering outcome a body reads: the lowered value, or the banked diagnostic of the first
/// failure.
pub(in crate::lower) type Lowered<T> = Result<T, Arc<dyn DiagnosticLike>>;

/// Lowering state for one module.
///
/// Lowering reports failures in three ways.
/// `CompilerResult::Internal` reports a compiler bug.
/// `CompilerResult::Diagnostic` reports a user error where lowering raises it.
/// `Lowered` banks an outcome at first read and raises it for every body that reads it.
pub(crate) struct ModuleLowerer<'a> {
    /// The module being lowered.
    pub(in crate::lower) module: ModuleId,
    /// The source string pool.
    pub(in crate::lower) strings: &'a StringPool,
    /// The state of every reachable module.
    pub(in crate::lower) modules: FxIndexMap<ModuleId, LowerModuleState>,

    /// The declaration outcome for each callable instance key.
    pub(in crate::lower) functions: FxIndexMap<GenericInstanceKey, FunctionDeclaration>,
    /// The MIR representation behind each type the bodies read.
    pub(in crate::lower) representations:
        FxIndexMap<ResolvedTypeKey, Lowered<mir::LocalNodeId<mir::Type>>>,
    /// The dispatch shape behind each constraint the bodies read.
    pub(in crate::lower) constraints:
        FxIndexMap<ResolvedTypeKey, Lowered<mir::LocalNodeId<mir::Type>>>,
    /// The nominal instance behind each application type the bodies read.
    pub(in crate::lower) stored_nominals: FxIndexMap<ResolvedTypeKey, Lowered<NominalInstance>>,
    /// The state of each nominal representation being lowered or already lowered.
    pub(in crate::lower) nominal_states: FxIndexMap<GenericInstanceKey, NominalState>,
    /// The global declared for each module constant.
    pub(in crate::lower) globals:
        FxIndexMap<dir::GlobalSymbolId, Lowered<mir::LocalNodeId<mir::Global>>>,
    /// The declaring symbol behind each loaded language item, scanned lazily.
    pub(in crate::lower) language_items: FxIndexMap<dir::LanguageItem, dir::GlobalSymbolId>,
    /// The immortal String object and value type per collected literal content.
    pub(in crate::lower) string_literals:
        FxIndexMap<StringId, Lowered<(mir::GlobalId, mir::LocalNodeId<mir::Type>)>>,
    /// The immortal BigInt object and value type per collected literal value.
    pub(in crate::lower) bigint_literals:
        FxIndexMap<i64, Lowered<(mir::GlobalId, mir::LocalNodeId<mir::Type>)>>,
    /// The runtime bindings stored by the module initializer, in order.
    pub(in crate::lower) initializers: Vec<(
        mir::LocalNodeId<mir::Global>,
        dir::LocalNodeId<dir::Expression>,
    )>,

    /// The dispatch shape registered for each lowered constraint.
    pub(in crate::lower) dynamic_shapes: FxIndexMap<mir::LocalNodeId<mir::Type>, mir::DynamicShape>,
    /// The implementer registered for each erased concrete type and constraint.
    pub(in crate::lower) implementers:
        FxIndexMap<(mir::LocalNodeId<mir::Type>, mir::LocalNodeId<mir::Type>), Implementer>,
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
            representations: FxIndexMap::default(),
            constraints: FxIndexMap::default(),
            stored_nominals: FxIndexMap::default(),
            nominal_states: FxIndexMap::default(),
            globals: FxIndexMap::default(),
            language_items: FxIndexMap::default(),
            string_literals: FxIndexMap::default(),
            bigint_literals: FxIndexMap::default(),
            initializers: Vec::new(),
            dynamic_shapes: FxIndexMap::default(),
            implementers: FxIndexMap::default(),
        }
    }

    /// Lower the module, returning the artifact and its diagnostics.
    pub(crate) fn lower(
        &mut self,
        target_layout: mir::TargetLayout,
    ) -> CompilerResult<(MirLowered, Vec<Box<dyn DiagnosticLike>>)> {
        let mut builder = mir::ModuleBuilder::new();
        builder.set_target_layout(target_layout);

        // scan the loaded modules for language items before any type lowering
        self.scan_language_items()?;

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
        self.build_dispatch_tables(&mut builder, &mut errors)?;

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

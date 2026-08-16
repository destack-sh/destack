use std::hash::{DefaultHasher, Hash, Hasher};
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
    /// The sema instance behind each closed selection, keyed by the arguments' structure.
    pub(in crate::lower) specializations:
        FxIndexMap<(dir::GlobalSymbolId, Vec<u64>), (ModuleId, dir::LocalInstanceId)>,
    /// The MIR representation behind each type the bodies read.
    pub(in crate::lower) representations:
        FxIndexMap<dir::GlobalTypeId, Lowered<mir::LocalNodeId<mir::Type>>>,
    /// The dispatch shape behind each constraint the bodies read.
    pub(in crate::lower) constraints:
        FxIndexMap<dir::GlobalTypeId, Lowered<mir::LocalNodeId<mir::Type>>>,
    /// The nominal instance behind each application type the bodies read.
    pub(in crate::lower) stored_nominals: FxIndexMap<dir::GlobalTypeId, Lowered<NominalInstance>>,
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
            specializations: FxIndexMap::default(),
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

    /// Index the instances sema closed by their structural selection.
    fn index_specializations(&mut self) -> CompilerResult<()> {
        // every loaded module contributes its own closed instances
        let mut rows = Vec::new();
        for (module, state) in &self.modules {
            for (instance, row) in state.generics.iter_instances() {
                let arguments: Vec<_> = row
                    .selection
                    .arguments
                    .iter()
                    .map(|binding| binding.argument)
                    .collect();
                rows.push((row.selection.symbol, arguments, *module, instance));
            }
        }

        // key each instance by the structure of its arguments
        let mut specializations = FxIndexMap::default();
        for (symbol, arguments, module, instance) in rows {
            let mut keys = Vec::with_capacity(arguments.len());
            for argument in arguments {
                keys.push(self.structural_type_key(argument)?);
            }
            specializations
                .entry((symbol, keys))
                .or_insert((module, instance));
        }
        self.specializations = specializations;

        Ok(())
    }

    /// Return the sema instance behind one closed selection.
    pub(in crate::lower) fn specialization_of(
        &self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<(ModuleId, dir::LocalInstanceId)>> {
        let mut keys = Vec::with_capacity(arguments.len());
        for argument in arguments {
            keys.push(self.structural_type_key(*argument)?);
        }

        Ok(self.specializations.get(&(symbol, keys)).copied())
    }

    /// Hash one type's structure, stable across module interning.
    pub(in crate::lower) fn structural_type_key(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<u64> {
        let mut hasher = DefaultHasher::new();
        let mut visiting = Vec::new();
        self.hash_type_structure(ty, &mut hasher, &mut visiting)?;

        Ok(hasher.finish())
    }

    /// Hash one type's head and children into the running structural key.
    fn hash_type_structure(
        &self,
        ty: dir::GlobalTypeId,
        hasher: &mut impl Hasher,
        visiting: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        // close recursive structures at their first revisit
        if visiting.contains(&ty) {
            u8::MAX.hash(hasher);

            return Ok(());
        }
        visiting.push(ty);

        // hash the head and its non-type payload
        let kind = self.ty(ty)?;
        std::mem::discriminant(&kind).hash(hasher);
        match &kind {
            dir::Type::Primitive(primitive) => primitive.hash(hasher),
            dir::Type::Literal(literal) => literal.hash(hasher),
            dir::Type::Memory(memory) => memory.hash(hasher),
            dir::Type::Application(application) => application.symbol.hash(hasher),
            dir::Type::Reference(reference) => reference.symbol.hash(hasher),
            dir::Type::Parameter(parameter) => parameter.hash(hasher),
            dir::Type::FunctionSignature(signature) => {
                (ty.module_id, *signature).hash(hasher);
            }
            _ => {}
        }

        // hash the children in structural order
        let mut children = Vec::new();
        self.types(ty.module_id)?
            .for_each_child(&kind, |child| children.push(child));
        for child in children {
            self.hash_type_structure(child, hasher, visiting)?;
        }
        visiting.pop();

        Ok(())
    }

    /// Resolve one template type through its instance's materialized types.
    pub(in crate::lower) fn instance_type(
        &self,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some((module, instance)) = instance else {
            return Ok(ty);
        };

        match self.state(module)?.generics.instance_type(instance, ty) {
            Some(resolved) => Ok(resolved),
            None => Ok(ty),
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

        // index the instances sema closed by their structural selection
        self.index_specializations()?;

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

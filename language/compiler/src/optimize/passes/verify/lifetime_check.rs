use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use destack_source::ModuleId;
use destack_workspace::TargetId;
use mir::{Instruction, Terminator, Type, Value};

use crate::optimize::common::terminator_arguments_for_successor;
use crate::optimize::{
    AnalysisPreservation, ControlFlowGraph, FunctionPass, Lattice, LifetimeAnalysis,
    PipelineContext, ResolvedLifetime, borrowed_parameter_indices_for_signature, forward_dataflow,
    signature_return_contains_borrowed_refs, type_contains_borrowed_refs,
};
use crate::{OptimizeError, OptimizeWarning};

declare_pass! {
    /// Verify explicit return lifetime annotations.
    ///
    /// Validates that `@lifetime(...)` matches the borrow origins that flow to
    /// return values. In strict borrow mode, mismatches are errors. In lenient
    /// mode, mismatches are warnings. Annotations on non borrowed returns always
    /// warn to flag redundant annotations.
    #[pass(id = "lifetime-check")]
    pub LifetimeCheck,
    "Verify lifetime annotations"
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum BorrowOrigin {
    Parameter(u32),
    Static,
    Local,
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct BorrowOriginSet {
    origins: HashSet<BorrowOrigin>,
}

impl BorrowOriginSet {
    fn from_origin(origin: BorrowOrigin) -> Self {
        // seed a new origin set
        let mut set = Self::default();
        set.origins.insert(origin);
        set
    }

    fn insert(&mut self, origin: BorrowOrigin) {
        // add a single origin
        self.origins.insert(origin);
    }

    fn union_with(&mut self, other: &BorrowOriginSet) {
        // merge origin sets
        for origin in &other.origins {
            self.origins.insert(origin.clone());
        }
    }

    fn is_empty(&self) -> bool {
        self.origins.is_empty()
    }

    fn iter(&self) -> impl Iterator<Item = &BorrowOrigin> {
        self.origins.iter()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct BorrowOriginMap {
    values: HashMap<Value, BorrowOriginSet>,
    locals: HashMap<mir::LocalNodeId<mir::Local>, BorrowOriginSet>,
}

impl BorrowOriginMap {
    fn value_origin(&self, value: Value) -> BorrowOriginSet {
        self.values.get(&value).cloned().unwrap_or_default()
    }

    fn set_value_origin(&mut self, value: Value, origins: BorrowOriginSet) {
        // update value origin tracking
        if origins.is_empty() {
            self.values.remove(&value);
        } else {
            self.values.insert(value, origins);
        }
    }

    fn merge_value_origin(&mut self, value: Value, origins: BorrowOriginSet) {
        // accumulate origin tracking
        let entry = self.values.entry(value).or_default();
        entry.union_with(&origins);
    }

    fn local_origin(&self, local: mir::LocalNodeId<mir::Local>) -> BorrowOriginSet {
        self.locals.get(&local).cloned().unwrap_or_default()
    }

    fn set_local_origin(&mut self, local: mir::LocalNodeId<mir::Local>, origins: BorrowOriginSet) {
        // update local origin tracking
        if origins.is_empty() {
            self.locals.remove(&local);
        } else {
            self.locals.insert(local, origins);
        }
    }
}

impl Lattice for BorrowOriginMap {
    fn meet(&self, other: &Self) -> Self {
        // merge value origins
        let mut values = self.values.clone();
        for (value, origins) in &other.values {
            let entry = values.entry(*value).or_default();
            entry.union_with(origins);
        }

        // merge local origins
        let mut locals = self.locals.clone();
        for (local, origins) in &other.locals {
            let entry = locals.entry(*local).or_default();
            entry.union_with(origins);
        }

        Self { values, locals }
    }
}

#[derive(Debug, Clone, Default)]
struct ValueTypes {
    values: HashMap<Value, mir::LocalNodeId<Type>>,
    locals: HashMap<mir::LocalNodeId<mir::Local>, mir::LocalNodeId<Type>>,
}

impl ValueTypes {
    fn value_type(&self, value: Value) -> Option<mir::LocalNodeId<Type>> {
        self.values.get(&value).copied()
    }

    fn local_type(&self, local: mir::LocalNodeId<mir::Local>) -> Option<mir::LocalNodeId<Type>> {
        self.locals.get(&local).copied()
    }

    fn set_value_type(&mut self, value: Value, ty: mir::LocalNodeId<Type>) {
        // record a value type
        self.values.insert(value, ty);
    }

    fn set_local_type(&mut self, local: mir::LocalNodeId<mir::Local>, ty: mir::LocalNodeId<Type>) {
        // record a local type
        self.locals.insert(local, ty);
    }
}

impl FunctionPass for LifetimeCheck {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // select explicit lifetimes only
        let return_lifetime = match &function.return_lifetime {
            mir::Lifetime::Inferred => return AnalysisPreservation::all(),
            lifetime => lifetime.clone(),
        };

        // warn on annotations for non borrowed returns
        let return_type = tree.get(function.return_type);
        if !type_contains_borrowed_refs(return_type, tree) {
            if let Some(block_id) = function.blocks.first() {
                ctx.emit_warning(OptimizeWarning::LifetimeAnnotationIgnored {
                    node: anchor_block(ctx.module_id(), ctx.target_id().clone(), *block_id),
                });
            }
            return AnalysisPreservation::all();
        }

        // resolve strict mode
        let strict_mode = ctx.options.strict_borrow_mode;

        // build value type lookup
        let value_types = collect_value_types(function, tree);

        // prepare analyses for dataflow
        let analyses = ctx.function_analyses(function, tree);
        let cfg = analyses.get::<ControlFlowGraph>();
        let lifetime_analysis = ctx.module_analyses(tree).get::<LifetimeAnalysis>();

        // run forward dataflow to collect origins
        let entry_state = build_entry_state(function, tree);
        let result = forward_dataflow(
            function,
            tree,
            &cfg,
            entry_state,
            |block_id, mut state, tree| {
                // wire predecessor arguments to block parameters
                let block = tree.get(block_id);
                for &pred_id in cfg.predecessors(block_id) {
                    let pred_block = tree.get(pred_id);
                    let arguments =
                        terminator_arguments_for_successor(&pred_block.terminator, block_id);
                    for (param, arg) in block.parameters.iter().zip(arguments) {
                        let origin = state.value_origin(*arg);
                        state.merge_value_origin(param.value, origin);
                    }
                }

                // apply instruction effects
                for &inst_id in &block.instructions {
                    let inst = tree.get(inst_id);
                    apply_instruction_effects(
                        &mut state,
                        inst,
                        tree,
                        &value_types,
                        lifetime_analysis.as_ref(),
                    );
                }

                state
            },
        );

        // validate returns against the declared lifetime
        let module_id = ctx.module_id();
        let target_id = ctx.target_id().clone();
        for (block_id, state) in &result.block_exit {
            let block = tree.get(*block_id);
            let Terminator::Return { value: Some(value) } = &block.terminator else {
                continue;
            };

            // collect origins for returned values
            let mut origins = state.value_origin(*value);
            if origins.is_empty() {
                origins.insert(BorrowOrigin::Unknown);
            }

            // report disallowed origins
            let Some(disallowed) = disallowed_origins(&origins, &return_lifetime) else {
                continue;
            };

            if strict_mode {
                ctx.emit_error(OptimizeError::LifetimeAnnotationMismatch {
                    node: anchor_block(module_id, target_id.clone(), *block_id),
                    origin: disallowed,
                });
            } else {
                ctx.emit_warning(OptimizeWarning::PotentialLifetimeAnnotationMismatch {
                    node: anchor_block(module_id, target_id.clone(), *block_id),
                });
            }
        }

        AnalysisPreservation::all()
    }

    fn name(&self) -> &'static str {
        "LifetimeCheck"
    }

    fn id(&self) -> &'static str {
        "lifetime-check"
    }
}

fn build_entry_state(function: &mir::Function, tree: &mir::NodeTree) -> BorrowOriginMap {
    // seed parameter origins for borrowed parameters
    let mut state = BorrowOriginMap::default();
    for (index, param) in function.parameters.iter().enumerate() {
        let param_type = tree.get(param.ty);
        if !type_contains_borrowed_refs(param_type, tree) {
            continue;
        }

        state.set_value_origin(
            param.value,
            BorrowOriginSet::from_origin(BorrowOrigin::Parameter(index as u32)),
        );
    }

    state
}

fn collect_value_types(function: &mir::Function, tree: &mir::NodeTree) -> ValueTypes {
    // build the initial map
    let mut types = ValueTypes::default();

    // seed parameter types
    for param in &function.parameters {
        types.set_value_type(param.value, param.ty);
    }

    // seed block parameter types
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for param in &block.parameters {
            types.set_value_type(param.value, param.ty);
        }
    }

    // seed local types
    for &local_id in &function.locals {
        let local = tree.get(local_id);
        types.set_local_type(local_id, local.ty);
    }

    // scan instructions for destination types
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &inst_id in &block.instructions {
            let inst = tree.get(inst_id);
            let Some(dest) = inst.destination() else {
                continue;
            };

            let Some(result_type) = instruction_result_type(inst, tree, &types) else {
                continue;
            };

            types.set_value_type(dest, result_type);
        }
    }

    types
}

fn instruction_result_type(
    inst: &Instruction,
    tree: &mir::NodeTree,
    types: &ValueTypes,
) -> Option<mir::LocalNodeId<Type>> {
    // derive result types for destinations
    match inst {
        Instruction::Const { .. } => None,
        Instruction::Binary { left, .. } => types.value_type(*left),
        Instruction::Unary { argument, .. } => types.value_type(*argument),
        Instruction::Cast { to_type, .. } => Some(*to_type),
        Instruction::Select {
            then_value,
            else_value,
            ..
        } => types
            .value_type(*then_value)
            .or_else(|| types.value_type(*else_value)),
        Instruction::LocalGet { local, .. } => types.local_type(*local),
        Instruction::GlobalAddr { result_type, .. } => Some(*result_type),
        Instruction::GlobalConst { global, .. } => {
            let global = tree.get(*global);
            Some(global.ty)
        }
        Instruction::Load { result_type, .. } => Some(*result_type),
        Instruction::FieldGet {
            aggregate, index, ..
        } => {
            let aggregate_type = types.value_type(*aggregate)?;
            field_type_for_value(tree, aggregate_type, *index)
        }
        Instruction::FieldAddr { result_type, .. } => Some(*result_type),
        Instruction::FieldSet { aggregate, .. } => types.value_type(*aggregate),
        Instruction::ElementGet { array, .. } => {
            let array_type = types.value_type(*array)?;
            element_type_for_value(tree, array_type)
        }
        Instruction::ElementAddr { result_type, .. } => Some(*result_type),
        Instruction::ElementSet { array, .. } => types.value_type(*array),
        Instruction::Struct { ty, .. } => Some(*ty),
        Instruction::Tuple { ty, .. } => Some(*ty),
        Instruction::Array { ty, .. } => Some(*ty),
        Instruction::Call { signature, .. }
        | Instruction::CallVirtual { signature, .. }
        | Instruction::CallInterface { signature, .. }
        | Instruction::CallIndirect { signature, .. } => signature_result_type(tree, *signature),
        Instruction::ManagedAlloc { result_type, .. } => Some(*result_type),
        Instruction::ManagedAllocArray { result_type, .. } => Some(*result_type),
        Instruction::RawAlloc { result_type, .. } => Some(*result_type),
        Instruction::StackAlloc { result_type, .. } => Some(*result_type),
        Instruction::Intrinsic { .. } => None,
        Instruction::Store { .. }
        | Instruction::LocalSet { .. }
        | Instruction::RawFree { .. }
        | Instruction::RawDrop { .. }
        | Instruction::StackDrop { .. }
        | Instruction::Assume { .. } => None,
    }
}

fn signature_result_type(
    tree: &mir::NodeTree,
    signature: mir::LocalNodeId<Type>,
) -> Option<mir::LocalNodeId<Type>> {
    // read return types from function pointers
    let signature_type = tree.get(signature);
    let Type::FunctionPointer { result, .. } = signature_type else {
        return None;
    };

    Some(*result)
}

fn field_type_for_value(
    tree: &mir::NodeTree,
    aggregate_type: mir::LocalNodeId<Type>,
    index: u32,
) -> Option<mir::LocalNodeId<Type>> {
    // resolve field types for aggregates
    let aggregate = tree.get(aggregate_type);
    match aggregate {
        Type::Struct { fields, .. } => fields.get(index as usize).map(|field_id| {
            let field = tree.get(*field_id);
            field.ty
        }),
        Type::Tuple { elements, .. } => elements.get(index as usize).copied(),
        _ => None,
    }
}

fn element_type_for_value(
    tree: &mir::NodeTree,
    array_type: mir::LocalNodeId<Type>,
) -> Option<mir::LocalNodeId<Type>> {
    // resolve element types for arrays
    let array_type = tree.get(array_type);
    let Type::Array { element, .. } = array_type else {
        return None;
    };

    Some(*element)
}

fn value_contains_borrowed_refs(value: Value, tree: &mir::NodeTree, types: &ValueTypes) -> bool {
    // default to borrowed when type is unknown
    let Some(ty_id) = types.value_type(value) else {
        return true;
    };

    let ty = tree.get(ty_id);
    type_contains_borrowed_refs(ty, tree)
}

fn local_contains_borrowed_refs(
    local: mir::LocalNodeId<mir::Local>,
    tree: &mir::NodeTree,
    types: &ValueTypes,
) -> bool {
    // default to borrowed when type is unknown
    let Some(ty_id) = types.local_type(local) else {
        return true;
    };

    let ty = tree.get(ty_id);
    type_contains_borrowed_refs(ty, tree)
}

fn assign_origin_if_borrowed(
    state: &mut BorrowOriginMap,
    destination: Value,
    origins: BorrowOriginSet,
    tree: &mir::NodeTree,
    types: &ValueTypes,
) {
    // track origins only for borrowed results
    if value_contains_borrowed_refs(destination, tree, types) {
        state.set_value_origin(destination, origins);
    } else {
        state.set_value_origin(destination, BorrowOriginSet::default());
    }
}

fn apply_instruction_effects(
    state: &mut BorrowOriginMap,
    instruction: &Instruction,
    tree: &mir::NodeTree,
    types: &ValueTypes,
    lifetime_analysis: &LifetimeAnalysis,
) {
    match instruction {
        // constants and arithmetic results do not borrow
        Instruction::Const { destination, .. }
        | Instruction::Binary { destination, .. }
        | Instruction::Unary { destination, .. } => {
            assign_origin_if_borrowed(state, *destination, BorrowOriginSet::default(), tree, types);
        }

        // casts preserve borrow origins when needed
        Instruction::Cast {
            destination,
            argument,
            ..
        } => {
            let origins = state.value_origin(*argument);
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // select merges origins from both branches
        Instruction::Select {
            destination,
            then_value,
            else_value,
            ..
        } => {
            let mut origins = state.value_origin(*then_value);
            origins.union_with(&state.value_origin(*else_value));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // locals carry their stored origins
        Instruction::LocalGet { destination, local } => {
            let origins = state.local_origin(*local);
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // local stores only matter for borrowed values
        Instruction::LocalSet { local, value } => {
            if local_contains_borrowed_refs(*local, tree, types) {
                let origins = state.value_origin(*value);
                state.set_local_origin(*local, origins);
            } else {
                state.set_local_origin(*local, BorrowOriginSet::default());
            }
        }

        // globals are static borrows
        Instruction::GlobalAddr { destination, .. }
        | Instruction::GlobalConst { destination, .. } => {
            assign_origin_if_borrowed(
                state,
                *destination,
                BorrowOriginSet::from_origin(BorrowOrigin::Static),
                tree,
                types,
            );
        }

        // loads preserve the pointer origin
        Instruction::Load {
            destination,
            pointer,
            ..
        } => {
            let origins = state.value_origin(*pointer);
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // field loads preserve origins only for borrowed fields
        Instruction::FieldGet {
            destination,
            aggregate,
            index,
            ..
        } => {
            let aggregate_type = types.value_type(*aggregate);
            let borrowed_field = aggregate_type
                .and_then(|ty| field_type_for_value(tree, ty, *index))
                .map(|field_ty| type_contains_borrowed_refs(tree.get(field_ty), tree))
                .unwrap_or(true);

            let origins = if borrowed_field {
                state.value_origin(*aggregate)
            } else {
                BorrowOriginSet::default()
            };
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // element loads preserve origins only for borrowed elements
        Instruction::ElementGet {
            destination, array, ..
        } => {
            let array_type = types.value_type(*array);
            let borrowed_element = array_type
                .and_then(|ty| element_type_for_value(tree, ty))
                .map(|element_ty| type_contains_borrowed_refs(tree.get(element_ty), tree))
                .unwrap_or(true);

            let origins = if borrowed_element {
                state.value_origin(*array)
            } else {
                BorrowOriginSet::default()
            };
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // field addresses borrow from the aggregate
        Instruction::FieldAddr {
            destination,
            aggregate,
            ..
        } => {
            let mut origins = state.value_origin(*aggregate);
            if origins.is_empty() {
                origins.insert(BorrowOrigin::Local);
            }

            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // element addresses borrow from the array
        Instruction::ElementAddr {
            destination, array, ..
        } => {
            let mut origins = state.value_origin(*array);
            if origins.is_empty() {
                origins.insert(BorrowOrigin::Local);
            }

            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // field sets merge origins from aggregate and value
        Instruction::FieldSet {
            destination,
            aggregate,
            value,
            ..
        } => {
            let mut origins = state.value_origin(*aggregate);
            origins.union_with(&state.value_origin(*value));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // element sets merge origins from array and value
        Instruction::ElementSet {
            destination,
            array,
            value,
            ..
        } => {
            let mut origins = state.value_origin(*array);
            origins.union_with(&state.value_origin(*value));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // struct construction merges borrowed fields
        Instruction::Struct {
            destination,
            ty,
            fields,
        } => {
            let struct_type = tree.get(*ty);
            let field_values = tree.get_arguments(*fields);

            let mut origins = BorrowOriginSet::default();
            if let Type::Struct { fields, .. } = struct_type {
                for (field_id, value) in fields.iter().zip(field_values) {
                    let field = tree.get(*field_id);
                    let field_ty = tree.get(field.ty);
                    if type_contains_borrowed_refs(field_ty, tree) {
                        origins.union_with(&state.value_origin(*value));
                    }
                }
            } else {
                for value in field_values {
                    origins.union_with(&state.value_origin(*value));
                }
            }

            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // tuple construction merges borrowed elements
        Instruction::Tuple {
            destination,
            ty,
            elements,
        } => {
            let tuple_type = tree.get(*ty);
            let values = tree.get_arguments(*elements);

            let mut origins = BorrowOriginSet::default();
            if let Type::Tuple { elements, .. } = tuple_type {
                for (element_type, value) in elements.iter().zip(values) {
                    let element_ty = tree.get(*element_type);
                    if type_contains_borrowed_refs(element_ty, tree) {
                        origins.union_with(&state.value_origin(*value));
                    }
                }
            } else {
                for value in values {
                    origins.union_with(&state.value_origin(*value));
                }
            }

            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // array construction merges borrowed elements
        Instruction::Array {
            destination,
            ty,
            elements,
        } => {
            let array_type = tree.get(*ty);
            let values = tree.get_arguments(*elements);

            let mut origins = BorrowOriginSet::default();
            if let Type::Array { element, .. } = array_type {
                let element_ty = tree.get(*element);
                if type_contains_borrowed_refs(element_ty, tree) {
                    for value in values {
                        origins.union_with(&state.value_origin(*value));
                    }
                }
            } else {
                for value in values {
                    origins.union_with(&state.value_origin(*value));
                }
            }

            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // direct calls use lifetime analysis
        Instruction::Call {
            destination: Some(dest),
            function,
            arguments,
            signature,
            ..
        } => {
            let signature_type = tree.get(*signature);
            let return_contains_borrow =
                signature_return_contains_borrowed_refs(signature_type, tree);
            let origins = origins_for_call(
                *function,
                tree.get_arguments(*arguments),
                state,
                lifetime_analysis,
                return_contains_borrow,
            );

            assign_origin_if_borrowed(state, *dest, origins, tree, types);
        }

        // virtual and interface calls use signature fallback
        Instruction::CallVirtual {
            destination: Some(dest),
            declared_target,
            arguments,
            signature,
            ..
        }
        | Instruction::CallInterface {
            destination: Some(dest),
            declared_target,
            arguments,
            signature,
            ..
        } => {
            let signature_type = tree.get(*signature);
            let return_contains_borrow =
                signature_return_contains_borrowed_refs(signature_type, tree);
            let origins = if let Some(function_id) = declared_target {
                origins_for_call(
                    *function_id,
                    tree.get_arguments(*arguments),
                    state,
                    lifetime_analysis,
                    return_contains_borrow,
                )
            } else {
                origins_for_signature(
                    signature_type,
                    tree,
                    tree.get_arguments(*arguments),
                    state,
                    return_contains_borrow,
                )
            };

            assign_origin_if_borrowed(state, *dest, origins, tree, types);
        }

        // indirect calls rely on the signature
        Instruction::CallIndirect {
            destination: Some(dest),
            arguments,
            signature,
            ..
        } => {
            let signature_type = tree.get(*signature);
            let return_contains_borrow =
                signature_return_contains_borrowed_refs(signature_type, tree);
            let origins = origins_for_signature(
                signature_type,
                tree,
                tree.get_arguments(*arguments),
                state,
                return_contains_borrow,
            );

            assign_origin_if_borrowed(state, *dest, origins, tree, types);
        }

        // calls without destination do not change borrow origins
        Instruction::Call {
            destination: None, ..
        }
        | Instruction::CallVirtual {
            destination: None, ..
        }
        | Instruction::CallInterface {
            destination: None, ..
        }
        | Instruction::CallIndirect {
            destination: None, ..
        } => {}

        // allocations produce local borrows
        Instruction::ManagedAlloc { destination, .. }
        | Instruction::ManagedAllocArray { destination, .. }
        | Instruction::RawAlloc { destination, .. }
        | Instruction::StackAlloc { destination, .. } => {
            assign_origin_if_borrowed(
                state,
                *destination,
                BorrowOriginSet::from_origin(BorrowOrigin::Local),
                tree,
                types,
            );
        }

        // intrinsic results are unknown
        Instruction::Intrinsic { destination, .. } => {
            let Some(destination) = destination else {
                return;
            };

            assign_origin_if_borrowed(
                state,
                *destination,
                BorrowOriginSet::from_origin(BorrowOrigin::Unknown),
                tree,
                types,
            );
        }

        // stores and drops do not change borrow origins
        Instruction::Store { .. }
        | Instruction::RawFree { .. }
        | Instruction::RawDrop { .. }
        | Instruction::StackDrop { .. }
        | Instruction::Assume { .. } => {}
    }
}

fn origins_for_call(
    function_id: mir::LocalNodeId<mir::Function>,
    arguments: &[Value],
    state: &BorrowOriginMap,
    lifetime_analysis: &LifetimeAnalysis,
    return_contains_borrow: bool,
) -> BorrowOriginSet {
    // skip origins when the return cannot borrow
    if !return_contains_borrow {
        return BorrowOriginSet::default();
    }

    // use lifetime analysis for direct calls
    match lifetime_analysis.get(function_id) {
        ResolvedLifetime::None => BorrowOriginSet::from_origin(BorrowOrigin::Unknown),
        ResolvedLifetime::Static => BorrowOriginSet::from_origin(BorrowOrigin::Static),
        ResolvedLifetime::Parameters(params) => {
            let mut origins = BorrowOriginSet::default();
            for param_index in params {
                let Some(arg) = arguments.get(*param_index as usize) else {
                    origins.insert(BorrowOrigin::Unknown);
                    continue;
                };
                origins.union_with(&state.value_origin(*arg));
            }
            origins
        }
    }
}

fn origins_for_signature(
    signature_type: &Type,
    tree: &mir::NodeTree,
    arguments: &[Value],
    state: &BorrowOriginMap,
    return_contains_borrow: bool,
) -> BorrowOriginSet {
    // skip origins when the return cannot borrow
    if !return_contains_borrow {
        return BorrowOriginSet::default();
    }

    // collect borrowed parameters from the signature
    let Some(borrowed_params) = borrowed_parameter_indices_for_signature(signature_type, tree)
    else {
        return BorrowOriginSet::from_origin(BorrowOrigin::Unknown);
    };

    // no borrowed params implies static borrow
    if borrowed_params.is_empty() {
        return BorrowOriginSet::from_origin(BorrowOrigin::Static);
    }

    // merge origins from borrowed parameters
    let mut origins = BorrowOriginSet::default();
    for index in borrowed_params {
        let Some(arg) = arguments.get(index) else {
            origins.insert(BorrowOrigin::Unknown);
            continue;
        };
        origins.union_with(&state.value_origin(*arg));
    }

    origins
}

fn disallowed_origins(origins: &BorrowOriginSet, lifetime: &mir::Lifetime) -> Option<String> {
    // collect disallowed origins
    let mut disallowed = Vec::new();
    for origin in origins.iter() {
        let allowed = match lifetime {
            mir::Lifetime::Static => matches!(origin, BorrowOrigin::Static),
            mir::Lifetime::Parameters(params) => match origin {
                BorrowOrigin::Parameter(index) => params.contains(index),
                _ => false,
            },
            mir::Lifetime::Inferred => true,
        };

        if !allowed {
            disallowed.push(format_origin(origin));
        }
    }

    if disallowed.is_empty() {
        return None;
    }

    Some(disallowed.join(", "))
}

fn format_origin(origin: &BorrowOrigin) -> String {
    // format origin descriptions
    match origin {
        BorrowOrigin::Parameter(index) => format!("parameter {index}"),
        BorrowOrigin::Static => "static".to_string(),
        BorrowOrigin::Local => "local".to_string(),
        BorrowOrigin::Unknown => "unknown".to_string(),
    }
}

fn anchor_block(
    module_id: ModuleId,
    target_id: TargetId,
    block_id: mir::LocalNodeId<mir::Block>,
) -> mir::AnchoredGlobalNodeId {
    // anchor diagnostics to the block
    block_id.into_any().into_anchored(module_id, target_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::PipelineOptions;
    use crate::optimize::common::tests::TestProgram;

    /// Returning a borrowed param matches lifetime annotations.
    #[test]
    fn test_verify_return_borrowed_param() {
        let input = r#"function @test(v0: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>):
    return v0
}"#;

        let mut options = PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::Lifetime::Parameters(vec![0]));
        test.run_pass_with_options(&LifetimeCheck, options);
        test.assert_no_errors();
    }

    /// Returning a borrowed field matches lifetime annotations.
    #[test]
    fn test_verify_return_borrowed_field() {
        let input = r#"function @test(v0: ref<borrowed { i32 }>) -> ref<borrowed i32> {
block0(v0: ref<borrowed { i32 }>):
    v1 = field.addr v0, 0 -> ref<borrowed i32>
    return v1
}"#;

        let mut options = PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::Lifetime::Parameters(vec![0]));
        test.run_pass_with_options(&LifetimeCheck, options);
        test.assert_no_errors();
    }

    /// Returning a local borrow is rejected by parameter lifetimes.
    #[test]
    fn test_verify_return_local_borrow_rejected() {
        let input = r#"function @test() -> ref<borrowed i32> {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = field.addr v0, 0 -> ref<borrowed i32>
    return v1
}"#;

        let mut options = PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::Lifetime::Parameters(vec![0]));
        test.run_pass_with_options(&LifetimeCheck, options);
        test.assert_error(|e| matches!(e, OptimizeError::LifetimeAnnotationMismatch { .. }));
    }

    /// Returning a borrowed param is rejected by static lifetimes.
    #[test]
    fn test_verify_static_lifetime_rejects_param() {
        let input = r#"function @test(v0: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>):
    return v0
}"#;

        let mut options = PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::Lifetime::Static);
        test.run_pass_with_options(&LifetimeCheck, options);
        test.assert_error(|e| matches!(e, OptimizeError::LifetimeAnnotationMismatch { .. }));
    }

    /// Mismatched lifetimes warn in lenient mode.
    #[test]
    fn test_verify_mismatch_warns_in_lenient_mode() {
        let input = r#"function @test(v0: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>):
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::Lifetime::Static);
        test.run_pass(&LifetimeCheck);
        test.assert_no_errors();
        test.assert_warning(|w| {
            matches!(
                w,
                OptimizeWarning::PotentialLifetimeAnnotationMismatch { .. }
            )
        });
    }

    /// Annotations on non borrowed returns are warned.
    #[test]
    fn test_verify_annotation_ignored_warns() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::Lifetime::Static);
        test.run_pass(&LifetimeCheck);
        test.assert_no_errors();
        test.assert_warning(|w| matches!(w, OptimizeWarning::LifetimeAnnotationIgnored { .. }));
    }

    /// Indirect calls fall back to signature borrowing rules.
    #[test]
    fn test_verify_signature_fallback_for_indirect_call() {
        let input = r#"function @test(v0: fn(ref<borrowed i32>) -> ref<borrowed i32>, v1: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: fn(ref<borrowed i32>) -> ref<borrowed i32>, v1: ref<borrowed i32>):
    v2 = call.indirect v0(v1) -> fn(ref<borrowed i32>) -> ref<borrowed i32>
    return v2
}"#;

        let mut options = PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::Lifetime::Parameters(vec![1]));
        test.run_pass_with_options(&LifetimeCheck, options);
        test.assert_no_errors();
    }
}

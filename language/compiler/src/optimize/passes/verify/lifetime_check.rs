use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_core::StringPool;
use destack_mir as mir;
use destack_source::{ModuleId, TargetId};
use mir::{Instruction, Terminator, Type, Value};

use crate::optimize::common::{ValueTypeMap, terminator_arguments_for_successor};
use crate::optimize::{
    AnalysisPreservation, CallTargetAnalysis, ControlFlowGraph, FunctionPass, Lattice,
    LifetimeAnalysis, PipelineContext, ResolvedLifetime, borrowed_parameter_indices_for_signature,
    forward_dataflow, signature_return_contains_borrowed_refs, type_contains_borrowed_refs,
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
    fn value_origin(&self, value: impl Into<mir::ValueReference>) -> BorrowOriginSet {
        let Some(value) = value.into().value() else {
            return BorrowOriginSet::default();
        };

        self.values.get(&value).cloned().unwrap_or_default()
    }

    fn set_value_origin(
        &mut self,
        value: impl Into<mir::ValueReference>,
        origins: BorrowOriginSet,
    ) {
        let Some(value) = value.into().value() else {
            return;
        };

        // update value origin tracking
        if origins.is_empty() {
            self.values.remove(&value);
        } else {
            self.values.insert(value, origins);
        }
    }

    fn merge_value_origin(
        &mut self,
        value: impl Into<mir::ValueReference>,
        origins: BorrowOriginSet,
    ) {
        let Some(value) = value.into().value() else {
            return;
        };

        // accumulate origin tracking
        let entry = self.values.entry(value).or_default();
        entry.union_with(&origins);
    }

    fn local_origin(&self, local: impl Into<mir::LocalReference>) -> BorrowOriginSet {
        let Some(local) = local.into().local() else {
            return BorrowOriginSet::default();
        };

        self.locals.get(&local).cloned().unwrap_or_default()
    }

    fn set_local_origin(
        &mut self,
        local: impl Into<mir::LocalReference>,
        origins: BorrowOriginSet,
    ) {
        let Some(local) = local.into().local() else {
            return;
        };

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

impl FunctionPass for LifetimeCheck {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // resolve the declared return region
        let return_region = function.return_region.clone();

        // warn on annotations for non borrowed returns
        let Some(return_type) = function.return_type.ty() else {
            return AnalysisPreservation::all();
        };
        let return_type = tree.get(return_type);
        if !type_contains_borrowed_refs(return_type, tree) {
            if !matches!(return_region, mir::BorrowRegion::Inferred)
                && let Some(block_id) = function.blocks.first()
            {
                ctx.emit_warning(OptimizeWarning::LifetimeAnnotationIgnored {
                    node: anchor_block(ctx.module_id(), *ctx.target_id(), *block_id),
                });
            }
            return AnalysisPreservation::all();
        }

        // resolve strict mode
        let strict_mode = ctx.options.strict_borrow_mode;

        // build value type lookup
        let value_types = ValueTypeMap::new(function, tree);

        // prepare analyses for dataflow
        let analyses = ctx.function_analyses(function, tree);
        let cfg = analyses.get::<ControlFlowGraph>();
        let module_analyses = ctx.module_analyses(tree);
        let lifetime_analysis = module_analyses.get::<LifetimeAnalysis>();
        let call_targets = module_analyses.get::<CallTargetAnalysis>();

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
                    let pred_terminator = tree.get(pred_block.terminator);
                    let arguments = terminator_arguments_for_successor(pred_terminator, block_id);
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
                        inst_id,
                        inst,
                        tree,
                        &value_types,
                        lifetime_analysis.as_ref(),
                        call_targets.as_ref(),
                    );
                }

                state
            },
        );

        // validate returns against the declared lifetime
        let module_id = ctx.module_id();
        let target_id = *ctx.target_id();
        for (block_id, state) in &result.block_exit {
            let block = tree.get(*block_id);
            let terminator = tree.get(block.terminator);

            let Terminator::Return { value: Some(value) } = terminator else {
                continue;
            };

            // collect origins for returned values
            let mut origins = state.value_origin(*value);
            if origins.is_empty() {
                origins.insert(BorrowOrigin::Unknown);
            }

            // report disallowed origins
            // inferred lifetimes only reject local or unknown origins
            if matches!(return_region, mir::BorrowRegion::Inferred) {
                let has_invalid_origin = origins
                    .iter()
                    .any(|origin| matches!(origin, BorrowOrigin::Local | BorrowOrigin::Unknown));
                if has_invalid_origin {
                    if strict_mode {
                        ctx.emit_error(OptimizeError::ReturnReferenceToLocal {
                            node: anchor_block(module_id, target_id, *block_id),
                        });
                    } else {
                        ctx.emit_warning(OptimizeWarning::PotentialBorrowEscape {
                            node: anchor_block(module_id, target_id, *block_id),
                        });
                    }
                }
                continue;
            }

            let Some(disallowed) = disallowed_origins(
                &origins,
                &return_region,
                &function.parameter_names,
                ctx.strings,
            ) else {
                continue;
            };

            if strict_mode {
                ctx.emit_error(OptimizeError::LifetimeAnnotationMismatch {
                    node: anchor_block(module_id, target_id, *block_id),
                    origin: disallowed,
                });
            } else {
                ctx.emit_warning(OptimizeWarning::PotentialLifetimeAnnotationMismatch {
                    node: anchor_block(module_id, target_id, *block_id),
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
        let Some(param_type) = param.ty.ty() else {
            continue;
        };
        let param_type = tree.get(param_type);
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

fn field_type_for_value(
    tree: &mir::NodeTree,
    aggregate_type: impl Into<mir::TypeReference>,
    index: u32,
) -> Option<mir::LocalNodeId<Type>> {
    // resolve field types for aggregates
    let aggregate_type = aggregate_type.into().ty()?;
    let aggregate = tree.get(aggregate_type);
    match aggregate {
        Type::Struct { fields, .. } => fields.get(index as usize).and_then(|field_id| {
            let field = tree.get(*field_id);
            field.ty.ty()
        }),
        Type::Tuple { elements, .. } => elements.get(index as usize).and_then(|ty| ty.ty()),
        Type::Newtype { inner, .. } => field_type_for_value(tree, *inner, index),
        _ => None,
    }
}

fn element_type_for_value(
    tree: &mir::NodeTree,
    array_type: impl Into<mir::TypeReference>,
) -> Option<mir::LocalNodeId<Type>> {
    // resolve element types for arrays
    let array_type = array_type.into().ty()?;
    let array_type = tree.get(array_type);
    match array_type {
        Type::Array { element, .. } => element.ty(),
        Type::Newtype { inner, .. } => element_type_for_value(tree, *inner),
        _ => None,
    }
}

fn value_contains_borrowed_refs(
    value: impl Into<mir::ValueReference>,
    tree: &mir::NodeTree,
    types: &ValueTypeMap,
) -> bool {
    // resolve the value type
    let Some(ty_id) = types.value_type(value) else {
        return true;
    };
    let ty = tree.get(ty_id);
    type_contains_borrowed_refs(ty, tree)
}

fn local_contains_borrowed_refs(
    local: impl Into<mir::LocalReference>,
    tree: &mir::NodeTree,
    types: &ValueTypeMap,
) -> bool {
    // resolve the local type
    let Some(ty_id) = types.local_type(local) else {
        return true;
    };
    let ty = tree.get(ty_id);
    type_contains_borrowed_refs(ty, tree)
}

fn assign_origin_if_borrowed(
    state: &mut BorrowOriginMap,
    destination: impl Into<mir::ValueReference>,
    origins: BorrowOriginSet,
    tree: &mir::NodeTree,
    types: &ValueTypeMap,
) {
    let destination = destination.into();

    // track origins only for borrowed results
    if value_contains_borrowed_refs(destination, tree, types) {
        state.set_value_origin(destination, origins);
    } else {
        state.set_value_origin(destination, BorrowOriginSet::default());
    }
}

fn apply_instruction_effects(
    state: &mut BorrowOriginMap,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &Instruction,
    tree: &mir::NodeTree,
    types: &ValueTypeMap,
    lifetime_analysis: &LifetimeAnalysis,
    call_targets: &CallTargetAnalysis,
) {
    match instruction {
        Instruction::Error => {
            panic!("recovered MIR instruction reached optimizer");
        }

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

        // vector operations
        Instruction::VectorSplat { destination, value } => {
            let origins = state.value_origin(*value);
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::VectorExtract {
            destination,
            vector,
            ..
        } => {
            let origins = state.value_origin(*vector);
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::VectorInsert {
            destination,
            vector,
            value,
            ..
        } => {
            let mut origins = state.value_origin(*vector);
            origins.union_with(&state.value_origin(*value));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::VectorShuffle {
            destination,
            left,
            right,
            ..
        } => {
            let mut origins = state.value_origin(*left);
            origins.union_with(&state.value_origin(*right));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::VectorSelect {
            destination,
            then_value,
            else_value,
            ..
        } => {
            let mut origins = state.value_origin(*then_value);
            origins.union_with(&state.value_origin(*else_value));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::VectorReduce {
            destination,
            vector,
            ..
        } => {
            let origins = state.value_origin(*vector);
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::VectorCompare {
            destination,
            left,
            right,
            ..
        } => {
            let mut origins = state.value_origin(*left);
            origins.union_with(&state.value_origin(*right));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::VectorConvert {
            destination,
            vector,
            ..
        } => {
            let origins = state.value_origin(*vector);
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // tensor operations
        Instruction::TensorLoad {
            destination, view, ..
        } => {
            let origins = state.value_origin(*view);
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::TensorReshape {
            destination,
            tensor,
            ..
        }
        | Instruction::TensorBroadcast {
            destination,
            tensor,
            ..
        }
        | Instruction::TensorTranspose {
            destination,
            tensor,
            ..
        }
        | Instruction::TensorSlice {
            destination,
            tensor,
            ..
        }
        | Instruction::TensorCast {
            destination,
            tensor,
            ..
        }
        | Instruction::TensorView {
            destination,
            view: tensor,
            ..
        }
        | Instruction::TensorConvert {
            destination,
            tensor,
            ..
        } => {
            let origins = state.value_origin(*tensor);
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::TensorCompare {
            destination,
            left,
            right,
            ..
        } => {
            let mut origins = state.value_origin(*left);
            origins.union_with(&state.value_origin(*right));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::TensorSelect {
            destination,
            then_value,
            else_value,
            ..
        } => {
            let mut origins = state.value_origin(*then_value);
            origins.union_with(&state.value_origin(*else_value));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::TensorPad {
            destination,
            tensor,
            value,
            ..
        } => {
            let mut origins = state.value_origin(*tensor);
            origins.union_with(&state.value_origin(*value));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::TensorConcat {
            destination,
            tensors,
            ..
        } => {
            let mut origins = BorrowOriginSet::default();
            for value in tree.get_arguments(*tensors) {
                origins.union_with(&state.value_origin(*value));
            }
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::TensorReduce {
            destination,
            tensor,
            initial,
            ..
        } => {
            let mut origins = state.value_origin(*tensor);
            origins.union_with(&state.value_origin(*initial));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::TensorDot {
            destination,
            left,
            right,
            ..
        } => {
            let mut origins = state.value_origin(*left);
            origins.union_with(&state.value_origin(*right));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::TensorConvolution {
            destination,
            input,
            kernel,
            ..
        } => {
            let mut origins = state.value_origin(*input);
            origins.union_with(&state.value_origin(*kernel));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::TensorGather {
            destination,
            operand,
            indices,
            ..
        } => {
            let mut origins = state.value_origin(*operand);
            origins.union_with(&state.value_origin(*indices));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }
        Instruction::TensorScatter {
            destination,
            operand,
            indices,
            updates,
            ..
        } => {
            let mut origins = state.value_origin(*operand);
            origins.union_with(&state.value_origin(*indices));
            origins.union_with(&state.value_origin(*updates));
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // locals carry their stored origins
        Instruction::LocalGet { destination, local } => {
            let origins = state.local_origin(*local);
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // function.environment is an implicit parameter, treat as unknown origin
        Instruction::FunctionEnvironment { destination } => {
            assign_origin_if_borrowed(
                state,
                *destination,
                BorrowOriginSet::from_origin(BorrowOrigin::Unknown),
                tree,
                types,
            );
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

        // write like operations do not produce borrowed values
        Instruction::TensorStore { .. }
        | Instruction::TensorFill { .. }
        | Instruction::TensorCopy { .. }
        | Instruction::AtomicStore { .. }
        | Instruction::AtomicFence { .. }
        | Instruction::Barrier { .. } => {}

        // globals are static borrows
        Instruction::GlobalAddr { destination, .. }
        | Instruction::FunctionAddr { destination, .. }
        | Instruction::GlobalConst { destination, .. } => {
            assign_origin_if_borrowed(
                state,
                *destination,
                BorrowOriginSet::from_origin(BorrowOrigin::Static),
                tree,
                types,
            );
        }

        // function values preserve the environment origin
        Instruction::FunctionBind {
            destination,
            environment,
            ..
        } => {
            let origins = state.value_origin(*environment);
            assign_origin_if_borrowed(state, *destination, origins, tree, types);
        }

        // locals are local borrows
        Instruction::LocalAddr { destination, .. } => {
            assign_origin_if_borrowed(
                state,
                *destination,
                BorrowOriginSet::from_origin(BorrowOrigin::Local),
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

        // atomic memory reads preserve the pointer origin
        Instruction::AtomicLoad {
            destination,
            pointer,
            ..
        }
        | Instruction::AtomicCompareExchange {
            destination,
            pointer,
            ..
        }
        | Instruction::AtomicRmw {
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
            let aggregate_type = types.require_value_type(*aggregate);
            let borrowed_field = field_type_for_value(tree, aggregate_type, *index)
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
            let array_type = types.require_value_type(*array);
            let borrowed_element = element_type_for_value(tree, array_type)
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
            let Some(struct_type) = ty.ty() else {
                assign_origin_if_borrowed(
                    state,
                    *destination,
                    BorrowOriginSet::default(),
                    tree,
                    types,
                );
                return;
            };
            let struct_type = tree.get(struct_type);
            let field_values = tree.get_arguments(*fields);

            let mut origins = BorrowOriginSet::default();
            if let Type::Struct { fields, .. } = struct_type {
                for (field_id, value) in fields.iter().zip(field_values) {
                    let field = tree.get(*field_id);
                    if field.ty.ty().is_none_or(|field_ty| {
                        type_contains_borrowed_refs(tree.get(field_ty), tree)
                    }) {
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
            let Some(tuple_type) = ty.ty() else {
                assign_origin_if_borrowed(
                    state,
                    *destination,
                    BorrowOriginSet::default(),
                    tree,
                    types,
                );
                return;
            };
            let tuple_type = tree.get(tuple_type);
            let values = tree.get_arguments(*elements);

            let mut origins = BorrowOriginSet::default();
            if let Type::Tuple { elements, .. } = tuple_type {
                for (element_type, value) in elements.iter().zip(values) {
                    if element_type.ty().is_none_or(|element_ty| {
                        type_contains_borrowed_refs(tree.get(element_ty), tree)
                    }) {
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
            let Some(array_type) = ty.ty() else {
                assign_origin_if_borrowed(
                    state,
                    *destination,
                    BorrowOriginSet::default(),
                    tree,
                    types,
                );
                return;
            };
            let array_type = tree.get(array_type);
            let values = tree.get_arguments(*elements);

            let mut origins = BorrowOriginSet::default();
            if let Type::Array { element, .. } = array_type {
                if element.ty().is_none_or(|element_ty| {
                    type_contains_borrowed_refs(tree.get(element_ty), tree)
                }) {
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
            call,
            ..
        } => {
            let return_contains_borrow =
                signature_return_contains_borrowed_refs(call.signature, tree);
            let origins = function.function().map_or_else(
                || BorrowOriginSet::from_origin(BorrowOrigin::Unknown),
                |function| {
                    origins_for_call(
                        function,
                        tree.get_arguments(call.arguments),
                        state,
                        lifetime_analysis,
                        return_contains_borrow,
                    )
                },
            );

            assign_origin_if_borrowed(state, *dest, origins, tree, types);
        }

        // virtual and interface calls use signature fallback
        Instruction::CallVirtual {
            destination: Some(dest),
            call,
            ..
        }
        | Instruction::CallInterface {
            destination: Some(dest),
            call,
            ..
        } => {
            let return_contains_borrow =
                signature_return_contains_borrowed_refs(call.signature, tree);
            let origins =
                if let Some(targets) = call_targets.targets_for_instruction(instruction_id) {
                    origins_for_targets(
                        targets,
                        tree.get_arguments(call.arguments),
                        state,
                        lifetime_analysis,
                        return_contains_borrow,
                    )
                } else {
                    origins_for_signature(
                        call.signature,
                        tree,
                        tree.get_arguments(call.arguments),
                        state,
                        return_contains_borrow,
                    )
                };

            assign_origin_if_borrowed(state, *dest, origins, tree, types);
        }

        // indirect calls rely on the signature
        Instruction::CallIndirect {
            destination: Some(dest),
            call,
            ..
        } => {
            let return_contains_borrow =
                signature_return_contains_borrowed_refs(call.signature, tree);
            let origins =
                if let Some(targets) = call_targets.targets_for_instruction(instruction_id) {
                    origins_for_targets(
                        targets,
                        tree.get_arguments(call.arguments),
                        state,
                        lifetime_analysis,
                        return_contains_borrow,
                    )
                } else {
                    origins_for_signature(
                        call.signature,
                        tree,
                        tree.get_arguments(call.arguments),
                        state,
                        return_contains_borrow,
                    )
                };

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
        Instruction::New { destination, .. }
        | Instruction::NewSlice { destination, .. }
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
        | Instruction::Dispose { .. }
        | Instruction::AsyncDispose { .. }
        | Instruction::Pin { .. }
        | Instruction::Unpin { .. }
        | Instruction::Drop { .. }
        | Instruction::Assume { .. } => {}
    }
}

fn origins_for_call(
    function_id: mir::LocalNodeId<mir::Function>,
    arguments: &[mir::ValueReference],
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

fn origins_for_targets(
    targets: &[mir::LocalNodeId<mir::Function>],
    arguments: &[mir::ValueReference],
    state: &BorrowOriginMap,
    lifetime_analysis: &LifetimeAnalysis,
    return_contains_borrow: bool,
) -> BorrowOriginSet {
    let mut origins = BorrowOriginSet::default();

    for target in targets {
        let target_origins = origins_for_call(
            *target,
            arguments,
            state,
            lifetime_analysis,
            return_contains_borrow,
        );
        origins.union_with(&target_origins);
    }

    origins
}

fn origins_for_signature(
    signature_type: mir::TypeReference,
    tree: &mir::NodeTree,
    arguments: &[mir::ValueReference],
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

fn disallowed_origins(
    origins: &BorrowOriginSet,
    region: &mir::BorrowRegion,
    parameter_names: &[Option<destack_core::StringId>],
    strings: &StringPool,
) -> Option<String> {
    // collect disallowed origins
    let mut disallowed = Vec::new();
    for origin in origins.iter() {
        let allowed = match region {
            mir::BorrowRegion::Static => matches!(origin, BorrowOrigin::Static),
            mir::BorrowRegion::Parameters(params) => match origin {
                BorrowOrigin::Parameter(index) => params.contains(index),
                _ => false,
            },
            mir::BorrowRegion::Inferred => true,
        };

        if !allowed {
            disallowed.push(format_origin(origin, parameter_names, strings));
        }
    }

    if disallowed.is_empty() {
        return None;
    }

    Some(disallowed.join(", "))
}

fn format_origin(
    origin: &BorrowOrigin,
    parameter_names: &[Option<destack_core::StringId>],
    strings: &StringPool,
) -> String {
    // format origin descriptions
    match origin {
        BorrowOrigin::Parameter(index) => parameter_names
            .get(*index as usize)
            .and_then(|name| *name)
            .map(|name_id| strings.get(name_id).to_string())
            .unwrap_or_else(|| format!("parameter {index}")),
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

    /// Build strict borrow mode options for verification tests.
    fn strict_options() -> PipelineOptions {
        PipelineOptions {
            strict_borrow_mode: true,
            ..Default::default()
        }
    }

    /// Returning a borrowed param matches lifetime annotations.
    #[test]
    fn test_verify_return_borrowed_param() {
        let input = r#"
function test(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    return v0
}"#;

        let options = strict_options();

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::BorrowRegion::Parameters(vec![0]));
        test.run_pass_with_options(&LifetimeCheck, options);
        test.assert_no_errors();
    }

    /// Returning a borrowed field matches lifetime annotations.
    #[test]
    fn test_verify_return_borrowed_field() {
        let input = r#"
function test(v0: ref<{ int32 }, borrowed>): ref<int32, borrowed> {
b0(v0: ref<{ int32 }, borrowed>):
    v1: ref<int32, borrowed> = field.address v0, 0
    return v1
}"#;

        let options = strict_options();

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::BorrowRegion::Parameters(vec![0]));
        test.run_pass_with_options(&LifetimeCheck, options);
        test.assert_no_errors();
    }

    /// Returning a local borrow is rejected by parameter lifetimes.
    #[test]
    fn test_verify_return_local_borrow_rejected() {
        let input = r#"
function test(): ref<int32, borrowed> {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, borrowed> = field.address v0, 0
    return v1
}"#;

        let options = strict_options();

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::BorrowRegion::Parameters(vec![0]));
        test.run_pass_with_options(&LifetimeCheck, options);
        test.assert_error(|e| matches!(e, OptimizeError::LifetimeAnnotationMismatch { .. }));
    }

    /// Returning a borrowed param is rejected by static lifetimes.
    #[test]
    fn test_verify_static_lifetime_rejects_param() {
        let input = r#"
function test(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    return v0
}"#;

        let options = strict_options();

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::BorrowRegion::Static);
        test.run_pass_with_options(&LifetimeCheck, options);
        test.assert_error(|e| matches!(e, OptimizeError::LifetimeAnnotationMismatch { .. }));
    }

    /// Mismatched lifetimes warn in lenient mode.
    #[test]
    fn test_verify_mismatch_warns_in_lenient_mode() {
        let input = r#"
function test(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::BorrowRegion::Static);
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
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::BorrowRegion::Static);
        test.run_pass(&LifetimeCheck);
        test.assert_no_errors();
        test.assert_warning(|w| matches!(w, OptimizeWarning::LifetimeAnnotationIgnored { .. }));
    }

    /// Indirect calls fall back to signature borrowing rules.
    #[test]
    fn test_verify_signature_fallback_for_indirect_call() {
        let input = r#"
function test(v0: fn(ref<int32, borrowed>) -> ref<int32, borrowed>, v1: ref<int32, borrowed>): ref<int32, borrowed>  {
b0(v0: fn(ref<int32, borrowed>) -> ref<int32, borrowed>, v1: ref<int32, borrowed>) -> v2: ref<int32, borrowed> = call.indirect v0(v1): (ref<int32, borrowed>) -> ref<int32, borrowed>
    return v2
}"#;

        let options = strict_options();

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("test", mir::BorrowRegion::Parameters(vec![1]));
        test.run_pass_with_options(&LifetimeCheck, options);
        test.assert_no_errors();
    }
}

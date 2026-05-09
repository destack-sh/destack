use std::collections::{HashMap, HashSet};

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::ConstantPropagation;
use crate::common::mir::instruction_requires_exact_access;
use crate::optimize::{
    AnalysisPreservation, FunctionPass, PipelineContext, instruction_substitute_uses_in_tree,
    remap_instruction_memory_accesses, terminator_substitute_uses, terminator_uses,
};

declare_mir_pass! {
    /// Scalar Replacement of Aggregates.
    ///
    /// Breaks apart aggregate stack allocations (structs, tuples, small arrays)
    /// into individual scalar allocations. This enables mem2reg to promote
    /// each scalar to an SSA value.
    ///
    /// ```mir
    /// // before SROA
    /// function before(): int32 {
    /// b0:
    ///     v0 = stack.alloc { int32, int32 }
    ///     v1 = field.address v0, 0
    ///     v2 = 1int32
    ///     store v1, v2
    ///     v3 = field.address v0, 1
    ///     v4 = 2int32
    ///     store v3, v4
    ///     v5 = load v1
    ///     return v5
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// // after SROA
    /// function after(): int32 {
    /// b0:
    ///     v0 = stack.alloc int32  // field 0
    ///     v1 = stack.alloc int32  // field 1
    ///     v2 = 1int32
    ///     store v0, v2
    ///     v3 = 2int32
    ///     store v1, v3
    ///     v4 = load v0
    ///     return v4
    /// }
    /// ```
    #[pass(id = "sroa")]
    pub Sroa,
    "Break aggregates into scalars"
}

impl FunctionPass for Sroa {
    /// Run scalar replacement of aggregates on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip empty functions
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get constant propagation analysis
        let constants = {
            let analyses = ctx.function_analyses(function, tree);
            analyses.get::<ConstantPropagation>().clone()
        };

        // run SROA
        let changed = run_sroa(
            function,
            tree,
            entry,
            ctx.options.sroa_max_array_elements,
            &constants,
        );

        // select preservation based on SROA changes
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "Sroa"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "sroa"
    }
}

/// Core SROA logic. Returns true if changes were made.
fn run_sroa(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    entry: mir::LocalNodeId<mir::Block>,
    max_array_elements: usize,
    constants: &ConstantPropagation,
) -> bool {
    // find splittable allocations
    let candidates =
        find_splittable_allocations_core(function, tree, max_array_elements, constants);
    if candidates.is_empty() {
        return false;
    }

    // ensure next_value_id is correct before allocating new values
    function.recompute_next_value_id(tree);

    // split each candidate
    let mut made_changes = false;
    for candidate in candidates {
        if split_allocation(&candidate, function, tree, entry) {
            made_changes = true;
        }
    }

    made_changes
}

/// A candidate allocation that can be split.
struct SplitCandidate {
    /// The stack allocation instruction.
    alloc_instruction: mir::LocalNodeId<mir::Instruction>,
    /// The aggregate layout type.
    layout: mir::LocalNodeId<mir::Type>,
    /// Reference metadata for derived stack slots.
    reference_spec: ReferenceSpec,
    /// The element types after splitting.
    element_types: Vec<mir::LocalNodeId<mir::Type>>,
    /// Uses of the allocation (field/element addresses).
    uses: Vec<UseInfo>,
    /// Loads performed directly on the base pointer.
    base_loads: Vec<mir::LocalNodeId<mir::Instruction>>,
    /// Stores performed directly on the base pointer.
    base_stores: Vec<mir::LocalNodeId<mir::Instruction>>,
}

/// Reference attributes used to rebuild stack slot types.
#[derive(Clone)]
struct ReferenceSpec {
    /// The reference kind.
    kind: mir::ReferenceKind,
    /// The address space for the reference.
    address_space: mir::AddressSpace,
    /// The access for the reference.
    access: mir::Access,
    /// The nullability for the reference.
    is_nullable: bool,
}

impl ReferenceSpec {
    /// Extract reference attributes from a reference type.
    fn from_type(ty: &mir::Type) -> Option<Self> {
        let mir::Type::Reference {
            kind,
            address_space,
            access,
            is_nullable,
            ..
        } = ty
        else {
            return None;
        };

        Some(Self {
            kind: *kind,
            address_space: address_space.clone(),
            access: *access,
            is_nullable: *is_nullable,
        })
    }
}

/// Information about a use of an allocation.
#[derive(Clone)]
struct UseInfo {
    /// The instruction using the allocation.
    instruction: mir::LocalNodeId<mir::Instruction>,
    /// The field/element index being accessed.
    index: usize,
    /// The destination value (the address).
    destination: mir::Value,
}

/// Aggregate uses discovered during analysis.
struct AllocationUses {
    /// Field or element address uses.
    uses: Vec<UseInfo>,
    /// Base pointer loads.
    base_loads: Vec<mir::LocalNodeId<mir::Instruction>>,
    /// Base pointer stores.
    base_stores: Vec<mir::LocalNodeId<mir::Instruction>>,
}

/// Find stack allocations that can be split into scalars.
fn find_splittable_allocations_core(
    function: &mir::Function,
    tree: &mir::Tree,
    max_array_elements: usize,
    constants: &ConstantPropagation,
) -> Vec<SplitCandidate> {
    let mut candidates = Vec::new();

    // collect all stack allocations of aggregate types
    let block_ids: Vec<_> = function.blocks.clone();

    for &block_id in &block_ids {
        let block = tree.get(block_id);

        for &inst_id in &block.instructions {
            let inst = tree.get(inst_id);

            if let mir::Instruction::StackAlloc {
                destination,
                layout,
                result_type,
            } = inst
            {
                let Some(result_type) = result_type.ty() else {
                    continue;
                };
                let Some(layout) = layout.ty() else {
                    continue;
                };
                let Some(destination) = destination.value() else {
                    continue;
                };

                let reference_spec = match ReferenceSpec::from_type(tree.get(result_type)) {
                    Some(spec) => spec,
                    None => continue,
                };
                let ty = tree.get(layout);

                // check if this is a splittable aggregate type
                let element_types = match get_element_types(ty, tree, max_array_elements) {
                    Some(types) => types,
                    None => continue,
                };

                // analyze uses to determine if splittable
                let uses = match analyze_uses(destination, function, tree, constants) {
                    Some(uses) => uses,
                    None => continue,
                };
                if uses
                    .uses
                    .iter()
                    .any(|use_info| use_info.index >= element_types.len())
                {
                    continue;
                }

                candidates.push(SplitCandidate {
                    alloc_instruction: inst_id,
                    layout,
                    reference_spec,
                    element_types,
                    uses: uses.uses,
                    base_loads: uses.base_loads,
                    base_stores: uses.base_stores,
                });
            }
        }
    }

    candidates
}

/// Get the element types for an aggregate type, if splittable.
///
/// Returns None if the type is not an aggregate or is too large to split.
/// Does not recursively flatten nested aggregates - those will be split in
/// subsequent passes.
fn get_element_types(
    ty: &mir::Type,
    tree: &mir::Tree,
    max_array_elements: usize,
) -> Option<Vec<mir::LocalNodeId<mir::Type>>> {
    match ty {
        mir::Type::Struct { fields, copy: _ } => {
            // collect field types without recursive flattening
            let types: Vec<_> = fields
                .iter()
                .map(|&field_id| {
                    let field = tree.get(field_id);
                    field.ty.ty()
                })
                .collect::<Option<_>>()?;

            Some(types)
        }

        mir::Type::Tuple { elements, copy: _ } => {
            // collect element types without recursive flattening
            elements
                .iter()
                .copied()
                .map(|element| element.ty())
                .collect::<Option<_>>()
        }

        mir::Type::Array {
            element,
            length,
            copy: _,
        } => {
            // only split small arrays
            if *length as usize > max_array_elements {
                return None;
            }

            // create element types for each array element
            let types = vec![element.ty()?; *length as usize];

            Some(types)
        }

        // not an aggregate type
        _ => None,
    }
}

/// Analyze uses of an allocation to determine if it can be split.
///
/// Returns None if the allocation escapes or has unsupported uses.
fn analyze_uses(
    alloc_value: mir::Value,
    function: &mir::Function,
    tree: &mir::Tree,
    constants: &ConstantPropagation,
) -> Option<AllocationUses> {
    let mut uses = Vec::new();
    let mut base_loads = Vec::new();
    let mut base_stores = Vec::new();
    let mut seen_values: HashSet<mir::Value> = HashSet::new();
    let mut worklist: Vec<mir::Value> = vec![alloc_value];

    while let Some(value) = worklist.pop() {
        if !seen_values.insert(value) {
            continue;
        }

        // find all uses of this value
        for &block_id in &function.blocks {
            let block = tree.get(block_id);

            for &inst_id in &block.instructions {
                let inst = tree.get(inst_id);

                match inst {
                    // field address: supported if it's the allocation pointer
                    mir::Instruction::FieldAddr {
                        destination,
                        aggregate,
                        index,
                        ..
                    } if aggregate.value() == Some(value) => {
                        let Some(destination) = destination.value() else {
                            return None;
                        };

                        uses.push(UseInfo {
                            instruction: inst_id,
                            index: *index as usize,
                            destination,
                        });

                        // the field address itself might be used
                        worklist.push(destination);
                    }

                    // element address: supported if index is constant
                    mir::Instruction::ElementAddr {
                        destination,
                        array,
                        index,
                        ..
                    } if array.value() == Some(value) => {
                        let Some(index) = index.value() else {
                            return None;
                        };

                        // check if index is a constant
                        let const_index = resolve_constant_index(index, block_id, constants)?;
                        let Some(destination) = destination.value() else {
                            return None;
                        };

                        uses.push(UseInfo {
                            instruction: inst_id,
                            index: const_index,
                            destination,
                        });

                        // the element address itself might be used
                        worklist.push(destination);
                    }

                    // loads and stores are allowed, base pointer uses are recorded
                    mir::Instruction::Load { pointer, .. } if pointer.value() == Some(value) => {
                        if instruction_requires_exact_access(tree, inst_id) {
                            return None;
                        }
                        if value == alloc_value {
                            base_loads.push(inst_id);
                        }
                    }

                    mir::Instruction::Store { pointer, .. } if pointer.value() == Some(value) => {
                        if instruction_requires_exact_access(tree, inst_id) {
                            return None;
                        }
                        if value == alloc_value {
                            base_stores.push(inst_id);
                        }
                    }

                    // calls: check if value is passed as argument (escapes)
                    mir::Instruction::Call { .. }
                    | mir::Instruction::CallVirtual { .. }
                    | mir::Instruction::CallInterface { .. }
                    | mir::Instruction::CallIndirect { .. } => {
                        // arguments are stored externally, access via argument_slice
                        if let Some(arg_slice) = inst.argument_slice() {
                            for &arg in tree.get_arguments(arg_slice) {
                                if arg.value() == Some(value) {
                                    // value escapes through call
                                    return None;
                                }
                            }
                        }
                    }

                    // any other use of the allocation value escapes
                    _ => {
                        if inst
                            .uses()
                            .into_iter()
                            .any(|used| used.value() == Some(value))
                        {
                            // this value escapes, can't split
                            return None;
                        }
                    }
                }
            }

            // check terminator uses
            let terminator = tree.get(block.terminator);
            if terminator_uses(terminator, value) {
                // value used in terminator, escapes
                return None;
            }
        }
    }

    Some(AllocationUses {
        uses,
        base_loads,
        base_stores,
    })
}

/// Resolve a constant integer index from constant propagation.
fn resolve_constant_index(
    value: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    constants: &ConstantPropagation,
) -> Option<usize> {
    // read constant at block exit
    let constant = constants.constant_at_exit(block_id, value)?;

    // convert constant into index
    constant_to_index(constant)
}

/// Convert a constant into a usable index value.
fn constant_to_index(constant: &mir::Constant) -> Option<usize> {
    // map integer constants to non negative indices
    match constant {
        mir::Constant::Int { value, .. } => {
            // reject negative indices
            if *value < 0 {
                return None;
            }

            // convert the value into an index
            usize::try_from(*value).ok()
        }
        mir::Constant::UInt { value, .. } => usize::try_from(*value).ok(),
        _ => None,
    }
}

/// Split an allocation into individual scalar allocations.
fn split_allocation(
    candidate: &SplitCandidate,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    entry: mir::LocalNodeId<mir::Block>,
) -> bool {
    // create new allocations for each element
    let mut new_allocs: Vec<mir::Value> = Vec::new();

    for &elem_type in &candidate.element_types {
        let result_type = tree.insert_type(mir::Type::Reference {
            kind: candidate.reference_spec.kind,
            lifetime: mir::Lifetime::empty(),
            address_space: candidate.reference_spec.address_space.clone(),
            access: candidate.reference_spec.access,
            pointee: elem_type.into(),
            is_nullable: candidate.reference_spec.is_nullable,
        });
        let new_value = function.next_typed_value(result_type);
        new_allocs.push(new_value);

        // create the new StackAlloc instruction
        let new_inst = mir::Instruction::StackAlloc {
            destination: new_value.into(),
            layout: elem_type.into(),
            result_type: result_type.into(),
        };

        // insert at the start of the entry block (after existing allocs)
        let new_inst_id = tree.insert(new_inst);
        let entry_block = tree.get_mut(entry);
        entry_block.instructions.insert(0, new_inst_id);
    }

    // build mapping from field/element index to new value
    let index_to_value: HashMap<usize, mir::Value> = new_allocs
        .iter()
        .enumerate()
        .map(|(i, &v)| (i, v))
        .collect();

    // rewrite uses: replace field/element addresses with new allocation values
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();

    for use_info in &candidate.uses {
        if let Some(&new_value) = index_to_value.get(&use_info.index) {
            substitutions.insert(use_info.destination, new_value);
        }
    }

    // apply substitutions to all instructions
    apply_substitutions(&substitutions, function, tree);

    // collect instructions to remove after rewriting
    let mut to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();
    to_remove.insert(candidate.alloc_instruction);

    for use_info in &candidate.uses {
        to_remove.insert(use_info.instruction);
    }

    // rewrite base pointer loads and stores
    let base_loads: HashSet<_> = candidate.base_loads.iter().copied().collect();
    let base_stores: HashSet<_> = candidate.base_stores.iter().copied().collect();

    // rewrite instructions per block
    let block_ids: Vec<_> = function.blocks.clone();
    for &block_id in &block_ids {
        let instruction_ids = {
            let block = tree.get(block_id);
            block.instructions.clone()
        };

        let mut new_instructions: Vec<mir::LocalNodeId<mir::Instruction>> = Vec::new();

        for instruction_id in instruction_ids {
            // rewrite base pointer loads into scalar loads
            if base_loads.contains(&instruction_id) {
                let mut rewritten =
                    rewrite_base_load(candidate, function, tree, &index_to_value, instruction_id);
                new_instructions.append(&mut rewritten);
                continue;
            }

            // rewrite base pointer stores into scalar stores
            if base_stores.contains(&instruction_id) {
                let mut rewritten =
                    rewrite_base_store(candidate, function, tree, &index_to_value, instruction_id);
                new_instructions.append(&mut rewritten);
                continue;
            }

            // drop instructions that are replaced or removed
            if to_remove.contains(&instruction_id) {
                continue;
            }

            // keep untouched instructions
            new_instructions.push(instruction_id);
        }

        let mut block = tree.get(block_id).clone();
        if block.instructions != new_instructions {
            block.instructions = new_instructions;
            tree.replace(block_id, block);
        }
    }

    true
}

/// Rewrite a base pointer load into scalar loads and aggregate rebuild.
fn rewrite_base_load(
    candidate: &SplitCandidate,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    index_to_value: &HashMap<usize, mir::Value>,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
) -> Vec<mir::LocalNodeId<mir::Instruction>> {
    // extract load destination
    let (destination, _pointer) = match tree.get(instruction_id) {
        mir::Instruction::Load {
            destination,
            pointer,
            ..
        } => {
            let Some(destination) = destination.value() else {
                return Vec::new();
            };
            let Some(pointer) = pointer.value() else {
                return Vec::new();
            };

            (destination, pointer)
        }
        _ => panic!("sroa base load rewrite expects a load instruction"),
    };

    // load each scalar element in order
    let mut element_values: Vec<mir::Value> = Vec::new();
    let mut new_instructions: Vec<mir::LocalNodeId<mir::Instruction>> = Vec::new();
    for index in 0..candidate.element_types.len() {
        let element_pointer = index_to_value
            .get(&index)
            .copied()
            .expect("missing scalar slot for aggregate element");
        let element_type = candidate.element_types[index];
        let element_value = function.next_typed_value(element_type);
        let load_inst = mir::Instruction::Load {
            destination: element_value.into(),
            pointer: element_pointer.into(),
            result_type: element_type.into(),
        };
        let load_id = tree.insert(load_inst);
        new_instructions.push(load_id);
        element_values.push(element_value);
    }

    // rebuild the aggregate value from the loaded elements
    let aggregate_inst =
        build_aggregate_instruction(tree, candidate.layout, destination, &element_values);
    let aggregate_id = tree.insert(aggregate_inst);
    new_instructions.push(aggregate_id);

    new_instructions
}

/// Rewrite a base pointer store into aggregate decompositions and scalar stores.
fn rewrite_base_store(
    candidate: &SplitCandidate,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    index_to_value: &HashMap<usize, mir::Value>,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
) -> Vec<mir::LocalNodeId<mir::Instruction>> {
    // extract stored value
    let stored_value = match tree.get(instruction_id) {
        mir::Instruction::Store { value, .. } => {
            let Some(value) = value.value() else {
                return Vec::new();
            };

            value
        }
        _ => panic!("sroa base store rewrite expects a store instruction"),
    };

    // select aggregate decomposition strategy
    let layout = tree.get(candidate.layout);
    let is_array = matches!(layout, mir::Type::Array { .. });

    let mut new_instructions: Vec<mir::LocalNodeId<mir::Instruction>> = Vec::new();
    for index in 0..candidate.element_types.len() {
        let element_pointer = index_to_value
            .get(&index)
            .copied()
            .expect("missing scalar slot for aggregate element");
        let element_type = candidate.element_types[index];
        let element_value = function.next_typed_value(element_type);

        // handle array extraction using element indices
        if is_array {
            let element_get = mir::Instruction::ElementGet {
                destination: element_value.into(),
                array: stored_value.into(),
                index: index as u32,
            };
            let element_get_id = tree.insert(element_get);
            new_instructions.push(element_get_id);
        }

        // handle struct or tuple extraction using field indices
        if !is_array {
            let field_get = mir::Instruction::FieldGet {
                destination: element_value.into(),
                aggregate: stored_value.into(),
                index: index as u32,
            };
            let field_get_id = tree.insert(field_get);
            new_instructions.push(field_get_id);
        }

        // store scalar into the split allocation slot
        let store_inst = mir::Instruction::Store {
            pointer: element_pointer.into(),
            value: element_value.into(),
        };
        let store_id = tree.insert(store_inst);
        new_instructions.push(store_id);
    }

    new_instructions
}

/// Build an aggregate construction instruction for the given layout.
fn build_aggregate_instruction(
    tree: &mut mir::Tree,
    layout: mir::LocalNodeId<mir::Type>,
    destination: mir::Value,
    element_values: &[mir::Value],
) -> mir::Instruction {
    // prepare aggregate arguments and layout
    let arguments: Vec<_> = element_values.iter().copied().map(Into::into).collect();
    let arguments = tree.add_arguments(&arguments);
    let layout_type = tree.get(layout);

    match layout_type {
        mir::Type::Struct { .. } => mir::Instruction::Struct {
            destination: destination.into(),
            ty: layout.into(),
            fields: arguments,
        },
        mir::Type::Tuple { .. } => mir::Instruction::Tuple {
            destination: destination.into(),
            ty: layout.into(),
            elements: arguments,
        },
        mir::Type::Array { .. } => mir::Instruction::Array {
            destination: destination.into(),
            ty: layout.into(),
            elements: arguments,
        },
        _ => panic!("sroa base load expects an aggregate layout type"),
    }
}

/// Apply value substitutions to all instructions and terminators.
fn apply_substitutions(
    substitutions: &HashMap<mir::Value, mir::Value>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
) {
    if substitutions.is_empty() {
        return;
    }

    for &block_id in &function.blocks {
        // substitute in instructions
        let instruction_ids = {
            let block = tree.get(block_id);
            block.instructions.clone()
        };

        for inst_id in instruction_ids {
            let instruction = tree.get(inst_id).clone();
            let new_instruction =
                instruction_substitute_uses_in_tree(&instruction, substitutions, tree);
            tree.replace(inst_id, new_instruction);
            remap_instruction_memory_accesses(tree, inst_id, substitutions);
        }

        // substitute in terminator
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator).clone();
        let new_terminator = terminator_substitute_uses(&terminator, substitutions);
        tree.replace(block.terminator, new_terminator);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use destack_mir as mir;

    fn first_load_id(
        test: &TestProgram,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> mir::LocalNodeId<mir::Instruction> {
        let function = test.tree.get(function_id);
        for block_id in &function.blocks {
            let block = test.tree.get(*block_id);
            for &instruction_id in &block.instructions {
                if matches!(test.tree.get(instruction_id), mir::Instruction::Load { .. }) {
                    return instruction_id;
                }
            }
        }

        panic!("missing load instruction");
    }

    /// Simple struct splitting.
    ///
    /// The struct allocation is split into separate allocations for each field.
    /// field.address instructions are replaced with direct references to the new allocations.
    #[test]
    fn test_split_struct() {
        let input = r#"
type Point {
    int32;
    int32;
}
function test(): int32 {
b0:
    v0: ref<Point, raw, space(stack)> = stack.alloc Point
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: int32 = 42int32
    store v1, v2
    v3: int32 = load v1
    return v3
}"#;
        let expected = r#"
type Point {
    int32;
    int32;
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v1, v2
    v3: int32 = load v1
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_output(expected);
    }

    /// Tuple splitting.
    ///
    /// Tuples are split like structs - each element gets its own allocation.
    #[test]
    fn test_split_tuple() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<(int32, int64), raw, space(stack)> = stack.alloc (int32, int64)
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: int32 = 42int32
    store v1, v2
    v3: int32 = load v1
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int64, raw, space(stack)> = stack.alloc int64
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v1, v2
    v3: int32 = load v1
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_output(expected);
    }

    /// Small array splitting.
    ///
    /// Arrays with constant indices are split into separate allocations per element.
    #[test]
    fn test_split_small_array() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32[4], raw, space(stack)> = stack.alloc int32[4]
    v1: int64 = 0int64
    v2: ref<int32, borrowed> = element.address v0, v1
    v3: int32 = 42int32
    store v2, v3
    v4: int32 = load v2
    return v4
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: ref<int32, raw, space(stack)> = stack.alloc int32
    v4: int64 = 0int64
    v5: int32 = 42int32
    store v3, v5
    v6: int32 = load v3
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_output(expected);
    }

    /// Array too large to split.
    ///
    /// Arrays exceeding the max element threshold are not split.
    #[test]
    fn test_preserve_large_array() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32[100], raw, space(stack)> = stack.alloc int32[100]
    v1: int64 = 0int64
    v2: ref<int32, borrowed> = element.address v0, v1
    v3: int32 = 42int32
    store v2, v3
    v4: int32 = load v2
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_unchanged(input);
    }

    /// Escaping allocation should not be split.
    ///
    /// When the allocation is passed to an external function, it escapes
    /// and cannot be split.
    #[test]
    fn test_preserve_escaping() {
        let input = r#"
type Point {
    int32;
    int32;
}
extern function external(ref<Point, raw>): void
function test(): void {
b0:
    v0: ref<Point, raw, space(stack)> = stack.alloc Point
    call external(v0): (ref<Point, raw>) -> void
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_unchanged(input);
    }

    /// Non-constant array index should not be split.
    ///
    /// When the array index is dynamic (not a compile-time constant),
    /// we cannot split because we don't know which element is accessed.
    #[test]
    fn test_preserve_dynamic_index() {
        let input = r#"
function test(v0: int64): int32 {
b0(v0: int64):
    v1: ref<int32[4], raw, space(stack)> = stack.alloc int32[4]
    v2: ref<int32, borrowed> = element.address v1, v0
    v3: int32 = 42int32
    store v2, v3
    v4: int32 = load v2
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_unchanged(input);
    }

    /// Multiple fields accessed.
    ///
    /// When multiple fields of a struct are accessed, all field.address
    /// instructions are replaced with the corresponding new allocations.
    #[test]
    fn test_multiple_fields() {
        let input = r#"
type Point {
    int32;
    int32;
}
function test(): int32 {
b0:
    v0: ref<Point, raw, space(stack)> = stack.alloc Point
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: int32 = 10int32
    store v1, v2
    v3: ref<int32, borrowed> = field.address v0, 1
    v4: int32 = 20int32
    store v3, v4
    v5: int32 = load v1
    v6: int32 = load v3
    v7: int32 = int.add v5, v6
    return v7
}"#;
        let expected = r#"
type Point {
    int32;
    int32;
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 10int32
    store v1, v2
    v3: int32 = 20int32
    store v0, v3
    v4: int32 = load v1
    v5: int32 = load v0
    v6: int32 = int.add v4, v5
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_output(expected);
    }

    /// Nested struct - only top level is split.
    ///
    /// SROA splits only the top-level aggregate; nested aggregates remain
    /// intact and can be split in subsequent passes.
    #[test]
    fn test_nested_struct() {
        let input = r#"
type Inner {
    int32;
    int32;
}
type Outer {
    Inner;
    int64;
}
function test(): int64 {
b0:
    v0: ref<Outer, raw, space(stack)> = stack.alloc Outer
    v1: ref<int64, borrowed> = field.address v0, 1
    v2: int64 = 42int64
    store v1, v2
    v3: int64 = load v1
    return v3
}"#;
        let expected = r#"
type Inner {
    int32;
    int32;
}
type Outer {
    Inner;
    int64;
}
function test(): int64 {
b0:
    v0: ref<int64, raw, space(stack)> = stack.alloc int64
    v1: ref<Inner, raw, space(stack)> = stack.alloc Inner
    v2: int64 = 42int64
    store v0, v2
    v3: int64 = load v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_output(expected);
    }

    /// No changes when no aggregates.
    ///
    /// Scalar allocations are not affected by SROA.
    #[test]
    fn test_no_aggregates() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_unchanged(input);
    }

    /// Allocation escaping via store is not split.
    ///
    /// When a pointer to the allocation is stored to memory, it escapes
    /// and cannot be safely split.
    #[test]
    fn test_preserve_escaping_via_store() {
        let input = r#"
type Point {
    int32;
    int32;
}
function test(v0: ref<ref<Point, raw>, raw>): void {
b0(v0: ref<ref<Point, raw>, raw>):
    v1: ref<Point, raw, space(stack)> = stack.alloc Point
    store v0, v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_unchanged(input);
    }

    /// Single-field struct can be split.
    ///
    /// Even a single-field aggregate benefits from SROA since it allows
    /// mem2reg to promote the value to SSA.
    #[test]
    fn test_split_single_field() {
        let input = r#"
type Wrapper {
    int32;
}
function test(): int32 {
b0:
    v0: ref<Wrapper, raw, space(stack)> = stack.alloc Wrapper
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: int32 = 42int32
    store v1, v2
    v3: int32 = load v1
    return v3
}"#;
        let expected = r#"
type Wrapper {
    int32;
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_output(expected);
    }

    /// Block argument escaping prevents splitting.
    ///
    /// If the allocation is passed as a block argument, it escapes.
    #[test]
    fn test_preserve_block_argument_escape() {
        let input = r#"
type Point {
    int32;
    int32;
}
function test(v0: boolean): void {
b0(v0: boolean):
    v1: ref<Point, raw, space(stack)> = stack.alloc Point
    branch v0, b1(v1), b2
b1(v2: ref<Point, raw>):
    return
b2:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_unchanged(input);
    }

    /// Constant propagation resolves array indices across block parameters.
    #[test]
    fn test_split_array_constant_param_index() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int64 = 0int64
    branch v0, b1(v1), b1(v1)
b1(v2: int64):
    v3: ref<int32[2], raw, space(stack)> = stack.alloc int32[2]
    v4: ref<int32, borrowed> = element.address v3, v2
    v5: int32 = 42int32
    store v4, v5
    v6: int32 = load v4
    return v6
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: int64 = 0int64
    branch v0, b1(v3), b1(v3)
b1(v4: int64):
    v5: int32 = 42int32
    store v2, v5
    v6: int32 = load v2
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_output(expected);
    }

    /// Base pointer loads and stores are rebuilt from scalar slots.
    #[test]
    fn test_preserve_base_pointer_load_store() {
        let input = r#"
type Point {
    int32;
    int32;
}
function test(): int32 {
b0:
    v0: ref<Point, raw, space(stack)> = stack.alloc Point
    v1: int32 = 1int32
    v2: int32 = 2int32
    v3: Point = struct Point (v1, v2)
    store v0, v3
    v4: Point = load v0
    v5: int32 = field.get v4, 0
    return v5
}"#;
        let expected = r#"
type Point {
    int32;
    int32;
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 1int32
    v3: int32 = 2int32
    v4: Point = struct Point (v2, v3)
    v5: int32 = field.get v4, 0
    store v1, v5
    v6: int32 = field.get v4, 1
    store v0, v6
    v7: int32 = load v1
    v8: int32 = load v0
    v9: Point = struct Point (v7, v8)
    v10: int32 = field.get v9, 0
    return v10
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_output(expected);
    }

    /// Base pointer array loads and stores are rebuilt from scalar slots.
    #[test]
    fn test_rewrite_base_pointer_array_load_store() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32[2], raw, space(stack)> = stack.alloc int32[2]
    v1: int32 = 10int32
    v2: int32 = 20int32
    v3: int32[2] = array int32[2] (v1, v2)
    store v0, v3
    v4: int32[2] = load v0
    v5: int64 = 1int64
    v6: int32 = element.get v4, v5
    return v6
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 10int32
    v3: int32 = 20int32
    v4: int32[2] = array int32[2] (v2, v3)
    v5: int64 = 0int64
    v6: int32 = element.get v4, v5
    store v1, v6
    v7: int64 = 1int64
    v8: int32 = element.get v4, v7
    store v0, v8
    v9: int32 = load v1
    v10: int32 = load v0
    v11: int32[2] = array int32[2] (v9, v10)
    v12: int64 = 1int64
    v13: int32 = element.get v11, v12
    return v13
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Sroa);
        test.assert_output(expected);
    }

    /// Volatile loads prevent splitting.
    #[test]
    fn test_skip_volatile_load() {
        let input = r#"
type Point {
    int32;
    int32;
}
function test(): int32 {
b0:
    v0: ref<Point, raw, space(stack)> = stack.alloc Point
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: int32 = load v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.entry_function_id();
        let load_id = first_load_id(&test, function_id);
        test.insert_pointer_access_with_options(
            load_id,
            mir::MemoryAccessKind::Read,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            None,
            true,
            None,
        );

        test.run_pass(&Sroa);
        test.assert_unchanged(input);
    }
}

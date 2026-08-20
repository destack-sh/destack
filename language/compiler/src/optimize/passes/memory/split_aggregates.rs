use destack_core::{FxIndexMap, FxIndexSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    ConstantTable, Mutation, instruction_substitute_uses_in_tree,
    remap_instruction_memory_accesses, terminator_substitute_uses,
};

declare_pass! {
    /// Scalar Replacement of Aggregates.
    ///
    /// ```mir
    /// function before(): int32 {
    ///     local l0: { int32, int32 }
    /// b0:
    ///     v0: ref<{ int32, int32 }, borrowed, mutable, frame> = local.address l0
    ///     v1: ref<int32, borrowed, mutable, frame> = field.address v0, 0
    ///     v2: int32 = 1
    ///     store v1, v2
    ///     v3: ref<int32, borrowed, mutable, frame> = field.address v0, 1
    ///     v4: int32 = 2
    ///     store v3, v4
    ///     v5: int32 = load v1
    ///     return v5
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(): int32 {
    ///     local l0: int32
    ///     local l1: int32
    /// b0:
    ///     v0: ref<int32, borrowed, mutable, frame> = local.address l0
    ///     v1: ref<int32, borrowed, mutable, frame> = local.address l1
    ///     v2: int32 = 1
    ///     store v0, v2
    ///     v3: int32 = 2
    ///     store v1, v3
    ///     v4: int32 = load v0
    ///     return v4
    /// }
    /// ```
    #[pass(id = "split-aggregates")]
    pub SplitAggregates,
    "Break aggregates into scalars"
}

impl FunctionPass for SplitAggregates {
    /// Run scalar replacement of aggregates on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        analyses: &mut mir::FunctionCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let layouts = &mut optimized.layouts;
        let accesses = &mut optimized.accesses;

        // skip empty functions
        let entry = match function.entry() {
            Some(entry) => entry,
            None => return Mutation::NONE,
        };

        // get constant propagation analysis
        let constants = { analyses.constant(function, tree).clone() };

        // run SROA
        let changed = run_split_aggregates(
            function,
            tree,
            layouts,
            accesses,
            entry,
            ctx.options.split_aggregates_max_array_elements,
            &constants,
        );

        // report what this pass changed
        if changed {
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }
}

/// Core SROA logic. Returns true if changes were made.
fn run_split_aggregates(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    layouts: &mut mir::LayoutTable,
    accesses: &mut mir::AccessTable,
    entry: mir::LocalNodeId<mir::Block>,
    max_array_elements: usize,
    constants: &ConstantTable,
) -> bool {
    // find splittable locals
    let candidates = find_candidates(function, tree, accesses, max_array_elements, constants);
    if candidates.is_empty() {
        return false;
    }

    // ensure next_value_id is correct before allocating new values
    function.recompute_next_value_id(tree);

    // split each candidate
    let mut made_changes = false;
    for candidate in candidates {
        if split_local(&candidate, function, tree, layouts, accesses, entry) {
            made_changes = true;
        }
    }

    made_changes
}

/// An addressable aggregate local that can be split.
struct SplitCandidate {
    /// The aggregate local.
    local: mir::LocalNodeId<mir::Local>,
    /// The instruction materializing the local address.
    address_instruction: mir::LocalNodeId<mir::Instruction>,
    /// The aggregate layout type.
    layout: mir::LocalNodeId<mir::Type>,
    /// The original local reference type.
    result_type: mir::LocalNodeId<mir::Type>,
    /// Reference tables for derived stack slots.
    reference_spec: ReferenceSpec,
    /// The element types after splitting.
    element_types: Vec<mir::LocalNodeId<mir::Type>>,
    /// Uses of the local address.
    uses: Vec<UseInfo>,
    /// Loads performed directly on the base reference.
    base_loads: Vec<mir::LocalNodeId<mir::Instruction>>,
    /// Stores performed directly on the base reference.
    base_stores: Vec<mir::LocalNodeId<mir::Instruction>>,
}

/// Reference attributes used to rebuild stack slot types.
#[derive(Clone)]
struct ReferenceSpec {
    /// The reference kind.
    kind: mir::ReferenceKind,
    /// The storage for the reference.
    storage: mir::Storage,
    /// The access for the reference.
    access: mir::Access,
    /// The nullability for the reference.
    nullability: mir::Nullability,
}

impl ReferenceSpec {
    /// Extract reference attributes from a reference type.
    fn from_type(ty: &mir::Type) -> Option<Self> {
        let mir::Type::Reference {
            kind,
            storage,
            access,
            nullability,
            ..
        } = ty
        else {
            return None;
        };

        Some(Self {
            kind: *kind,
            storage: *storage,
            access: *access,
            nullability: *nullability,
        })
    }
}

/// One projected use of a local address.
#[derive(Clone)]
struct UseInfo {
    /// The projection instruction.
    instruction: mir::LocalNodeId<mir::Instruction>,
    /// The field/element index being accessed.
    index: usize,
    /// The destination value (the address).
    destination: mir::Value,
}

/// Uses of one aggregate local address.
struct LocalUses {
    /// Field or element address uses.
    uses: Vec<UseInfo>,
    /// Base pointer loads.
    base_loads: Vec<mir::LocalNodeId<mir::Instruction>>,
    /// Base pointer stores.
    base_stores: Vec<mir::LocalNodeId<mir::Instruction>>,
}

/// Find addressable aggregate locals that can be split into scalars.
fn find_candidates(
    function: &mir::Function,
    tree: &mir::Tree,
    accesses: &mir::AccessTable,
    max_array_elements: usize,
    constants: &ConstantTable,
) -> Vec<SplitCandidate> {
    let mut candidates = Vec::new();

    // count address materializations per local
    let block_ids = function.blocks().to_vec();
    let mut address_counts = FxIndexMap::default();
    for &block_id in &block_ids {
        for &instruction_id in &tree.get(block_id).instructions {
            if let mir::Instruction::LocalAddr { local, .. } = tree.get(instruction_id) {
                *address_counts.entry(*local).or_insert(0usize) += 1;
            }
        }
    }

    // collect aggregate locals with one canonical address
    for &block_id in &block_ids {
        let block = tree.get(block_id).clone();

        for &inst_id in &block.instructions {
            let inst = tree.get(inst_id);

            if let mir::Instruction::LocalAddr {
                destination,
                local,
                result_type,
            } = inst
            {
                if address_counts.get(local) != Some(&1) {
                    continue;
                }

                let result_type = *result_type;
                let local = *local;
                let layout = tree.get(local).ty;
                let destination = *destination;

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

                // reject escaping or unsupported address uses
                let uses = match analyze_uses(destination, function, tree, accesses, constants) {
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
                    local,
                    address_instruction: inst_id,
                    layout,
                    result_type,
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
                    field.ty
                })
                .collect();

            Some(types)
        }

        mir::Type::Tuple { elements, copy: _ } => {
            // collect element types without recursive flattening
            Some(elements.to_vec())
        }

        mir::Type::FixedArray {
            element,
            length,
            copy: _,
        } => {
            // only split small arrays
            if *length as usize > max_array_elements {
                return None;
            }

            // create element types for each array element
            let types = vec![*element; *length as usize];

            Some(types)
        }

        // not an aggregate type
        _ => None,
    }
}

/// Analyze uses of a local address to determine whether its local can be split.
///
/// Returns `None` if the address escapes or has unsupported uses.
fn analyze_uses(
    local_address: mir::Value,
    function: &mir::Function,
    tree: &mir::Tree,
    accesses: &mir::AccessTable,
    constants: &ConstantTable,
) -> Option<LocalUses> {
    let mut uses = Vec::new();
    let mut base_loads = Vec::new();
    let mut base_stores = Vec::new();
    let mut seen_values: FxIndexSet<mir::Value> = FxIndexSet::default();
    let mut worklist: Vec<mir::Value> = vec![local_address];

    while let Some(value) = worklist.pop() {
        if !seen_values.insert(value) {
            continue;
        }

        // find all uses of this value
        for &block_id in function.blocks() {
            let block = tree.get(block_id);

            for &inst_id in &block.instructions {
                let inst = tree.get(inst_id);

                match inst {
                    // follow field addresses
                    mir::Instruction::FieldAddr {
                        destination,
                        aggregate,
                        field,
                        ..
                    } if *aggregate == value => {
                        uses.push(UseInfo {
                            instruction: inst_id,
                            index: *field as usize,
                            destination: *destination,
                        });

                        // the field address itself might be used
                        worklist.push(*destination);
                    }

                    // follow statically indexed element addresses
                    mir::Instruction::ElementAddr {
                        destination,
                        base,
                        index,
                        ..
                    } if *base == value => {
                        // check if index is a constant
                        let const_index = resolve_constant_index(*index, block_id, constants)?;

                        uses.push(UseInfo {
                            instruction: inst_id,
                            index: const_index,
                            destination: *destination,
                        });

                        // the element address itself might be used
                        worklist.push(*destination);
                    }

                    // loads and stores are allowed, base reference uses are recorded
                    mir::Instruction::Load { pointer, .. } if *pointer == value => {
                        if accesses.requires_exact_position(inst_id, tree) {
                            return None;
                        }
                        if value == local_address {
                            base_loads.push(inst_id);
                        }
                    }

                    mir::Instruction::Store { pointer, .. } if *pointer == value => {
                        if accesses.requires_exact_position(inst_id, tree) {
                            return None;
                        }
                        if value == local_address {
                            base_stores.push(inst_id);
                        }
                    }

                    // calls: check if value is passed as argument (escapes)
                    mir::Instruction::Call { .. } => {
                        // arguments are stored externally, access via argument_slice
                        if let Some(arg_slice) = inst.argument_slice() {
                            for &arg in tree.get_values(arg_slice) {
                                if arg == value {
                                    // value escapes through call
                                    return None;
                                }
                            }
                        }
                    }

                    // reject every other use of the local address
                    _ => {
                        if inst.uses().into_iter().any(|used| used == value) {
                            // this value escapes, can't split
                            return None;
                        }
                    }
                }
            }

            // check terminator uses
            let terminator = tree.get(block.terminator);
            if terminator.uses(tree).contains(&value) {
                // value used in terminator, escapes
                return None;
            }
        }
    }

    Some(LocalUses {
        uses,
        base_loads,
        base_stores,
    })
}

/// Resolve a constant integer index from constant propagation.
fn resolve_constant_index(
    value: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    constants: &ConstantTable,
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

/// Split one aggregate local into individual scalar locals.
fn split_local(
    candidate: &SplitCandidate,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    layouts: &mut mir::LayoutTable,
    accesses: &mut mir::AccessTable,
    entry: mir::LocalNodeId<mir::Block>,
) -> bool {
    // create scalar locals and their addresses
    let mut addresses: Vec<mir::Value> = Vec::new();
    let mut address_instructions: Vec<mir::LocalNodeId<mir::Instruction>> = Vec::new();

    for &elem_type in &candidate.element_types {
        let local = tree.insert(mir::Local::mutable(elem_type));
        function.add_local(local);

        let result_type = tree.intern_type(mir::Type::Reference {
            kind: candidate.reference_spec.kind,
            lifetime: mir::Lifetime::empty(),
            storage: candidate.reference_spec.storage,
            access: candidate.reference_spec.access,
            pointee: elem_type,
            nullability: candidate.reference_spec.nullability,
        });
        layouts.copy_type_entries(candidate.result_type, result_type);
        let new_value = function.next_typed_value(result_type);
        addresses.push(new_value);

        // materialize the scalar local address
        let new_inst = mir::Instruction::LocalAddr {
            destination: new_value,
            local,
            result_type,
        };

        // materialize each scalar address at function entry
        let new_inst_id = tree.insert(new_inst);
        address_instructions.push(new_inst_id);
    }

    // prepend replacement addresses to the entry block
    if !address_instructions.is_empty() {
        let entry_block = tree.get(entry);
        let mut entry_instructions = address_instructions;
        entry_instructions.extend(entry_block.instructions.iter().copied());
        function.replace_block_instructions(entry, entry_instructions, tree);
    }

    // build mapping from field/element index to new value
    let index_to_value: FxIndexMap<usize, mir::Value> =
        addresses.iter().enumerate().map(|(i, &v)| (i, v)).collect();

    // replace projections with scalar local addresses
    let mut substitutions: FxIndexMap<mir::Value, mir::Value> = FxIndexMap::default();

    for use_info in &candidate.uses {
        if let Some(&new_value) = index_to_value.get(&use_info.index) {
            substitutions.insert(use_info.destination, new_value);
        }
    }

    // apply substitutions to all instructions
    apply_substitutions(&substitutions, function, tree, accesses);

    // remove the original local address and its derived projections
    let mut to_remove: FxIndexSet<mir::LocalNodeId<mir::Instruction>> = FxIndexSet::default();
    to_remove.insert(candidate.address_instruction);

    for use_info in &candidate.uses {
        to_remove.insert(use_info.instruction);
    }

    // rewrite base reference loads and stores
    let base_loads: FxIndexSet<_> = candidate.base_loads.iter().copied().collect();
    let base_stores: FxIndexSet<_> = candidate.base_stores.iter().copied().collect();

    // rewrite instructions per block
    let block_ids = function.blocks().to_vec();
    for &block_id in &block_ids {
        let instruction_ids = {
            let block = tree.get(block_id);
            block.instructions.clone()
        };

        let mut new_instructions: Vec<mir::LocalNodeId<mir::Instruction>> = Vec::new();

        for instruction_id in instruction_ids {
            // rewrite base reference loads into scalar loads
            if base_loads.contains(&instruction_id) {
                let mut rewritten =
                    rewrite_base_load(candidate, function, tree, &index_to_value, instruction_id);
                new_instructions.append(&mut rewritten);
                continue;
            }

            // rewrite base reference stores into scalar stores
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

        if tree.get(block_id).instructions != new_instructions {
            function.replace_block_instructions(block_id, new_instructions, tree);
        }
    }

    // remove the replaced aggregate local
    function.retain_locals(|local| local != candidate.local);

    true
}

/// Rewrite a base reference load into scalar loads and aggregate rebuild.
fn rewrite_base_load(
    candidate: &SplitCandidate,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    index_to_value: &FxIndexMap<usize, mir::Value>,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
) -> Vec<mir::LocalNodeId<mir::Instruction>> {
    // extract load destination
    let (destination, _pointer) = match tree.get(instruction_id) {
        mir::Instruction::Load {
            destination,
            pointer,
            ..
        } => (*destination, *pointer),
        _ => panic!("split-aggregates base load rewrite expects a load instruction"),
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
            destination: element_value,
            pointer: element_pointer,
            result_type: element_type,
        };
        let load_id = tree.insert(load_inst);
        new_instructions.push(load_id);
        element_values.push(element_value);
    }

    // rebuild the aggregate value from the loaded elements
    let aggregate_inst = build_aggregate_instruction(tree, destination, &element_values);
    let aggregate_id = tree.insert(aggregate_inst);
    new_instructions.push(aggregate_id);

    new_instructions
}

/// Rewrite a base reference store into aggregate decompositions and scalar stores.
fn rewrite_base_store(
    candidate: &SplitCandidate,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    index_to_value: &FxIndexMap<usize, mir::Value>,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
) -> Vec<mir::LocalNodeId<mir::Instruction>> {
    // extract stored value
    let stored_value = match tree.get(instruction_id) {
        mir::Instruction::Store { value, .. } => *value,
        _ => panic!("split-aggregates base store rewrite expects a store instruction"),
    };

    let mut new_instructions: Vec<mir::LocalNodeId<mir::Instruction>> = Vec::new();
    for index in 0..candidate.element_types.len() {
        let element_pointer = index_to_value
            .get(&index)
            .copied()
            .expect("missing scalar slot for aggregate element");
        let element_type = candidate.element_types[index];
        let element_value = function.next_typed_value(element_type);

        // extract the static aggregate slot
        let slot_get = if matches!(tree.get(candidate.layout), mir::Type::FixedArray { .. }) {
            mir::Instruction::ElementGet {
                destination: element_value,
                aggregate: stored_value,
                index: index as u32,
            }
        } else {
            mir::Instruction::FieldGet {
                destination: element_value,
                aggregate: stored_value,
                field: index as u32,
            }
        };
        let slot_get = tree.insert(slot_get);
        new_instructions.push(slot_get);

        // store the scalar into its local
        let store_inst = mir::Instruction::Store {
            pointer: element_pointer,
            value: element_value,
        };
        let store_id = tree.insert(store_inst);
        new_instructions.push(store_id);
    }

    new_instructions
}

/// Build an aggregate construction instruction for the given layout.
fn build_aggregate_instruction(
    tree: &mut mir::Tree,
    destination: mir::Value,
    element_values: &[mir::Value],
) -> mir::Instruction {
    // prepare aggregate arguments and layout
    let arguments: Vec<_> = element_values.to_vec();
    let arguments = tree.add_values(&arguments);
    mir::Instruction::Aggregate {
        destination,
        values: arguments,
    }
}

/// Apply value substitutions to all instructions and terminators.
fn apply_substitutions(
    substitutions: &FxIndexMap<mir::Value, mir::Value>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
) {
    if substitutions.is_empty() {
        return;
    }

    for &block_id in function.blocks() {
        // substitute in instructions
        let instruction_ids = {
            let block = tree.get(block_id);
            block.instructions.clone()
        };

        for inst_id in instruction_ids {
            let instruction = tree.get(inst_id).clone();
            let new_instruction =
                instruction_substitute_uses_in_tree(&instruction, substitutions, tree);
            tree.set(inst_id, new_instruction);
            remap_instruction_memory_accesses(accesses, inst_id, substitutions);
        }

        // substitute in terminator
        let terminator_id = tree.get(block_id).terminator;
        let terminator = tree.get(terminator_id).clone();
        let new_terminator = terminator_substitute_uses(tree, &terminator, substitutions);
        tree.set(terminator_id, new_terminator);
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
        let function = test.optimized.tree.get(function_id);
        for block_id in function.blocks() {
            let block = test.optimized.tree.get(*block_id);
            for &instruction_id in &block.instructions {
                if matches!(
                    test.optimized.tree.get(instruction_id),
                    mir::Instruction::Load { .. }
                ) {
                    return instruction_id;
                }
            }
        }

        panic!("missing load instruction");
    }

    /// Simple struct splitting.
    ///
    /// The struct local is split into one local per field.
    /// Field addresses become direct references to the scalar locals.
    #[test]
    fn test_split_struct() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(): int32 {
    local l0: Point
entry:
    v0: ref<Point, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: int32 = 42
    store v1, v2
    v3: int32 = load v1
    return v3
}
"#;
        let expected = r#"
type Point {
    int32;
    int32;
}

function test(): int32 {
    local l0: int32
    local l1: int32

entry:
    v4: ref<int32, borrowed, mutable, frame> = local.address l0
    v5: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int32 = 42
    store v4, v2
    v3: int32 = load v4
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
        test.assert_output(expected);
    }

    /// Tuple splitting.
    ///
    /// Tuples are split like structs, with one local per element.
    #[test]
    fn test_split_tuple() {
        let input = r#"
function test(): int32 {
    local l0: (int32, int64)
entry:
    v0: ref<(int32, int64), borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: int32 = 42
    store v1, v2
    v3: int32 = load v1
    return v3
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32
    local l1: int64

entry:
    v4: ref<int32, borrowed, mutable, frame> = local.address l0
    v5: ref<int64, borrowed, mutable, frame> = local.address l1
    v2: int32 = 42
    store v4, v2
    v3: int32 = load v4
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
        test.assert_output(expected);
    }

    /// Small array splitting.
    ///
    /// Arrays with constant indices are split into one local per element.
    #[test]
    fn test_split_small_array() {
        let input = r#"
function test(): int32 {
    local l0: [int32; 4]
entry:
    v0: ref<[int32; 4], borrowed, mutable, frame> = local.address l0
    v1: int64 = 0
    v2: ref<int32, borrowed, mutable> = element.address v0, v1
    v3: int32 = 42
    store v2, v3
    v4: int32 = load v2
    return v4
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32
    local l1: int32
    local l2: int32
    local l3: int32

entry:
    v5: ref<int32, borrowed, mutable, frame> = local.address l0
    v6: ref<int32, borrowed, mutable, frame> = local.address l1
    v7: ref<int32, borrowed, mutable, frame> = local.address l2
    v8: ref<int32, borrowed, mutable, frame> = local.address l3
    v1: int64 = 0
    v3: int32 = 42
    store v5, v3
    v4: int32 = load v5
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
        test.assert_output(expected);
    }

    /// Array too large to split.
    ///
    /// Arrays exceeding the max element threshold are not split.
    #[test]
    fn test_preserve_large_array() {
        let input = r#"
function test(): int32 {
    local l0: [int32; 100]
entry:
    v0: ref<[int32; 100], borrowed, mutable, frame> = local.address l0
    v1: int64 = 0
    v2: ref<int32, borrowed, mutable> = element.address v0, v1
    v3: int32 = 42
    store v2, v3
    v4: int32 = load v2
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
        test.assert_unchanged(input);
    }

    /// An escaping local address is not split.
    ///
    /// Passing the local address to an external function makes it escape.
    /// and cannot be split.
    #[test]
    fn test_preserve_escaping() {
        let input = r#"
type Point {
    int32;
    int32;
}

external function imported(ref<Point, borrowed, mutable>): void

function test(): void {
    local l0: Point
entry:
    v0: ref<Point, borrowed, mutable, frame> = local.address l0
    call imported(v0): (ref<Point, borrowed, mutable>) => void
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
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
    local l0: [int32; 4]
entry(v0: int64):
    v1: ref<[int32; 4], borrowed, mutable, frame> = local.address l0
    v2: ref<int32, borrowed, mutable> = element.address v1, v0
    v3: int32 = 42
    store v2, v3
    v4: int32 = load v2
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
        test.assert_unchanged(input);
    }

    /// Multiple fields accessed.
    ///
    /// When multiple fields of a struct are accessed, all field.address
    /// instructions are replaced with the corresponding scalar addresses.
    #[test]
    fn test_multiple_fields() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(): int32 {
    local l0: Point
entry:
    v0: ref<Point, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: int32 = 10
    store v1, v2
    v3: ref<int32, borrowed, mutable> = field.address v0, 1
    v4: int32 = 20
    store v3, v4
    v5: int32 = load v1
    v6: int32 = load v3
    v7: int32 = add v5, v6
    return v7
}
"#;
        let expected = r#"
type Point {
    int32;
    int32;
}

function test(): int32 {
    local l0: int32
    local l1: int32

entry:
    v8: ref<int32, borrowed, mutable, frame> = local.address l0
    v9: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int32 = 10
    store v8, v2
    v4: int32 = 20
    store v9, v4
    v5: int32 = load v8
    v6: int32 = load v9
    v7: int32 = add v5, v6
    return v7
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
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
    local l0: Outer
entry:
    v0: ref<Outer, borrowed, mutable, frame> = local.address l0
    v1: ref<int64, borrowed, mutable> = field.address v0, 1
    v2: int64 = 42
    store v1, v2
    v3: int64 = load v1
    return v3
}
"#;
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
    local l0: Inner
    local l1: int64

entry:
    v4: ref<Inner, borrowed, mutable, frame> = local.address l0
    v5: ref<int64, borrowed, mutable, frame> = local.address l1
    v2: int64 = 42
    store v5, v2
    v3: int64 = load v5
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
        test.assert_output(expected);
    }

    /// No changes when no aggregates.
    ///
    /// Scalar locals are not affected by SROA.
    #[test]
    fn test_no_aggregates() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    v2: int32 = load v0
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
        test.assert_unchanged(input);
    }

    /// A local address escaping through a store is not split.
    ///
    /// Storing a local address elsewhere makes it escape.
    #[test]
    fn test_preserve_escaping_via_store() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(v0: ref<ref<Point, borrowed, mutable>, borrowed, mutable>): void {
    local l0: Point
entry(v0: ref<ref<Point, borrowed, mutable>, borrowed, mutable>):
    v1: ref<Point, borrowed, mutable, frame> = local.address l0
    store v0, v1
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
        test.assert_unchanged(input);
    }

    /// Single-field struct can be split.
    ///
    /// Even a single-field aggregate benefits from SROA since it allows
    /// promote-memory-to-registers to promote the value to SSA.
    #[test]
    fn test_split_single_field() {
        let input = r#"
type Wrapper {
    int32;
}

function test(): int32 {
    local l0: Wrapper
entry:
    v0: ref<Wrapper, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: int32 = 42
    store v1, v2
    v3: int32 = load v1
    return v3
}
"#;
        let expected = r#"
type Wrapper {
    int32;
}

function test(): int32 {
    local l0: int32

entry:
    v4: ref<int32, borrowed, mutable, frame> = local.address l0
    v2: int32 = 42
    store v4, v2
    v3: int32 = load v4
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
        test.assert_output(expected);
    }

    /// Block argument escaping prevents splitting.
    ///
    /// Passing the local address as a block argument makes it escape.
    #[test]
    fn test_preserve_block_argument_escape() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(v0: boolean): void {
    local l0: Point
entry(v0: boolean):
    v1: ref<Point, borrowed, mutable, frame> = local.address l0
    branch v0 => b1(v1) | b2

b1(v2: ref<Point, borrowed, mutable>):
    return

b2:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
        test.assert_unchanged(input);
    }

    /// Constant propagation resolves array indices across block parameters.
    #[test]
    fn test_split_array_constant_param_index() {
        let input = r#"
function test(v0: boolean): int32 {
    local l0: [int32; 2]
entry(v0: boolean):
    v1: int64 = 0
    branch v0 => b1(v1) | b1(v1)

b1(v2: int64):
    v3: ref<[int32; 2], borrowed, mutable, frame> = local.address l0
    v4: ref<int32, borrowed, mutable> = element.address v3, v2
    v5: int32 = 42
    store v4, v5
    v6: int32 = load v4
    return v6
}
"#;
        let expected = r#"
function test(v0: boolean): int32 {
    local l0: int32
    local l1: int32

entry(v0: boolean):
    v7: ref<int32, borrowed, mutable, frame> = local.address l0
    v8: ref<int32, borrowed, mutable, frame> = local.address l1
    v1: int64 = 0
    branch v0 => b1(v1) | b1(v1)

b1(v2: int64):
    v5: int32 = 42
    store v7, v5
    v6: int32 = load v7
    return v6
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
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
    local l0: Point
entry:
    v0: ref<Point, borrowed, mutable, frame> = local.address l0
    v1: int32 = 1
    v2: int32 = 2
    v3: Point = aggregate (v1, v2)
    store v0, v3
    v4: Point = load v0
    v5: int32 = field.get v4, 0
    return v5
}
"#;
        let expected = r#"
type Point {
    int32;
    int32;
}

function test(): int32 {
    local l0: int32
    local l1: int32

entry:
    v6: ref<int32, borrowed, mutable, frame> = local.address l0
    v7: ref<int32, borrowed, mutable, frame> = local.address l1
    v1: int32 = 1
    v2: int32 = 2
    v3: Point = aggregate (v1, v2)
    v8: int32 = field.get v3, 0
    store v6, v8
    v9: int32 = field.get v3, 1
    store v7, v9
    v10: int32 = load v6
    v11: int32 = load v7
    v4: Point = aggregate (v10, v11)
    v5: int32 = field.get v4, 0
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
        test.assert_output(expected);
    }

    /// Base pointer array loads and stores are rebuilt from scalar slots.
    #[test]
    fn test_rewrite_base_pointer_array_load_store() {
        let input = r#"
function test(): int32 {
    local l0: [int32; 2]
entry:
    v0: ref<[int32; 2], borrowed, mutable, frame> = local.address l0
    v1: int32 = 10
    v2: int32 = 20
    v3: [int32; 2] = aggregate (v1, v2)
    store v0, v3
    v4: [int32; 2] = load v0
    v5: int64 = 1
    v6: int32 = element.get v4, 1
    return v6
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32
    local l1: int32

entry:
    v7: ref<int32, borrowed, mutable, frame> = local.address l0
    v8: ref<int32, borrowed, mutable, frame> = local.address l1
    v1: int32 = 10
    v2: int32 = 20
    v3: [int32; 2] = aggregate (v1, v2)
    v9: int32 = element.get v3, 0
    store v7, v9
    v10: int32 = element.get v3, 1
    store v8, v10
    v11: int32 = load v7
    v12: int32 = load v8
    v4: [int32; 2] = aggregate (v11, v12)
    v5: int64 = 1
    v6: int32 = element.get v4, 1
    return v6
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SplitAggregates);
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
    local l0: Point
entry:
    v0: ref<Point, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: int32 = load v1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        let function_id = test.entry_function_id();
        let load_id = first_load_id(&test, function_id);
        test.insert_pointer_access_with_options(
            load_id,
            mir::MemoryOperation::Read,
            mir::Value::new(1),
            Some(4),
            true,
            None,
        );

        test.run_pass(&SplitAggregates);
        test.assert_unchanged(input);
    }
}

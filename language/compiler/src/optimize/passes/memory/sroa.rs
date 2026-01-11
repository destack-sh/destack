use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::ConstantPropagation;
use crate::optimize::{
    AnalysisPreservation, FunctionAnalyses, FunctionPass, PipelineContext,
    terminator_substitute_uses, terminator_uses,
};

// TODO #Architecture: rebuild aggregate loads and stores when base pointers are used

declare_pass! {
    /// Scalar Replacement of Aggregates.
    ///
    /// Breaks apart aggregate stack allocations (structs, tuples, small arrays)
    /// into individual scalar allocations. This enables mem2reg to promote
    /// each scalar to an SSA value.
    ///
    /// ```mir
    /// // before SROA
    /// function @before() -> i32 {
    /// block0:
    ///     v0 = stack.alloc { i32, i32 }
    ///     v1 = field.addr v0, 0
    ///     v2 = iconst 1i32
    ///     store v1, v2
    ///     v3 = field.addr v0, 1
    ///     v4 = iconst 2i32
    ///     store v3, v4
    ///     v5 = load v1
    ///     return v5
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// // after SROA
    /// function @after() -> i32 {
    /// block0:
    ///     v0 = stack.alloc i32  // field 0
    ///     v1 = stack.alloc i32  // field 1
    ///     v2 = iconst 1i32
    ///     store v0, v2
    ///     v3 = iconst 2i32
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
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip empty functions
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get constant propagation analysis
        let constants = {
            let analyses = FunctionAnalyses::new(function, tree);
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
    tree: &mut mir::NodeTree,
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
    /// The element types after splitting.
    element_types: Vec<mir::LocalNodeId<mir::Type>>,
    /// Uses of the allocation (field/element addresses).
    uses: Vec<UseInfo>,
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

/// Find stack allocations that can be split into scalars.
fn find_splittable_allocations_core(
    function: &mir::Function,
    tree: &mir::NodeTree,
    max_array_elements: usize,
    constants: &ConstantPropagation,
) -> Vec<SplitCandidate> {
    let mut candidates = Vec::new();

    // collect all stack allocations of aggregate types
    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        for &inst_id in &block.instructions {
            let inst = tree.get(inst_id);

            if let mir::Instruction::StackAlloc {
                destination,
                layout,
            } = inst
            {
                let ty = tree.get(*layout);

                // check if this is a splittable aggregate type
                let element_types = match get_element_types(ty, tree, max_array_elements) {
                    Some(types) => types,
                    None => continue,
                };

                // analyze uses to determine if splittable
                let uses = match analyze_uses(*destination, function, tree, constants) {
                    Some(uses) => uses,
                    None => continue,
                };
                if uses
                    .iter()
                    .any(|use_info| use_info.index >= element_types.len())
                {
                    continue;
                }

                candidates.push(SplitCandidate {
                    alloc_instruction: inst_id,
                    element_types,
                    uses,
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
    tree: &mir::NodeTree,
    max_array_elements: usize,
) -> Option<Vec<mir::LocalNodeId<mir::Type>>> {
    match ty {
        mir::Type::Struct {
            fields,
            copyability: _,
        } => {
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

        mir::Type::Tuple {
            elements,
            copyability: _,
        } => {
            // collect element types without recursive flattening
            Some(elements.clone())
        }

        mir::Type::Array {
            element,
            length,
            copyability: _,
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

/// Analyze uses of an allocation to determine if it can be split.
///
/// Returns None if the allocation escapes or has unsupported uses.
fn analyze_uses(
    alloc_value: mir::Value,
    function: &mir::Function,
    tree: &mir::NodeTree,
    constants: &ConstantPropagation,
) -> Option<Vec<UseInfo>> {
    let mut uses = Vec::new();
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
                    } if *aggregate == value => {
                        uses.push(UseInfo {
                            instruction: inst_id,
                            index: *index as usize,
                            destination: *destination,
                        });

                        // the field address itself might be used
                        worklist.push(*destination);
                    }

                    // element address: supported if index is constant
                    mir::Instruction::ElementAddr {
                        destination,
                        array,
                        index,
                    } if *array == value => {
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

                    // load and store are fine (they use the derived address, not the base)
                    mir::Instruction::Load { pointer, .. } if *pointer == value => {
                        if value == alloc_value {
                            return None;
                        }
                    }

                    mir::Instruction::Store { pointer, .. } if *pointer == value => {
                        if value == alloc_value {
                            return None;
                        }
                    }

                    // calls: check if value is passed as argument (escapes)
                    mir::Instruction::Call { .. } | mir::Instruction::CallIndirect { .. } => {
                        // arguments are stored externally, access via argument_slice
                        if let Some(arg_slice) = inst.argument_slice() {
                            for &arg in tree.get_arguments(arg_slice) {
                                if arg == value {
                                    // value escapes through call
                                    return None;
                                }
                            }
                        }
                    }

                    // any other use of the allocation value escapes
                    _ => {
                        if inst.uses().contains(&value) {
                            // this value escapes, can't split
                            return None;
                        }
                    }
                }
            }

            // check terminator uses
            if terminator_uses(&block.terminator, value) {
                // value used in terminator, escapes
                return None;
            }
        }
    }

    Some(uses)
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
    tree: &mut mir::NodeTree,
    entry: mir::LocalNodeId<mir::Block>,
) -> bool {
    // create new allocations for each element
    let mut new_allocs: Vec<mir::Value> = Vec::new();

    for &elem_type in &candidate.element_types {
        let new_value = function.next_value();
        new_allocs.push(new_value);

        // create the new StackAlloc instruction
        let new_inst = mir::Instruction::StackAlloc {
            destination: new_value,
            layout: elem_type,
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

    // remove the original allocation and field/element address instructions
    let mut to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();
    to_remove.insert(candidate.alloc_instruction);

    for use_info in &candidate.uses {
        to_remove.insert(use_info.instruction);
    }

    // remove instructions from blocks
    for &block_id in &function.blocks {
        let block = tree.get_mut(block_id);
        block.instructions.retain(|id| !to_remove.contains(id));
    }

    true
}

/// Apply value substitutions to all instructions and terminators.
fn apply_substitutions(
    substitutions: &HashMap<mir::Value, mir::Value>,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
) {
    if substitutions.is_empty() {
        return;
    }

    for &block_id in &function.blocks {
        // substitute in instructions
        let block = tree.get(block_id);
        let instructions = block.instructions.clone();

        for &inst_id in &instructions {
            let inst = tree.get_mut(inst_id);
            substitute_in_instruction(inst, substitutions);
        }

        // substitute in terminator
        let block = tree.get_mut(block_id);
        block.terminator = terminator_substitute_uses(&block.terminator, substitutions);
    }
}

/// Substitute values in an instruction.
fn substitute_in_instruction(
    inst: &mut mir::Instruction,
    substitutions: &HashMap<mir::Value, mir::Value>,
) {
    match inst {
        mir::Instruction::Binary { left, right, .. } => {
            if let Some(&new) = substitutions.get(left) {
                *left = new;
            }
            if let Some(&new) = substitutions.get(right) {
                *right = new;
            }
        }
        mir::Instruction::Unary { argument, .. } => {
            if let Some(&new) = substitutions.get(argument) {
                *argument = new;
            }
        }
        mir::Instruction::Cast { argument, .. } => {
            if let Some(&new) = substitutions.get(argument) {
                *argument = new;
            }
        }
        mir::Instruction::Select {
            condition,
            then_value,
            else_value,
            ..
        } => {
            if let Some(&new) = substitutions.get(condition) {
                *condition = new;
            }
            if let Some(&new) = substitutions.get(then_value) {
                *then_value = new;
            }
            if let Some(&new) = substitutions.get(else_value) {
                *else_value = new;
            }
        }
        mir::Instruction::Load { pointer, .. } => {
            if let Some(&new) = substitutions.get(pointer) {
                *pointer = new;
            }
        }
        mir::Instruction::Store { pointer, value, .. } => {
            if let Some(&new) = substitutions.get(pointer) {
                *pointer = new;
            }
            if let Some(&new) = substitutions.get(value) {
                *value = new;
            }
        }
        mir::Instruction::FieldGet { aggregate, .. } => {
            if let Some(&new) = substitutions.get(aggregate) {
                *aggregate = new;
            }
        }
        mir::Instruction::FieldAddr { aggregate, .. } => {
            if let Some(&new) = substitutions.get(aggregate) {
                *aggregate = new;
            }
        }
        mir::Instruction::FieldSet {
            aggregate, value, ..
        } => {
            if let Some(&new) = substitutions.get(aggregate) {
                *aggregate = new;
            }
            if let Some(&new) = substitutions.get(value) {
                *value = new;
            }
        }
        mir::Instruction::ElementGet { array, index, .. } => {
            if let Some(&new) = substitutions.get(array) {
                *array = new;
            }
            if let Some(&new) = substitutions.get(index) {
                *index = new;
            }
        }
        mir::Instruction::ElementAddr { array, index, .. } => {
            if let Some(&new) = substitutions.get(array) {
                *array = new;
            }
            if let Some(&new) = substitutions.get(index) {
                *index = new;
            }
        }
        mir::Instruction::ElementSet {
            array,
            index,
            value,
            ..
        } => {
            if let Some(&new) = substitutions.get(array) {
                *array = new;
            }
            if let Some(&new) = substitutions.get(index) {
                *index = new;
            }
            if let Some(&new) = substitutions.get(value) {
                *value = new;
            }
        }
        mir::Instruction::RawFree { pointer } => {
            if let Some(&new) = substitutions.get(pointer) {
                *pointer = new;
            }
        }
        mir::Instruction::RawDrop { value, .. } => {
            if let Some(&new) = substitutions.get(value) {
                *value = new;
            }
        }
        mir::Instruction::StackDrop { value, .. } => {
            if let Some(&new) = substitutions.get(value) {
                *value = new;
            }
        }
        mir::Instruction::LocalSet { value, .. } => {
            if let Some(&new) = substitutions.get(value) {
                *value = new;
            }
        }
        mir::Instruction::Assume { condition } => {
            if let Some(&new) = substitutions.get(condition) {
                *condition = new;
            }
        }

        // instructions without value operands or with external arguments
        mir::Instruction::Const { .. }
        | mir::Instruction::LocalGet { .. }
        | mir::Instruction::GlobalAddr { .. }
        | mir::Instruction::GlobalConst { .. }
        | mir::Instruction::Struct { .. }
        | mir::Instruction::Tuple { .. }
        | mir::Instruction::Array { .. }
        | mir::Instruction::Call { .. }
        | mir::Instruction::CallIndirect { .. }
        | mir::Instruction::ManagedAlloc { .. }
        | mir::Instruction::ManagedAllocArray { .. }
        | mir::Instruction::RawAlloc { .. }
        | mir::Instruction::StackAlloc { .. }
        | mir::Instruction::Intrinsic { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Simple struct splitting.
    ///
    /// The struct allocation is split into separate allocations for each field.
    /// field.addr instructions are replaced with direct references to the new allocations.
    #[test]
    fn test_split_struct() {
        let input = r#"type @Point = { i32, i32 }
function @test() -> i32 {
block0:
    v0 = stack.alloc @Point
    v1 = field.addr v0, 0
    v2 = iconst 42i32
    store v1, v2
    v3 = load v1
    return v3
}"#;
        let expected = r#"type @Point = { i32, i32 }
function @test() -> i32 {
block0:
    v5 = stack.alloc i32
    v4 = stack.alloc i32
    v2 = iconst 42i32
    store v4, v2
    v3 = load v4
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_output(expected);
    }

    /// Tuple splitting.
    ///
    /// Tuples are split like structs - each element gets its own allocation.
    #[test]
    fn test_split_tuple() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc (i32, i64)
    v1 = field.addr v0, 0
    v2 = iconst 42i32
    store v1, v2
    v3 = load v1
    return v3
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v5 = stack.alloc i64
    v4 = stack.alloc i32
    v2 = iconst 42i32
    store v4, v2
    v3 = load v4
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_output(expected);
    }

    /// Small array splitting.
    ///
    /// Arrays with constant indices are split into separate allocations per element.
    #[test]
    fn test_split_small_array() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc [i32; 4]
    v1 = iconst 0i64
    v2 = element.addr v0, v1
    v3 = iconst 42i32
    store v2, v3
    v4 = load v2
    return v4
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v8 = stack.alloc i32
    v7 = stack.alloc i32
    v6 = stack.alloc i32
    v5 = stack.alloc i32
    v1 = iconst 0i64
    v3 = iconst 42i32
    store v5, v3
    v4 = load v5
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_output(expected);
    }

    /// Array too large to split.
    ///
    /// Arrays exceeding the max element threshold are not split.
    #[test]
    fn test_preserve_large_array() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc [i32; 100]
    v1 = iconst 0i64
    v2 = element.addr v0, v1
    v3 = iconst 42i32
    store v2, v3
    v4 = load v2
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_unchanged(input);
    }

    /// Escaping allocation should not be split.
    ///
    /// When the allocation is passed to an external function, it escapes
    /// and cannot be split.
    #[test]
    fn test_preserve_escaping() {
        let input = r#"type @Point = { i32, i32 }
extern function @external(ref<raw @Point>) -> void
function @test() -> void {
block0:
    v0 = stack.alloc @Point
    call @external(v0)
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_unchanged(input);
    }

    /// Non-constant array index should not be split.
    ///
    /// When the array index is dynamic (not a compile-time constant),
    /// we cannot split because we don't know which element is accessed.
    #[test]
    fn test_preserve_dynamic_index() {
        let input = r#"function @test(v0: i64) -> i32 {
block0(v0: i64):
    v1 = stack.alloc [i32; 4]
    v2 = element.addr v1, v0
    v3 = iconst 42i32
    store v2, v3
    v4 = load v2
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_unchanged(input);
    }

    /// Multiple fields accessed.
    ///
    /// When multiple fields of a struct are accessed, all field.addr
    /// instructions are replaced with the corresponding new allocations.
    #[test]
    fn test_multiple_fields() {
        let input = r#"type @Point = { i32, i32 }
function @test() -> i32 {
block0:
    v0 = stack.alloc @Point
    v1 = field.addr v0, 0
    v2 = iconst 10i32
    store v1, v2
    v3 = field.addr v0, 1
    v4 = iconst 20i32
    store v3, v4
    v5 = load v1
    v6 = load v3
    v7 = iadd v5, v6
    return v7
}"#;
        let expected = r#"type @Point = { i32, i32 }
function @test() -> i32 {
block0:
    v9 = stack.alloc i32
    v8 = stack.alloc i32
    v2 = iconst 10i32
    store v8, v2
    v4 = iconst 20i32
    store v9, v4
    v5 = load v8
    v6 = load v9
    v7 = iadd v5, v6
    return v7
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_output(expected);
    }

    /// Nested struct - only top level is split.
    ///
    /// SROA splits only the top-level aggregate; nested aggregates remain
    /// intact and can be split in subsequent passes.
    #[test]
    fn test_nested_struct() {
        let input = r#"type @Inner = { i32, i32 }
type @Outer = { @Inner, i64 }
function @test() -> i64 {
block0:
    v0 = stack.alloc @Outer
    v1 = field.addr v0, 1
    v2 = iconst 42i64
    store v1, v2
    v3 = load v1
    return v3
}"#;
        let expected = r#"type @Inner = { i32, i32 }
type @Outer = { @Inner, i64 }
function @test() -> i64 {
block0:
    v5 = stack.alloc i64
    v4 = stack.alloc @Inner
    v2 = iconst 42i64
    store v5, v2
    v3 = load v5
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_output(expected);
    }

    /// No changes when no aggregates.
    ///
    /// Scalar allocations are not affected by SROA.
    #[test]
    fn test_no_aggregates() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_unchanged(input);
    }

    /// Allocation escaping via store is not split.
    ///
    /// When a pointer to the allocation is stored to memory, it escapes
    /// and cannot be safely split.
    #[test]
    fn test_preserve_escaping_via_store() {
        let input = r#"type @Point = { i32, i32 }
function @test(v0: ref<raw ref<raw @Point>>) -> void {
block0(v0: ref<raw ref<raw @Point>>):
    v1 = stack.alloc @Point
    store v0, v1
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_unchanged(input);
    }

    /// Single-field struct can be split.
    ///
    /// Even a single-field aggregate benefits from SROA since it allows
    /// mem2reg to promote the value to SSA.
    #[test]
    fn test_split_single_field() {
        let input = r#"type @Wrapper = { i32 }
function @test() -> i32 {
block0:
    v0 = stack.alloc @Wrapper
    v1 = field.addr v0, 0
    v2 = iconst 42i32
    store v1, v2
    v3 = load v1
    return v3
}"#;
        let expected = r#"type @Wrapper = { i32 }
function @test() -> i32 {
block0:
    v4 = stack.alloc i32
    v2 = iconst 42i32
    store v4, v2
    v3 = load v4
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_output(expected);
    }

    /// Block argument escaping prevents splitting.
    ///
    /// If the allocation is passed as a block argument, it escapes.
    #[test]
    fn test_preserve_block_argument_escape() {
        let input = r#"type @Point = { i32, i32 }
function @test(v0: bool) -> void {
block0(v0: bool):
    v1 = stack.alloc @Point
    branch v0, block1(v1), block2
block1(v2: ref<raw @Point>):
    return
block2:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_unchanged(input);
    }

    /// Constant propagation resolves array indices across block parameters.
    #[test]
    fn test_split_array_constant_param_index() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0i64
    branch v0, block1(v1), block1(v1)
block1(v2: i64):
    v3 = stack.alloc [i32; 2]
    v4 = element.addr v3, v2
    v5 = iconst 42i32
    store v4, v5
    v6 = load v4
    return v6
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v8 = stack.alloc i32
    v7 = stack.alloc i32
    v1 = iconst 0i64
    branch v0, block1(v1), block1(v1)
block1(v2: i64):
    v5 = iconst 42i32
    store v7, v5
    v6 = load v7
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_output(expected);
    }

    /// Base pointer loads and stores prevent splitting.
    #[test]
    fn test_preserve_base_pointer_load_store() {
        let input = r#"type @Point = { i32, i32 }
function @test() -> i32 {
block0:
    v0 = stack.alloc @Point
    v1 = iconst 1i32
    v2 = iconst 2i32
    v3 = struct @Point (v1, v2)
    store v0, v3
    v4 = load v0
    v5 = field.get v4, 0
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Sroa);
        program.assert_unchanged(input);
    }
}

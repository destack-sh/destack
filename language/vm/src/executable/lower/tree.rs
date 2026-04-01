use std::collections::{HashMap, HashSet};

use {destack_engine as engine, destack_mir as mir};

use super::super::layout::repr_type;

/// One lowered runtime value slot.
#[derive(Clone, Copy, Debug)]
pub(in crate::executable) struct LoweredValueSlot {
    /// The logical source carried by this slot.
    pub source: engine::FrameSlotSource,
    /// The MIR type stored in this slot.
    pub ty: mir::LocalNodeId<mir::Type>,
}

/// One recursively decomposed value slot tree.
#[derive(Clone, Debug)]
pub(super) struct ValueTree {
    /// The slot id for this value or component.
    pub slot: mir::Value,
    /// The semantic type stored at this node.
    pub ty: mir::LocalNodeId<mir::Type>,
    /// The child values in semantic source order.
    pub child: Vec<ValueTree>,
}

/// One lowering-time decomposition table for block parameters and SSA values.
#[derive(Clone, Debug, Default)]
pub(in crate::executable) struct BlockParameterMap {
    /// The decomposed value tree by root semantic value.
    pub(super) tree_by_value: HashMap<mir::Value, ValueTree>,
    /// The decomposed parameter tree by block and root parameter value.
    pub(super) tree_by_block: HashMap<mir::LocalNodeId<mir::Block>, HashMap<mir::Value, ValueTree>>,
}

/// One block-local decomposed value.
#[derive(Clone, Debug)]
pub(super) struct ValueDecomposition {
    /// The child values in semantic source order.
    pub(super) child: Vec<mir::Value>,
}

/// Return whether this semantic type can stay decomposed.
pub(super) fn can_decompose_value_type(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> bool {
    matches!(
        tree.get(repr_type(tree, ty)),
        mir::Type::Struct { .. } | mir::Type::Tuple { .. } | mir::Type::Array { .. }
    )
}

/// Seed one decomposed slot tree into the value decomposition map.
pub(super) fn seed_value_tree(
    tree: &ValueTree,
    decomposition_by_value: &mut HashMap<mir::Value, ValueDecomposition>,
) {
    if tree.child.is_empty() {
        return;
    }

    decomposition_by_value.insert(
        tree.slot,
        ValueDecomposition {
            child: tree.child.iter().map(|child| child.slot).collect(),
        },
    );

    for child in &tree.child {
        seed_value_tree(child, decomposition_by_value);
    }
}

/// Analyze lowered runtime value slots and decomposed block parameters for one function.
pub(in crate::executable) fn analyze_lowered_value_slots(
    tree: &mir::NodeTree,
    function: &mir::Function,
) -> (Vec<LoweredValueSlot>, BlockParameterMap) {
    let slot = function
        .value_types
        .iter()
        .enumerate()
        .map(|(index, ty)| LoweredValueSlot {
            source: engine::FrameSlotSource::Value(mir::Value::new(index as u32)),
            ty: *ty,
        })
        .collect::<Vec<_>>();
    let mut slot_tree_builder = SlotTreeBuilder::new(tree, slot);
    let mut block_parameter_map = BlockParameterMap::default();
    let resume_target_block_set = resume_target_block_set(function, tree);

    // reserve hidden child slots for all decomposable semantic values
    for (index, ty) in function.value_types.iter().enumerate() {
        let value = mir::Value::new(index as u32);
        if !can_decompose_value_type(tree, *ty) {
            continue;
        }

        let child = slot_tree_builder.build_child_value_tree(value, *ty);
        block_parameter_map.tree_by_value.insert(
            value,
            ValueTree {
                slot: value,
                ty: *ty,
                child,
            },
        );
    }

    // reserve hidden child slots for ordinary CFG block parameters
    for block_id in &function.blocks {
        if function.entry == Some(*block_id) || resume_target_block_set.contains(block_id) {
            continue;
        }

        let block = tree.get(*block_id);
        let mut tree_by_value = HashMap::new();

        for parameter in &block.parameters {
            if !can_decompose_value_type(tree, parameter.ty) {
                continue;
            }

            let value_tree = block_parameter_map
                .tree_by_value
                .get(&parameter.value)
                .cloned()
                .unwrap_or_else(|| ValueTree {
                    slot: parameter.value,
                    ty: parameter.ty,
                    child: slot_tree_builder.build_child_value_tree(parameter.value, parameter.ty),
                });

            tree_by_value.insert(parameter.value, value_tree);
        }

        if !tree_by_value.is_empty() {
            block_parameter_map
                .tree_by_block
                .insert(*block_id, tree_by_value);
        }
    }

    (slot_tree_builder.finish(), block_parameter_map)
}

/// Return the blocks whose entry parameters must stay materialized for resume.
fn resume_target_block_set(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashSet<mir::LocalNodeId<mir::Block>> {
    let mut block_set = HashSet::new();

    // reserve the semantic resume targets
    for block_id in &function.blocks {
        let block = tree.get(*block_id);

        match &block.terminator {
            mir::Terminator::Yield { resume, .. } => {
                block_set.insert(*resume);
            }
            mir::Terminator::Call {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::CallIndirect {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::CallVirtual {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::CallInterface {
                normal_target,
                unwind_target,
                ..
            } => {
                block_set.insert(*normal_target);
                block_set.insert(*unwind_target);
            }
            _ => {}
        }
    }

    block_set
}

/// Build one lowered slot tree.
struct SlotTreeBuilder<'a> {
    tree: &'a mir::NodeTree,
    slot: Vec<LoweredValueSlot>,
}

impl<'a> SlotTreeBuilder<'a> {
    /// Create one slot tree builder.
    fn new(tree: &'a mir::NodeTree, slot: Vec<LoweredValueSlot>) -> Self {
        Self { tree, slot }
    }

    /// Return the completed slot list.
    fn finish(self) -> Vec<LoweredValueSlot> {
        self.slot
    }

    /// Build one disaggregated hidden slot tree for the given component type.
    fn build_value_tree(
        &mut self,
        owner: mir::Value,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> ValueTree {
        let value = mir::Value::new(self.slot.len() as u32);
        let repr_ty = repr_type(self.tree, ty);

        // allocate this node first so nested materialization has one destination
        self.slot.push(LoweredValueSlot {
            source: engine::FrameSlotSource::DisaggregatedValue(owner),
            ty,
        });

        let child = match self.tree.get(repr_ty) {
            mir::Type::Struct { fields, .. } => fields
                .iter()
                .map(|field| self.build_value_tree(owner, self.tree.get(*field).ty))
                .collect(),
            mir::Type::Tuple { elements, .. } => elements
                .iter()
                .map(|element| self.build_value_tree(owner, *element))
                .collect(),
            mir::Type::Array {
                element, length, ..
            } => (0..array_length(*length))
                .map(|_| self.build_value_tree(owner, *element))
                .collect(),
            _ => Vec::new(),
        };

        ValueTree {
            slot: value,
            ty,
            child,
        }
    }

    /// Build the hidden child slot tree for the given semantic type.
    fn build_child_value_tree(
        &mut self,
        owner: mir::Value,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Vec<ValueTree> {
        match self.tree.get(repr_type(self.tree, ty)) {
            mir::Type::Struct { fields, .. } => fields
                .iter()
                .map(|field| self.build_value_tree(owner, self.tree.get(*field).ty))
                .collect(),
            mir::Type::Tuple { elements, .. } => elements
                .iter()
                .map(|element| self.build_value_tree(owner, *element))
                .collect(),
            mir::Type::Array {
                element, length, ..
            } => (0..array_length(*length))
                .map(|_| self.build_value_tree(owner, *element))
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// Convert one MIR array length into one host index length.
fn array_length(length: u64) -> usize {
    usize::try_from(length)
        .unwrap_or_else(|_| panic!("array length does not fit host usize: {length}"))
}

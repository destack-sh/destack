use std::collections::{HashMap, HashSet};

use {destack_engine as engine, destack_mir as mir};

use crate::executable::layout::repr_type;
use crate::{Error, Result};

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
) -> Result<(Vec<LoweredValueSlot>, BlockParameterMap)> {
    let slot = function
        .value_types
        .iter()
        .enumerate()
        .filter_map(|(index, ty)| {
            let Some(ty) = *ty else {
                return None;
            };

            Some(LoweredValueSlot {
                source: engine::FrameSlotSource::Value(mir::Value::new(index as u32)),
                ty,
            })
        })
        .collect::<Vec<_>>();
    let mut slot_tree_builder = SlotTreeBuilder::new(tree, slot);
    let mut block_parameter_map = BlockParameterMap::default();
    let resume_target_block_set = resume_target_block_set(function, tree);

    // reserve hidden child slots for all decomposable semantic values
    for (index, ty) in function.value_types.iter().enumerate() {
        let Some(ty) = *ty else {
            continue;
        };

        let value = mir::Value::new(index as u32);
        if !can_decompose_value_type(tree, ty) {
            continue;
        }

        let child = slot_tree_builder.build_child_value_tree(value, ty)?;
        block_parameter_map.tree_by_value.insert(
            value,
            ValueTree {
                slot: value,
                ty,
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
            let Some(value) = parameter.value.value() else {
                continue;
            };
            let Some(ty) = parameter.ty.ty() else {
                continue;
            };

            if !can_decompose_value_type(tree, ty) {
                continue;
            }

            let value_tree =
                if let Some(value_tree) = block_parameter_map.tree_by_value.get(&value).cloned() {
                    value_tree
                } else {
                    ValueTree {
                        slot: value,
                        ty,
                        child: slot_tree_builder.build_child_value_tree(value, ty)?,
                    }
                };

            tree_by_value.insert(value, value_tree);
        }

        if !tree_by_value.is_empty() {
            block_parameter_map
                .tree_by_block
                .insert(*block_id, tree_by_value);
        }
    }

    Ok((slot_tree_builder.finish(), block_parameter_map))
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
        let terminator = tree.get(block.terminator);

        match terminator {
            mir::Terminator::Yield { resume, .. } => {
                let Some(block) = resume.block.block() else {
                    continue;
                };
                block_set.insert(block);
            }
            mir::Terminator::Invoke {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::InvokeIndirect {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::InvokeVirtual {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::InvokeInterface {
                normal_target,
                unwind_target,
                ..
            } => {
                let Some(normal_target) = normal_target.block.block() else {
                    continue;
                };
                let Some(unwind_target) = unwind_target.block.block() else {
                    continue;
                };
                block_set.insert(normal_target);
                block_set.insert(unwind_target);
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
    ) -> Result<ValueTree> {
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
                .filter_map(|field| {
                    let field = self.tree.get(*field);
                    let ty = field.ty.ty()?;
                    Some(self.build_value_tree(owner, ty))
                })
                .collect::<Result<Vec<_>>>()?,
            mir::Type::Tuple { elements, .. } => elements
                .iter()
                .filter_map(|element| {
                    let ty = element.ty()?;
                    Some(self.build_value_tree(owner, ty))
                })
                .collect::<Result<Vec<_>>>()?,
            mir::Type::Array {
                element, length, ..
            } => (0..array_length(*length)?)
                .filter_map(|_| {
                    let ty = element.ty()?;
                    Some(self.build_value_tree(owner, ty))
                })
                .collect::<Result<Vec<_>>>()?,
            _ => Vec::new(),
        };

        Ok(ValueTree {
            slot: value,
            ty,
            child,
        })
    }

    /// Build the hidden child slot tree for the given semantic type.
    fn build_child_value_tree(
        &mut self,
        owner: mir::Value,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<Vec<ValueTree>> {
        Ok(match self.tree.get(repr_type(self.tree, ty)) {
            mir::Type::Struct { fields, .. } => fields
                .iter()
                .filter_map(|field| {
                    let field = self.tree.get(*field);
                    let ty = field.ty.ty()?;
                    Some(self.build_value_tree(owner, ty))
                })
                .collect::<Result<Vec<_>>>()?,
            mir::Type::Tuple { elements, .. } => elements
                .iter()
                .filter_map(|element| {
                    let ty = element.ty()?;
                    Some(self.build_value_tree(owner, ty))
                })
                .collect::<Result<Vec<_>>>()?,
            mir::Type::Array {
                element, length, ..
            } => (0..array_length(*length)?)
                .filter_map(|_| {
                    let ty = element.ty()?;
                    Some(self.build_value_tree(owner, ty))
                })
                .collect::<Result<Vec<_>>>()?,
            _ => Vec::new(),
        })
    }
}

/// Convert one MIR array length into one host index length.
fn array_length(length: u64) -> Result<usize> {
    usize::try_from(length).map_err(|_| Error::InvariantViolation {
        context: format!("array length does not fit host usize: {length}"),
    })
}

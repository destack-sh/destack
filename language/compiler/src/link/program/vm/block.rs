use std::collections::{HashMap, HashSet};

use destack_heap as heap;
use destack_mir as mir;

use destack_program::vm::CallTarget;
use destack_program::{CellLayout, FrameLayout, FrameSlot, FrameStateId, FunctionId, ScalarFormat};

use crate::{LinkError, LinkResult};

use super::super::TypeLinker;
use super::layout::StorageLayout;
use super::linker::Linker;
use super::value::{Operand, OperandLowerer, OperandMap};

/// One lowered block traversal order.
pub(crate) struct BlockOrder {
    /// The lowered block index by MIR block id.
    pub(crate) index_by_id: HashMap<mir::LocalNodeId<mir::Block>, usize>,
    /// The MIR blocks in lowered traversal order.
    pub(crate) blocks: Vec<mir::LocalNodeId<mir::Block>>,
}

impl BlockOrder {
    /// Build one lowered block traversal order from the entry block.
    pub(crate) fn new(
        tree: &mir::Tree,
        entry_block: mir::LocalNodeId<mir::Block>,
        program: &super::super::ProgramLinker,
    ) -> LinkResult<Self> {
        let mut index_by_id = HashMap::new();
        let mut blocks = Vec::new();
        let mut queue = vec![entry_block];
        let mut visited = HashSet::new();

        while let Some(block_id) = queue.pop() {
            // skip visited blocks
            if visited.contains(&block_id) {
                continue;
            }

            // record block index
            visited.insert(block_id);
            let block_index = blocks.len();
            index_by_id.insert(block_id, block_index);
            blocks.push(block_id);

            // enqueue successor blocks
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);
            match terminator {
                mir::Terminator::Jump { target, .. } => {
                    queue.push(target.block);
                }
                mir::Terminator::Branch {
                    then_target,
                    else_target,
                    ..
                } => {
                    queue.push(then_target.block);
                    queue.push(else_target.block);
                }
                mir::Terminator::Check {
                    success, failure, ..
                } => {
                    queue.push(success.block);
                    queue.push(failure.block);
                }
                mir::Terminator::NewZeroedTry {
                    success, failure, ..
                }
                | mir::Terminator::NewUninitTry {
                    success, failure, ..
                }
                | mir::Terminator::NewSliceZeroedTry {
                    success, failure, ..
                }
                | mir::Terminator::NewSliceUninitTry {
                    success, failure, ..
                } => {
                    queue.push(success.block);
                    queue.push(failure.block);
                }
                mir::Terminator::Switch { cases, default, .. } => {
                    for case in tree.get_switch_cases(*cases) {
                        queue.push(case.target.block);
                    }
                    queue.push(default.block);
                }
                mir::Terminator::Yield { resume, .. } => {
                    queue.push(resume.block);
                }
                mir::Terminator::Call { target, unwind, .. }
                | mir::Terminator::CallIndirect { target, unwind, .. }
                | mir::Terminator::CallVirtual { target, unwind, .. }
                | mir::Terminator::CallDynamic { target, unwind, .. } => {
                    queue.push(target.block);
                    if let Some(unwind) = unwind {
                        queue.push(unwind.block);
                    }
                }
                mir::Terminator::Error => {
                    return Err(program.invalid_input("terminator"));
                }
                mir::Terminator::Return { .. }
                | mir::Terminator::Panic { .. }
                | mir::Terminator::UnwindResume
                | mir::Terminator::Trap { .. }
                | mir::Terminator::Unreachable
                | mir::Terminator::TailCall { .. }
                | mir::Terminator::TailCallIndirect { .. }
                | mir::Terminator::TailCallVirtual { .. }
                | mir::Terminator::TailCallDynamic { .. } => {}
            }
        }

        debug_assert!(
            blocks.len() <= u32::MAX as usize,
            "too many blocks for lowered block indices"
        );

        Ok(Self {
            index_by_id,
            blocks,
        })
    }
}

/// One shared function-scoped lowerer context.
pub(super) struct FunctionContext<'a> {
    /// The MIR tree.
    pub(super) tree: &'a mir::Tree,
    /// Target ABI layout.
    pub(super) target_layout: &'a mir::TargetLayout,
    /// Canonical MIR type table.
    pub(super) types: &'a mir::TypeTable,
    /// The current MIR function id.
    pub(super) function_id: mir::LocalNodeId<mir::Function>,
    /// The lowered entry block index.
    pub(super) entry_block: u32,
    /// The lowered yield frame state by MIR block id.
    pub(super) yield_frame_states: &'a HashMap<mir::LocalNodeId<mir::Block>, FrameStateId>,
    /// The lowered call terminator frame state by MIR block id.
    pub(super) call_frame_states: &'a HashMap<mir::LocalNodeId<mir::Block>, FrameStateId>,
    /// The call target by program function id.
    pub(super) call_targets: &'a [Option<CallTarget>],
    /// VM linker state.
    pub(super) program: &'a Linker<'a>,
    /// The byte layout for this lowered function frame.
    pub(super) frame_layout: &'a FrameLayout,
    /// The lowered operand by SSA value id.
    pub(super) operand_map: OperandMap,
    /// The lowered value type by SSA value id.
    pub(super) value_types: Vec<mir::LocalNodeId<mir::Type>>,
    /// The lowered storage layout by MIR type id.
    pub(super) layouts: &'a HashMap<mir::LocalNodeId<mir::Type>, StorageLayout>,
    /// Canonical MIR layout table.
    pub(super) layout_table: &'a mir::LayoutTable,
    /// The worker-local heap allocation geometry.
    pub(super) heap_options: &'a heap::HeapOptions,
    /// The runtime-shared heap allocation geometry.
    pub(super) shared_heap_options: &'a heap::SharedHeapOptions,
    /// The lowered block index by MIR block id.
    pub(super) block_index_by_id: HashMap<mir::LocalNodeId<mir::Block>, usize>,
    /// The lowered block parameter values by block index.
    pub(super) block_parameters: Vec<Vec<mir::Value>>,
    /// The lowered local index by MIR local id.
    pub(super) local_index_by_id: HashMap<mir::LocalNodeId<mir::Local>, u32>,
    /// The lowered SSA value use count by SSA value id.
    pub(super) value_use_counts: Vec<u32>,
}

impl<'a> FunctionContext<'a> {
    /// Return the program type linker.
    pub(super) fn type_linker(&self) -> TypeLinker<'_> {
        TypeLinker::new(
            self.tree,
            self.target_layout,
            self.types,
            self.program.program(),
        )
    }

    /// Return the MIR to VM operand lowerer.
    pub(super) fn operand_lowerer(&self) -> OperandLowerer<'_> {
        OperandLowerer::new(self.tree, self.target_layout, self.types, self.program)
    }

    /// Return the target pointer size in bytes.
    pub(super) const fn pointer_bytes(&self) -> u8 {
        self.target_layout.pointer_bytes()
    }

    /// Return the MIR layout entry for one type when present.
    pub(super) fn layout_for_type(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<&mir::Layout> {
        self.layout_table.type_layout(self.tree.repr_type(ty))
    }

    /// Return whether one frame slot is lowered as one VM cell.
    pub(super) fn slot_is_cell(&self, slot: &FrameSlot) -> bool {
        self.program.frame_slot_is_cell(slot)
    }

    /// Return one value slot inside this function frame.
    pub(super) fn frame_value_slot(&self, value: u32) -> Option<&FrameSlot> {
        self.program.frame_value_slot(self.frame_layout, value)
    }

    /// Return one local slot inside this function frame.
    pub(super) fn frame_local_slot(&self, local: u32) -> Option<&FrameSlot> {
        self.program.frame_local_slot(self.frame_layout, local)
    }

    /// Return the program function id for one MIR function.
    pub(super) fn program_function(&self, function: mir::LocalNodeId<mir::Function>) -> FunctionId {
        self.program.function_id(function)
    }

    /// Return the VM operand for one MIR type.
    pub(super) fn operand_for_type(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<Operand> {
        self.operand_lowerer().operand(ty)
    }

    /// Return the storage space for one MIR reference type.
    pub(super) fn space_for_type(&self, ty: mir::LocalNodeId<mir::Type>) -> LinkResult<mir::Space> {
        match self.operand_for_type(ty) {
            Some(Operand::Reference { space, .. }) => Ok(space),
            actual => Err(self.invalid_pointer_type(format!("{actual:?}"))),
        }
    }

    /// Return the VM cell layout for one MIR type.
    pub(super) fn cell_layout_for_type(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Option<CellLayout> {
        self.type_linker().cell_layout(ty)
    }

    /// Return the VM scalar layout for one MIR type.
    pub(super) fn scalar_layout_for_type(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Option<ScalarFormat> {
        self.type_linker().scalar_format(ty)
    }

    /// Return one VM scalar layout or fail loudly.
    pub(super) fn require_scalar_format(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        expected: &'static str,
    ) -> LinkResult<ScalarFormat> {
        self.scalar_layout_for_type(ty)
            .ok_or_else(|| self.type_mismatch(expected, format!("{ty:?}")))
    }

    /// Return one invalid lowered input diagnostic.
    pub(super) fn invalid_input(&self, context: impl Into<String>) -> LinkError {
        self.program.program().invalid_input(context)
    }

    /// Return one link type mismatch diagnostic.
    pub(super) fn type_mismatch(
        &self,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> LinkError {
        self.program.program().type_mismatch(expected, actual)
    }

    /// Return one invalid instruction diagnostic.
    pub(super) fn invalid_instruction(&self, context: impl Into<String>) -> LinkError {
        self.program.program().invalid_instruction(context)
    }

    /// Return one invalid cast diagnostic.
    pub(super) fn invalid_cast(&self, context: impl Into<String>) -> LinkError {
        self.program.program().invalid_cast(context)
    }

    /// Return one invalid field access diagnostic.
    pub(super) fn invalid_field_access(&self, index: u32, field_count: usize) -> LinkError {
        self.program
            .program()
            .invalid_field_access(index, field_count)
    }

    /// Return one invalid pointer type diagnostic.
    pub(super) fn invalid_pointer_type(&self, actual: impl Into<String>) -> LinkError {
        self.program.program().invalid_pointer_type(actual)
    }

    /// Return one unsupported instruction diagnostic.
    pub(super) fn unsupported_instruction(&self, name: impl Into<String>) -> LinkError {
        self.program.program().unsupported_instruction(name)
    }

    /// Return one undefined function diagnostic.
    pub(super) fn undefined_function(&self, function: impl Into<String>) -> LinkError {
        self.program.program().undefined_function(function)
    }

    /// Return one layout overflow diagnostic.
    pub(super) fn layout_overflow(&self, context: impl Into<String>) -> LinkError {
        self.program.program().layout_overflow(context)
    }

    /// Return one internal link diagnostic.
    pub(super) fn internal(&self, message: impl Into<String>) -> LinkError {
        self.program.program().internal(message)
    }

    /// Return the pointer cell layout for one MIR reference type.
    pub(super) fn reference_cell_layout(
        &self,
        space: mir::Space,
        kind: mir::ReferenceKind,
    ) -> CellLayout {
        self.type_linker().reference_cell_layout(space, kind)
    }

    /// Return values stored in one MIR value slice.
    #[inline]
    pub(super) fn values(&self, slice: mir::ValueSlice) -> &'a [mir::Value] {
        self.tree.get_values(slice)
    }

    /// Return indices stored in one MIR index slice.
    #[inline]
    pub(super) fn indices(&self, slice: mir::IndexSlice) -> &'a [u32] {
        self.tree.get_indices(slice)
    }

    /// Return switch cases stored in one MIR switch slice.
    #[inline]
    pub(super) fn switch_cases(&self, slice: mir::SwitchCaseSlice) -> &'a [mir::SwitchCase] {
        self.tree.get_switch_cases(slice)
    }

    /// Return arguments stored on one MIR block target.
    #[inline]
    pub(super) fn target_arguments(&self, target: &mir::BlockTarget) -> &'a [mir::Value] {
        target.arguments(self.tree)
    }
}

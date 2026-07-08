use std::collections::HashMap;

use destack_heap as heap;
use destack_mir as mir;
use destack_mir::{TraceId, TraceMap, TraceTable};

use destack_program::vm::{
    AllocationPlanId, ArgumentRange, Check, CheckId, ConstValueBuilder, ConstValueId, Edge, EdgeId,
    Instruction, MovePair, MoveRange, MoveSlot, MoveSource, Op, Projection, ProjectionId,
    SideRecord, SideTableBuilder, SignatureId, SliceProjection, SliceProjectionId,
    SmallAllocationPlanId, SwitchCase, SwitchCasesId, SwitchTableBuilder, SwitchTableId,
    TensorConvolutionDimensionsBuilder, TensorConvolutionId, TensorConvolutionWindowBuilder,
    TensorDotDimensionsBuilder, TensorDotId, TensorGatherDimensionsBuilder, TensorGatherId,
    TensorLayoutBuilder, TensorLayoutId, TensorScatterDimensionsBuilder, TensorScatterId,
    TensorWindowId, U32RangeId,
};
use destack_program::{FrameLayout, Signature};

use crate::LinkResult;

use super::linker::Linker;

/// One lowering pool for shared variable-length lowering data.
pub(super) struct Pool<'layout, 'table> {
    /// The frame layout being lowered.
    frame_layout: &'layout FrameLayout,
    /// VM linker state.
    program: &'layout Linker<'layout>,
    /// The canonical program trace table.
    trace_table: &'layout TraceTable,
    /// The pooled argument slots.
    argument: Vec<MoveSlot>,
    /// The pooled move pairs.
    move_pair: Vec<MovePair>,
    /// The program side table.
    side_table: &'table mut SideTableBuilder,
}

impl<'layout, 'table> Pool<'layout, 'table> {
    /// Create one empty lowering pool.
    pub(crate) fn new(
        side_table: &'table mut SideTableBuilder,
        frame_layout: &'layout FrameLayout,
        program: &'layout Linker<'layout>,
        trace_table: &'layout TraceTable,
    ) -> Self {
        Self {
            frame_layout,
            program,
            trace_table,
            argument: Vec::new(),
            move_pair: Vec::new(),
            side_table,
        }
    }

    /// Return one pooled side record id.
    pub(super) fn side_record<T: SideRecord>(&mut self, record: T) -> u32 {
        T::push(self.side_table, record)
    }

    /// Return one lowered instruction backed entirely by side-table data.
    pub(super) fn instruction_with_side<T: SideRecord>(
        &mut self,
        op: Op,
        record: T,
    ) -> Instruction {
        let record = self.side_record(record);

        Instruction::new(op, record, 0, 0, 0)
    }

    /// Finish the pool.
    pub(super) fn finish(self) -> (Vec<MoveSlot>, Vec<MovePair>) {
        (self.argument, self.move_pair)
    }

    /// Return one pooled allocation plan id.
    pub(super) fn allocation_plan(&mut self, allocation: heap::AllocationPlan) -> AllocationPlanId {
        self.side_table.push_allocation_plan(allocation)
    }

    /// Return one pooled small allocation plan id.
    pub(super) fn small_allocation_plan(
        &mut self,
        allocation: heap::SmallAllocationPlan,
    ) -> SmallAllocationPlanId {
        self.side_table.push_small_allocation_plan(allocation)
    }

    /// Return one pooled constant id.
    pub(super) fn constant(&mut self, constant: ConstValueBuilder) -> ConstValueId {
        self.side_table.push_constant(constant)
    }

    /// Return one pooled callable signature id.
    pub(crate) fn signature(&mut self, signature: Signature) -> SignatureId {
        self.side_table.push_signature(signature)
    }

    /// Return one pooled trace map id.
    pub(super) fn trace_map(&self, trace_map: &TraceMap) -> LinkResult<TraceId> {
        self.trace_table
            .id(trace_map)
            .ok_or_else(|| self.program.program().invalid_input("trace map"))
    }

    /// Return one pooled projection id.
    pub(super) fn projection(&mut self, projection: Projection) -> ProjectionId {
        self.side_table.push_projection(projection)
    }

    /// Return one pooled slice projection id.
    pub(super) fn slice_projection(&mut self, access: SliceProjection) -> SliceProjectionId {
        self.side_table.push_slice_projection(access)
    }

    /// Return one argument range from the pool.
    pub(super) fn argument_range(&mut self, arguments: &[mir::Value]) -> LinkResult<ArgumentRange> {
        if arguments.is_empty() {
            return Ok(ArgumentRange::empty());
        }

        let start = self.argument.len();
        debug_assert!(
            start + arguments.len() <= u32::MAX as usize,
            "argument pool overflow"
        );

        for argument in arguments {
            let slot = self.move_slot(*argument)?;
            self.argument.push(slot);
        }

        Ok(ArgumentRange {
            start: start as u32,
            len: arguments.len() as u32,
        })
    }

    /// Return one move range from the pool.
    pub(super) fn move_range(
        &mut self,
        parameters: &[mir::Value],
        arguments: &[mir::Value],
    ) -> LinkResult<MoveRange> {
        self.move_range_from_values(parameters, arguments)
    }

    /// Return one parameter move range from the pool.
    pub(super) fn parameter_move_range(
        &mut self,
        parameters: &[mir::FunctionParameter],
        arguments: &[mir::Value],
    ) -> LinkResult<MoveRange> {
        if parameters.is_empty() {
            return Ok(MoveRange::empty());
        }

        let start = self.move_pair.len();
        debug_assert!(
            start + parameters.len() <= u32::MAX as usize,
            "move pool overflow"
        );

        for (argument_index, parameter) in parameters.iter().enumerate() {
            let source = self.move_source(arguments, argument_index)?;
            let dest = self.move_slot(parameter.value)?;

            self.move_pair.push(MovePair { dest, source });
        }

        Ok(MoveRange {
            start: start as u32,
            len: parameters.len() as u32,
        })
    }

    /// Return one block edge move range from the pool.
    pub(super) fn edge_moves(
        &mut self,
        parameters: &[mir::Value],
        arguments: &[mir::Value],
    ) -> LinkResult<MoveRange> {
        self.move_range_from_values(parameters, arguments)
    }

    /// Return one pooled control edge id.
    pub(super) fn edge(&mut self, target: u32, moves: MoveRange) -> EdgeId {
        self.side_table.push_edge(Edge { target, moves })
    }

    /// Return one switch-case range from the pool.
    pub(super) fn switch_case_range(
        &mut self,
        tree: &mir::Tree,
        block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
        block_parameters: &[Vec<mir::Value>],
        cases: &[mir::SwitchCase],
    ) -> LinkResult<SwitchCasesId> {
        let cases = self.lower_switch_cases(tree, block_index_map, block_parameters, cases)?;

        Ok(self.side_table.push_switch_cases(cases))
    }

    /// Return one switch-table range from the pool.
    pub(super) fn switch_table_range(
        &mut self,
        tree: &mir::Tree,
        block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
        block_parameters: &[Vec<mir::Value>],
        cases: &[mir::SwitchCase],
        default_target: u32,
        default_moves: MoveRange,
    ) -> LinkResult<Option<SwitchTableId>> {
        let table = self.lower_switch_table(
            tree,
            block_index_map,
            block_parameters,
            cases,
            default_target,
            default_moves,
        )?;

        Ok(table.map(|(min, cases)| {
            self.side_table
                .push_switch_table(SwitchTableBuilder { min, cases })
        }))
    }

    /// Return one pooled check constraint id.
    pub(super) fn check(&mut self, constraint: Check) -> CheckId {
        self.side_table.push_check(constraint)
    }

    /// Return one pooled u32 slice id.
    pub(super) fn u32_range(&mut self, values: &[u32]) -> U32RangeId {
        self.side_table.push_u32_range(values)
    }

    /// Return one pooled tensor dot descriptor id.
    pub(super) fn tensor_dot(&mut self, dimensions: TensorDotDimensionsBuilder) -> TensorDotId {
        self.side_table.push_tensor_dot(dimensions)
    }

    /// Return one pooled tensor convolution dimension descriptor id.
    pub(super) fn tensor_convolution(
        &mut self,
        dimensions: TensorConvolutionDimensionsBuilder,
    ) -> TensorConvolutionId {
        self.side_table.push_tensor_convolution(dimensions)
    }

    /// Return one pooled tensor convolution window descriptor id.
    pub(super) fn tensor_window(
        &mut self,
        window: TensorConvolutionWindowBuilder,
    ) -> TensorWindowId {
        self.side_table.push_tensor_window(window)
    }

    /// Return one pooled tensor gather descriptor id.
    pub(super) fn tensor_gather(
        &mut self,
        dimensions: TensorGatherDimensionsBuilder,
    ) -> TensorGatherId {
        self.side_table.push_tensor_gather(dimensions)
    }

    /// Return one pooled tensor scatter descriptor id.
    pub(super) fn tensor_scatter(
        &mut self,
        dimensions: TensorScatterDimensionsBuilder,
    ) -> TensorScatterId {
        self.side_table.push_tensor_scatter(dimensions)
    }

    /// Return one pooled tensor layout id.
    pub(super) fn tensor_layout(&mut self, layout: TensorLayoutBuilder) -> TensorLayoutId {
        self.side_table.push_tensor_layout(layout)
    }

    /// Return one lowered frame slot for one SSA value.
    pub(super) fn move_slot(&self, value: mir::Value) -> LinkResult<MoveSlot> {
        let slot = self
            .program
            .frame_value_slot(self.frame_layout, value.0)
            .ok_or_else(|| self.program.program().invalid_instruction("move slot"))?;
        let is_cell = self.program.frame_slot_is_cell(slot);

        Ok(MoveSlot::new(
            slot.ty,
            slot.offset,
            slot.byte_len(),
            is_cell,
        ))
    }

    /// Return one move range for SSA value parameters.
    fn move_range_from_values(
        &mut self,
        parameters: &[mir::Value],
        arguments: &[mir::Value],
    ) -> LinkResult<MoveRange> {
        if parameters.is_empty() {
            return Ok(MoveRange::empty());
        }

        let start = self.move_pair.len();
        debug_assert!(
            start + parameters.len() <= u32::MAX as usize,
            "move pool overflow"
        );

        for (argument_index, parameter) in parameters.iter().enumerate() {
            let source = self.move_source(arguments, argument_index)?;
            let dest = self.move_slot(*parameter)?;

            self.move_pair.push(MovePair { dest, source });
        }

        Ok(MoveRange {
            start: start as u32,
            len: parameters.len() as u32,
        })
    }

    /// Return one switch-case range from the pool.
    fn lower_switch_cases(
        &mut self,
        tree: &mir::Tree,
        block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
        block_parameters: &[Vec<mir::Value>],
        cases: &[mir::SwitchCase],
    ) -> LinkResult<Box<[SwitchCase]>> {
        let mut lowered_cases = Vec::with_capacity(cases.len());

        for case in cases {
            let target = case.target.block;
            let target_index = block_index_map[&target];
            let target_parameters = block_parameters[target_index].as_slice();
            let arguments = case.target.arguments(tree);
            let moves = self.move_range_from_values(target_parameters, arguments)?;
            lowered_cases.push(SwitchCase {
                value: case.value,
                target: target_index as u32,
                moves,
            });
        }

        Ok(lowered_cases.into_boxed_slice())
    }

    /// Return one switch-table range when density is high enough.
    fn lower_switch_table(
        &mut self,
        tree: &mir::Tree,
        block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
        block_parameters: &[Vec<mir::Value>],
        cases: &[mir::SwitchCase],
        default_target: u32,
        default_moves: MoveRange,
    ) -> LinkResult<Option<(i128, Box<[SwitchCase]>)>> {
        if cases.is_empty() {
            return Ok(None);
        }

        let mut min_value = cases[0].value;
        let mut max_value = min_value;
        for case in cases {
            let value = case.value;
            min_value = min_value.min(value);
            max_value = max_value.max(value);
        }

        let range_len = max_value - min_value + 1;
        if range_len <= 0 {
            return Ok(None);
        }
        if range_len > u32::MAX as i128 {
            return Ok(None);
        }

        let range_len = range_len as usize;
        let max_range_len = cases.len() * 2;
        if range_len > max_range_len {
            return Ok(None);
        }

        let mut table = Vec::with_capacity(range_len);

        for offset in 0..range_len {
            let value = min_value + offset as i128;
            table.push(SwitchCase {
                value,
                target: default_target,
                moves: default_moves,
            });
        }

        for case in cases {
            let case_value = case.value;
            let target = case.target.block;
            let target_index = block_index_map[&target];
            let target_parameters = block_parameters[target_index].as_slice();
            let arguments = case.target.arguments(tree);
            let moves = self.move_range_from_values(target_parameters, arguments)?;
            let offset = (case_value - min_value) as usize;
            let entry = &mut table[offset];
            entry.value = case_value;
            entry.target = target_index as u32;
            entry.moves = moves;
        }

        Ok(Some((min_value, table.into_boxed_slice())))
    }

    /// Return the lowered move source for one argument index.
    fn move_source(&self, arguments: &[mir::Value], index: usize) -> LinkResult<MoveSource> {
        let Some(value) = arguments.get(index) else {
            return Ok(MoveSource::void());
        };

        Ok(MoveSource::slot(self.move_slot(*value)?))
    }
}

use std::collections::HashMap;

use destack_engine as engine;
use destack_mir as mir;

use crate::program::{
    AllocationSite, AllocationSiteId, ArgumentRange, CallTarget, Check, CheckId, ConstValue,
    ConstValueId, Edge, EdgeId, Instruction, MovePair, MoveRange, MoveSlot, MoveSource, Op,
    Projection, ProjectionId, SideRecord, SideTableBuilder, SliceProjection, SliceProjectionId,
    SmallAllocationSite, SmallAllocationSiteId, SwitchCase, SwitchCasesId, SwitchTable,
    SwitchTableId, TensorConvolutionId, TensorDotId, TensorGatherId, TensorLayout, TensorLayoutId,
    TensorScatterId, TensorWindowId, U32RangeId,
};
use crate::{Error, Result};

/// One lowering pool for shared variable-length lowering data.
pub(super) struct Pool<'layout, 'table> {
    /// The frame layout being lowered.
    frame_layout: &'layout engine::FrameLayout,
    /// The canonical program trace table.
    trace_table: &'layout mir::TraceTable,
    /// The pooled argument values.
    argument: Vec<mir::Value>,
    /// The pooled move pairs.
    move_pair: Vec<MovePair>,
    /// The program side table.
    side_table: &'table mut SideTableBuilder,
}

impl<'layout, 'table> Pool<'layout, 'table> {
    /// Create one empty lowering pool.
    pub(super) fn new(
        side_table: &'table mut SideTableBuilder,
        frame_layout: &'layout engine::FrameLayout,
        trace_table: &'layout mir::TraceTable,
    ) -> Self {
        Self {
            frame_layout,
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
    pub(super) fn finish(self) -> (Vec<mir::Value>, Vec<MovePair>) {
        (self.argument, self.move_pair)
    }

    /// Return one pooled allocation site id.
    pub(super) fn allocation_site(&mut self, allocation: AllocationSite) -> AllocationSiteId {
        self.side_table.push_allocation_site(allocation)
    }

    /// Return one pooled small allocation site id.
    pub(super) fn small_allocation_site(
        &mut self,
        allocation: SmallAllocationSite,
    ) -> SmallAllocationSiteId {
        self.side_table.push_small_allocation_site(allocation)
    }

    /// Return one pooled constant id.
    pub(super) fn constant(&mut self, constant: ConstValue) -> ConstValueId {
        self.side_table.push_constant(constant)
    }

    /// Return one pooled trace map id.
    pub(super) fn trace_map(&self, trace_map: &mir::TraceMap) -> Result<mir::TraceId> {
        self.trace_table
            .id(trace_map)
            .ok_or_else(|| Error::internal("missing program trace map"))
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
    pub(super) fn argument_range(&mut self, arguments: &[mir::Value]) -> ArgumentRange {
        argument_range(&mut self.argument, arguments)
    }

    /// Return one argument range from MIR value references.
    pub(super) fn argument_reference_range(
        &mut self,
        arguments: &[mir::ValueReference],
        context: &str,
    ) -> Result<ArgumentRange> {
        let arguments = arguments
            .iter()
            .map(|argument| {
                (*argument)
                    .value()
                    .ok_or_else(|| Error::invalid_program(context))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(argument_range(&mut self.argument, &arguments))
    }

    /// Return one move range from the pool.
    pub(super) fn move_range(
        &mut self,
        parameters: &[mir::Value],
        arguments: &[mir::Value],
    ) -> Result<MoveRange> {
        let frame_layout = self.frame_layout;

        move_range(frame_layout, &mut self.move_pair, parameters, arguments)
    }

    /// Return one parameter move range from the pool.
    pub(super) fn parameter_move_range(
        &mut self,
        parameters: &[mir::Parameter],
        arguments: &[mir::Value],
    ) -> Result<MoveRange> {
        let frame_layout = self.frame_layout;

        parameter_move_range(frame_layout, &mut self.move_pair, parameters, arguments)
    }

    /// Return one block edge move range from the pool.
    pub(super) fn edge_moves(
        &mut self,
        parameters: &[mir::Value],
        arguments: &[mir::Value],
    ) -> Result<MoveRange> {
        let frame_layout = self.frame_layout;

        move_range(frame_layout, &mut self.move_pair, parameters, arguments)
    }

    /// Return one pooled control edge id.
    pub(super) fn edge(&mut self, target: u32, moves: MoveRange) -> EdgeId {
        self.side_table.push_edge(Edge { target, moves })
    }

    /// Return one switch-case range from the pool.
    pub(super) fn switch_case_range(
        &mut self,
        block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
        block_parameter: &[Vec<mir::Value>],
        cases: &[mir::SwitchCase],
    ) -> Result<SwitchCasesId> {
        let frame_layout = self.frame_layout;
        let cases = switch_case_range(
            frame_layout,
            &mut self.move_pair,
            block_index_map,
            block_parameter,
            cases,
        )?;

        Ok(self.side_table.push_switch_cases(cases))
    }

    /// Return one switch-table range from the pool.
    pub(super) fn switch_table_range(
        &mut self,
        block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
        block_parameter: &[Vec<mir::Value>],
        cases: &[mir::SwitchCase],
        default_target: u32,
        default_moves: MoveRange,
    ) -> Result<Option<SwitchTableId>> {
        let frame_layout = self.frame_layout;
        let table = switch_table_range(
            frame_layout,
            &mut self.move_pair,
            block_index_map,
            block_parameter,
            cases,
            default_target,
            default_moves,
        )?;

        Ok(table.map(|(min, cases)| {
            self.side_table
                .push_switch_table(SwitchTable { min, cases })
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
    pub(super) fn tensor_dot(&mut self, dimensions: mir::TensorDotDimensionNumbers) -> TensorDotId {
        self.side_table.push_tensor_dot(dimensions)
    }

    /// Return one pooled tensor convolution dimension descriptor id.
    pub(super) fn tensor_convolution(
        &mut self,
        dimensions: mir::TensorConvolutionDimensionNumbers,
    ) -> TensorConvolutionId {
        self.side_table.push_tensor_convolution(dimensions)
    }

    /// Return one pooled tensor convolution window descriptor id.
    pub(super) fn tensor_window(&mut self, window: mir::TensorConvolutionWindow) -> TensorWindowId {
        self.side_table.push_tensor_window(window)
    }

    /// Return one pooled tensor gather descriptor id.
    pub(super) fn tensor_gather(
        &mut self,
        dimensions: mir::TensorGatherDimensionNumbers,
    ) -> TensorGatherId {
        self.side_table.push_tensor_gather(dimensions)
    }

    /// Return one pooled tensor scatter descriptor id.
    pub(super) fn tensor_scatter(
        &mut self,
        dimensions: mir::TensorScatterDimensionNumbers,
    ) -> TensorScatterId {
        self.side_table.push_tensor_scatter(dimensions)
    }

    /// Return one pooled tensor layout id.
    pub(super) fn tensor_layout(&mut self, layout: TensorLayout) -> TensorLayoutId {
        self.side_table.push_tensor_layout(layout)
    }
}

/// Return one argument range from the pool.
fn argument_range(pool: &mut Vec<mir::Value>, arguments: &[mir::Value]) -> ArgumentRange {
    // empty argument range
    if arguments.is_empty() {
        return ArgumentRange::empty();
    }

    // compute range start
    let start = pool.len();

    // validate bounds in debug builds
    debug_assert!(
        start + arguments.len() <= u32::MAX as usize,
        "argument pool overflow"
    );

    // append arguments
    pool.extend_from_slice(arguments);

    // return range
    ArgumentRange {
        start: start as u32,
        len: arguments.len() as u32,
    }
}

/// Return one move range from the pool.
fn move_range(
    frame_layout: &engine::FrameLayout,
    pool: &mut Vec<MovePair>,
    parameters: &[mir::Value],
    arguments: &[mir::Value],
) -> Result<MoveRange> {
    // empty move range
    if parameters.is_empty() {
        return Ok(MoveRange::empty());
    }

    // compute range start
    let start = pool.len();

    // validate bounds in debug builds
    debug_assert!(
        start + parameters.len() <= u32::MAX as usize,
        "move pool overflow"
    );

    // append move pairs
    for (index, param) in parameters.iter().enumerate() {
        let source = move_source(frame_layout, arguments, index)?;
        let dest = move_slot(frame_layout, *param)?;

        pool.push(MovePair { dest, source });
    }

    // return range
    Ok(MoveRange {
        start: start as u32,
        len: parameters.len() as u32,
    })
}

/// Return one parameter move range from the pool.
fn parameter_move_range(
    frame_layout: &engine::FrameLayout,
    pool: &mut Vec<MovePair>,
    parameters: &[mir::Parameter],
    arguments: &[mir::Value],
) -> Result<MoveRange> {
    // empty move range
    if parameters.is_empty() {
        return Ok(MoveRange::empty());
    }

    // compute range start
    let start = pool.len();

    // validate bounds in debug builds
    debug_assert!(
        start + parameters.len() <= u32::MAX as usize,
        "move pool overflow"
    );

    // append move pairs
    for (index, param) in parameters.iter().enumerate() {
        let parameter = (param.value)
            .value()
            .ok_or_else(|| Error::invalid_program("function parameter value"))?;
        let source = move_source(frame_layout, arguments, index)?;
        let dest = move_slot(frame_layout, parameter)?;

        pool.push(MovePair { dest, source });
    }

    // return range
    Ok(MoveRange {
        start: start as u32,
        len: parameters.len() as u32,
    })
}

/// Resolve one call target for the given function id.
pub(super) fn lookup_call_target(
    call_targets: &HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
    function: mir::LocalNodeId<mir::Function>,
) -> Option<CallTarget> {
    call_targets.get(&function).copied()
}

/// Return one switch-case range from the pool.
fn switch_case_range(
    frame_layout: &engine::FrameLayout,
    move_pool: &mut Vec<MovePair>,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    block_parameters: &[Vec<mir::Value>],
    cases: &[mir::SwitchCase],
) -> Result<Box<[SwitchCase]>> {
    let mut lowered_cases = Vec::with_capacity(cases.len());

    // append cases
    for case in cases {
        let target = (case.target.block)
            .block()
            .ok_or_else(|| Error::invalid_program("switch case target"))?;
        let target_index = block_index_map[&target];
        let target_parameters = block_parameters[target_index].as_slice();
        let arguments = case
            .target
            .arguments
            .iter()
            .map(|argument| {
                (*argument)
                    .value()
                    .ok_or_else(|| Error::invalid_program("switch case argument"))
            })
            .collect::<Result<Vec<_>>>()?;
        let moves = move_range(frame_layout, move_pool, target_parameters, &arguments)?;
        lowered_cases.push(SwitchCase {
            value: (case.value)
                .integer()
                .ok_or_else(|| Error::invalid_program("switch case value"))?,
            target: target_index as u32,
            moves,
        });
    }

    Ok(lowered_cases.into_boxed_slice())
}

/// Return one switch-table range when density is high enough.
fn switch_table_range(
    frame_layout: &engine::FrameLayout,
    move_pool: &mut Vec<MovePair>,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    block_parameters: &[Vec<mir::Value>],
    cases: &[mir::SwitchCase],
    default_target: u32,
    default_moves: MoveRange,
) -> Result<Option<(i128, Box<[SwitchCase]>)>> {
    // bail if there are no cases
    if cases.is_empty() {
        return Ok(None);
    }

    // compute min and max case values
    let mut min_value = (cases[0].value)
        .integer()
        .ok_or_else(|| Error::invalid_program("switch table min value"))?;
    let mut max_value = min_value;
    for case in cases {
        let value = (case.value)
            .integer()
            .ok_or_else(|| Error::invalid_program("switch table case value"))?;
        min_value = min_value.min(value);
        max_value = max_value.max(value);
    }

    // compute dense table range
    let range_len = max_value - min_value + 1;
    if range_len <= 0 {
        return Ok(None);
    }
    if range_len > u32::MAX as i128 {
        return Ok(None);
    }

    // require at least one explicit case for every hole
    let range_len = range_len as usize;
    let max_range_len = cases.len() * 2;
    if range_len > max_range_len {
        return Ok(None);
    }

    let mut table = Vec::with_capacity(range_len);

    // seed with default targets
    for offset in 0..range_len {
        let value = min_value + offset as i128;
        table.push(SwitchCase {
            value,
            target: default_target,
            moves: default_moves,
        });
    }

    // populate explicit cases
    for case in cases {
        let case_value = (case.value)
            .integer()
            .ok_or_else(|| Error::invalid_program("switch table case value"))?;
        let target = (case.target.block)
            .block()
            .ok_or_else(|| Error::invalid_program("switch table target"))?;
        let target_index = block_index_map[&target];
        let target_parameters = block_parameters[target_index].as_slice();
        let arguments = case
            .target
            .arguments
            .iter()
            .map(|argument| {
                (*argument)
                    .value()
                    .ok_or_else(|| Error::invalid_program("switch table argument"))
            })
            .collect::<Result<Vec<_>>>()?;
        let moves = move_range(frame_layout, move_pool, target_parameters, &arguments)?;
        let offset = (case_value - min_value) as usize;
        let entry = &mut table[offset];
        entry.value = case_value;
        entry.target = target_index as u32;
        entry.moves = moves;
    }

    // return table range
    Ok(Some((min_value, table.into_boxed_slice())))
}

/// Return the lowered move source for one argument index.
fn move_source(
    frame_layout: &engine::FrameLayout,
    arguments: &[mir::Value],
    index: usize,
) -> Result<MoveSource> {
    let Some(value) = arguments.get(index) else {
        return Ok(MoveSource::Void);
    };

    Ok(MoveSource::Slot(move_slot(frame_layout, *value)?))
}

/// Return the lowered frame slot for one SSA value.
fn move_slot(frame_layout: &engine::FrameLayout, value: mir::Value) -> Result<MoveSlot> {
    let slot = frame_layout
        .value(value.0)
        .ok_or(Error::invalid_instruction())?;

    Ok(MoveSlot {
        layout: slot.layout,
        offset: slot.offset,
        byte_len: slot.byte_len,
        is_word: slot.is_word,
    })
}

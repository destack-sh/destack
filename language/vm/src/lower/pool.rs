use std::collections::HashMap;

use destack_mir as mir;

use crate::program::{
    ArgumentRange, CallTarget, INVALID_VALUE_ID, MovePair, MoveRange, SwitchCase,
};
use crate::{Error, Result};

/// Return the lowered move source for one argument index.
fn move_source(arguments: &[mir::Value], index: usize) -> u32 {
    arguments
        .get(index)
        .map(|value| value.0)
        .unwrap_or(INVALID_VALUE_ID)
}

/// One lowering pool for shared variable-length lowering data.
pub(super) struct Pool {
    /// The pooled argument values.
    argument: Vec<mir::Value>,
    /// The pooled move pairs.
    move_pair: Vec<MovePair>,
}

impl Pool {
    /// Create one empty lowering pool.
    pub(super) fn new() -> Self {
        Self {
            argument: Vec::new(),
            move_pair: Vec::new(),
        }
    }

    /// Finish the pool.
    pub(super) fn finish(self) -> (Vec<mir::Value>, Vec<MovePair>) {
        (self.argument, self.move_pair)
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
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: context.to_string(),
                    })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(argument_range(&mut self.argument, &arguments))
    }

    /// Return one move range from the pool.
    pub(super) fn move_range(
        &mut self,
        parameters: &[mir::Value],
        arguments: &[mir::Value],
    ) -> MoveRange {
        move_range(&mut self.move_pair, parameters, arguments)
    }

    /// Return one parameter move range from the pool.
    pub(super) fn parameter_move_range(
        &mut self,
        parameters: &[mir::Parameter],
        arguments: &[mir::Value],
    ) -> Result<MoveRange> {
        parameter_move_range(&mut self.move_pair, parameters, arguments)
    }

    /// Return one block edge move plan from the pool.
    pub(super) fn edge_move_plan(
        &mut self,
        parameters: &[mir::Value],
        arguments: &[mir::Value],
    ) -> MoveRange {
        move_range(&mut self.move_pair, parameters, arguments)
    }

    /// Return one switch-case range from the pool.
    pub(super) fn switch_case_range(
        &mut self,
        block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
        block_parameter: &[Vec<mir::Value>],
        cases: &[mir::SwitchCase],
    ) -> Result<Box<[SwitchCase]>> {
        switch_case_range(&mut self.move_pair, block_index_map, block_parameter, cases)
    }

    /// Return one switch-table range from the pool.
    pub(super) fn switch_table_range(
        &mut self,
        block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
        block_parameter: &[Vec<mir::Value>],
        cases: &[mir::SwitchCase],
        default_target: u32,
        default_moves: MoveRange,
    ) -> Result<Option<(i128, Box<[SwitchCase]>)>> {
        switch_table_range(
            &mut self.move_pair,
            block_index_map,
            block_parameter,
            cases,
            default_target,
            default_moves,
        )
    }
}

/// Return one argument range from the pool.
fn argument_range(pool: &mut Vec<mir::Value>, arguments: &[mir::Value]) -> ArgumentRange {
    // empty argument range
    if arguments.is_empty() {
        return ArgumentRange::empty();
    }

    // detect contiguous argument ids
    let mut is_contiguous = true;
    let contiguous_start = arguments[0].0;
    for (offset, argument) in arguments.iter().enumerate() {
        let expected = contiguous_start + offset as u32;
        if argument.0 != expected {
            is_contiguous = false;
            break;
        }
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
        is_contiguous,
        contiguous_start: if is_contiguous { contiguous_start } else { 0 },
    }
}

/// Return one move range from the pool.
fn move_range(
    pool: &mut Vec<MovePair>,
    parameters: &[mir::Value],
    arguments: &[mir::Value],
) -> MoveRange {
    // empty move range
    if parameters.is_empty() {
        return MoveRange::empty();
    }

    // detect contiguous move pairs
    let mut is_contiguous = true;
    let mut contiguous_src = 0;
    let mut contiguous_dest = 0;

    // compute range start
    let start = pool.len();

    // validate bounds in debug builds
    debug_assert!(
        start + parameters.len() <= u32::MAX as usize,
        "move pool overflow"
    );

    // append move pairs
    for (index, param) in parameters.iter().enumerate() {
        let src = move_source(arguments, index);
        if index == 0 {
            contiguous_dest = param.0;
            contiguous_src = src;
            if src == INVALID_VALUE_ID {
                is_contiguous = false;
            }
        } else if is_contiguous {
            let expected_src = contiguous_src + index as u32;
            let expected_dest = contiguous_dest + index as u32;
            if src != expected_src || param.0 != expected_dest {
                is_contiguous = false;
            }
        }
        pool.push(MovePair { dest: param.0, src });
    }

    // return range
    MoveRange {
        start: start as u32,
        len: parameters.len() as u32,
        is_contiguous,
        contiguous_src: if is_contiguous { contiguous_src } else { 0 },
        contiguous_dest: if is_contiguous { contiguous_dest } else { 0 },
    }
}

/// Return one parameter move range from the pool.
fn parameter_move_range(
    pool: &mut Vec<MovePair>,
    parameters: &[mir::Parameter],
    arguments: &[mir::Value],
) -> Result<MoveRange> {
    // empty move range
    if parameters.is_empty() {
        return Ok(MoveRange::empty());
    }

    // detect contiguous move pairs
    let mut is_contiguous = true;
    let mut contiguous_src = 0;
    let mut contiguous_dest = 0;

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
            .ok_or_else(|| Error::MissingRepresentation {
                context: "function parameter value".to_string(),
            })?;
        let src = move_source(arguments, index);
        if index == 0 {
            contiguous_dest = parameter.0;
            contiguous_src = src;
            if src == INVALID_VALUE_ID {
                is_contiguous = false;
            }
        } else if is_contiguous {
            let expected_src = contiguous_src + index as u32;
            let expected_dest = contiguous_dest + index as u32;
            if src != expected_src || parameter.0 != expected_dest {
                is_contiguous = false;
            }
        }
        pool.push(MovePair {
            dest: parameter.0,
            src,
        });
    }

    // return range
    Ok(MoveRange {
        start: start as u32,
        len: parameters.len() as u32,
        is_contiguous,
        contiguous_src: if is_contiguous { contiguous_src } else { 0 },
        contiguous_dest: if is_contiguous { contiguous_dest } else { 0 },
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
            .ok_or_else(|| Error::MissingRepresentation {
                context: "switch case target".to_string(),
            })?;
        let target_index = block_index_map[&target];
        let target_parameters = block_parameters
            .get(target_index)
            .map(|params| params.as_slice())
            .unwrap_or_default();
        let arguments = case
            .target
            .arguments
            .iter()
            .map(|argument| {
                (*argument)
                    .value()
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: "switch case argument".to_string(),
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let moves = move_range(move_pool, target_parameters, &arguments);
        lowered_cases.push(SwitchCase {
            value: (case.value)
                .integer()
                .ok_or_else(|| Error::MissingRepresentation {
                    context: "switch case value".to_string(),
                })?,
            target: target_index as u32,
            moves,
        });
    }

    Ok(lowered_cases.into_boxed_slice())
}

/// Return one switch-table range when density is high enough.
fn switch_table_range(
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
        .ok_or_else(|| Error::MissingRepresentation {
            context: "switch table min value".to_string(),
        })?;
    let mut max_value = min_value;
    for case in cases {
        let value = (case.value)
            .integer()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "switch table case value".to_string(),
            })?;
        min_value = min_value.min(value);
        max_value = max_value.max(value);
    }

    // compute range length with overflow protection
    let range_len = max_value - min_value + 1;
    if range_len <= 0 {
        return Ok(None);
    }
    if range_len > u32::MAX as i128 {
        return Ok(None);
    }

    // require at least one explicit case for every hole
    let range_len = range_len as usize;
    let max_range_len = cases.len().saturating_mul(2);
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
            .ok_or_else(|| Error::MissingRepresentation {
                context: "switch table case value".to_string(),
            })?;
        let target = (case.target.block)
            .block()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "switch table target".to_string(),
            })?;
        let target_index = block_index_map[&target];
        let target_parameters = block_parameters
            .get(target_index)
            .map(|params| params.as_slice())
            .unwrap_or_default();
        let arguments = case
            .target
            .arguments
            .iter()
            .map(|argument| {
                (*argument)
                    .value()
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: "switch table argument".to_string(),
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let moves = move_range(move_pool, target_parameters, &arguments);
        let offset = (case_value - min_value) as usize;
        let entry = &mut table[offset];
        entry.value = case_value;
        entry.target = target_index as u32;
        entry.moves = moves;
    }

    // return table range
    Ok(Some((min_value, table.into_boxed_slice())))
}

use std::collections::HashMap;

use destack_mir as mir;

use crate::module::{
    ArgumentRange, CallTarget, CopyPair, CopyRange, INVALID_VALUE_ID, SwitchCase, SwitchRange,
};
use crate::{Error, Result};

// switch table density threshold
const SWITCH_TABLE_MIN_DENSITY: f64 = 0.5;
// cap the number of jump table entries
const SWITCH_TABLE_MAX_RANGE: usize = 2048;

/// Return the lowered copy source for one argument index.
fn copy_source(arguments: &[mir::Value], index: usize) -> u32 {
    arguments
        .get(index)
        .map(|value| value.0)
        .unwrap_or(INVALID_VALUE_ID)
}

/// One lowering pool for shared variable-length payloads.
pub(super) struct Pool {
    /// The pooled argument values.
    argument: Vec<mir::Value>,
    /// The pooled switch cases.
    switch_case: Vec<SwitchCase>,
    /// The pooled copy pairs.
    copy: Vec<CopyPair>,
}

impl Pool {
    /// Create one empty lowering pool.
    pub(super) fn new() -> Self {
        Self {
            argument: Vec::new(),
            switch_case: Vec::new(),
            copy: Vec::new(),
        }
    }

    /// Return the finished pool parts.
    pub(super) fn into_parts(self) -> (Vec<mir::Value>, Vec<SwitchCase>, Vec<CopyPair>) {
        (self.argument, self.switch_case, self.copy)
    }

    /// Return one argument range from the pool.
    pub(super) fn argument_range(&mut self, arguments: &[mir::Value]) -> ArgumentRange {
        argument_range(&mut self.argument, arguments)
    }

    /// Return one argument range from recoverable MIR references.
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
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: context.to_string(),
                    })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(argument_range(&mut self.argument, &arguments))
    }

    /// Return one copy range from the pool.
    pub(super) fn copy_range(
        &mut self,
        parameters: &[mir::Value],
        arguments: &[mir::Value],
    ) -> CopyRange {
        copy_range(&mut self.copy, parameters, arguments)
    }

    /// Return one parameter copy range from the pool.
    pub(super) fn parameter_copy_range(
        &mut self,
        parameters: &[mir::Parameter],
        arguments: &[mir::Value],
    ) -> Result<CopyRange> {
        parameter_copy_range(&mut self.copy, parameters, arguments)
    }

    /// Return one block-edge copy plan from the pool.
    pub(super) fn edge_copy_plan(
        &mut self,
        parameters: &[mir::Value],
        arguments: &[mir::Value],
    ) -> CopyRange {
        copy_range(&mut self.copy, parameters, arguments)
    }

    /// Return one switch-case range from the pool.
    pub(super) fn switch_case_range(
        &mut self,
        block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
        block_parameter: &[Vec<mir::Value>],
        cases: &[mir::SwitchCase],
    ) -> Result<SwitchRange> {
        switch_case_range(
            &mut self.switch_case,
            &mut self.copy,
            block_index_map,
            block_parameter,
            cases,
        )
    }

    /// Return one switch-table range from the pool.
    pub(super) fn switch_table_range(
        &mut self,
        block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
        block_parameter: &[Vec<mir::Value>],
        cases: &[mir::SwitchCase],
        default_target: u32,
        default_copies: CopyRange,
    ) -> Result<Option<(i64, SwitchRange)>> {
        switch_table_range(
            &mut self.switch_case,
            &mut self.copy,
            block_index_map,
            block_parameter,
            cases,
            default_target,
            default_copies,
        )
    }
}

/// Return one argument range from the pool.
fn argument_range(pool: &mut Vec<mir::Value>, arguments: &[mir::Value]) -> ArgumentRange {
    // fast path: no arguments
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

/// Return one copy range from the pool.
fn copy_range(
    pool: &mut Vec<CopyPair>,
    parameters: &[mir::Value],
    arguments: &[mir::Value],
) -> CopyRange {
    // fast path: no parameters
    if parameters.is_empty() {
        return CopyRange::empty();
    }

    // detect contiguous copy pairs
    let mut is_contiguous = true;
    let mut contiguous_src = 0;
    let mut contiguous_dest = 0;

    // compute range start
    let start = pool.len();

    // validate bounds in debug builds
    debug_assert!(
        start + parameters.len() <= u32::MAX as usize,
        "copy pool overflow"
    );

    // append copy pairs
    for (index, param) in parameters.iter().enumerate() {
        let src = copy_source(arguments, index);
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
        pool.push(CopyPair { dest: param.0, src });
    }

    // return range
    CopyRange {
        start: start as u32,
        len: parameters.len() as u32,
        is_contiguous,
        contiguous_src: if is_contiguous { contiguous_src } else { 0 },
        contiguous_dest: if is_contiguous { contiguous_dest } else { 0 },
    }
}

/// Return one parameter copy range from the pool.
fn parameter_copy_range(
    pool: &mut Vec<CopyPair>,
    parameters: &[mir::Parameter],
    arguments: &[mir::Value],
) -> Result<CopyRange> {
    // fast path: no parameters
    if parameters.is_empty() {
        return Ok(CopyRange::empty());
    }

    // detect contiguous copy pairs
    let mut is_contiguous = true;
    let mut contiguous_src = 0;
    let mut contiguous_dest = 0;

    // compute range start
    let start = pool.len();

    // validate bounds in debug builds
    debug_assert!(
        start + parameters.len() <= u32::MAX as usize,
        "copy pool overflow"
    );

    // append copy pairs
    for (index, param) in parameters.iter().enumerate() {
        let parameter = (param.value)
            .value()
            .ok_or_else(|| Error::ConcreteMirRequired {
                context: "function parameter value".to_string(),
            })?;
        let src = copy_source(arguments, index);
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
        pool.push(CopyPair {
            dest: parameter.0,
            src,
        });
    }

    // return range
    Ok(CopyRange {
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
    switch_case_pool: &mut Vec<SwitchCase>,
    copy_pool: &mut Vec<CopyPair>,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    block_parameters: &[Vec<mir::Value>],
    cases: &[mir::SwitchCase],
) -> Result<SwitchRange> {
    // fast path: no cases
    if cases.is_empty() {
        return Ok(SwitchRange::empty());
    }

    // compute range start
    let start = switch_case_pool.len();

    // validate bounds in debug builds
    debug_assert!(
        start + cases.len() <= u32::MAX as usize,
        "switch case pool overflow"
    );

    // append cases
    for case in cases {
        let target = (case.target.block)
            .block()
            .ok_or_else(|| Error::ConcreteMirRequired {
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
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "switch case argument".to_string(),
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let copies = copy_range(copy_pool, target_parameters, &arguments);
        switch_case_pool.push(SwitchCase {
            value: (case.value)
                .integer()
                .ok_or_else(|| Error::ConcreteMirRequired {
                    context: "switch case value".to_string(),
                })?,
            target: target_index as u32,
            copies,
        });
    }

    // return range
    Ok(SwitchRange {
        start: start as u32,
        len: cases.len() as u32,
    })
}

/// Return one switch-table range when density is high enough.
fn switch_table_range(
    switch_case_pool: &mut Vec<SwitchCase>,
    copy_pool: &mut Vec<CopyPair>,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
    block_parameters: &[Vec<mir::Value>],
    cases: &[mir::SwitchCase],
    default_target: u32,
    default_copies: CopyRange,
) -> Result<Option<(i64, SwitchRange)>> {
    // bail if there are no cases
    if cases.is_empty() {
        return Ok(None);
    }

    // compute min and max case values
    let mut min_value = (cases[0].value)
        .integer()
        .ok_or_else(|| Error::ConcreteMirRequired {
            context: "switch table min value".to_string(),
        })?;
    let mut max_value = min_value;
    for case in cases {
        let value = (case.value)
            .integer()
            .ok_or_else(|| Error::ConcreteMirRequired {
                context: "switch table case value".to_string(),
            })?;
        min_value = min_value.min(value);
        max_value = max_value.max(value);
    }

    // compute range length with overflow protection
    let range_len = i128::from(max_value) - i128::from(min_value) + 1;
    if range_len <= 0 {
        return Ok(None);
    }
    if range_len > SWITCH_TABLE_MAX_RANGE as i128 {
        return Ok(None);
    }
    if range_len > u32::MAX as i128 {
        return Ok(None);
    }

    // require sufficient density
    let range_len = range_len as usize;
    let density = cases.len() as f64 / range_len as f64;
    if density < SWITCH_TABLE_MIN_DENSITY {
        return Ok(None);
    }

    // reserve table slots
    let start = switch_case_pool.len();
    debug_assert!(
        start + range_len <= u32::MAX as usize,
        "switch case pool overflow"
    );

    // seed with default targets
    for offset in 0..range_len {
        let value = min_value + offset as i64;
        switch_case_pool.push(SwitchCase {
            value,
            target: default_target,
            copies: default_copies,
        });
    }

    // populate explicit cases
    for case in cases {
        let case_value = (case.value)
            .integer()
            .ok_or_else(|| Error::ConcreteMirRequired {
                context: "switch table case value".to_string(),
            })?;
        let target = (case.target.block)
            .block()
            .ok_or_else(|| Error::ConcreteMirRequired {
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
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "switch table argument".to_string(),
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let copies = copy_range(copy_pool, target_parameters, &arguments);
        let offset = (case_value - min_value) as usize;
        let slot = &mut switch_case_pool[start + offset];
        slot.value = case_value;
        slot.target = target_index as u32;
        slot.copies = copies;
    }

    // return table range
    Ok(Some((
        min_value,
        SwitchRange {
            start: start as u32,
            len: range_len as u32,
        },
    )))
}

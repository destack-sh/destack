use std::collections::{HashMap, HashSet};

use crate as mir;
use destack_core::{float_from_bits, float_to_bits};

use crate::{
    Analysis, BlockParamForwarding, FunctionAnalyses, TargetLayout, ValueDefinition,
    ValueDefinitions, constant_is_one, constant_is_zero, constant_zero_for_type,
    constant_zero_like, fold_binary, fold_cast, instruction_is_pure,
};

use super::{ControlFlowGraph, Loop, LoopAnalysis};

/// Symbolic expression for scalar evolution.
#[derive(Debug, Clone, PartialEq)]
pub enum Scev {
    /// A constant value.
    Constant(mir::Constant),
    /// A symbolic value that is not modeled.
    Unknown(mir::Value),
    /// A negated expression.
    Neg(Box<Scev>),
    /// Sum of two expressions.
    Add(Box<Scev>, Box<Scev>),
    /// Product of two expressions.
    Mul(Box<Scev>, Box<Scev>),
    /// Signed division of two expressions.
    SignedDivide(Box<Scev>, Box<Scev>),
    /// Unsigned division of two expressions.
    UnsignedDivide(Box<Scev>, Box<Scev>),
    /// Signed remainder of two expressions.
    SignedRemainder(Box<Scev>, Box<Scev>),
    /// Unsigned remainder of two expressions.
    UnsignedRemainder(Box<Scev>, Box<Scev>),
    /// Shift left by the right operand.
    ShiftLeft(Box<Scev>, Box<Scev>),
    /// Arithmetic shift right by the right operand.
    ArithmeticShiftRight(Box<Scev>, Box<Scev>),
    /// Logical shift right by the right operand.
    LogicalShiftRight(Box<Scev>, Box<Scev>),
    /// Zero extend to a wider integer width.
    ZeroExtend {
        /// Operand being extended.
        value: Box<Scev>,
        /// Target width.
        width: u16,
    },
    /// Sign extend to a wider integer width.
    SignExtend {
        /// Operand being extended.
        value: Box<Scev>,
        /// Target width.
        width: u16,
    },
    /// Truncate to a smaller integer width.
    Truncate {
        /// Operand being truncated.
        value: Box<Scev>,
        /// Target width.
        width: u16,
    },
    /// Additive recurrence for a loop header.
    AddRec {
        /// Start value for the recurrence.
        start: Box<Scev>,
        /// Step value added per iteration.
        step: Box<Scev>,
        /// Loop header that defines the recurrence.
        loop_header: mir::LocalNodeId<mir::Block>,
    },
}

impl Scev {
    /// Check if the expression is invariant with respect to a loop header.
    pub fn is_loop_invariant(&self, loop_header: mir::LocalNodeId<mir::Block>) -> bool {
        match self {
            Scev::Constant(_) => true,
            Scev::Unknown(_) => true,
            Scev::Neg(inner) => inner.is_loop_invariant(loop_header),
            Scev::Add(left, right) => {
                left.is_loop_invariant(loop_header) && right.is_loop_invariant(loop_header)
            }
            Scev::Mul(left, right) => {
                left.is_loop_invariant(loop_header) && right.is_loop_invariant(loop_header)
            }
            Scev::SignedDivide(left, right) => {
                left.is_loop_invariant(loop_header) && right.is_loop_invariant(loop_header)
            }
            Scev::UnsignedDivide(left, right) => {
                left.is_loop_invariant(loop_header) && right.is_loop_invariant(loop_header)
            }
            Scev::SignedRemainder(left, right) => {
                left.is_loop_invariant(loop_header) && right.is_loop_invariant(loop_header)
            }
            Scev::UnsignedRemainder(left, right) => {
                left.is_loop_invariant(loop_header) && right.is_loop_invariant(loop_header)
            }
            Scev::ShiftLeft(left, right) => {
                left.is_loop_invariant(loop_header) && right.is_loop_invariant(loop_header)
            }
            Scev::ArithmeticShiftRight(left, right) => {
                left.is_loop_invariant(loop_header) && right.is_loop_invariant(loop_header)
            }
            Scev::LogicalShiftRight(left, right) => {
                left.is_loop_invariant(loop_header) && right.is_loop_invariant(loop_header)
            }
            Scev::ZeroExtend { value, .. } => value.is_loop_invariant(loop_header),
            Scev::SignExtend { value, .. } => value.is_loop_invariant(loop_header),
            Scev::Truncate { value, .. } => value.is_loop_invariant(loop_header),
            Scev::AddRec {
                loop_header: header,
                ..
            } => *header != loop_header,
        }
    }
}

/// Scalar evolution results for a function.
#[derive(Debug)]
pub struct ScalarEvolution {
    /// SCEV expressions keyed by loop index and SSA value.
    loop_scev: HashMap<usize, HashMap<mir::Value, Scev>>,
}

impl ScalarEvolution {
    /// Build scalar evolution for a function.
    fn build(
        function: &mir::Function,
        tree: &mir::Tree,
        cfg: &ControlFlowGraph,
        loops: &LoopAnalysis,
        definitions: &ValueDefinitions,
        target_layout: TargetLayout,
    ) -> Self {
        // handle functions without bodies
        if function.entry().is_none() {
            return Self {
                loop_scev: HashMap::new(),
            };
        }

        // build block parameter forwarding
        let forwarding = BlockParamForwarding::build(function, tree, cfg);

        // compute per loop SCEV maps
        let mut loop_scev = HashMap::new();
        for (loop_index, lp) in loops.loops().iter().enumerate() {
            let mut builder = LoopScevBuilder::new(
                function,
                tree,
                cfg,
                lp,
                definitions,
                &forwarding,
                target_layout,
            );
            let scev_map = builder.build();
            loop_scev.insert(loop_index, scev_map);
        }

        Self { loop_scev }
    }

    /// Get the SCEV for a value in a specific loop.
    pub fn value_scev(&self, loop_index: usize, value: mir::Value) -> Option<&Scev> {
        self.loop_scev.get(&loop_index)?.get(&value)
    }

    /// Get all SCEVs for a loop.
    pub fn loop_scevs(&self, loop_index: usize) -> Option<&HashMap<mir::Value, Scev>> {
        self.loop_scev.get(&loop_index)
    }
}

impl Analysis for ScalarEvolution {}

impl ScalarEvolution {
    pub(crate) fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &mut FunctionAnalyses,
    ) -> Self {
        let cfg = analyses.control_flow(function, tree);
        let loops = analyses.loops(function, tree);
        let definitions = analyses.value_definitions(function, tree);

        Self::build(
            function,
            tree,
            &cfg,
            &loops,
            &definitions,
            analyses.target_layout(),
        )
    }
}

/// Builder for loop local scalar evolution expressions.
struct LoopScevBuilder<'a> {
    /// Function being analyzed.
    function: &'a mir::Function,
    /// MIR tree.
    tree: &'a mir::Tree,
    /// Control flow graph.
    cfg: &'a ControlFlowGraph,
    /// Loop being analyzed.
    lp: &'a Loop,
    /// Value definition tables.
    definitions: &'a ValueDefinitions,
    /// Block parameter forwarding information.
    forwarding: &'a BlockParamForwarding,
    /// Loop invariant values.
    invariants: HashSet<mir::Value>,
    /// Cached SCEV expressions.
    cache: HashMap<mir::Value, Scev>,
    /// Values currently being computed.
    in_progress: HashSet<mir::Value>,
    /// Type context for layout sensitive operations.
    target_layout: TargetLayout,
}

impl<'a> LoopScevBuilder<'a> {
    /// Create a new builder for a loop.
    fn new(
        function: &'a mir::Function,
        tree: &'a mir::Tree,
        cfg: &'a ControlFlowGraph,
        lp: &'a Loop,
        definitions: &'a ValueDefinitions,
        forwarding: &'a BlockParamForwarding,
        target_layout: TargetLayout,
    ) -> Self {
        let invariants = collect_loop_invariants(function, tree, lp, definitions);

        Self {
            function,
            tree,
            cfg,
            lp,
            definitions,
            forwarding,
            invariants,
            cache: HashMap::new(),
            in_progress: HashSet::new(),
            target_layout,
        }
    }

    /// Build the SCEV map for this loop.
    fn build(&mut self) -> HashMap<mir::Value, Scev> {
        let mut values = Vec::new();

        // collect values defined in the loop
        for &block_id in &self.lp.blocks {
            let block = self.tree.get(block_id);
            for param in &block.parameters {
                let value = param.value;

                values.push(value);
            }
            for &instruction_id in &block.instructions {
                let instruction = self.tree.get(instruction_id);
                if let Some(destination) = instruction.destination() {
                    values.push(destination);
                }
            }
        }

        // compute SCEV for each value
        for value in values {
            self.scev_for_value(value);
        }

        self.cache.clone()
    }

    /// Compute the SCEV for a value.
    fn scev_for_value(&mut self, value: mir::Value) -> Scev {
        // return cached values
        if let Some(scev) = self.cache.get(&value) {
            return scev.clone();
        }

        // break recursion cycles
        if self.in_progress.contains(&value) {
            return Scev::Unknown(value);
        }

        // mark value as in progress
        self.in_progress.insert(value);

        // compute expression for the value
        let scev = self.compute_scev(value);

        // finish computation and cache
        self.in_progress.remove(&value);
        self.cache.insert(value, scev.clone());

        scev
    }

    /// Compute the SCEV for a value without caching.
    fn compute_scev(&mut self, value: mir::Value) -> Scev {
        // use constants when available
        if let Some(constant) = constant_for_value(value, self.tree, self.definitions) {
            return Scev::Constant(constant);
        }

        // handle header parameters with induction patterns
        if let Some(param_index) = self.header_param_index(value)
            && let Some(addrec) = self.addrec_for_param(value, param_index)
        {
            return addrec;
        }

        // treat values defined outside the loop as unknown invariants
        if !self.value_defined_in_loop(value) {
            return Scev::Unknown(value);
        }

        // handle instruction defined values
        let Some(definition) = self.definitions.definition(value) else {
            return Scev::Unknown(value);
        };

        let ValueDefinition::Instruction { instruction, .. } = definition else {
            return Scev::Unknown(value);
        };

        let instruction_data = self.tree.get(instruction);
        match instruction_data {
            mir::Instruction::Binary {
                operator,
                left,
                right,
                ..
            } => {
                let left = *left;
                let right = *right;

                self.scev_for_binary(value, *operator, left, right)
            }
            mir::Instruction::Unary {
                operator, argument, ..
            } => {
                let argument = *argument;

                self.scev_for_unary(value, *operator, argument)
            }
            mir::Instruction::Cast {
                operator,
                argument,
                to_type,
                ..
            } => {
                let argument = *argument;
                let to_type = *to_type;

                self.scev_for_cast(value, *operator, argument, to_type)
            }
            _ => Scev::Unknown(value),
        }
    }

    /// Compute a SCEV for a binary operation.
    fn scev_for_binary(
        &mut self,
        destination: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    ) -> Scev {
        // require the integer representation modeled by scalar evolution
        let ty = self.function.expect_value_type(left);
        let Some((_, is_signed)) = self
            .tree
            .get(ty)
            .int_info_with_pointer_width(self.target_layout.pointer_bits())
        else {
            return Scev::Unknown(destination);
        };

        // dispatch based on operator
        match operator {
            mir::BinaryOperator::Add => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);
                scev_add(left_scev, right_scev)
            }
            mir::BinaryOperator::Subtract => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);
                scev_sub(left_scev, right_scev)
            }
            mir::BinaryOperator::Multiply => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);

                // allow invariant multipliers to keep linear recurrences
                if let Scev::AddRec {
                    start,
                    step,
                    loop_header,
                } = &left_scev
                    && self.is_invariant_value(right)
                {
                    let invariant = right_scev.clone();
                    let start = scev_mul(start.as_ref().clone(), invariant.clone());
                    let step = scev_mul(step.as_ref().clone(), invariant);
                    return Scev::AddRec {
                        start: Box::new(start),
                        step: Box::new(step),
                        loop_header: *loop_header,
                    };
                }

                // allow invariant multipliers on the right side as well
                if let Scev::AddRec {
                    start,
                    step,
                    loop_header,
                } = &right_scev
                    && self.is_invariant_value(left)
                {
                    let invariant = left_scev.clone();
                    let start = scev_mul(start.as_ref().clone(), invariant.clone());
                    let step = scev_mul(step.as_ref().clone(), invariant);
                    return Scev::AddRec {
                        start: Box::new(start),
                        step: Box::new(step),
                        loop_header: *loop_header,
                    };
                }

                scev_mul(left_scev, right_scev)
            }
            mir::BinaryOperator::Divide => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);
                if is_signed {
                    scev_signed_divide(left_scev, right_scev)
                } else {
                    scev_unsigned_divide(left_scev, right_scev)
                }
            }
            mir::BinaryOperator::Remainder => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);
                if is_signed {
                    scev_signed_remainder(left_scev, right_scev)
                } else {
                    scev_unsigned_remainder(left_scev, right_scev)
                }
            }
            mir::BinaryOperator::ShiftLeft => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);
                scev_shift_left(left_scev, right_scev)
            }
            mir::BinaryOperator::ShiftRight => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);
                if is_signed {
                    scev_arithmetic_shift_right(left_scev, right_scev)
                } else {
                    scev_logical_shift_right(left_scev, right_scev)
                }
            }
            mir::BinaryOperator::UnsignedShiftRight => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);
                scev_logical_shift_right(left_scev, right_scev)
            }
            _ => Scev::Unknown(destination),
        }
    }

    /// Compute a SCEV for a unary operation.
    fn scev_for_unary(
        &mut self,
        destination: mir::Value,
        operator: mir::UnaryOperator,
        argument: mir::Value,
    ) -> Scev {
        // compute unary expression from operand
        let argument_scev = self.scev_for_value(argument);

        match operator {
            mir::UnaryOperator::Negate => scev_negate(argument_scev),
            _ => Scev::Unknown(destination),
        }
    }

    /// Compute a SCEV for a cast operation.
    fn scev_for_cast(
        &mut self,
        destination: mir::Value,
        operator: mir::CastOperator,
        argument: mir::Value,
        to_type: mir::LocalNodeId<mir::Type>,
    ) -> Scev {
        // compute the operand expression
        let argument_scev = self.scev_for_value(argument);

        // fold constant casts when possible
        if let Scev::Constant(constant) = &argument_scev
            && let Some(result) = fold_cast(
                operator,
                constant.clone(),
                to_type,
                self.target_layout.pointer_bits(),
                self.tree,
            )
        {
            return Scev::Constant(result);
        }

        // read the target type
        let target_type = self.tree.get(to_type);
        let Some((width, _)) =
            target_type.int_info_with_pointer_width(self.target_layout.pointer_bits())
        else {
            return Scev::Unknown(destination);
        };

        match operator {
            mir::CastOperator::ZeroExtend => Scev::ZeroExtend {
                value: Box::new(argument_scev),
                width,
            },
            mir::CastOperator::SignExtend => Scev::SignExtend {
                value: Box::new(argument_scev),
                width,
            },
            mir::CastOperator::Truncate => Scev::Truncate {
                value: Box::new(argument_scev),
                width,
            },
            _ => Scev::Unknown(destination),
        }
    }

    /// Check if a value is defined inside the loop.
    fn value_defined_in_loop(&self, value: mir::Value) -> bool {
        // treat missing definitions as not in loop
        let definition = match self.definitions.definition(value) {
            Some(definition) => definition,
            None => return false,
        };

        definition
            .block()
            .is_some_and(|block| self.lp.blocks.contains(&block))
    }

    /// Get the index of a header parameter if this value is one.
    fn header_param_index(&self, value: mir::Value) -> Option<usize> {
        // require a header parameter definition
        let definition = self.definitions.definition(value)?;
        let ValueDefinition::BlockParameter { block, index } = definition else {
            return None;
        };

        if block != self.lp.header {
            return None;
        }

        Some(index)
    }

    /// Compute an AddRec for a loop header parameter when possible.
    fn addrec_for_param(&mut self, value: mir::Value, param_index: usize) -> Option<Scev> {
        // find outside predecessors that provide the initial value
        let outside_preds: Vec<_> = self
            .cfg
            .predecessors(self.lp.header)
            .iter()
            .copied()
            .filter(|pred| !self.lp.blocks.contains(pred))
            .collect();

        if outside_preds.is_empty() {
            return None;
        }

        // get the initial argument from outside preds
        let mut outside_args = Vec::new();
        for pred in &outside_preds {
            let arg = header_argument_from_pred(self.tree, *pred, self.lp.header, param_index)?;
            outside_args.push(arg);
        }

        let first_outside = *outside_args.first()?;
        if outside_args.iter().any(|&arg| arg != first_outside) {
            return None;
        }

        // compute the step from each latch
        let mut latch_steps = Vec::new();
        for &latch in &self.lp.latches {
            let latch_arg =
                header_argument_from_pred(self.tree, latch, self.lp.header, param_index)?;
            let step = self.step_from_latch(value, latch_arg, param_index)?;
            latch_steps.push(step);
        }

        let first_step = latch_steps.first()?.clone();
        if latch_steps.iter().skip(1).any(|step| *step != first_step) {
            return None;
        }

        let start_scev = self.scev_for_value(first_outside);
        Some(Scev::AddRec {
            start: Box::new(start_scev),
            step: Box::new(first_step),
            loop_header: self.lp.header,
        })
    }

    /// Compute the step value for a latch argument.
    fn step_from_latch(
        &mut self,
        param_value: mir::Value,
        latch_arg: mir::Value,
        param_index: usize,
    ) -> Option<Scev> {
        // resolve block parameter forwarding
        let latch_arg = self.forwarding.resolve(latch_arg);

        // build a zero step for the parameter type
        let step_zero = zero_constant_for_param(
            self.tree,
            self.lp.header,
            param_index,
            self.target_layout.pointer_bits(),
        )?;
        let step_zero = Scev::Constant(step_zero);

        // extract an additive step from the latch expression
        self.step_from_expression(param_value, latch_arg, &step_zero)
    }

    /// Extract an additive step for an expression of the form param plus step.
    fn step_from_expression(
        &mut self,
        param_value: mir::Value,
        value: mir::Value,
        step_zero: &Scev,
    ) -> Option<Scev> {
        // resolve forwarded block parameters
        let value = self.forwarding.resolve(value);

        // param itself represents a zero step
        if value == param_value {
            return Some(step_zero.clone());
        }

        // require instruction definition
        let definition = self.definitions.definition(value)?;
        let ValueDefinition::Instruction { instruction, .. } = definition else {
            return None;
        };

        let instruction_data = self.tree.get(instruction);
        let mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } = instruction_data
        else {
            return None;
        };

        // derive the step for supported additive chains
        match operator {
            mir::BinaryOperator::Add => {
                let left = *left;
                let right = *right;

                if self.is_invariant_value(right)
                    && let Some(step) = self.step_from_expression(param_value, left, step_zero)
                {
                    let rhs = self.scev_for_value(right);
                    return Some(scev_add(step, rhs));
                }

                if self.is_invariant_value(left)
                    && let Some(step) = self.step_from_expression(param_value, right, step_zero)
                {
                    let lhs = self.scev_for_value(left);
                    return Some(scev_add(step, lhs));
                }

                None
            }
            mir::BinaryOperator::Subtract => {
                let left = *left;
                let right = *right;

                if self.is_invariant_value(right)
                    && let Some(step) = self.step_from_expression(param_value, left, step_zero)
                {
                    let rhs = self.scev_for_value(right);
                    return Some(scev_sub(step, rhs));
                }

                None
            }
            _ => None,
        }
    }

    /// Check if a value is loop invariant.
    fn is_invariant_value(&self, value: mir::Value) -> bool {
        // use fixed invariant set
        self.invariants.contains(&value)
    }
}

/// Collect loop invariant values using a fixed point scan.
fn collect_loop_invariants(
    function: &mir::Function,
    tree: &mir::Tree,
    lp: &Loop,
    definitions: &ValueDefinitions,
) -> HashSet<mir::Value> {
    let mut invariants = HashSet::new();

    // seed invariants with function parameters
    for param in &function.parameters {
        let value = param.value;

        invariants.insert(value);
    }

    // seed invariants with values defined outside the loop
    for (value, definition) in definitions.definitions() {
        if definition
            .block()
            .is_some_and(|block| !lp.blocks.contains(&block))
        {
            invariants.insert(value);
        }
    }

    // expand invariants by scanning loop blocks
    let mut changed = true;
    while changed {
        changed = false;

        for &block_id in &lp.blocks {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                let Some(destination) = instruction.destination() else {
                    continue;
                };

                if invariants.contains(&destination) {
                    continue;
                }

                if !instruction_is_pure(instruction) {
                    continue;
                }

                if instruction_uses_invariants(instruction, tree, &invariants) {
                    invariants.insert(destination);
                    changed = true;
                }
            }
        }
    }

    invariants
}

/// Check if all operands of an instruction are loop invariant.
fn instruction_uses_invariants(
    instruction: &mir::Instruction,
    tree: &mir::Tree,
    invariants: &HashSet<mir::Value>,
) -> bool {
    // check inline operands
    for value in instruction.uses() {
        if !invariants.contains(&value) {
            return false;
        }
    }

    // check externalized operands
    let Some(args_slice) = instruction.argument_slice() else {
        return true;
    };

    tree.get_values(args_slice)
        .iter()
        .all(|value| invariants.contains(value))
}

/// Get the header argument for a specific predecessor and parameter.
fn header_argument_from_pred(
    tree: &mir::Tree,
    pred: mir::LocalNodeId<mir::Block>,
    header: mir::LocalNodeId<mir::Block>,
    param_index: usize,
) -> Option<mir::Value> {
    // collect predecessor arguments for the header edge
    let pred_block = tree.get(pred);
    let pred_terminator = tree.get(pred_block.terminator);
    let args = pred_terminator.successor_arguments(tree, header);
    let parameters = pred_terminator.successor_parameters(tree, header);
    let header_block = tree.get(header);
    let parameter = header_block.parameters.get(param_index)?;

    parameters
        .iter()
        .zip(args)
        .find(|(candidate, _)| candidate.value == parameter.value)
        .map(|(_, argument)| *argument)
}

/// Get a constant value for an SSA value if it is constant.
fn constant_for_value(
    value: mir::Value,
    tree: &mir::Tree,
    definitions: &ValueDefinitions,
) -> Option<mir::Constant> {
    // require an instruction definition
    let definition = definitions.definition(value)?;
    let ValueDefinition::Instruction { instruction, .. } = definition else {
        return None;
    };

    // match supported constant sources
    let instruction_data = tree.get(instruction);
    match instruction_data {
        mir::Instruction::Const { value, .. } => Some(value.clone()),
        _ => None,
    }
}

/// Build a zero constant for a parameter type.
fn zero_constant_for_param(
    tree: &mir::Tree,
    header: mir::LocalNodeId<mir::Block>,
    param_index: usize,
    pointer_width_bits: u16,
) -> Option<mir::Constant> {
    // resolve the parameter type
    let header_block = tree.get(header);
    let param = header_block.parameters.get(param_index)?;
    let ty = tree.get(Some(param.ty)?);
    constant_zero_for_type(ty, pointer_width_bits)
}

/// Build an additive SCEV with basic simplifications.
fn scev_add(left: Scev, right: Scev) -> Scev {
    // fold constant additions when possible
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && let Some(result) = fold_binary(
            mir::BinaryOperator::Add,
            left_const.clone(),
            right_const.clone(),
        )
    {
        return Scev::Constant(result);
    }

    // drop zero terms
    if let Scev::Constant(constant) = &left
        && constant_is_zero(Some(constant))
    {
        return right;
    }

    if let Scev::Constant(constant) = &right
        && constant_is_zero(Some(constant))
    {
        return left;
    }

    match (left, right) {
        (
            Scev::AddRec {
                start,
                step,
                loop_header,
            },
            Scev::Constant(constant),
        ) => {
            let adjusted_start = scev_add(*start, Scev::Constant(constant));
            Scev::AddRec {
                start: Box::new(adjusted_start),
                step,
                loop_header,
            }
        }
        (
            Scev::Constant(constant),
            Scev::AddRec {
                start,
                step,
                loop_header,
            },
        ) => {
            let adjusted_start = scev_add(*start, Scev::Constant(constant));
            Scev::AddRec {
                start: Box::new(adjusted_start),
                step,
                loop_header,
            }
        }
        (
            Scev::AddRec {
                start: left_start,
                step: left_step,
                loop_header: left_header,
            },
            Scev::AddRec {
                start: right_start,
                step: right_step,
                loop_header: right_header,
            },
        ) if left_header == right_header => {
            let start = scev_add(*left_start, *right_start);
            let step = scev_add(*left_step, *right_step);
            Scev::AddRec {
                start: Box::new(start),
                step: Box::new(step),
                loop_header: left_header,
            }
        }
        (left, right) => Scev::Add(Box::new(left), Box::new(right)),
    }
}

/// Build a subtractive SCEV with basic simplifications.
fn scev_sub(left: Scev, right: Scev) -> Scev {
    // fold constant subtraction when possible
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && let Some(result) = fold_binary(
            mir::BinaryOperator::Subtract,
            left_const.clone(),
            right_const.clone(),
        )
    {
        return Scev::Constant(result);
    }

    scev_add(left, scev_negate(right))
}

/// Build a multiplicative SCEV with basic simplifications.
fn scev_mul(left: Scev, right: Scev) -> Scev {
    // fold constant multiplication when possible
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && let Some(result) = fold_binary(
            mir::BinaryOperator::Multiply,
            left_const.clone(),
            right_const.clone(),
        )
    {
        return Scev::Constant(result);
    }

    // drop zero terms
    if let Scev::Constant(constant) = &left
        && constant_is_zero(Some(constant))
    {
        return left;
    }

    if let Scev::Constant(constant) = &right
        && constant_is_zero(Some(constant))
    {
        return right;
    }

    // drop one terms
    if let Scev::Constant(constant) = &left
        && constant_is_one(Some(constant))
    {
        return right;
    }

    if let Scev::Constant(constant) = &right
        && constant_is_one(Some(constant))
    {
        return left;
    }

    match (left, right) {
        (
            Scev::AddRec {
                start,
                step,
                loop_header,
            },
            Scev::Constant(constant),
        ) => {
            let multiplier = Scev::Constant(constant.clone());
            let start = scev_mul(*start, multiplier.clone());
            let step = scev_mul(*step, multiplier);
            Scev::AddRec {
                start: Box::new(start),
                step: Box::new(step),
                loop_header,
            }
        }
        (
            Scev::Constant(constant),
            Scev::AddRec {
                start,
                step,
                loop_header,
            },
        ) => {
            let multiplier = Scev::Constant(constant.clone());
            let start = scev_mul(*start, multiplier.clone());
            let step = scev_mul(*step, multiplier);
            Scev::AddRec {
                start: Box::new(start),
                step: Box::new(step),
                loop_header,
            }
        }
        (left, right) => Scev::Mul(Box::new(left), Box::new(right)),
    }
}

/// Build a signed division SCEV with basic simplifications.
fn scev_signed_divide(left: Scev, right: Scev) -> Scev {
    // fold constant division when possible
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && let Some(result) = fold_binary(
            mir::BinaryOperator::Divide,
            left_const.clone(),
            right_const.clone(),
        )
    {
        return Scev::Constant(result);
    }

    // divide by one preserves the left operand
    if let Scev::Constant(constant) = &right
        && constant_is_one(Some(constant))
    {
        return left;
    }

    // zero divided by a nonzero constant stays zero
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && constant_is_zero(Some(left_const))
        && !constant_is_zero(Some(right_const))
    {
        return Scev::Constant(constant_zero_like(left_const));
    }

    // divide by negative one uses wrapping negation
    if let Scev::Constant(mir::Constant::Int {
        value: -1,
        is_signed: true,
        ..
    }) = &right
    {
        return scev_negate(left);
    }

    Scev::SignedDivide(Box::new(left), Box::new(right))
}

/// Build an unsigned division SCEV with basic simplifications.
fn scev_unsigned_divide(left: Scev, right: Scev) -> Scev {
    // fold constant division when possible
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && let Some(result) = fold_binary(
            mir::BinaryOperator::Divide,
            left_const.clone(),
            right_const.clone(),
        )
    {
        return Scev::Constant(result);
    }

    // divide by one preserves the left operand
    if let Scev::Constant(constant) = &right
        && constant_is_one(Some(constant))
    {
        return left;
    }

    // zero divided by a nonzero constant stays zero
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && constant_is_zero(Some(left_const))
        && !constant_is_zero(Some(right_const))
    {
        return Scev::Constant(constant_zero_like(left_const));
    }

    Scev::UnsignedDivide(Box::new(left), Box::new(right))
}

/// Build a signed remainder SCEV with basic simplifications.
fn scev_signed_remainder(left: Scev, right: Scev) -> Scev {
    // fold constant remainder when possible
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && let Some(result) = fold_binary(
            mir::BinaryOperator::Remainder,
            left_const.clone(),
            right_const.clone(),
        )
    {
        return Scev::Constant(result);
    }

    // remainder by one yields zero
    if let Scev::Constant(constant) = &right
        && constant_is_one(Some(constant))
    {
        return Scev::Constant(constant_zero_like(constant));
    }

    // remainder by negative one yields zero for signed integers
    if let Scev::Constant(constant) = &right
        && matches!(
            constant,
            mir::Constant::Int {
                value: -1,
                is_signed: true,
                ..
            }
        )
    {
        return Scev::Constant(constant_zero_like(constant));
    }

    // zero remainder by a nonzero constant stays zero
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && constant_is_zero(Some(left_const))
        && !constant_is_zero(Some(right_const))
    {
        return Scev::Constant(constant_zero_like(left_const));
    }

    Scev::SignedRemainder(Box::new(left), Box::new(right))
}

/// Build an unsigned remainder SCEV with basic simplifications.
fn scev_unsigned_remainder(left: Scev, right: Scev) -> Scev {
    // fold constant remainder when possible
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && let Some(result) = fold_binary(
            mir::BinaryOperator::Remainder,
            left_const.clone(),
            right_const.clone(),
        )
    {
        return Scev::Constant(result);
    }

    // remainder by one yields zero
    if let Scev::Constant(constant) = &right
        && constant_is_one(Some(constant))
    {
        return Scev::Constant(constant_zero_like(constant));
    }

    // zero remainder by a nonzero constant stays zero
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && constant_is_zero(Some(left_const))
        && !constant_is_zero(Some(right_const))
    {
        return Scev::Constant(constant_zero_like(left_const));
    }

    Scev::UnsignedRemainder(Box::new(left), Box::new(right))
}

/// Build a shift left SCEV with basic simplifications.
fn scev_shift_left(left: Scev, right: Scev) -> Scev {
    // fold constant shifts when possible
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && let Some(result) = fold_binary(
            mir::BinaryOperator::ShiftLeft,
            left_const.clone(),
            right_const.clone(),
        )
    {
        return Scev::Constant(result);
    }

    // zero shifted by anything remains zero
    if let Scev::Constant(constant) = &left
        && constant_is_zero(Some(constant))
    {
        return left;
    }

    // shifting by zero preserves the left operand
    if let Scev::Constant(constant) = &right
        && constant_is_zero(Some(constant))
    {
        return left;
    }

    Scev::ShiftLeft(Box::new(left), Box::new(right))
}

/// Build an arithmetic shift right SCEV with basic simplifications.
fn scev_arithmetic_shift_right(left: Scev, right: Scev) -> Scev {
    // fold constant shifts when possible
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && let Some(result) = fold_binary(
            mir::BinaryOperator::ShiftRight,
            left_const.clone(),
            right_const.clone(),
        )
    {
        return Scev::Constant(result);
    }

    // zero shifted by anything remains zero
    if let Scev::Constant(constant) = &left
        && constant_is_zero(Some(constant))
    {
        return left;
    }

    // shifting by zero preserves the left operand
    if let Scev::Constant(constant) = &right
        && constant_is_zero(Some(constant))
    {
        return left;
    }

    Scev::ArithmeticShiftRight(Box::new(left), Box::new(right))
}

/// Build a logical shift right SCEV with basic simplifications.
fn scev_logical_shift_right(left: Scev, right: Scev) -> Scev {
    // fold constant shifts when possible
    if let (Scev::Constant(left_const), Scev::Constant(right_const)) = (&left, &right)
        && let Some(result) = fold_binary(
            mir::BinaryOperator::UnsignedShiftRight,
            left_const.clone(),
            right_const.clone(),
        )
    {
        return Scev::Constant(result);
    }

    // zero shifted by anything remains zero
    if let Scev::Constant(constant) = &left
        && constant_is_zero(Some(constant))
    {
        return left;
    }

    // shifting by zero preserves the left operand
    if let Scev::Constant(constant) = &right
        && constant_is_zero(Some(constant))
    {
        return left;
    }

    Scev::LogicalShiftRight(Box::new(left), Box::new(right))
}

/// Build a negated SCEV with basic simplifications.
fn scev_negate(value: Scev) -> Scev {
    // fold negation for constants
    match value {
        Scev::Constant(constant) => match constant {
            mir::Constant::Int {
                value,
                width,
                is_signed,
            } => Scev::Constant(mir::Constant::Int {
                value: value.wrapping_neg(),
                width,
                is_signed,
            }),
            mir::Constant::UInt { value, width } => Scev::Constant(mir::Constant::UInt {
                value: value.wrapping_neg(),
                width,
            }),
            mir::Constant::Float { bits, format } => {
                let value = -float_from_bits(format.format(), bits);
                Scev::Constant(mir::Constant::Float {
                    bits: float_to_bits(format.format(), value),
                    format,
                })
            }
            mir::Constant::Boolean { value } => {
                Scev::Constant(mir::Constant::Boolean { value: !value })
            }
            other => Scev::Constant(other),
        },
        Scev::AddRec {
            start,
            step,
            loop_header,
        } => {
            let start = scev_negate(*start);
            let step = scev_negate(*step);
            Scev::AddRec {
                start: Box::new(start),
                step: Box::new(step),
                loop_header,
            }
        }
        other => Scev::Neg(Box::new(other)),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestProgram;

    /// Simple induction variable becomes an add recurrence.
    #[test]
    fn test_addrec_simple_loop() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    jump b1(v2)

b1(v3: int32):
    v4: int32 = 1
    v5: int32 = add v3, v4
    v6: boolean = lt v5, v1
    branch v6 => b1(v5) | b2(v5)

b2(v7: int32):
    return v7
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let loops = analyses.loops(function, &test.tree);
        let scev = analyses.scalar_evolution(function, &test.tree);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.block(1))
            .unwrap();

        let header_block = test.tree.get(function.block(1));
        let param_value = header_block.parameters[0].value;

        let expected = Scev::AddRec {
            start: Box::new(Scev::Constant(mir::Constant::Int {
                value: 0,
                width: 32,
                is_signed: true,
            })),
            step: Box::new(Scev::Constant(mir::Constant::Int {
                value: 1,
                width: 32,
                is_signed: true,
            })),
            loop_header: function.block(1),
        };

        let actual = scev.value_scev(loop_index, param_value).unwrap();
        assert_eq!(actual, &expected);
    }

    /// Nonlinear recurrence does not produce an add recurrence.
    #[test]
    fn test_no_addrec_for_non_linear_update() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 1
    jump b1(v2)

b1(v3: int32):
    v4: int32 = mul v3, v1
    v5: boolean = lt v4, v1
    branch v5 => b1(v4) | b2(v4)

b2(v6: int32):
    return v6
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let loops = analyses.loops(function, &test.tree);
        let scev = analyses.scalar_evolution(function, &test.tree);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.block(1))
            .unwrap();

        let header_block = test.tree.get(function.block(1));
        let param_value = header_block.parameters[0].value;

        let actual = scev.value_scev(loop_index, param_value).unwrap();
        assert!(matches!(actual, Scev::Unknown(_)));
    }

    /// Subtractive induction variables produce negative steps.
    #[test]
    fn test_addrec_subtract_step() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 10
    jump b1(v1)

b1(v2: int32):
    v3: int32 = 1
    v4: int32 = sub v2, v3
    v5: boolean = gt v4, v0
    branch v5 => b1(v4) | b2(v4)

b2(v6: int32):
    return v6
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let loops = analyses.loops(function, &test.tree);
        let scev = analyses.scalar_evolution(function, &test.tree);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.block(1))
            .unwrap();

        let header_block = test.tree.get(function.block(1));
        let param_value = header_block.parameters[0].value;

        let expected = Scev::AddRec {
            start: Box::new(Scev::Constant(mir::Constant::Int {
                value: 10,
                width: 32,
                is_signed: true,
            })),
            step: Box::new(Scev::Constant(mir::Constant::Int {
                value: -1,
                width: 32,
                is_signed: true,
            })),
            loop_header: function.block(1),
        };

        let actual = scev.value_scev(loop_index, param_value).unwrap();
        assert_eq!(actual, &expected);
    }

    /// Unchanged induction variables produce zero steps.
    #[test]
    fn test_addrec_zero_step() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 5
    jump b1(v1)

b1(v2: int32):
    v3: boolean = lt v2, v0
    branch v3 => b2(v2) | b3(v2)

b2(v4: int32):
    jump b1(v4)

b3(v5: int32):
    return v5
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let loops = analyses.loops(function, &test.tree);
        let scev = analyses.scalar_evolution(function, &test.tree);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.block(1))
            .unwrap();

        let header_block = test.tree.get(function.block(1));
        let param_value = header_block.parameters[0].value;

        let expected = Scev::AddRec {
            start: Box::new(Scev::Constant(mir::Constant::Int {
                value: 5,
                width: 32,
                is_signed: true,
            })),
            step: Box::new(Scev::Constant(mir::Constant::Int {
                value: 0,
                width: 32,
                is_signed: true,
            })),
            loop_header: function.block(1),
        };

        let actual = scev.value_scev(loop_index, param_value).unwrap();
        assert_eq!(actual, &expected);
    }

    /// Derived induction expressions are expressed in terms of add recurrences.
    #[test]
    fn test_addrec_derived_value() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    jump b1(v2)

b1(v3: int32):
    v4: int32 = 2
    v5: int32 = add v3, v4
    v6: int32 = 1
    v7: int32 = add v3, v6
    v8: boolean = lt v7, v1
    branch v8 => b1(v7) | b2(v5)

b2(v9: int32):
    return v9
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let loops = analyses.loops(function, &test.tree);
        let scev = analyses.scalar_evolution(function, &test.tree);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.block(1))
            .unwrap();

        let block1 = test.tree.get(function.block(1));
        let instruction_id = block1.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let derived_value = instruction
            .destination()
            .expect("derived value should be concrete");

        let expected = Scev::AddRec {
            start: Box::new(Scev::Constant(mir::Constant::Int {
                value: 2,
                width: 32,
                is_signed: true,
            })),
            step: Box::new(Scev::Constant(mir::Constant::Int {
                value: 1,
                width: 32,
                is_signed: true,
            })),
            loop_header: function.block(1),
        };

        let actual = scev.value_scev(loop_index, derived_value).unwrap();
        assert_eq!(actual, &expected);
    }

    /// Additive sums of add recurrences stay linear.
    #[test]
    fn test_addrec_addrec_sum() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    jump b1(v1)

b1(v2: int32):
    v3: int32 = add v2, v2
    v4: int32 = 1
    v5: int32 = add v2, v4
    v6: boolean = lt v5, v0
    branch v6 => b1(v5) | b2(v3)

b2(v7: int32):
    return v7
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let loops = analyses.loops(function, &test.tree);
        let scev = analyses.scalar_evolution(function, &test.tree);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.block(1))
            .unwrap();

        let block1 = test.tree.get(function.block(1));
        let instruction = test.tree.get(block1.instructions[0]);
        let derived_value = instruction
            .destination()
            .expect("derived value should be concrete");

        let expected = Scev::AddRec {
            start: Box::new(Scev::Constant(mir::Constant::Int {
                value: 0,
                width: 32,
                is_signed: true,
            })),
            step: Box::new(Scev::Constant(mir::Constant::Int {
                value: 2,
                width: 32,
                is_signed: true,
            })),
            loop_header: function.block(1),
        };

        let actual = scev.value_scev(loop_index, derived_value).unwrap();
        assert_eq!(actual, &expected);
    }

    /// Multiplying add recurrences by constants stays linear.
    #[test]
    fn test_addrec_mul_constant() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    jump b1(v1)

b1(v2: int32):
    v3: int32 = 2
    v4: int32 = mul v2, v3
    v5: int32 = 1
    v6: int32 = add v2, v5
    v7: boolean = lt v6, v0
    branch v7 => b1(v6) | b2(v4)

b2(v8: int32):
    return v8
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let loops = analyses.loops(function, &test.tree);
        let scev = analyses.scalar_evolution(function, &test.tree);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.block(1))
            .unwrap();

        let block1 = test.tree.get(function.block(1));
        let instruction = test.tree.get(block1.instructions[1]);
        let derived_value = instruction
            .destination()
            .expect("derived value should be concrete");

        let expected = Scev::AddRec {
            start: Box::new(Scev::Constant(mir::Constant::Int {
                value: 0,
                width: 32,
                is_signed: true,
            })),
            step: Box::new(Scev::Constant(mir::Constant::Int {
                value: 2,
                width: 32,
                is_signed: true,
            })),
            loop_header: function.block(1),
        };

        let actual = scev.value_scev(loop_index, derived_value).unwrap();
        assert_eq!(actual, &expected);
    }

    /// Multiplying add recurrences by invariants stays linear.
    #[test]
    fn test_addrec_mul_invariant() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    jump b1(v2)

b1(v3: int32):
    v4: int32 = mul v3, v1
    v5: int32 = 1
    v6: int32 = add v3, v5
    v7: boolean = lt v6, v0
    branch v7 => b1(v6) | b2(v4)

b2(v8: int32):
    return v8
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let loops = analyses.loops(function, &test.tree);
        let scev = analyses.scalar_evolution(function, &test.tree);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.block(1))
            .unwrap();

        let block1 = test.tree.get(function.block(1));
        let instruction = test.tree.get(block1.instructions[0]);
        let derived_value = instruction
            .destination()
            .expect("derived value should be concrete");

        let invariant_value = function.parameters[1].value;
        let expected = Scev::AddRec {
            start: Box::new(Scev::Constant(mir::Constant::Int {
                value: 0,
                width: 32,
                is_signed: true,
            })),
            step: Box::new(Scev::Unknown(invariant_value)),
            loop_header: function.block(1),
        };

        let actual = scev.value_scev(loop_index, derived_value).unwrap();
        assert_eq!(actual, &expected);
    }

    /// Nested additive chains still produce linear add recurrences.
    #[test]
    fn test_addrec_nested_additive_step() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    jump b1(v1)

b1(v2: int32):
    v3: int32 = 1
    v4: int32 = add v2, v3
    v5: int32 = 2
    v6: int32 = add v4, v5
    v7: boolean = lt v6, v0
    branch v7 => b1(v6) | b2(v6)

b2(v8: int32):
    return v8
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let loops = analyses.loops(function, &test.tree);
        let scev = analyses.scalar_evolution(function, &test.tree);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.block(1))
            .unwrap();

        let header_block = test.tree.get(function.block(1));
        let param_value = header_block.parameters[0].value;

        let expected = Scev::AddRec {
            start: Box::new(Scev::Constant(mir::Constant::Int {
                value: 0,
                width: 32,
                is_signed: true,
            })),
            step: Box::new(Scev::Constant(mir::Constant::Int {
                value: 3,
                width: 32,
                is_signed: true,
            })),
            loop_header: function.block(1),
        };

        let actual = scev.value_scev(loop_index, param_value).unwrap();
        assert_eq!(actual, &expected);
    }

    /// Nested subtractive chains still produce linear add recurrences.
    #[test]
    fn test_addrec_nested_subtract_step() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 10
    jump b1(v1)

b1(v2: int32):
    v3: int32 = 1
    v4: int32 = sub v2, v3
    v5: int32 = 2
    v6: int32 = sub v4, v5
    v7: boolean = gt v6, v0
    branch v7 => b1(v6) | b2(v6)

b2(v8: int32):
    return v8
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let loops = analyses.loops(function, &test.tree);
        let scev = analyses.scalar_evolution(function, &test.tree);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.block(1))
            .unwrap();

        let header_block = test.tree.get(function.block(1));
        let param_value = header_block.parameters[0].value;

        let expected = Scev::AddRec {
            start: Box::new(Scev::Constant(mir::Constant::Int {
                value: 10,
                width: 32,
                is_signed: true,
            })),
            step: Box::new(Scev::Constant(mir::Constant::Int {
                value: -3,
                width: 32,
                is_signed: true,
            })),
            loop_header: function.block(1),
        };

        let actual = scev.value_scev(loop_index, param_value).unwrap();
        assert_eq!(actual, &expected);
    }

    /// Signed division by negative one produces a negated recurrence.
    #[test]
    fn test_scev_signed_divide_by_negative_one() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    jump b1(v1)

b1(v2: int32):
    v3: int32 = -1
    v4: int32 = div v2, v3
    v5: int32 = 1
    v6: int32 = add v2, v5
    v7: boolean = lt v6, v0
    branch v7 => b1(v6) | b2(v4)

b2(v8: int32):
    return v8
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let loops = analyses.loops(function, &test.tree);
        let scev = analyses.scalar_evolution(function, &test.tree);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.block(1))
            .unwrap();

        let block1 = test.tree.get(function.block(1));
        let instruction = test.tree.get(block1.instructions[1]);
        let divide_value = instruction
            .destination()
            .expect("divide value should be concrete");

        let expected = Scev::AddRec {
            start: Box::new(Scev::Constant(mir::Constant::Int {
                value: 0,
                width: 32,
                is_signed: true,
            })),
            step: Box::new(Scev::Constant(mir::Constant::Int {
                value: -1,
                width: 32,
                is_signed: true,
            })),
            loop_header: function.block(1),
        };

        let actual = scev.value_scev(loop_index, divide_value).unwrap();
        assert_eq!(actual, &expected);
    }

    /// Shift right operations are represented in SCEV.
    #[test]
    fn test_scev_shift_right_operations() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    jump b1(v1)

b1(v2: int32):
    v3: int32 = 1
    v4: int32 = shr v2, v3
    v5: int32 = ushr v2, v3
    v6: int32 = add v2, v3
    v7: boolean = lt v6, v0
    branch v7 => b1(v6) | b2(v4)

b2(v8: int32):
    return v8
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let loops = analyses.loops(function, &test.tree);
        let scev = analyses.scalar_evolution(function, &test.tree);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.block(1))
            .unwrap();

        let header_block = test.tree.get(function.block(1));
        let param_value = header_block.parameters[0].value;
        let param_scev = scev.value_scev(loop_index, param_value).unwrap().clone();

        let block1 = test.tree.get(function.block(1));
        let arithmetic_value = test
            .tree
            .get(block1.instructions[1])
            .destination()
            .expect("arithmetic shift value should be concrete");
        let logical_value = test
            .tree
            .get(block1.instructions[2])
            .destination()
            .expect("logical shift value should be concrete");

        let expected_arithmetic = Scev::ArithmeticShiftRight(
            Box::new(param_scev.clone()),
            Box::new(Scev::Constant(mir::Constant::Int {
                value: 1,
                width: 32,
                is_signed: true,
            })),
        );
        let expected_logical = Scev::LogicalShiftRight(
            Box::new(param_scev),
            Box::new(Scev::Constant(mir::Constant::Int {
                value: 1,
                width: 32,
                is_signed: true,
            })),
        );

        let actual_arithmetic = scev.value_scev(loop_index, arithmetic_value).unwrap();
        let actual_logical = scev.value_scev(loop_index, logical_value).unwrap();

        assert_eq!(actual_arithmetic, &expected_arithmetic);
        assert_eq!(actual_logical, &expected_logical);
    }

    /// Extended arithmetic and casts are represented in SCEV.
    #[test]
    fn test_scev_extended_operations() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32): int64 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    jump b1(v2)

b1(v3: int32):
    v4: int32 = 2
    v5: int32 = div v3, v4
    v6: int32 = 3
    v7: int32 = shl v3, v6
    v8: int32 = 1
    v9: int32 = rem v3, v8
    v10: int64 = cast.extend.s v3 -> int64
    v11: uint64 = cast.extend.u v3 -> uint64
    v12: boolean = lt v3, v1
    branch v12 => b1(v3) | b2(v10)

b2(v13: int64):
    return v13
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let loops = analyses.loops(function, &test.tree);
        let scev = analyses.scalar_evolution(function, &test.tree);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.block(1))
            .unwrap();

        let header_block = test.tree.get(function.block(1));
        let param_value = header_block.parameters[0].value;
        let param_scev = scev.value_scev(loop_index, param_value).unwrap().clone();

        let block1 = test.tree.get(function.block(1));
        let divide_value = test
            .tree
            .get(block1.instructions[1])
            .destination()
            .expect("divide value should be concrete");
        let shift_value = test
            .tree
            .get(block1.instructions[3])
            .destination()
            .expect("shift value should be concrete");
        let remainder_value = test
            .tree
            .get(block1.instructions[5])
            .destination()
            .expect("remainder value should be concrete");
        let sext_value = test
            .tree
            .get(block1.instructions[6])
            .destination()
            .expect("sign-extend value should be concrete");
        let uext_value = test
            .tree
            .get(block1.instructions[7])
            .destination()
            .expect("zero-extend value should be concrete");

        let expected_divide = Scev::SignedDivide(
            Box::new(param_scev.clone()),
            Box::new(Scev::Constant(mir::Constant::Int {
                value: 2,
                width: 32,
                is_signed: true,
            })),
        );
        let expected_shift = Scev::ShiftLeft(
            Box::new(param_scev.clone()),
            Box::new(Scev::Constant(mir::Constant::Int {
                value: 3,
                width: 32,
                is_signed: true,
            })),
        );
        let expected_remainder = Scev::Constant(mir::Constant::Int {
            value: 0,
            width: 32,
            is_signed: true,
        });
        let expected_sext = Scev::SignExtend {
            value: Box::new(param_scev.clone()),
            width: 64,
        };
        let expected_uext = Scev::ZeroExtend {
            value: Box::new(param_scev.clone()),
            width: 64,
        };

        let actual_divide = scev.value_scev(loop_index, divide_value).unwrap();
        let actual_shift = scev.value_scev(loop_index, shift_value).unwrap();
        let actual_remainder = scev.value_scev(loop_index, remainder_value).unwrap();
        let actual_sext = scev.value_scev(loop_index, sext_value).unwrap();
        let actual_uext = scev.value_scev(loop_index, uext_value).unwrap();

        assert_eq!(actual_divide, &expected_divide);
        assert_eq!(actual_shift, &expected_shift);
        assert_eq!(actual_remainder, &expected_remainder);
        assert_eq!(actual_sext, &expected_sext);
        assert_eq!(actual_uext, &expected_uext);
    }
}

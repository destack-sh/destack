use std::collections::{HashMap, HashSet};

use destack_core::{float_from_bits, float_to_bits};
use destack_mir as mir;

use crate::common::mir::{
    Analysis, AnalysisId, BlockParamForwarding, FunctionAnalyses, FunctionAnalysis, TypeContext,
    constant_is_one, constant_is_zero, constant_zero_for_type, constant_zero_like, fold_binary,
    fold_cast, instruction_is_pure, terminator_arguments_for_successor,
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
        type_context: TypeContext,
    ) -> Self {
        // handle functions without bodies
        if function.entry.is_none() {
            return Self {
                loop_scev: HashMap::new(),
            };
        }

        // build definition metadata for values
        let definitions = ValueDefinitions::build(function, tree);

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
                &definitions,
                &forwarding,
                type_context,
            );
            let scev_map = builder.build();
            loop_scev.insert(loop_index, scev_map);
        }

        Self { loop_scev }
    }

    /// Get the SCEV for a value in a specific loop.
    pub fn scev_for_value_in_loop(&self, loop_index: usize, value: mir::Value) -> Option<&Scev> {
        self.loop_scev.get(&loop_index)?.get(&value)
    }

    /// Get all SCEVs for a loop.
    pub fn loop_scevs(&self, loop_index: usize) -> Option<&HashMap<mir::Value, Scev>> {
        self.loop_scev.get(&loop_index)
    }
}

impl Analysis for ScalarEvolution {
    const ID: AnalysisId = AnalysisId("scev");
    const DEPENDENCIES: &'static [AnalysisId] = &[LoopAnalysis::ID];
}

impl FunctionAnalysis for ScalarEvolution {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        let cfg = analyses.get::<ControlFlowGraph>();
        let loops = analyses.get::<LoopAnalysis>();
        Self::build(function, tree, &cfg, &loops, analyses.type_context())
    }
}

/// Definition kind for an SSA value.
#[derive(Debug, Clone, Copy)]
enum ValueDefinitionKind {
    /// Block parameter definition.
    Parameter {
        /// Parameter index within the block.
        index: usize,
    },
    /// Instruction definition.
    Instruction {
        /// Instruction id that defines the value.
        instruction: mir::LocalNodeId<mir::Instruction>,
    },
}

/// Definition metadata for a value.
#[derive(Debug, Clone, Copy)]
struct ValueDefinition {
    /// Block that defines the value.
    block: mir::LocalNodeId<mir::Block>,
    /// Kind of definition.
    kind: ValueDefinitionKind,
}

/// Map of SSA values to their definition metadata.
#[derive(Debug)]
struct ValueDefinitions {
    /// Definition metadata by value.
    definitions: HashMap<mir::Value, ValueDefinition>,
}

impl ValueDefinitions {
    /// Build a definition map for a function.
    fn build(function: &mir::Function, tree: &mir::Tree) -> Self {
        let mut definitions = HashMap::new();

        // record block parameters
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for (index, param) in block.parameters.iter().enumerate() {
                let Some(value) = param.value.value() else {
                    continue;
                };

                definitions.insert(
                    value,
                    ValueDefinition {
                        block: block_id,
                        kind: ValueDefinitionKind::Parameter { index },
                    },
                );
            }
        }

        // record instruction destinations
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination().and_then(|value| value.value())
                {
                    definitions.insert(
                        destination,
                        ValueDefinition {
                            block: block_id,
                            kind: ValueDefinitionKind::Instruction {
                                instruction: instruction_id,
                            },
                        },
                    );
                }
            }
        }

        Self { definitions }
    }

    /// Get the definition for a value.
    fn definition_for(&self, value: mir::Value) -> Option<ValueDefinition> {
        self.definitions.get(&value).copied()
    }
}

/// Builder for loop local scalar evolution expressions.
struct LoopScevBuilder<'a> {
    /// MIR tree.
    tree: &'a mir::Tree,
    /// Control flow graph.
    cfg: &'a ControlFlowGraph,
    /// Loop being analyzed.
    lp: &'a Loop,
    /// Value definition metadata.
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
    type_context: TypeContext,
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
        type_context: TypeContext,
    ) -> Self {
        let invariants = collect_loop_invariants(function, tree, lp, definitions);

        Self {
            tree,
            cfg,
            lp,
            definitions,
            forwarding,
            invariants,
            cache: HashMap::new(),
            in_progress: HashSet::new(),
            type_context,
        }
    }

    /// Build the SCEV map for this loop.
    fn build(&mut self) -> HashMap<mir::Value, Scev> {
        let mut values = Vec::new();

        // collect values defined in the loop
        for &block_id in &self.lp.blocks {
            let block = self.tree.get(block_id);
            for param in &block.parameters {
                let Some(value) = param.value.value() else {
                    continue;
                };

                values.push(value);
            }
            for &instruction_id in &block.instructions {
                let instruction = self.tree.get(instruction_id);
                if let Some(destination) = instruction.destination().and_then(|value| value.value())
                {
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
        let Some(definition) = self.definitions.definition_for(value) else {
            return Scev::Unknown(value);
        };

        let ValueDefinitionKind::Instruction { instruction } = definition.kind else {
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
                let Some(left) = left.value() else {
                    return Scev::Unknown(value);
                };
                let Some(right) = right.value() else {
                    return Scev::Unknown(value);
                };

                self.scev_for_binary(value, *operator, left, right)
            }
            mir::Instruction::Unary {
                operator, argument, ..
            } => {
                let Some(argument) = argument.value() else {
                    return Scev::Unknown(value);
                };

                self.scev_for_unary(value, *operator, argument)
            }
            mir::Instruction::Cast {
                operator,
                argument,
                to_type,
                ..
            } => {
                let Some(argument) = argument.value() else {
                    return Scev::Unknown(value);
                };
                let Some(to_type) = to_type.ty() else {
                    return Scev::Unknown(value);
                };

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
            mir::BinaryOperator::SignedDivide => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);
                scev_signed_divide(left_scev, right_scev)
            }
            mir::BinaryOperator::UnsignedDivide => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);
                scev_unsigned_divide(left_scev, right_scev)
            }
            mir::BinaryOperator::SignedRemainder => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);
                scev_signed_remainder(left_scev, right_scev)
            }
            mir::BinaryOperator::UnsignedRemainder => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);
                scev_unsigned_remainder(left_scev, right_scev)
            }
            mir::BinaryOperator::ShiftLeft => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);
                scev_shift_left(left_scev, right_scev)
            }
            mir::BinaryOperator::ArithmeticShiftRight => {
                let left_scev = self.scev_for_value(left);
                let right_scev = self.scev_for_value(right);
                scev_arithmetic_shift_right(left_scev, right_scev)
            }
            mir::BinaryOperator::LogicalShiftRight => {
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
            mir::UnaryOperator::Negate | mir::UnaryOperator::FloatNegate => {
                scev_negate(argument_scev)
            }
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
                self.type_context.pointer_width_bits,
                self.tree,
            )
        {
            return Scev::Constant(result);
        }

        // read the target type
        let target_type = self.tree.get(to_type);
        let Some((width, _)) =
            target_type.int_info_with_pointer_width(self.type_context.pointer_width_bits)
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
        let definition = match self.definitions.definition_for(value) {
            Some(definition) => definition,
            None => return false,
        };

        self.lp.blocks.contains(&definition.block)
    }

    /// Get the index of a header parameter if this value is one.
    fn header_param_index(&self, value: mir::Value) -> Option<usize> {
        // require a header parameter definition
        let definition = self.definitions.definition_for(value)?;

        if definition.block != self.lp.header {
            return None;
        }

        match definition.kind {
            ValueDefinitionKind::Parameter { index } => Some(index),
            _ => None,
        }
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
            self.type_context.pointer_width_bits,
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
        let definition = self.definitions.definition_for(value)?;
        let ValueDefinitionKind::Instruction { instruction } = definition.kind else {
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
                let left = left.value()?;
                let right = right.value()?;

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
                let left = left.value()?;
                let right = right.value()?;

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
        let Some(value) = param.value.value() else {
            continue;
        };

        invariants.insert(value);
    }

    // seed invariants with values defined outside the loop
    for (&value, definition) in &definitions.definitions {
        if !lp.blocks.contains(&definition.block) {
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
                let Some(destination) = instruction.destination().and_then(|value| value.value())
                else {
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
        let Some(value) = value.value() else {
            return false;
        };

        if !invariants.contains(&value) {
            return false;
        }
    }

    // check externalized operands
    let Some(args_slice) = instruction.argument_slice() else {
        return true;
    };

    tree.get_arguments(args_slice).iter().all(|value| {
        value
            .value()
            .is_some_and(|value| invariants.contains(&value))
    })
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
    let args = terminator_arguments_for_successor(pred_terminator, header);

    args.get(param_index).and_then(|value| value.value())
}

/// Get a constant value for an SSA value if it is constant.
fn constant_for_value(
    value: mir::Value,
    tree: &mir::Tree,
    definitions: &ValueDefinitions,
) -> Option<mir::Constant> {
    // require an instruction definition
    let definition = definitions.definition_for(value)?;
    let ValueDefinitionKind::Instruction { instruction } = definition.kind else {
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
    let ty = tree.get(param.ty.ty()?);
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
            mir::BinaryOperator::SignedDivide,
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
            mir::BinaryOperator::UnsignedDivide,
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
            mir::BinaryOperator::SignedRemainder,
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
            mir::BinaryOperator::UnsignedRemainder,
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
            mir::BinaryOperator::ArithmeticShiftRight,
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
            mir::BinaryOperator::LogicalShiftRight,
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
    use crate::optimize::common::tests::TestProgram;

    /// Simple induction variable becomes an add recurrence.
    #[test]
    fn test_addrec_simple_loop() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = 0int32
    jump b1(v2)
b1(v3: int32):
    v4: int32 = 1int32
    v5: int32 = int.add v3, v4
    v6: boolean = int.lt.s v5, v1
    branch v6, b1(v5), b2(v5)
b2(v7: int32):
    return v7
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let loops = analyses.get::<LoopAnalysis>();
        let scev = analyses.get::<ScalarEvolution>();

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let header_block = test.tree.get(function.blocks[1]);
        let param_value = header_block.parameters[0]
            .value
            .value()
            .expect("header parameter should be concrete");

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
            loop_header: function.blocks[1],
        };

        let actual = scev
            .scev_for_value_in_loop(loop_index, param_value)
            .unwrap();
        assert_eq!(actual, &expected);
    }

    /// Nonlinear recurrence does not produce an add recurrence.
    #[test]
    fn test_no_addrec_for_non_linear_update() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = 1int32
    jump b1(v2)
b1(v3: int32):
    v4: int32 = int.mul v3, v1
    v5: boolean = int.lt.s v4, v1
    branch v5, b1(v4), b2(v4)
b2(v6: int32):
    return v6
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let loops = analyses.get::<LoopAnalysis>();
        let scev = analyses.get::<ScalarEvolution>();

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let header_block = test.tree.get(function.blocks[1]);
        let param_value = header_block.parameters[0]
            .value
            .value()
            .expect("header parameter should be concrete");

        let actual = scev
            .scev_for_value_in_loop(loop_index, param_value)
            .unwrap();
        assert!(matches!(actual, Scev::Unknown(_)));
    }

    /// Subtractive induction variables produce negative steps.
    #[test]
    fn test_addrec_subtract_step() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 10int32
    jump b1(v1)
b1(v2: int32):
    v3: int32 = 1int32
    v4: int32 = int.sub v2, v3
    v5: boolean = int.gt.s v4, v0
    branch v5, b1(v4), b2(v4)
b2(v6: int32):
    return v6
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let loops = analyses.get::<LoopAnalysis>();
        let scev = analyses.get::<ScalarEvolution>();

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let header_block = test.tree.get(function.blocks[1]);
        let param_value = header_block.parameters[0]
            .value
            .value()
            .expect("header parameter should be concrete");

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
            loop_header: function.blocks[1],
        };

        let actual = scev
            .scev_for_value_in_loop(loop_index, param_value)
            .unwrap();
        assert_eq!(actual, &expected);
    }

    /// Unchanged induction variables produce zero steps.
    #[test]
    fn test_addrec_zero_step() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 5int32
    jump b1(v1)
b1(v2: int32):
    v3: boolean = int.lt.s v2, v0
    branch v3, b2(v2), b3(v2)
b2(v4: int32):
    jump b1(v4)
b3(v5: int32):
    return v5
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let loops = analyses.get::<LoopAnalysis>();
        let scev = analyses.get::<ScalarEvolution>();

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let header_block = test.tree.get(function.blocks[1]);
        let param_value = header_block.parameters[0]
            .value
            .value()
            .expect("header parameter should be concrete");

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
            loop_header: function.blocks[1],
        };

        let actual = scev
            .scev_for_value_in_loop(loop_index, param_value)
            .unwrap();
        assert_eq!(actual, &expected);
    }

    /// Derived induction expressions are expressed in terms of add recurrences.
    #[test]
    fn test_addrec_derived_value() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = 0int32
    jump b1(v2)
b1(v3: int32):
    v4: int32 = 2int32
    v5: int32 = int.add v3, v4
    v6: int32 = 1int32
    v7: int32 = int.add v3, v6
    v8: boolean = int.lt.s v7, v1
    branch v8, b1(v7), b2(v5)
b2(v9: int32):
    return v9
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let loops = analyses.get::<LoopAnalysis>();
        let scev = analyses.get::<ScalarEvolution>();

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let block1 = test.tree.get(function.blocks[1]);
        let instruction_id = block1.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let derived_value = instruction
            .destination()
            .and_then(|value| value.value())
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
            loop_header: function.blocks[1],
        };

        let actual = scev
            .scev_for_value_in_loop(loop_index, derived_value)
            .unwrap();
        assert_eq!(actual, &expected);
    }

    /// Additive sums of add recurrences stay linear.
    #[test]
    fn test_addrec_addrec_sum() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1)
b1(v2: int32):
    v3: int32 = int.add v2, v2
    v4: int32 = 1int32
    v5: int32 = int.add v2, v4
    v6: boolean = int.lt.s v5, v0
    branch v6, b1(v5), b2(v3)
b2(v7: int32):
    return v7
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let loops = analyses.get::<LoopAnalysis>();
        let scev = analyses.get::<ScalarEvolution>();

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let block1 = test.tree.get(function.blocks[1]);
        let instruction = test.tree.get(block1.instructions[0]);
        let derived_value = instruction
            .destination()
            .and_then(|value| value.value())
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
            loop_header: function.blocks[1],
        };

        let actual = scev
            .scev_for_value_in_loop(loop_index, derived_value)
            .unwrap();
        assert_eq!(actual, &expected);
    }

    /// Multiplying add recurrences by constants stays linear.
    #[test]
    fn test_addrec_mul_constant() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1)
b1(v2: int32):
    v3: int32 = 2int32
    v4: int32 = int.mul v2, v3
    v5: int32 = 1int32
    v6: int32 = int.add v2, v5
    v7: boolean = int.lt.s v6, v0
    branch v7, b1(v6), b2(v4)
b2(v8: int32):
    return v8
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let loops = analyses.get::<LoopAnalysis>();
        let scev = analyses.get::<ScalarEvolution>();

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let block1 = test.tree.get(function.blocks[1]);
        let instruction = test.tree.get(block1.instructions[1]);
        let derived_value = instruction
            .destination()
            .and_then(|value| value.value())
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
            loop_header: function.blocks[1],
        };

        let actual = scev
            .scev_for_value_in_loop(loop_index, derived_value)
            .unwrap();
        assert_eq!(actual, &expected);
    }

    /// Multiplying add recurrences by invariants stays linear.
    #[test]
    fn test_addrec_mul_invariant() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = 0int32
    jump b1(v2)
b1(v3: int32):
    v4: int32 = int.mul v3, v1
    v5: int32 = 1int32
    v6: int32 = int.add v3, v5
    v7: boolean = int.lt.s v6, v0
    branch v7, b1(v6), b2(v4)
b2(v8: int32):
    return v8
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let loops = analyses.get::<LoopAnalysis>();
        let scev = analyses.get::<ScalarEvolution>();

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let block1 = test.tree.get(function.blocks[1]);
        let instruction = test.tree.get(block1.instructions[0]);
        let derived_value = instruction
            .destination()
            .and_then(|value| value.value())
            .expect("derived value should be concrete");

        let invariant_value = function.parameters[1]
            .value
            .value()
            .expect("invariant parameter should be concrete");
        let expected = Scev::AddRec {
            start: Box::new(Scev::Constant(mir::Constant::Int {
                value: 0,
                width: 32,
                is_signed: true,
            })),
            step: Box::new(Scev::Unknown(invariant_value)),
            loop_header: function.blocks[1],
        };

        let actual = scev
            .scev_for_value_in_loop(loop_index, derived_value)
            .unwrap();
        assert_eq!(actual, &expected);
    }

    /// Nested additive chains still produce linear add recurrences.
    #[test]
    fn test_addrec_nested_additive_step() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1)
b1(v2: int32):
    v3: int32 = 1int32
    v4: int32 = int.add v2, v3
    v5: int32 = 2int32
    v6: int32 = int.add v4, v5
    v7: boolean = int.lt.s v6, v0
    branch v7, b1(v6), b2(v6)
b2(v8: int32):
    return v8
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let loops = analyses.get::<LoopAnalysis>();
        let scev = analyses.get::<ScalarEvolution>();

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let header_block = test.tree.get(function.blocks[1]);
        let param_value = header_block.parameters[0]
            .value
            .value()
            .expect("header parameter should be concrete");

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
            loop_header: function.blocks[1],
        };

        let actual = scev
            .scev_for_value_in_loop(loop_index, param_value)
            .unwrap();
        assert_eq!(actual, &expected);
    }

    /// Nested subtractive chains still produce linear add recurrences.
    #[test]
    fn test_addrec_nested_subtract_step() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 10int32
    jump b1(v1)
b1(v2: int32):
    v3: int32 = 1int32
    v4: int32 = int.sub v2, v3
    v5: int32 = 2int32
    v6: int32 = int.sub v4, v5
    v7: boolean = int.gt.s v6, v0
    branch v7, b1(v6), b2(v6)
b2(v8: int32):
    return v8
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let loops = analyses.get::<LoopAnalysis>();
        let scev = analyses.get::<ScalarEvolution>();

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let header_block = test.tree.get(function.blocks[1]);
        let param_value = header_block.parameters[0]
            .value
            .value()
            .expect("header parameter should be concrete");

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
            loop_header: function.blocks[1],
        };

        let actual = scev
            .scev_for_value_in_loop(loop_index, param_value)
            .unwrap();
        assert_eq!(actual, &expected);
    }

    /// Signed division by negative one produces a negated recurrence.
    #[test]
    fn test_scev_signed_divide_by_negative_one() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1)
b1(v2: int32):
    v3: int32 = -1int32
    v4: int32 = int.div.s v2, v3
    v5: int32 = 1int32
    v6: int32 = int.add v2, v5
    v7: boolean = int.lt.s v6, v0
    branch v7, b1(v6), b2(v4)
b2(v8: int32):
    return v8
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let loops = analyses.get::<LoopAnalysis>();
        let scev = analyses.get::<ScalarEvolution>();

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let block1 = test.tree.get(function.blocks[1]);
        let instruction = test.tree.get(block1.instructions[1]);
        let divide_value = instruction
            .destination()
            .and_then(|value| value.value())
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
            loop_header: function.blocks[1],
        };

        let actual = scev
            .scev_for_value_in_loop(loop_index, divide_value)
            .unwrap();
        assert_eq!(actual, &expected);
    }

    /// Shift right operations are represented in SCEV.
    #[test]
    fn test_scev_shift_right_operations() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1)
b1(v2: int32):
    v3: int32 = 1int32
    v4: int32 = int.shiftRight.s v2, v3
    v5: int32 = int.shiftRight.u v2, v3
    v6: int32 = int.add v2, v3
    v7: boolean = int.lt.s v6, v0
    branch v7, b1(v6), b2(v4)
b2(v8: int32):
    return v8
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let loops = analyses.get::<LoopAnalysis>();
        let scev = analyses.get::<ScalarEvolution>();

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let header_block = test.tree.get(function.blocks[1]);
        let param_value = header_block.parameters[0]
            .value
            .value()
            .expect("header parameter should be concrete");
        let param_scev = scev
            .scev_for_value_in_loop(loop_index, param_value)
            .unwrap()
            .clone();

        let block1 = test.tree.get(function.blocks[1]);
        let arithmetic_value = test
            .tree
            .get(block1.instructions[1])
            .destination()
            .and_then(|value| value.value())
            .expect("arithmetic shift value should be concrete");
        let logical_value = test
            .tree
            .get(block1.instructions[2])
            .destination()
            .and_then(|value| value.value())
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

        let actual_arithmetic = scev
            .scev_for_value_in_loop(loop_index, arithmetic_value)
            .unwrap();
        let actual_logical = scev
            .scev_for_value_in_loop(loop_index, logical_value)
            .unwrap();

        assert_eq!(actual_arithmetic, &expected_arithmetic);
        assert_eq!(actual_logical, &expected_logical);
    }

    /// Extended arithmetic and casts are represented in SCEV.
    #[test]
    fn test_scev_extended_operations() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32): int64 {
b0(v0: int32, v1: int32):
    v2: int32 = 0int32
    jump b1(v2)
b1(v3: int32):
    v4: int32 = 2int32
    v5: int32 = int.div.s v3, v4
    v6: int32 = 3int32
    v7: int32 = int.shiftLeft v3, v6
    v8: int32 = 1int32
    v9: int32 = int.rem.s v3, v8
    v10: int64 = cast.extend.s v3 -> int64
    v11: uint64 = cast.extend.u v3 -> uint64
    v12: boolean = int.lt.s v3, v1
    branch v12, b1(v3), b2(v10)
b2(v13: int64):
    return v13
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let loops = analyses.get::<LoopAnalysis>();
        let scev = analyses.get::<ScalarEvolution>();

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let header_block = test.tree.get(function.blocks[1]);
        let param_value = header_block.parameters[0]
            .value
            .value()
            .expect("header parameter should be concrete");
        let param_scev = scev
            .scev_for_value_in_loop(loop_index, param_value)
            .unwrap()
            .clone();

        let block1 = test.tree.get(function.blocks[1]);
        let divide_value = test
            .tree
            .get(block1.instructions[1])
            .destination()
            .and_then(|value| value.value())
            .expect("divide value should be concrete");
        let shift_value = test
            .tree
            .get(block1.instructions[3])
            .destination()
            .and_then(|value| value.value())
            .expect("shift value should be concrete");
        let remainder_value = test
            .tree
            .get(block1.instructions[5])
            .destination()
            .and_then(|value| value.value())
            .expect("remainder value should be concrete");
        let sext_value = test
            .tree
            .get(block1.instructions[6])
            .destination()
            .and_then(|value| value.value())
            .expect("sign-extend value should be concrete");
        let uext_value = test
            .tree
            .get(block1.instructions[7])
            .destination()
            .and_then(|value| value.value())
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

        let actual_divide = scev
            .scev_for_value_in_loop(loop_index, divide_value)
            .unwrap();
        let actual_shift = scev
            .scev_for_value_in_loop(loop_index, shift_value)
            .unwrap();
        let actual_remainder = scev
            .scev_for_value_in_loop(loop_index, remainder_value)
            .unwrap();
        let actual_sext = scev.scev_for_value_in_loop(loop_index, sext_value).unwrap();
        let actual_uext = scev.scev_for_value_in_loop(loop_index, uext_value).unwrap();

        assert_eq!(actual_divide, &expected_divide);
        assert_eq!(actual_shift, &expected_shift);
        assert_eq!(actual_remainder, &expected_remainder);
        assert_eq!(actual_sext, &expected_sext);
        assert_eq!(actual_uext, &expected_uext);
    }
}

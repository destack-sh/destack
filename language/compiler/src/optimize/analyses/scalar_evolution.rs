use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use destack_mir as mir;

use crate::optimize::common::{
    SuccessorArguments, constant_from_global, constant_is_zero, constant_zero_for_type,
    fold_binary, instruction_is_pure, terminator_arguments_for_successor_checked,
};
use crate::optimize::{Analysis, AnalysisKind, OptimizationContext};

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
        tree: &mir::NodeTree,
        cfg: &ControlFlowGraph,
        loops: &LoopAnalysis,
    ) -> Self {
        // handle functions without bodies
        if function.entry.is_none() {
            return Self {
                loop_scev: HashMap::new(),
            };
        }

        // build definition metadata for values
        let definitions = ValueDefinitions::build(function, tree);

        // compute per loop SCEV maps
        let mut loop_scev = HashMap::new();
        for (loop_index, lp) in loops.loops().iter().enumerate() {
            let mut builder = LoopScevBuilder::new(function, tree, cfg, lp, &definitions);
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
    const KIND: AnalysisKind = AnalysisKind::ScalarEvolution;

    fn compute(
        function: &mir::Function,
        tree: &mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> Arc<Self> {
        let cfg = context
            .analyses
            .get::<ControlFlowGraph>(function, tree, context);
        let loops = context
            .analyses
            .get::<LoopAnalysis>(function, tree, context);

        Arc::new(Self::build(function, tree, &cfg, &loops))
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
    fn build(function: &mir::Function, tree: &mir::NodeTree) -> Self {
        let mut definitions = HashMap::new();

        // record block parameters
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for (index, param) in block.parameters.iter().enumerate() {
                definitions.insert(
                    param.value,
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
                if let Some(destination) = instruction.destination() {
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

/// Helper for building loop local scalar evolution expressions.
struct LoopScevBuilder<'a> {
    /// MIR node tree.
    tree: &'a mir::NodeTree,
    /// Control flow graph.
    cfg: &'a ControlFlowGraph,
    /// Loop being analyzed.
    lp: &'a Loop,
    /// Value definition metadata.
    definitions: &'a ValueDefinitions,
    /// Loop invariant values.
    invariants: HashSet<mir::Value>,
    /// Cached SCEV expressions.
    cache: HashMap<mir::Value, Scev>,
    /// Values currently being computed.
    in_progress: HashSet<mir::Value>,
}

impl<'a> LoopScevBuilder<'a> {
    /// Create a new builder for a loop.
    fn new(
        function: &'a mir::Function,
        tree: &'a mir::NodeTree,
        cfg: &'a ControlFlowGraph,
        lp: &'a Loop,
        definitions: &'a ValueDefinitions,
    ) -> Self {
        let invariants = collect_loop_invariants(function, tree, lp, definitions);

        Self {
            tree,
            cfg,
            lp,
            definitions,
            invariants,
            cache: HashMap::new(),
            in_progress: HashSet::new(),
        }
    }

    /// Build the SCEV map for this loop.
    fn build(&mut self) -> HashMap<mir::Value, Scev> {
        let mut values = Vec::new();

        // collect values defined in the loop
        for &block_id in &self.lp.blocks {
            let block = self.tree.get(block_id);
            for param in &block.parameters {
                values.push(param.value);
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
            } => self.scev_for_binary(value, *operator, *left, *right),
            mir::Instruction::Unary {
                operator, argument, ..
            } => self.scev_for_unary(value, *operator, *argument),
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
                scev_mul(left_scev, right_scev)
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
        let latch_arg = self.resolve_forwarded_value(latch_arg);

        // unchanged parameter implies zero step
        if latch_arg == param_value {
            let step_zero = zero_constant_for_param(self.tree, self.lp.header, param_index)?;
            return Some(Scev::Constant(step_zero));
        }

        // require latch argument to be defined by a binary op
        let definition = self.definitions.definition_for(latch_arg)?;
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

        // derive the step for supported linear patterns
        match operator {
            mir::BinaryOperator::Add => {
                if *left == param_value && self.is_invariant_value(*right) {
                    Some(self.scev_for_value(*right))
                } else if *right == param_value && self.is_invariant_value(*left) {
                    Some(self.scev_for_value(*left))
                } else {
                    None
                }
            }
            mir::BinaryOperator::Subtract => {
                if *left == param_value && self.is_invariant_value(*right) {
                    let step = self.scev_for_value(*right);
                    Some(scev_negate(step))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Resolve forwarded block parameters to their consistent arguments.
    fn resolve_forwarded_value(&self, value: mir::Value) -> mir::Value {
        let mut current = value;
        let mut visited = HashSet::new();

        // follow consistent parameter forwarding chains
        loop {
            if !visited.insert(current) {
                return current;
            }

            let Some(definition) = self.definitions.definition_for(current) else {
                return current;
            };

            let ValueDefinitionKind::Parameter { index } = definition.kind else {
                return current;
            };

            let block_id = definition.block;
            let block = self.tree.get(block_id);
            if block.parameters.len() <= index {
                return current;
            }

            let mut forwarded: Option<mir::Value> = None;
            let mut saw_pred = false;

            for &pred in self.cfg.predecessors(block_id) {
                let args = match terminator_arguments_for_successor_checked(
                    &self.tree.get(pred).terminator,
                    block_id,
                ) {
                    SuccessorArguments::Missing => continue,
                    SuccessorArguments::Conflict => return current,
                    SuccessorArguments::Consistent(args) => args,
                };

                saw_pred = true;
                let Some(arg) = args.get(index).copied() else {
                    return current;
                };

                if let Some(existing) = forwarded {
                    if existing != arg {
                        return current;
                    }
                } else {
                    forwarded = Some(arg);
                }
            }

            if !saw_pred {
                return current;
            }

            let Some(arg) = forwarded else {
                return current;
            };

            current = arg;
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
    tree: &mir::NodeTree,
    lp: &Loop,
    definitions: &ValueDefinitions,
) -> HashSet<mir::Value> {
    let mut invariants = HashSet::new();

    // seed invariants with function parameters
    for param in &function.parameters {
        invariants.insert(param.value);
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
    tree: &mir::NodeTree,
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

    tree.get_arguments(args_slice)
        .iter()
        .all(|value| invariants.contains(value))
}

/// Get the header argument for a specific predecessor and parameter.
fn header_argument_from_pred(
    tree: &mir::NodeTree,
    pred: mir::LocalNodeId<mir::Block>,
    header: mir::LocalNodeId<mir::Block>,
    param_index: usize,
) -> Option<mir::Value> {
    // collect predecessor arguments for the header edge
    let pred_block = tree.get(pred);
    let args =
        crate::optimize::common::terminator_arguments_for_successor(&pred_block.terminator, header);

    args.get(param_index).copied()
}

/// Get a constant value for an SSA value if it is constant.
fn constant_for_value(
    value: mir::Value,
    tree: &mir::NodeTree,
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
        mir::Instruction::GlobalConst { global, .. } => constant_from_global(*global, tree),
        _ => None,
    }
}

/// Build a zero constant for a parameter type.
fn zero_constant_for_param(
    tree: &mir::NodeTree,
    header: mir::LocalNodeId<mir::Block>,
    param_index: usize,
) -> Option<mir::Constant> {
    // resolve the parameter type
    let header_block = tree.get(header);
    let param = header_block.parameters.get(param_index)?;
    let ty = tree.get(param.ty);
    constant_zero_for_type(ty)
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

    Scev::Add(Box::new(left), Box::new(right))
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

    Scev::Mul(Box::new(left), Box::new(right))
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
            mir::Constant::Float { bits, width } => {
                let negated = match width {
                    32 => {
                        let value = f32::from_bits(bits as u32);
                        (-(value)).to_bits() as u64
                    }
                    64 => {
                        let value = f64::from_bits(bits);
                        (-(value)).to_bits()
                    }
                    _ => bits,
                };
                Scev::Constant(mir::Constant::Float {
                    bits: negated,
                    width,
                })
            }
            mir::Constant::Boolean { value } => {
                Scev::Constant(mir::Constant::Boolean { value: !value })
            }
            other => Scev::Constant(other),
        },
        other => Scev::Neg(Box::new(other)),
    }
}

/// Check if a constant is zero.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Simple induction variable becomes an add recurrence.
    #[test]
    fn test_addrec_simple_loop() {
        let program = TestProgram::new(
            r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    v4 = iconst 1i32
    v5 = iadd v3, v4
    v6 = icmp_slt v5, v1
    branch v6, block1(v5), block2(v5)
block2(v7: i32):
    return v7
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let context = program.context();

        let loops = context
            .analyses
            .get::<LoopAnalysis>(function, &program.tree, &context);
        let scev = context
            .analyses
            .get::<ScalarEvolution>(function, &program.tree, &context);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let header_block = program.tree.get(function.blocks[1]);
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
        let program = TestProgram::new(
            r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 1i32
    jump block1(v2)
block1(v3: i32):
    v4 = imul v3, v1
    v5 = icmp_slt v4, v1
    branch v5, block1(v4), block2(v4)
block2(v6: i32):
    return v6
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let context = program.context();

        let loops = context
            .analyses
            .get::<LoopAnalysis>(function, &program.tree, &context);
        let scev = context
            .analyses
            .get::<ScalarEvolution>(function, &program.tree, &context);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let header_block = program.tree.get(function.blocks[1]);
        let param_value = header_block.parameters[0].value;

        let actual = scev
            .scev_for_value_in_loop(loop_index, param_value)
            .unwrap();
        assert!(matches!(actual, Scev::Unknown(_)));
    }

    /// Subtractive induction variables produce negative steps.
    #[test]
    fn test_addrec_subtract_step() {
        let program = TestProgram::new(
            r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 10i32
    jump block1(v1)
block1(v2: i32):
    v3 = iconst 1i32
    v4 = isub v2, v3
    v5 = icmp_sgt v4, v0
    branch v5, block1(v4), block2(v4)
block2(v6: i32):
    return v6
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let context = program.context();

        let loops = context
            .analyses
            .get::<LoopAnalysis>(function, &program.tree, &context);
        let scev = context
            .analyses
            .get::<ScalarEvolution>(function, &program.tree, &context);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let header_block = program.tree.get(function.blocks[1]);
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
        let program = TestProgram::new(
            r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 5i32
    jump block1(v1)
block1(v2: i32):
    v3 = icmp_slt v2, v0
    branch v3, block2(v2), block3(v2)
block2(v4: i32):
    jump block1(v4)
block3(v5: i32):
    return v5
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let context = program.context();

        let loops = context
            .analyses
            .get::<LoopAnalysis>(function, &program.tree, &context);
        let scev = context
            .analyses
            .get::<ScalarEvolution>(function, &program.tree, &context);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let header_block = program.tree.get(function.blocks[1]);
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
        let program = TestProgram::new(
            r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 0i32
    jump block1(v2)
block1(v3: i32):
    v4 = iconst 2i32
    v5 = iadd v3, v4
    v6 = iconst 1i32
    v7 = iadd v3, v6
    v8 = icmp_slt v7, v1
    branch v8, block1(v7), block2(v5)
block2(v9: i32):
    return v9
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let context = program.context();

        let loops = context
            .analyses
            .get::<LoopAnalysis>(function, &program.tree, &context);
        let scev = context
            .analyses
            .get::<ScalarEvolution>(function, &program.tree, &context);

        let loop_index = loops
            .loops()
            .iter()
            .position(|lp| lp.header == function.blocks[1])
            .unwrap();

        let header_block = program.tree.get(function.blocks[1]);
        let param_value = header_block.parameters[0].value;

        let block1 = program.tree.get(function.blocks[1]);
        let instruction_id = block1.instructions[1];
        let instruction = program.tree.get(instruction_id);
        let derived_value = instruction.destination().unwrap();

        let param_scev = scev
            .scev_for_value_in_loop(loop_index, param_value)
            .unwrap()
            .clone();

        let expected = Scev::Add(
            Box::new(param_scev),
            Box::new(Scev::Constant(mir::Constant::Int {
                value: 2,
                width: 32,
                is_signed: true,
            })),
        );

        let actual = scev
            .scev_for_value_in_loop(loop_index, derived_value)
            .unwrap();
        assert_eq!(actual, &expected);
    }
}

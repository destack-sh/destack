use std::collections::{HashMap, HashSet};

use crate as mir;

use super::{Analysis, AnalysisId, FunctionAnalysis, FunctionAnalysisCache, Mutation};

/// Definition site for one SSA value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueDefinition {
    /// Function parameter definition.
    FunctionParameter(usize),
    /// Block parameter definition.
    BlockParameter {
        /// The block that owns the parameter.
        block: mir::LocalNodeId<mir::Block>,
        /// The parameter index inside the block.
        index: usize,
    },
    /// Instruction result definition.
    Instruction {
        /// The block that owns the instruction.
        block: mir::LocalNodeId<mir::Block>,
        /// The instruction id.
        instruction: mir::LocalNodeId<mir::Instruction>,
    },
}

impl ValueDefinition {
    /// Return the defining block when this value is block local.
    pub fn block(self) -> Option<mir::LocalNodeId<mir::Block>> {
        match self {
            Self::FunctionParameter(_) => None,
            Self::BlockParameter { block, .. } | Self::Instruction { block, .. } => Some(block),
        }
    }

    /// Return the defining instruction when this value is an instruction result.
    pub fn instruction(self) -> Option<mir::LocalNodeId<mir::Instruction>> {
        match self {
            Self::Instruction { instruction, .. } => Some(instruction),
            _ => None,
        }
    }
}

/// Definitions and integer constants for one MIR function.
#[derive(Debug, Clone, Default)]
pub struct ValueDefinitions {
    /// Definition site for each SSA value.
    definitions: Vec<Option<ValueDefinition>>,
    /// Integer constant value for each SSA value known to be constant.
    constants: Vec<Option<i64>>,
    /// Values written into each local.
    local_values: HashMap<mir::LocalId, Vec<mir::Value>>,
    /// Values passed to each block parameter.
    block_parameter_values: HashMap<mir::Value, Vec<mir::Value>>,
}

impl ValueDefinitions {
    /// Build value definitions for one function.
    pub fn build(function: &mir::Function, tree: &mir::Tree) -> Self {
        let value_count = function.value_capacity();
        let mut definitions = vec![None; value_count];
        let mut constants = vec![None; value_count];
        let mut local_values = HashMap::new();

        // record function parameter definitions
        for (index, parameter) in function.parameters.iter().enumerate() {
            definitions[parameter.value.0 as usize] =
                Some(ValueDefinition::FunctionParameter(index));
        }

        // record block parameter and instruction definitions
        for &block_id in function.blocks() {
            let block = tree.get(block_id);

            for (index, parameter) in block.parameters.iter().enumerate() {
                definitions[parameter.value.0 as usize] = Some(ValueDefinition::BlockParameter {
                    block: block_id,
                    index,
                });
            }

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination() {
                    definitions[destination.0 as usize] = Some(ValueDefinition::Instruction {
                        block: block_id,
                        instruction: instruction_id,
                    });
                }

                if let mir::Instruction::Const { destination, value } = instruction
                    && let mir::Constant::Int { value, .. } = value
                    && let Ok(value) = i64::try_from(*value)
                {
                    constants[destination.0 as usize] = Some(value);
                }

                if let mir::Instruction::LocalSet { local, value } = instruction {
                    local_values
                        .entry(*local)
                        .or_insert_with(Vec::new)
                        .push(*value);
                }
            }
        }

        let block_parameter_values = Self::build_block_parameter_values(function, tree);

        Self {
            definitions,
            constants,
            local_values,
            block_parameter_values,
        }
    }

    /// Return the definition site for one value.
    pub fn definition(&self, value: impl Into<mir::Value>) -> Option<ValueDefinition> {
        let value = value.into();

        self.definitions.get(value.0 as usize).copied().flatten()
    }

    /// Return the defining instruction for one value.
    pub fn instruction(
        &self,
        value: impl Into<mir::Value>,
    ) -> Option<mir::LocalNodeId<mir::Instruction>> {
        self.definition(value)
            .and_then(ValueDefinition::instruction)
    }

    /// Return the defining block for one value.
    pub fn block(&self, value: impl Into<mir::Value>) -> Option<mir::LocalNodeId<mir::Block>> {
        self.definition(value).and_then(ValueDefinition::block)
    }

    /// Return the integer constant for one value.
    pub fn int_constant(&self, value: impl Into<mir::Value>) -> Option<i64> {
        let value = value.into();

        self.constants.get(value.0 as usize).copied().flatten()
    }

    /// Return the frame allocation base for a derived reference value.
    pub fn frame_alloc_base(
        &self,
        value: impl Into<mir::Value>,
        tree: &mir::Tree,
    ) -> Option<mir::Value> {
        let mut current = value.into();
        let mut visited = HashSet::new();

        loop {
            // stop on cycles
            if !visited.insert(current) {
                return None;
            }

            // read the defining instruction
            let instruction_id = self.instruction(current)?;
            let instruction = tree.get(instruction_id);

            // walk through address computations
            match instruction {
                mir::Instruction::FrameAllocZeroed { destination, .. }
                | mir::Instruction::FrameAllocUninit { destination, .. }
                    if *destination == current =>
                {
                    return Some(current);
                }
                mir::Instruction::FieldAddr { aggregate, .. } => {
                    current = *aggregate;
                }
                mir::Instruction::ElementAddr { base, .. } => {
                    current = *base;
                }
                mir::Instruction::Cast { argument, .. } => {
                    current = *argument;
                }
                _ => return None,
            }
        }
    }

    /// Return the raw instruction definition map.
    pub fn instruction_map(&self) -> HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>> {
        self.definitions
            .iter()
            .enumerate()
            .filter_map(|(value, definition)| {
                definition
                    .and_then(ValueDefinition::instruction)
                    .map(|instruction| (mir::Value(value as u32), instruction))
            })
            .collect()
    }

    /// Collect frame allocation bases reachable from one value.
    pub fn collect_frame_alloc_bases(
        &self,
        value: mir::Value,
        tree: &mir::Tree,
        frame_allocs: &HashSet<mir::Value>,
        visited: &mut HashSet<mir::Value>,
        bases: &mut HashSet<mir::Value>,
    ) {
        // avoid repeating work for values
        if !visited.insert(value) {
            return;
        }

        // resolve the frame allocation base directly
        if let Some(base) = self.frame_alloc_base(value, tree) {
            if frame_allocs.contains(&base) {
                bases.insert(base);
            }
            return;
        }

        // walk through block parameter definitions
        let Some(instruction_id) = self.instruction(value) else {
            if let Some(params) = self.block_parameter_values.get(&value) {
                for &arg in params {
                    self.collect_frame_alloc_bases(arg, tree, frame_allocs, visited, bases);
                }
            }
            return;
        };

        // walk through aggregate and local projections
        let instruction = tree.get(instruction_id);
        match instruction {
            mir::Instruction::Aggregate { values, .. } => {
                for arg in tree.get_values(*values).iter().copied() {
                    self.collect_frame_alloc_bases(arg, tree, frame_allocs, visited, bases);
                }
            }
            mir::Instruction::Select {
                then_value,
                else_value,
                ..
            } => {
                self.collect_frame_alloc_bases(*then_value, tree, frame_allocs, visited, bases);
                self.collect_frame_alloc_bases(*else_value, tree, frame_allocs, visited, bases);
            }
            mir::Instruction::FieldGet { aggregate, .. }
            | mir::Instruction::ElementGet { aggregate, .. } => {
                self.collect_frame_alloc_bases(*aggregate, tree, frame_allocs, visited, bases);
            }
            mir::Instruction::LocalGet { local, .. } => {
                if let Some(values) = self.local_values.get(local) {
                    for &arg in values {
                        self.collect_frame_alloc_bases(arg, tree, frame_allocs, visited, bases);
                    }
                }
            }
            _ => {}
        }
    }

    /// Iterate over SSA values and their definition sites.
    pub fn definitions(&self) -> impl Iterator<Item = (mir::Value, ValueDefinition)> + '_ {
        self.definitions
            .iter()
            .enumerate()
            .filter_map(|(value, definition)| {
                definition.map(|definition| (mir::Value(value as u32), definition))
            })
    }

    /// Return values written into locals.
    pub fn local_values(&self) -> &HashMap<mir::LocalId, Vec<mir::Value>> {
        &self.local_values
    }

    /// Return values passed to block parameters.
    pub fn block_parameter_values(&self) -> &HashMap<mir::Value, Vec<mir::Value>> {
        &self.block_parameter_values
    }

    /// Build block parameter definitions from predecessor arguments.
    fn build_block_parameter_values(
        function: &mir::Function,
        tree: &mir::Tree,
    ) -> HashMap<mir::Value, Vec<mir::Value>> {
        let mut values = HashMap::new();

        // collect arguments from every outgoing target
        for &block_id in function.blocks() {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            for (_, target) in terminator.targets(tree, block_id) {
                Self::add_block_parameter_values(&mut values, terminator, target, tree);
            }
        }

        values
    }

    /// Add one target's arguments to the block parameter value map.
    fn add_block_parameter_values(
        values: &mut HashMap<mir::Value, Vec<mir::Value>>,
        terminator: &mir::Terminator,
        target: &mir::BlockTarget,
        tree: &mir::Tree,
    ) {
        let parameters = terminator.successor_parameters(tree, target.block);
        let arguments = target.arguments(tree);

        // require verified edge arity
        assert_eq!(
            parameters.len(),
            arguments.len(),
            "block target arity mismatch for {:?}",
            target.block
        );

        // pair target arguments with the destination block parameters
        for (parameter, argument) in parameters.iter().zip(arguments) {
            values.entry(parameter.value).or_default().push(*argument);
        }
    }
}

impl Analysis for ValueDefinitions {
    const ID: AnalysisId = AnalysisId("value-definitions");
    const INVALIDATED_BY: Mutation = Mutation::VALUE;
}

impl FunctionAnalysis for ValueDefinitions {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        _analyses: &FunctionAnalysisCache,
    ) -> Self {
        Self::build(function, tree)
    }
}

/// One operand occurrence of an SSA value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueUse {
    /// An instruction reads the value.
    Instruction {
        /// The block that owns the instruction.
        block: mir::BlockId,
        /// The instruction that reads the value.
        instruction: mir::LocalNodeId<mir::Instruction>,
        /// The operand index among the instruction's read values.
        index: usize,
    },
    /// A terminator reads the value.
    Terminator {
        /// The block that owns the terminator.
        block: mir::BlockId,
        /// The operand index among the terminator's read values.
        index: usize,
    },
}

/// Operand uses for SSA values in one MIR function.
#[derive(Debug, Clone, Default)]
pub struct ValueUses {
    /// Operand uses indexed by value id.
    uses: Vec<Vec<ValueUse>>,
}

impl ValueUses {
    /// Build operand uses for one function.
    pub fn build(function: &mir::Function, tree: &mir::Tree) -> Self {
        let mut value_uses = Self {
            uses: vec![Vec::new(); function.value_capacity()],
        };

        // scan every executable block
        for &block_id in function.blocks() {
            let block = tree.get(block_id);

            // scan instruction operands
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                for (index, value) in instruction.reads(tree).into_iter().enumerate() {
                    value_uses.record(
                        value,
                        ValueUse::Instruction {
                            block: block_id,
                            instruction: instruction_id,
                            index,
                        },
                    );
                }
            }

            // scan terminator operands
            let terminator = tree.get(block.terminator);
            for (index, value) in terminator.uses(tree).into_iter().enumerate() {
                value_uses.record(
                    value,
                    ValueUse::Terminator {
                        block: block_id,
                        index,
                    },
                );
            }
        }

        value_uses
    }

    /// Return operand uses for one value.
    pub fn uses(&self, value: impl Into<mir::Value>) -> &[ValueUse] {
        let value = value.into();
        let index = value.0 as usize;

        match self.uses.get(index) {
            Some(uses) => uses,
            None => unreachable!("value use outside function value table: {value:?}"),
        }
    }

    /// Return how many operand occurrences read one value.
    pub fn count(&self, value: impl Into<mir::Value>) -> usize {
        self.uses(value).len()
    }

    /// Return whether the value has at least one operand use.
    pub fn is_used(&self, value: impl Into<mir::Value>) -> bool {
        !self.uses(value).is_empty()
    }

    /// Record one operand use.
    fn record(&mut self, value: mir::Value, value_use: ValueUse) {
        let index = value.0 as usize;

        let Some(uses) = self.uses.get_mut(index) else {
            unreachable!("value use outside function value table: {value:?}");
        };

        uses.push(value_use);
    }
}

impl Analysis for ValueUses {
    const ID: AnalysisId = AnalysisId("value-uses");
    const INVALIDATED_BY: Mutation = Mutation::VALUE;
}

impl FunctionAnalysis for ValueUses {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        _analyses: &FunctionAnalysisCache,
    ) -> Self {
        Self::build(function, tree)
    }
}

/// Value and local type lookup for a MIR function.
#[derive(Debug, Clone)]
pub struct ValueTypes {
    /// SSA value types indexed by value id.
    values: Vec<Option<mir::LocalNodeId<mir::Type>>>,
    /// Local types indexed by local id.
    locals: Vec<Option<mir::LocalNodeId<mir::Type>>>,
}

impl ValueTypes {
    /// Build value types for a function.
    pub fn new(function: &mir::Function, tree: &mir::Tree) -> Self {
        // seed value types from the function table
        let values = function.value_types().to_vec();

        // seed local types from the local table
        let local_count = function.local_capacity();
        let mut locals = vec![None; local_count];
        for &local_id in function.locals() {
            let local = tree.get(local_id);
            locals[local_id.id as usize] = Some(local.ty);
        }

        Self { values, locals }
    }

    /// Return the type of a value.
    pub fn value_type(&self, value: impl Into<mir::Value>) -> Option<mir::LocalNodeId<mir::Type>> {
        let value = value.into();
        self.values.get(value.0 as usize).copied().flatten()
    }

    /// Return the expected type of a value.
    pub fn expect_value_type(&self, value: impl Into<mir::Value>) -> mir::LocalNodeId<mir::Type> {
        let value = value.into();
        match self.value_type(value) {
            Some(type_id) => type_id,
            None => panic!("missing value type for {value:?}"),
        }
    }

    /// Return the type of a local when available.
    pub fn local_type(
        &self,
        local: impl Into<mir::LocalId>,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        let local = local.into();

        self.locals.get(local.id as usize).copied().flatten()
    }

    /// Return the expected type of a local.
    pub fn expect_local_type(&self, local: impl Into<mir::LocalId>) -> mir::LocalNodeId<mir::Type> {
        let local = local.into();
        match self.local_type(local) {
            Some(type_id) => type_id,
            None => panic!("missing type for local {local:?}"),
        }
    }

    /// Resolve a reference's referent type when statically known.
    pub fn reference_referent_type(
        &self,
        reference: impl Into<mir::Value>,
        tree: &mir::Tree,
    ) -> Option<mir::TypeId> {
        let type_id = self.expect_value_type(reference);
        match tree.get(type_id) {
            mir::Type::Reference { pointee, .. } => Some(*pointee),
            mir::Type::Slice { element, .. } => Some(*element),
            mir::Type::TensorView { element, .. } => Some(*element),
            _ => None,
        }
    }

    /// Resolve a reference's TS++ space when statically known.
    pub fn reference_space(
        &self,
        reference: impl Into<mir::Value>,
        tree: &mir::Tree,
    ) -> Option<mir::Space> {
        let type_id = self.expect_value_type(reference);
        match tree.get(type_id) {
            mir::Type::Reference { space, .. } => Some(*space),
            mir::Type::Slice { space, .. } => Some(*space),
            mir::Type::TensorView { space, .. } => Some(*space),
            _ => None,
        }
    }

    /// Resolve a reference's kind when statically known.
    pub fn reference_kind(
        &self,
        reference: impl Into<mir::Value>,
        tree: &mir::Tree,
    ) -> Option<mir::ReferenceKind> {
        let type_id = self.expect_value_type(reference);
        match tree.get(type_id) {
            mir::Type::Reference { kind, .. } => Some(*kind),
            mir::Type::Slice { kind, .. } => Some(*kind),
            mir::Type::TensorView { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// Return the raw value type table.
    pub fn values(&self) -> &[Option<mir::LocalNodeId<mir::Type>>] {
        &self.values
    }
}

impl Analysis for ValueTypes {
    const ID: AnalysisId = AnalysisId("value-types");
    const INVALIDATED_BY: Mutation = Mutation::VALUE;
}

impl FunctionAnalysis for ValueTypes {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        _analyses: &FunctionAnalysisCache,
    ) -> Self {
        Self::new(function, tree)
    }
}

#[cfg(test)]
mod tests {
    use destack_core::StringPool;
    use destack_source::FileId;

    use crate as mir;
    use crate::parse::{ParseOptions, Parser};

    use super::{ValueDefinitions, ValueUse, ValueUses};

    /// Parse one MIR tree for value definition tests.
    fn parse_tree(source: &str) -> (mir::Tree, StringPool) {
        Parser::parse(FileId::new(0), source, ParseOptions::default())
            .finish()
            .expect("parse failed")
    }

    /// Return one named block from a tree.
    fn block_by_name(
        tree: &mir::Tree,
        strings: &StringPool,
        name: &str,
    ) -> mir::LocalNodeId<mir::Block> {
        tree.iter_nodes::<mir::Block>()
            .find(|(_, block)| {
                block
                    .name
                    .map(|name_id| strings.get(name_id) == name)
                    .unwrap_or(false)
            })
            .expect("missing block")
            .0
    }

    /// Return the first function from a tree.
    fn first_function(tree: &mir::Tree) -> mir::LocalNodeId<mir::Function> {
        tree.iter_nodes::<mir::Function>()
            .next()
            .expect("missing function")
            .0
    }

    /// Value uses include instruction operands and terminator operands.
    #[test]
    fn test_collect_value_uses() {
        let (tree, _) = parse_tree(
            r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.add v2, v1
    jump b1(v3)

b1(v4: int32):
    return v4
}
"#,
        );
        let function = tree.get(first_function(&tree));
        let uses = ValueUses::build(function, &tree);
        let entry = function.block(0);
        let entry_block = tree.get(entry);
        let exit = function.block(1);

        // v1 is used by both arithmetic instructions
        assert_eq!(
            uses.uses(mir::Value(1)),
            &[
                ValueUse::Instruction {
                    block: entry,
                    instruction: entry_block.instructions[0],
                    index: 1,
                },
                ValueUse::Instruction {
                    block: entry,
                    instruction: entry_block.instructions[1],
                    index: 1,
                }
            ]
        );

        // v3 is passed as a block argument
        assert_eq!(
            uses.uses(mir::Value(3)),
            &[ValueUse::Terminator {
                block: entry,
                index: 0
            }]
        );

        // v4 is returned by the exit block
        assert_eq!(
            uses.uses(mir::Value(4)),
            &[ValueUse::Terminator {
                block: exit,
                index: 0
            }]
        );
    }

    /// Fallible allocation success results are not treated as edge arguments.
    #[test]
    fn test_block_parameter_values_skip_fallible_allocation_result() {
        let (tree, strings) = parse_tree(
            r#"
function test(v0: int64, v1: int32): int32 {
entry(v0: int64, v1: int32):
    new.slice.uninit.try int32, v0 => b1(v1), b2(v1)

b1(v2: uninit<slice<int32, managed, mutable>>, v3: int32):
    return v3

b2(v4: int32):
    return v4
}
"#,
        );
        let function = tree.get(first_function(&tree));
        let definitions = ValueDefinitions::build(function, &tree);
        let success = tree.get(block_by_name(&tree, &strings, "b1"));
        let failure = tree.get(block_by_name(&tree, &strings, "b2"));
        let success_result = success.parameters[0].value;
        let success_payload = success.parameters[1].value;
        let failure_payload = failure.parameters[0].value;

        // success result is produced by the terminator and has no edge argument
        assert!(
            !definitions
                .block_parameter_values()
                .contains_key(&success_result)
        );

        // explicit payloads on both edges come from the same source value
        assert_eq!(
            definitions.block_parameter_values()[&success_payload],
            definitions.block_parameter_values()[&failure_payload]
        );
    }
}

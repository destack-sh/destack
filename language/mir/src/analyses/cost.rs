use crate as mir;

use super::{Analysis, FunctionCache, Mutation};
use crate::{Block, NodeTable};

/// MIR cost model for one function.
#[derive(Debug, Clone)]
pub struct CostTable {
    /// Cost weights used by this model.
    weights: CostWeights,
    /// Function operation inventory.
    function: OperationCost,
    /// Score per block.
    blocks: NodeTable<Block, u64>,
}

/// MIR operation inventory and weighted score for one function.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OperationCost {
    /// Number of MIR blocks.
    pub blocks: usize,
    /// Number of MIR instructions.
    pub instructions: usize,
    /// Number of MIR terminators.
    pub terminators: usize,
    /// Arithmetic, conversion, selection, and scalar descriptor operations.
    pub arithmetic: usize,
    /// Aggregate construction, projection, vector, and tensor value operations.
    pub aggregate: usize,
    /// Address operations.
    pub address: usize,
    /// Non-atomic memory loads.
    pub load: usize,
    /// Non-atomic memory stores.
    pub store: usize,
    /// Atomic memory operations.
    pub atomic: usize,
    /// Heap allocation operations.
    pub allocate: usize,
    /// Explicit unique-storage release operations.
    pub release: usize,
    /// Runtime-erased drop operations.
    pub drop: usize,
    /// Collector write-barrier operations.
    pub write_barrier: usize,
    /// Direct calls.
    pub direct_call: usize,
    /// Function-pointer or closure calls.
    pub indirect_call: usize,
    /// Class virtual calls.
    pub virtual_call: usize,
    /// Dynamic dispatch calls.
    pub dynamic_call: usize,
    /// Intrinsic calls.
    pub intrinsic_call: usize,
    /// Control-flow split or transfer operations.
    pub branch: usize,
    /// Weighted score derived from this inventory.
    pub score: u64,
}

impl OperationCost {
    /// Return the total number of call-like operations.
    pub fn calls(self) -> usize {
        self.direct_call
            + self.indirect_call
            + self.virtual_call
            + self.dynamic_call
            + self.intrinsic_call
    }

    /// Create operation cost for one block.
    fn block() -> Self {
        Self {
            blocks: 1,
            ..Self::default()
        }
    }

    /// Add another operation inventory.
    fn record(&mut self, other: Self) {
        self.blocks += other.blocks;
        self.instructions += other.instructions;
        self.terminators += other.terminators;
        self.arithmetic += other.arithmetic;
        self.aggregate += other.aggregate;
        self.address += other.address;
        self.load += other.load;
        self.store += other.store;
        self.atomic += other.atomic;
        self.allocate += other.allocate;
        self.release += other.release;
        self.drop += other.drop;
        self.write_barrier += other.write_barrier;
        self.direct_call += other.direct_call;
        self.indirect_call += other.indirect_call;
        self.virtual_call += other.virtual_call;
        self.dynamic_call += other.dynamic_call;
        self.intrinsic_call += other.intrinsic_call;
        self.branch += other.branch;
        self.score = self.score.saturating_add(other.score);
    }

    /// Add the dispatch cost for one call.
    fn add_call(&mut self, call: &mir::Call) {
        match call.callee.dispatch() {
            mir::CallDispatch::Direct => self.direct_call += 1,
            mir::CallDispatch::Indirect => self.indirect_call += 1,
            mir::CallDispatch::Virtual { .. } => self.virtual_call += 1,
            mir::CallDispatch::Dynamic { .. } => self.dynamic_call += 1,
        }
    }

    /// Score this operation inventory with the given weights.
    fn scored(mut self, weights: CostWeights) -> Self {
        let mut score = 0u64;

        // structural weights
        score = score.saturating_add(weights.block.saturating_mul(self.blocks as u64));
        score = score.saturating_add(weights.instruction.saturating_mul(self.instructions as u64));
        score = score.saturating_add(weights.terminator.saturating_mul(self.terminators as u64));

        // operation-family weights
        score = score.saturating_add(weights.arithmetic.saturating_mul(self.arithmetic as u64));
        score = score.saturating_add(weights.aggregate.saturating_mul(self.aggregate as u64));
        score = score.saturating_add(weights.address.saturating_mul(self.address as u64));
        score = score.saturating_add(weights.load.saturating_mul(self.load as u64));
        score = score.saturating_add(weights.store.saturating_mul(self.store as u64));
        score = score.saturating_add(weights.atomic.saturating_mul(self.atomic as u64));
        score = score.saturating_add(weights.allocate.saturating_mul(self.allocate as u64));
        score = score.saturating_add(weights.release.saturating_mul(self.release as u64));
        score = score.saturating_add(weights.drop.saturating_mul(self.drop as u64));
        score = score.saturating_add(
            weights
                .write_barrier
                .saturating_mul(self.write_barrier as u64),
        );

        // call weights
        score = score.saturating_add(weights.direct_call.saturating_mul(self.direct_call as u64));
        score = score.saturating_add(
            weights
                .indirect_call
                .saturating_mul(self.indirect_call as u64),
        );
        score = score.saturating_add(
            weights
                .virtual_call
                .saturating_mul(self.virtual_call as u64),
        );
        score = score.saturating_add(
            weights
                .dynamic_call
                .saturating_mul(self.dynamic_call as u64),
        );
        score = score.saturating_add(
            weights
                .intrinsic_call
                .saturating_mul(self.intrinsic_call as u64),
        );
        score = score.saturating_add(weights.branch.saturating_mul(self.branch as u64));
        self.score = score;

        self
    }
}

/// Target and runtime weights for MIR operation scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostWeights {
    /// Structural cost of one block.
    pub block: u64,
    /// Structural cost of one MIR instruction.
    pub instruction: u64,
    /// Structural cost of one MIR terminator.
    pub terminator: u64,
    /// Cost of one arithmetic-like operation.
    pub arithmetic: u64,
    /// Cost of one aggregate-like operation.
    pub aggregate: u64,
    /// Cost of one address operation.
    pub address: u64,
    /// Cost of one load.
    pub load: u64,
    /// Cost of one store.
    pub store: u64,
    /// Cost of one atomic operation.
    pub atomic: u64,
    /// Cost of one allocation operation.
    pub allocate: u64,
    /// Cost of one explicit release operation.
    pub release: u64,
    /// Cost of one runtime-erased drop operation.
    pub drop: u64,
    /// Cost of one collector write barrier.
    pub write_barrier: u64,
    /// Cost of one direct call.
    pub direct_call: u64,
    /// Cost of one indirect call.
    pub indirect_call: u64,
    /// Cost of one virtual call.
    pub virtual_call: u64,
    /// Cost of one dynamic call.
    pub dynamic_call: u64,
    /// Cost of one intrinsic call.
    pub intrinsic_call: u64,
    /// Cost of one control-flow split or transfer.
    pub branch: u64,
}

impl Default for CostWeights {
    fn default() -> Self {
        Self {
            block: 3,
            instruction: 1,
            terminator: 1,
            arithmetic: 1,
            aggregate: 2,
            address: 1,
            load: 4,
            store: 5,
            atomic: 20,
            allocate: 25,
            release: 10,
            drop: 35,
            write_barrier: 8,
            direct_call: 25,
            indirect_call: 40,
            virtual_call: 45,
            dynamic_call: 50,
            intrinsic_call: 10,
            branch: 2,
        }
    }
}

impl CostTable {
    /// Return the cost weights.
    pub fn weights(&self) -> CostWeights {
        self.weights
    }

    /// Return the whole function operation inventory.
    pub fn function(&self) -> &OperationCost {
        &self.function
    }

    /// Return one block's weighted score.
    pub fn block(&self, block: mir::BlockId) -> u64 {
        *self.blocks.get(block)
    }

    /// Return one instruction's weighted score.
    pub fn instruction(&self, instruction: &mir::Instruction, tree: &mir::Tree) -> u64 {
        Self::instruction_cost(instruction, tree)
            .scored(self.weights)
            .score
    }

    /// Return one terminator's weighted score.
    pub fn terminator(&self, terminator: &mir::Terminator) -> u64 {
        Self::terminator_cost(terminator).scored(self.weights).score
    }

    /// Build a cost model for one function.
    fn build(function: &mir::Function, tree: &mir::Tree, weights: CostWeights) -> Self {
        let mut model = Self {
            weights,
            function: OperationCost::default(),
            blocks: NodeTable::from_nodes(function.blocks(), || 0),
        };

        // compute block and function costs together
        for &block_id in function.blocks() {
            let block_cost = model.compute_block(block_id, tree);
            *model.blocks.get_mut(block_id) = block_cost.score;
            model.function.record(block_cost);
        }

        model
    }

    /// Compute one block's cost.
    fn compute_block(&self, block_id: mir::BlockId, tree: &mir::Tree) -> OperationCost {
        let block = tree.get(block_id);
        let mut cost = OperationCost::block();

        // count instructions
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            cost.record(Self::instruction_cost(instruction, tree));
        }

        // count terminator
        let terminator = tree.get(block.terminator);
        cost.record(Self::terminator_cost(terminator));

        cost.scored(self.weights)
    }

    /// Compute one instruction's operation cost.
    fn instruction_cost(instruction: &mir::Instruction, tree: &mir::Tree) -> OperationCost {
        let mut cost = OperationCost {
            instructions: 1,
            ..OperationCost::default()
        };

        match instruction {
            mir::Instruction::Error => cost.instructions = 0,
            mir::Instruction::Binary { .. }
            | mir::Instruction::Unary { .. }
            | mir::Instruction::Cast { .. }
            | mir::Instruction::Select { .. }
            | mir::Instruction::Const { .. }
            | mir::Instruction::SliceLength { .. }
            | mir::Instruction::DynamicBind { .. }
            | mir::Instruction::DynamicPayload { .. }
            | mir::Instruction::DynamicType { .. }
            | mir::Instruction::DynamicRead { .. }
            | mir::Instruction::DynamicFind { .. }
            | mir::Instruction::FunctionEnvironment { .. }
            | mir::Instruction::FunctionEnvironmentCurrent { .. }
            | mir::Instruction::ProfileIncrement { .. }
            | mir::Instruction::ProfileSample { .. }
            | mir::Instruction::NewComplete { .. }
            | mir::Instruction::Assume { .. } => cost.arithmetic += 1,
            mir::Instruction::Breakpoint => cost.branch += 1,
            mir::Instruction::Aggregate { values, .. } => {
                cost.aggregate += 1 + tree.get_values(*values).len();
            }
            mir::Instruction::FieldGet { .. }
            | mir::Instruction::FieldSet { .. }
            | mir::Instruction::ElementGet { .. }
            | mir::Instruction::ElementSet { .. }
            | mir::Instruction::VariantNew { .. }
            | mir::Instruction::VariantTag { .. }
            | mir::Instruction::VariantPayload { .. }
            | mir::Instruction::SliceView { .. }
            | mir::Instruction::VectorSplat { .. }
            | mir::Instruction::VectorExtract { .. }
            | mir::Instruction::VectorInsert { .. }
            | mir::Instruction::VectorShuffle { .. }
            | mir::Instruction::VectorSelect { .. }
            | mir::Instruction::VectorReduce { .. }
            | mir::Instruction::VectorCompare { .. }
            | mir::Instruction::VectorConvert { .. }
            | mir::Instruction::TensorSplat { .. }
            | mir::Instruction::TensorExtract { .. }
            | mir::Instruction::TensorReshape { .. }
            | mir::Instruction::TensorBroadcast { .. }
            | mir::Instruction::TensorTranspose { .. }
            | mir::Instruction::TensorCast { .. }
            | mir::Instruction::TensorView { .. }
            | mir::Instruction::TensorSlice { .. }
            | mir::Instruction::TensorPad { .. }
            | mir::Instruction::TensorConcat { .. }
            | mir::Instruction::TensorCompare { .. }
            | mir::Instruction::TensorSelect { .. }
            | mir::Instruction::TensorReduce { .. }
            | mir::Instruction::TensorIndexReduce { .. }
            | mir::Instruction::TensorDot { .. }
            | mir::Instruction::TensorConvolution { .. }
            | mir::Instruction::TensorGather { .. }
            | mir::Instruction::TensorScatter { .. }
            | mir::Instruction::TensorConvert { .. } => cost.aggregate += 1,
            mir::Instruction::LocalAddr { .. }
            | mir::Instruction::GlobalAddr { .. }
            | mir::Instruction::FunctionAddr { .. }
            | mir::Instruction::FunctionBind { .. }
            | mir::Instruction::FieldAddr { .. }
            | mir::Instruction::ElementAddr { .. }
            | mir::Instruction::VariantPayloadAddr { .. } => cost.address += 1,
            mir::Instruction::LocalGet { .. }
            | mir::Instruction::Load { .. }
            | mir::Instruction::VariantTagLoad { .. }
            | mir::Instruction::ContextCurrent { .. }
            | mir::Instruction::TensorLoad { .. } => cost.load += 1,
            mir::Instruction::LocalSet { .. }
            | mir::Instruction::Store { .. }
            | mir::Instruction::TensorStore { .. }
            | mir::Instruction::TensorFill { .. }
            | mir::Instruction::TensorCopy { .. } => cost.store += 1,
            mir::Instruction::AtomicLoad { .. }
            | mir::Instruction::AtomicStore { .. }
            | mir::Instruction::AtomicCompareExchange { .. }
            | mir::Instruction::AtomicRmw { .. }
            | mir::Instruction::AtomicFence { .. } => cost.atomic += 1,
            mir::Instruction::NewZeroed { .. }
            | mir::Instruction::NewUninit { .. }
            | mir::Instruction::NewSliceZeroed { .. }
            | mir::Instruction::NewSliceUninit { .. }
            | mir::Instruction::ContextBind { .. }
            | mir::Instruction::Pin { .. }
            | mir::Instruction::Unpin { .. } => cost.allocate += 1,
            mir::Instruction::Free { .. } => {
                cost.release += 1;
            }
            mir::Instruction::BarrierWrite { .. } => cost.write_barrier += 1,
            mir::Instruction::Call { call, .. } => cost.add_call(call),
            mir::Instruction::Drop { .. } => cost.drop += 1,
            mir::Instruction::ContextReplace { .. }
            | mir::Instruction::ContextGet { .. }
            | mir::Instruction::Poll
            | mir::Instruction::Intrinsic { .. } => cost.intrinsic_call += 1,
        }

        cost
    }

    /// Compute one terminator's operation cost.
    fn terminator_cost(terminator: &mir::Terminator) -> OperationCost {
        let mut cost = OperationCost {
            terminators: 1,
            ..OperationCost::default()
        };

        match terminator {
            mir::Terminator::Error => cost.terminators = 0,
            mir::Terminator::Return { .. }
            | mir::Terminator::Jump { .. }
            | mir::Terminator::Unreachable
            | mir::Terminator::Panic { .. }
            | mir::Terminator::UnwindResume
            | mir::Terminator::Abort { .. } => {}
            mir::Terminator::Branch { .. }
            | mir::Terminator::Check { .. }
            | mir::Terminator::Switch { .. }
            | mir::Terminator::VariantSwitch { .. } => {
                cost.branch += 1;
            }
            mir::Terminator::Invoke { call, .. } | mir::Terminator::TailCall { call } => {
                cost.add_call(call);
            }
            mir::Terminator::NewZeroedTry { .. }
            | mir::Terminator::NewUninitTry { .. }
            | mir::Terminator::NewSliceZeroedTry { .. }
            | mir::Terminator::NewSliceUninitTry { .. } => cost.allocate += 1,
        }

        cost
    }
}

impl Analysis for CostTable {
    const INVALIDATED_BY: Mutation = Mutation::ALL;
}

impl CostTable {
    /// Compute the MIR cost model for one function.
    pub(crate) fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &mut FunctionCache,
    ) -> Self {
        Self::build(function, tree, analyses.options().cost_weights)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::AnalysisOptions;
    use crate::analyses::tests::TestProgram;

    /// Cost model counts factual operations separately from weighted score.
    #[test]
    fn test_cost_model_counts_operation_families() {
        let program = TestProgram::new(
            r#"
function test(v0: ref<int32, borrowed, mutable>): int32 {
entry(v0: ref<int32, borrowed, mutable>):
    v1: int32 = load v0
    v2: int32 = add v1, v1
    return v2
}
"#,
        );

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let mut analyses = program.function_analyses();
        let cost = analyses.cost(function, &program.tree);
        let weights = cost.weights();

        assert_eq!(cost.function().blocks, 1);
        assert_eq!(cost.function().instructions, 2);
        assert_eq!(cost.function().terminators, 1);
        assert_eq!(cost.function().load, 1);
        assert_eq!(cost.function().arithmetic, 1);
        assert_eq!(
            cost.function().score,
            weights.block
                + weights.instruction * 2
                + weights.terminator
                + weights.load
                + weights.arithmetic
        );
    }

    /// Dynamic dispatch is counted as dynamic dispatch, not generic call cost.
    #[test]
    fn test_cost_model_counts_dispatch_call_kind() {
        let program = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.dynamic v0, int32, 0(v0): (int32) => int32
    return v1
}
"#,
        );

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let mut analyses = program.function_analyses();
        let cost = analyses.cost(function, &program.tree);
        let weights = cost.weights();

        assert_eq!(cost.function().dynamic_call, 1);
        assert_eq!(cost.function().direct_call, 0);
        assert_eq!(
            cost.function().score,
            weights.block + weights.instruction + weights.terminator + weights.dynamic_call
        );
    }

    /// Loads, stores, atomics, allocations, releases, and barriers stay distinct.
    #[test]
    fn test_cost_model_counts_memory_protocols() {
        let program = TestProgram::new(
            r#"
function test(v0: ref<int32, borrowed, mutable>, v1: ref<atomic<int32>, borrowed, mutable>, v2: ref<int32, managed, mutable>): void {
entry(v0: ref<int32, borrowed, mutable>, v1: ref<atomic<int32>, borrowed, mutable>, v2: ref<int32, managed, mutable>):
    v3: int32 = load v0
    store v0, v3
    v4: ref<int32, unique, mutable> = new.zeroed int32
    barrier.write v4, v3, v3
    free v4
    drop v2
    atomic.store v1, v3, sequentiallyConsistent
    return
}
"#,
        );

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let mut analyses = program.function_analyses();
        let cost = analyses.cost(function, &program.tree);

        assert_eq!(cost.function().load, 1);
        assert_eq!(cost.function().store, 1);
        assert_eq!(cost.function().allocate, 1);
        assert_eq!(cost.function().write_barrier, 1);
        assert_eq!(cost.function().release, 1);
        assert_eq!(cost.function().drop, 1);
        assert_eq!(cost.function().atomic, 1);
    }

    /// Analysis options control cost weights.
    #[test]
    fn test_cost_model_uses_analysis_options() {
        let program = TestProgram::new(
            r#"
function test(v0: ref<int32, borrowed, mutable>): int32 {
entry(v0: ref<int32, borrowed, mutable>):
    v1: int32 = load v0
    return v1
}
"#,
        );

        let weights = CostWeights {
            block: 23,
            instruction: 2,
            terminator: 3,
            arithmetic: 5,
            aggregate: 7,
            address: 11,
            load: 13,
            store: 17,
            atomic: 19,
            allocate: 29,
            release: 31,
            drop: 33,
            write_barrier: 37,
            direct_call: 41,
            indirect_call: 43,
            virtual_call: 47,
            dynamic_call: 53,
            intrinsic_call: 59,
            branch: 61,
        };
        let analysis_options = AnalysisOptions::default().with_cost_weights(weights);
        let mut analyses = FunctionCache::with_options(analysis_options);

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let cost = analyses.cost(function, &program.tree);

        assert_eq!(cost.weights(), weights);
        assert_eq!(
            cost.function().score,
            weights.block + weights.instruction + weights.terminator + weights.load
        );
    }
}

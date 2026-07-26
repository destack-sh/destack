use crate as mir;
use destack_core::BitSet;

use super::{Analysis, AnalysisId, FunctionAnalysis, FunctionAnalysisCache, Mutation};

/// Escape analysis for allocation roots in one function.
#[derive(Debug, Clone)]
pub struct EscapeAnalysis {
    /// Allocation root by SSA value id.
    roots: Vec<Option<mir::Value>>,
    /// Allocation roots that escape the function.
    escaped: BitSet,
}

impl EscapeAnalysis {
    /// Return the allocation root for a value when known.
    pub fn allocation(&self, value: impl Into<mir::Value>) -> Option<mir::Value> {
        let value = value.into();

        self.roots.get(value.0 as usize).copied().flatten()
    }

    /// Return true when a value's allocation root escapes.
    pub fn escapes(&self, value: impl Into<mir::Value>) -> bool {
        self.allocation(value)
            .is_some_and(|allocation| self.escaped.contains(allocation.id() as usize))
    }

    /// Return true when an allocation root stays inside this function.
    pub fn stays_local(&self, value: impl Into<mir::Value>) -> bool {
        self.allocation(value)
            .is_some_and(|allocation| !self.escaped.contains(allocation.id() as usize))
    }

    /// Build escape analysis for one function.
    fn build(function: &mir::Function, tree: &mir::Tree) -> Self {
        let mut analysis = Self {
            roots: vec![None; function.value_capacity()],
            escaped: BitSet::new(function.value_capacity()),
        };

        // propagate allocation roots through local dataflow
        let mut propagation = EscapePropagation::new(function, tree, &mut analysis.roots);
        propagation.run();

        // mark roots that cross function or memory boundaries
        let mut escapes = EscapeMarker {
            tree,
            roots: &analysis.roots,
            escaped: &mut analysis.escaped,
        };
        escapes.mark_function(function);

        analysis
    }
}

impl Analysis for EscapeAnalysis {
    const ID: AnalysisId = AnalysisId("escape");
    const INVALIDATED_BY: Mutation = Mutation::ALL;
}

impl FunctionAnalysis for EscapeAnalysis {
    /// Compute escape analysis for one function.
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        _analyses: &FunctionAnalysisCache,
    ) -> Self {
        Self::build(function, tree)
    }
}

/// Allocation root propagation state.
struct EscapePropagation<'a, 'b> {
    /// The function being analyzed.
    function: &'a mir::Function,
    /// The MIR tree being analyzed.
    tree: &'a mir::Tree,
    /// Allocation root by SSA value id.
    roots: &'b mut [Option<mir::Value>],
    /// Root stored in each local slot.
    locals: Vec<Option<mir::Value>>,
}

impl<'a, 'b> EscapePropagation<'a, 'b> {
    /// Create propagation state.
    fn new(
        function: &'a mir::Function,
        tree: &'a mir::Tree,
        roots: &'b mut [Option<mir::Value>],
    ) -> Self {
        Self {
            function,
            tree,
            roots,
            locals: vec![None; function.local_capacity()],
        }
    }

    /// Propagate roots to a fixed point.
    fn run(&mut self) {
        let mut changed = true;

        while changed {
            changed = false;

            // propagate through each block
            for &block_id in self.function.blocks() {
                changed |= self.propagate_block(block_id);
            }
        }
    }

    /// Propagate roots through one block.
    fn propagate_block(&mut self, block_id: mir::BlockId) -> bool {
        let block = self.tree.get(block_id);
        let mut changed = false;

        // propagate instruction roots
        for &instruction_id in &block.instructions {
            let instruction = self.tree.get(instruction_id);
            changed |= self.propagate_instruction(instruction);
        }

        // propagate successor argument roots into block parameters
        let terminator = self.tree.get(block.terminator);
        for successor in terminator.successors(self.tree) {
            changed |= self.propagate_successor(terminator, successor);
        }

        changed
    }

    /// Propagate roots through one instruction.
    fn propagate_instruction(&mut self, instruction: &mir::Instruction) -> bool {
        match instruction {
            mir::Instruction::NewZeroed { destination, .. }
            | mir::Instruction::NewUninit { destination, .. }
            | mir::Instruction::NewSliceZeroed { destination, .. }
            | mir::Instruction::NewSliceUninit { destination, .. } => {
                self.set_root(*destination, *destination)
            }
            mir::Instruction::NewComplete {
                destination, value, ..
            }
            | mir::Instruction::Cast {
                destination,
                argument: value,
                ..
            }
            | mir::Instruction::FieldAddr {
                destination,
                aggregate: value,
                ..
            }
            | mir::Instruction::ElementAddr {
                destination,
                base: value,
                ..
            } => self.copy_root(*destination, *value),
            mir::Instruction::Select {
                destination,
                then_value,
                else_value,
                ..
            } => self.copy_matching_root(*destination, *then_value, *else_value),
            mir::Instruction::LocalGet { destination, local } => {
                if let Some(root) = self.locals.get(local.id as usize).copied().flatten() {
                    self.set_root(*destination, root)
                } else {
                    false
                }
            }
            mir::Instruction::LocalSet { local, value } => {
                let root = self.root(*value);
                self.set_local(*local, root)
            }
            _ => false,
        }
    }

    /// Propagate roots through successor arguments.
    fn propagate_successor(
        &mut self,
        terminator: &mir::Terminator,
        successor: mir::BlockId,
    ) -> bool {
        let arguments = terminator.successor_arguments(self.tree, successor);
        let parameters = &self.tree.get(successor).parameters;
        let mut changed = false;

        // copy each argument root to its matching block parameter
        for (parameter, argument) in parameters.iter().zip(arguments.iter()) {
            changed |= self.copy_root(parameter.value, *argument);
        }

        changed
    }

    /// Copy a root when the source has one.
    fn copy_root(&mut self, destination: mir::Value, source: mir::Value) -> bool {
        if let Some(root) = self.root(source) {
            self.set_root(destination, root)
        } else {
            false
        }
    }

    /// Copy a root when two sources share the same root.
    fn copy_matching_root(
        &mut self,
        destination: mir::Value,
        left: mir::Value,
        right: mir::Value,
    ) -> bool {
        let Some(left) = self.root(left) else {
            return false;
        };
        let Some(right) = self.root(right) else {
            return false;
        };

        if left == right {
            self.set_root(destination, left)
        } else {
            false
        }
    }

    /// Return the root for one value.
    fn root(&self, value: mir::Value) -> Option<mir::Value> {
        self.roots.get(value.0 as usize).copied().flatten()
    }

    /// Set a value root.
    fn set_root(&mut self, value: mir::Value, root: mir::Value) -> bool {
        let slot = &mut self.roots[value.0 as usize];
        if *slot == Some(root) {
            false
        } else {
            *slot = Some(root);
            true
        }
    }

    /// Set a local root.
    fn set_local(&mut self, local: mir::LocalId, root: Option<mir::Value>) -> bool {
        let slot = &mut self.locals[local.id as usize];
        if *slot == root {
            false
        } else {
            *slot = root;
            true
        }
    }
}

/// Escape marker for propagated allocation roots.
struct EscapeMarker<'a, 'b> {
    /// The MIR tree being analyzed.
    tree: &'a mir::Tree,
    /// Allocation root by SSA value id.
    roots: &'a [Option<mir::Value>],
    /// Allocation roots that escape.
    escaped: &'b mut BitSet,
}

impl<'a, 'b> EscapeMarker<'a, 'b> {
    /// Mark escaping roots in one function.
    fn mark_function(&mut self, function: &mir::Function) {
        for &block_id in function.blocks() {
            self.mark_block(block_id);
        }
    }

    /// Mark escaping roots in one block.
    fn mark_block(&mut self, block_id: mir::BlockId) {
        let block = self.tree.get(block_id);

        // mark instruction escapes
        for &instruction_id in &block.instructions {
            let instruction = self.tree.get(instruction_id);
            self.mark_instruction(instruction);
        }

        // mark terminator escapes
        let terminator = self.tree.get(block.terminator);
        self.mark_terminator(terminator);
    }

    /// Mark roots escaping through one instruction.
    fn mark_instruction(&mut self, instruction: &mir::Instruction) {
        match instruction {
            mir::Instruction::Store { value, .. } => self.mark_value(*value),
            mir::Instruction::ContinuationNew { arguments, .. } => {
                self.mark_values(self.tree.get_values(*arguments));
            }
            mir::Instruction::WaiterQueue { value, .. }
            | mir::Instruction::TaskResolve { value, .. } => self.mark_value(*value),
            mir::Instruction::Call { call, .. } => {
                self.mark_values(&call.callee.uses());
                self.mark_values(self.tree.get_values(call.arguments));
            }
            mir::Instruction::Intrinsic { arguments, .. } => {
                self.mark_values(self.tree.get_values(*arguments));
            }
            _ => {}
        }
    }

    /// Mark roots escaping through one terminator.
    fn mark_terminator(&mut self, terminator: &mir::Terminator) {
        match terminator {
            mir::Terminator::Return { value: Some(value) }
            | mir::Terminator::Await { value, .. }
            | mir::Terminator::Yield { value, .. } => self.mark_value(*value),
            mir::Terminator::ContinuationResume { value, .. }
            | mir::Terminator::ContinuationComplete { value, .. } => self.mark_value(*value),
            mir::Terminator::Invoke { call, .. } | mir::Terminator::TailCall { call } => {
                self.mark_values(&call.callee.uses());
                self.mark_values(self.tree.get_values(call.arguments));
            }
            _ => {}
        }
    }

    /// Mark all roots in a value slice as escaping.
    fn mark_values(&mut self, values: &[mir::Value]) {
        for &value in values {
            self.mark_value(value);
        }
    }

    /// Mark one value's root as escaping.
    fn mark_value(&mut self, value: mir::Value) {
        if let Some(root) = self.roots.get(value.0 as usize).copied().flatten() {
            self.escaped.insert(root.id() as usize);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::analyses::tests::TestProgram;

    /// Returned allocations escape their function.
    #[test]
    fn test_escape_marks_returned_allocation() {
        let program = TestProgram::new(
            r#"
function test(): ref<int32, unique, mutable> {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32
    return v0
}
"#,
        );

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analysis_cache();
        let escape = analyses.get::<EscapeAnalysis>(function, &program.tree);

        assert!(escape.escapes(mir::Value::new(0)));
    }

    /// Awaited allocations escape into the selected park implementation.
    #[test]
    fn test_escape_marks_awaited_allocation() {
        let program = TestProgram::new(
            r#"
external function park(ref<int32, unique, mutable>, waiter<int32>): void

async function test(): int32 {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32
    await park(v0) => resumed | cancelled | failed

resumed(v1: int32):
    return v1

cancelled:
    return

failed:
    unwind.resume
}
"#,
        );

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analysis_cache();
        let escape = analyses.get::<EscapeAnalysis>(function, &program.tree);

        assert!(escape.escapes(mir::Value::new(0)));
    }

    /// Task results escape into runtime task storage.
    #[test]
    fn test_escape_marks_task_result() {
        let program = TestProgram::new(
            r#"
type Task {
    uint64;
}

function test(): void {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32
    v1: Task = task.resolve v0
    task.detach v1
    return
}
"#,
        );

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analysis_cache();
        let escape = analyses.get::<EscapeAnalysis>(function, &program.tree);

        assert!(escape.escapes(mir::Value::new(0)));
    }

    /// Unpublished allocations remain local to their function.
    #[test]
    fn test_escape_keeps_unpublished_allocation_local() {
        let program = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32
    v1: int32 = 0
    return v1
}
"#,
        );

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analysis_cache();
        let escape = analyses.get::<EscapeAnalysis>(function, &program.tree);

        assert!(escape.stays_local(mir::Value::new(0)));
    }

    /// Local slots propagate allocation roots before escaping.
    #[test]
    fn test_escape_propagates_through_local_slots() {
        let program = TestProgram::new(
            r#"
function test(): ref<int32, unique, mutable> {
    local l0: ref<int32, unique, mutable>

entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32
    local.set l0, v0
    v1: ref<int32, unique, mutable> = local.get l0
    return v1
}
"#,
        );

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analysis_cache();
        let escape = analyses.get::<EscapeAnalysis>(function, &program.tree);

        assert_eq!(
            escape.allocation(mir::Value::new(1)),
            Some(mir::Value::new(0))
        );
        assert!(escape.escapes(mir::Value::new(1)));
    }

    /// Block arguments propagate allocation roots through control flow.
    #[test]
    fn test_escape_propagates_through_block_arguments() {
        let program = TestProgram::new(
            r#"
function test(v0: boolean): ref<int32, unique, mutable> {
entry(v0: boolean):
    v1: ref<int32, unique, mutable> = new.zeroed int32
    branch v0, b1(v1), b2(v1)

b1(v2: ref<int32, unique, mutable>):
    return v2

b2(v3: ref<int32, unique, mutable>):
    return v3
}
"#,
        );

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analysis_cache();
        let escape = analyses.get::<EscapeAnalysis>(function, &program.tree);

        assert_eq!(
            escape.allocation(mir::Value::new(2)),
            Some(mir::Value::new(1))
        );
        assert_eq!(
            escape.allocation(mir::Value::new(3)),
            Some(mir::Value::new(1))
        );
        assert!(escape.escapes(mir::Value::new(1)));
    }

    /// Call arguments escape across the function boundary.
    #[test]
    fn test_escape_marks_call_arguments() {
        let program = TestProgram::new(
            r#"
function sink(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(): int32 {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32
    call sink(v0): (ref<int32, unique, mutable>) => void
    v1: int32 = 0
    return v1
}
"#,
        );

        let function_id = program.function_id_by_name("test");
        let function = program.tree.get(function_id);
        let analyses = program.function_analysis_cache();
        let escape = analyses.get::<EscapeAnalysis>(function, &program.tree);

        assert!(escape.escapes(mir::Value::new(0)));
    }
}

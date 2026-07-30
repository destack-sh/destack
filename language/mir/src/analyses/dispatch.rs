use std::collections::HashMap;

use crate as mir;

use super::{
    Analysis, AnalysisId, ModuleAnalysis, Mutation, OpenCallSite, TreeAnalysisCache, ValueTypes,
};

/// Static dispatch target analysis for MIR callsites.
#[derive(Debug, Default)]
pub struct DispatchAnalysis {
    /// Resolved targets keyed by callsite.
    targets: HashMap<mir::CallSite, mir::FunctionId>,
    /// Calls whose target set is still open.
    open_callsites: Vec<OpenCallSite>,
}

impl DispatchAnalysis {
    /// Return the resolved target for one callsite.
    pub fn target(&self, callsite: mir::CallSite) -> Option<mir::FunctionId> {
        self.targets.get(&callsite).copied()
    }

    /// Iterate open callsites.
    pub fn open_callsites(&self) -> &[OpenCallSite] {
        &self.open_callsites
    }

    /// Build dispatch analysis for one MIR tree.
    fn build(tree: &mir::Tree, analyses: &TreeAnalysisCache) -> Self {
        let mut analysis = Self::default();

        // scan each function body
        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            if function.entry().is_some() {
                let value_types = analyses.get_function::<ValueTypes>(function_id, tree);
                let mut resolver = DispatchResolver {
                    tree,
                    dispatch: analyses.dispatch(),
                    function,
                    value_types: &value_types,
                    analysis: &mut analysis,
                };

                resolver.record_function(function_id);
            }
        }

        analysis
    }

    /// Record a resolved target.
    fn resolve(&mut self, callsite: mir::CallSite, target: mir::FunctionId) {
        self.targets.insert(callsite, target);
    }

    /// Record an open callsite.
    fn record_open_callsite(&mut self, callsite: OpenCallSite) {
        self.open_callsites.push(callsite);
    }
}

impl Analysis for DispatchAnalysis {
    const ID: AnalysisId = AnalysisId("dispatch");
    const INVALIDATED_BY: Mutation = Mutation::VALUE;
}

impl ModuleAnalysis for DispatchAnalysis {
    /// Compute static dispatch targets for the module.
    fn compute(tree: &mir::Tree, analyses: &TreeAnalysisCache) -> Self {
        Self::build(tree, analyses)
    }
}

/// Resolver for one function body.
struct DispatchResolver<'a, 'b> {
    /// The MIR tree being analyzed.
    tree: &'a mir::Tree,
    /// Dispatch table being analyzed.
    dispatch: &'a mir::DispatchTable,
    /// The function being scanned.
    function: &'a mir::Function,
    /// Value types for the function.
    value_types: &'b ValueTypes,
    /// Analysis being populated.
    analysis: &'b mut DispatchAnalysis,
}

impl<'a, 'b> DispatchResolver<'a, 'b> {
    /// Record dispatch targets in one function.
    fn record_function(&mut self, caller: mir::FunctionId) {
        for &block_id in self.function.blocks() {
            self.record_block(caller, block_id);
        }
    }

    /// Record dispatch targets in one block.
    fn record_block(&mut self, caller: mir::FunctionId, block_id: mir::BlockId) {
        let block = self.tree.get(block_id);

        // record call instructions
        for &instruction_id in &block.instructions {
            let instruction = self.tree.get(instruction_id);
            self.record_instruction(caller, instruction_id, instruction);
        }

        // record the block terminator
        let terminator = self.tree.get(block.terminator);
        self.record_terminator(caller, block_id, terminator);
    }

    /// Record one call instruction.
    fn record_instruction(
        &mut self,
        caller: mir::FunctionId,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) {
        let callsite = mir::CallSite::Instruction(instruction_id);
        let mir::Instruction::Call { call, .. } = instruction else {
            return;
        };

        self.record_call(caller, callsite, call);
    }

    /// Record one terminator call.
    fn record_terminator(
        &mut self,
        caller: mir::FunctionId,
        block_id: mir::BlockId,
        terminator: &mir::Terminator,
    ) {
        let callsite = mir::CallSite::Terminator(block_id);
        let call = match terminator {
            mir::Terminator::Invoke { call, .. } | mir::Terminator::TailCall { call } => call,
            _ => return,
        };

        self.record_call(caller, callsite, call);
    }

    /// Record one call operation.
    fn record_call(&mut self, caller: mir::FunctionId, callsite: mir::CallSite, call: &mir::Call) {
        match &call.callee {
            mir::Callee::Direct { function } => self.analysis.resolve(callsite, *function),
            mir::Callee::Indirect { .. } => {
                self.record_open_callsite(caller, callsite, mir::CallDispatch::Indirect);
            }
            mir::Callee::Virtual {
                receiver,
                class,
                slot,
            } => {
                let target = self.virtual_target(*receiver, *class, *slot);
                self.record_target_or_open(
                    caller,
                    callsite,
                    mir::CallDispatch::Virtual { slot: *slot },
                    target,
                );
            }
            mir::Callee::Dynamic {
                receiver,
                constraint,
                slot,
            } => {
                let target = self.dynamic_target(*receiver, *constraint, *slot);
                self.record_target_or_open(
                    caller,
                    callsite,
                    mir::CallDispatch::Dynamic { slot: *slot },
                    target,
                );
            }
        }
    }

    /// Record a resolved target or open callsite.
    fn record_target_or_open(
        &mut self,
        caller: mir::FunctionId,
        callsite: mir::CallSite,
        dispatch: mir::CallDispatch,
        target: Option<mir::FunctionId>,
    ) {
        if let Some(target) = target {
            self.analysis.resolve(callsite, target);
        } else {
            self.record_open_callsite(caller, callsite, dispatch);
        }
    }

    /// Record an open callsite.
    fn record_open_callsite(
        &mut self,
        caller: mir::FunctionId,
        callsite: mir::CallSite,
        dispatch: mir::CallDispatch,
    ) {
        self.analysis.record_open_callsite(OpenCallSite {
            caller,
            callsite,
            dispatch,
            known_target: None,
        });
    }

    /// Resolve a virtual dispatch target.
    fn virtual_target(
        &self,
        receiver: mir::Value,
        class: mir::TypeId,
        slot: mir::DispatchSlot,
    ) -> Option<mir::FunctionId> {
        let receiver_type = self.receiver_type(receiver)?;
        if receiver_type != class {
            return None;
        }

        self.dispatch
            .virtual_table(receiver_type)?
            .methods
            .get(slot.index())
            .copied()
    }

    /// Resolve a dynamic dispatch target.
    fn dynamic_target(
        &self,
        receiver: mir::Value,
        constraint: mir::TypeId,
        slot: mir::DispatchSlot,
    ) -> Option<mir::FunctionId> {
        let concrete = self.receiver_type(receiver)?;
        let entry = self
            .dispatch
            .dynamic_table(concrete, constraint)?
            .entries
            .get(slot.index())?;

        match entry {
            mir::DynamicEntry::Function { function } => Some(*function),
            mir::DynamicEntry::Field { .. } | mir::DynamicEntry::Absent => None,
        }
    }

    /// Resolve the concrete receiver type when statically known.
    fn receiver_type(&self, receiver: mir::Value) -> Option<mir::TypeId> {
        let receiver_type = self.value_types.value_type(receiver)?;
        match self.tree.get(receiver_type) {
            mir::Type::Reference { pointee, .. } => Some(*pointee),
            mir::Type::Dynamic { .. } => None,
            _ => Some(receiver_type),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate as mir;

    use crate::DispatchAnalysis;
    use crate::analyses::tests::TestProgram;

    /// Virtual calls resolve when receiver type and virtual table are closed.
    #[test]
    fn test_dispatch_resolves_virtual_call() {
        let mut program = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.virtual v0, int32, 0(v0): (int32) => int32
    return v1
}
"#,
        );

        let callee = program.function_id_by_name("callee");
        let test = program.function_id_by_name("test");
        let (callsite, class) = first_virtual_call(&program, test);

        // attach the exact virtual table needed by the callsite
        program.dispatch.insert_virtual_table(mir::VirtualTable {
            ty: class,
            methods: vec![callee],
        });

        let analyses = program.tree_analysis_cache();
        let dispatch = analyses.get::<DispatchAnalysis>(&program.tree);

        assert_eq!(dispatch.target(callsite), Some(callee));
        assert!(dispatch.open_callsites().is_empty());
    }

    /// Virtual calls do not resolve when the slot is outside the method table.
    #[test]
    fn test_dispatch_keeps_missing_virtual_slot_open() {
        let mut program = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.virtual v0, int32, 1(v0): (int32) => int32
    return v1
}
"#,
        );

        let callee = program.function_id_by_name("callee");
        let test = program.function_id_by_name("test");
        let (callsite, class) = first_virtual_call(&program, test);

        // attach one method while the call targets another slot
        program.dispatch.insert_virtual_table(mir::VirtualTable {
            ty: class,
            methods: vec![callee],
        });

        let analyses = program.tree_analysis_cache();
        let dispatch = analyses.get::<DispatchAnalysis>(&program.tree);

        assert_eq!(dispatch.target(callsite), None);
        assert_eq!(dispatch.open_callsites().len(), 1);
    }

    /// Dynamic calls resolve when receiver and constraint tables are closed.
    #[test]
    fn test_dispatch_resolves_dynamic_call() {
        let mut program = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.dynamic v0, int32, 0(v0): (int32) => int32
    return v1
}
"#,
        );

        let callee = program.function_id_by_name("callee");
        let test = program.function_id_by_name("test");
        let (callsite, concrete, constraint) = first_dynamic_call(&program, test);

        // attach the concrete dynamic table selected by receiver and constraint
        program.dispatch.insert_dynamic_table(mir::DynamicTable {
            concrete,
            constraint,
            entries: vec![mir::DynamicEntry::Function { function: callee }],
            names: Vec::new(),
        });

        let analyses = program.tree_analysis_cache();
        let dispatch = analyses.get::<DispatchAnalysis>(&program.tree);

        assert_eq!(dispatch.target(callsite), Some(callee));
        assert!(dispatch.open_callsites().is_empty());
    }

    /// Dynamic calls do not resolve against field-offset slots.
    #[test]
    fn test_dispatch_keeps_dynamic_field_slot_open() {
        let mut program = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.dynamic v0, int32, 0(v0): (int32) => int32
    return v1
}
"#,
        );

        let test = program.function_id_by_name("test");
        let (callsite, concrete, constraint) = first_dynamic_call(&program, test);

        // attach the dynamic table selected by receiver and constraint
        program.dispatch.insert_dynamic_table(mir::DynamicTable {
            concrete,
            constraint,
            entries: vec![mir::DynamicEntry::Field { offset: 0 }],
            names: Vec::new(),
        });

        let analyses = program.tree_analysis_cache();
        let dispatch = analyses.get::<DispatchAnalysis>(&program.tree);

        assert_eq!(dispatch.target(callsite), None);
        assert_eq!(dispatch.open_callsites().len(), 1);
    }

    /// Open calls stay visible when no dispatch table proves a target.
    #[test]
    fn test_dispatch_records_open_virtual_call() {
        let program = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.virtual v0, int32, 0(v0): (int32) => int32
    return v1
}
"#,
        );

        let analyses = program.tree_analysis_cache();
        let dispatch = analyses.get::<DispatchAnalysis>(&program.tree);

        assert!(
            dispatch
                .target(first_virtual_call(&program, program.entry_function_id()).0)
                .is_none()
        );
        assert_eq!(dispatch.open_callsites().len(), 1);
    }

    /// Return the first virtual callsite and class type.
    fn first_virtual_call(
        program: &TestProgram,
        function: mir::FunctionId,
    ) -> (mir::CallSite, mir::TypeId) {
        let function = program.tree.get(function);

        // scan the function body
        for &block_id in function.blocks() {
            let block = program.tree.get(block_id);
            for &instruction_id in &block.instructions {
                if let mir::Instruction::Call {
                    call:
                        mir::Call {
                            callee: mir::Callee::Virtual { class, .. },
                            ..
                        },
                    ..
                } = program.tree.get(instruction_id)
                {
                    return (mir::CallSite::Instruction(instruction_id), *class);
                }
            }
        }

        panic!("missing virtual call");
    }

    /// Return the first dynamic callsite, concrete receiver type, and constraint.
    fn first_dynamic_call(
        program: &TestProgram,
        function: mir::FunctionId,
    ) -> (mir::CallSite, mir::TypeId, mir::TypeId) {
        let function = program.tree.get(function);

        // scan the function body
        for &block_id in function.blocks() {
            let block = program.tree.get(block_id);
            for &instruction_id in &block.instructions {
                if let mir::Instruction::Call {
                    call:
                        mir::Call {
                            callee:
                                mir::Callee::Dynamic {
                                    receiver,
                                    constraint,
                                    ..
                                },
                            ..
                        },
                    ..
                } = program.tree.get(instruction_id)
                {
                    let concrete = function
                        .parameters
                        .iter()
                        .find(|parameter| parameter.value == *receiver)
                        .map(|parameter| parameter.ty)
                        .expect("missing receiver parameter type");

                    return (
                        mir::CallSite::Instruction(instruction_id),
                        concrete,
                        *constraint,
                    );
                }
            }
        }

        panic!("missing dynamic call");
    }
}

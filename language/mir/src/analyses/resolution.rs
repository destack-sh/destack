use crate as mir;

use super::{Analysis, AnalysisCache, Mutation};

/// Static dispatch target analysis for MIR callsites.
#[derive(Debug, Default)]
pub struct ResolutionTable {
    /// Resolved targets sorted by callsite.
    targets: Vec<(mir::Point, mir::FunctionId)>,
}

impl ResolutionTable {
    /// Return the resolved target for one callsite.
    pub fn target(&self, callsite: mir::Point) -> Option<mir::FunctionId> {
        let index = self
            .targets
            .binary_search_by_key(&callsite, |(callsite, _)| *callsite)
            .ok()?;

        Some(self.targets[index].1)
    }

    /// Build static callsite resolutions for one MIR tree.
    fn build(tree: &mir::Tree, dispatch_table: &mir::DispatchTable) -> Self {
        let mut analysis = Self::default();

        // scan each function body
        for (_, function) in tree.iter_nodes::<mir::Function>() {
            if function.entry().is_some() {
                let mut resolver = DispatchResolver {
                    tree,
                    dispatch: dispatch_table,
                    function,
                    analysis: &mut analysis,
                };

                resolver.record_function();
            }
        }

        analysis
            .targets
            .sort_unstable_by_key(|(callsite, _)| *callsite);

        // reject duplicate callsite ownership
        if analysis
            .targets
            .windows(2)
            .any(|targets| targets[0].0 == targets[1].0)
        {
            unreachable!("callsite resolves to multiple targets");
        }

        analysis
    }

    /// Record a resolved target.
    fn resolve(&mut self, callsite: mir::Point, target: mir::FunctionId) {
        self.targets.push((callsite, target));
    }
}

impl Analysis for ResolutionTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL.union(Mutation::VALUE);
}

impl ResolutionTable {
    /// Compute static dispatch targets for the module.
    pub(crate) fn compute(
        tree: &mir::Tree,
        _analyses: &mut AnalysisCache,
        dispatch_table: &mir::DispatchTable,
    ) -> Self {
        Self::build(tree, dispatch_table)
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
    /// Analysis being populated.
    analysis: &'b mut ResolutionTable,
}

impl<'a, 'b> DispatchResolver<'a, 'b> {
    /// Record dispatch targets in one function.
    fn record_function(&mut self) {
        for &block_id in self.function.blocks() {
            self.record_block(block_id);
        }
    }

    /// Record dispatch targets in one block.
    fn record_block(&mut self, block_id: mir::BlockId) {
        let block = self.tree.get(block_id);

        // record call instructions
        for &instruction_id in &block.instructions {
            let instruction = self.tree.get(instruction_id);
            self.record_instruction(instruction_id, instruction);
        }

        // record the block terminator
        let terminator = self.tree.get(block.terminator);
        self.record_terminator(block_id, terminator);
    }

    /// Record one call instruction.
    fn record_instruction(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) {
        let callsite = mir::Point::Instruction(instruction_id);
        let mir::Instruction::Call { call, .. } = instruction else {
            return;
        };

        self.record_call(callsite, call);
    }

    /// Record one terminator call.
    fn record_terminator(&mut self, block_id: mir::BlockId, terminator: &mir::Terminator) {
        let callsite = mir::Point::Terminator(block_id);
        let call = match terminator {
            mir::Terminator::Invoke { call, .. } | mir::Terminator::TailCall { call } => call,
            _ => return,
        };

        self.record_call(callsite, call);
    }

    /// Record one call operation.
    fn record_call(&mut self, callsite: mir::Point, call: &mir::Call) {
        match &call.callee {
            mir::Callee::Direct { .. }
            | mir::Callee::Indirect { .. }
            | mir::Callee::Witness { .. } => {}
            mir::Callee::Virtual {
                receiver,
                class,
                slot,
                ..
            } => {
                let target = self.virtual_target(*receiver, *class, *slot);
                self.record_target(callsite, target);
            }
            mir::Callee::Dynamic {
                receiver,
                constraint,
                slot,
                ..
            } => {
                let target = self.dynamic_target(*receiver, *constraint, *slot);
                self.record_target(callsite, target);
            }
        }
    }

    /// Record a resolved target when present.
    fn record_target(&mut self, callsite: mir::Point, target: Option<mir::FunctionId>) {
        if let Some(target) = target {
            self.analysis.resolve(callsite, target);
        }
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
        let receiver_type = self.function.value_type(receiver)?;
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

    use crate::analyses::tests::TestProgram;

    /// Virtual calls resolve when receiver type and virtual table are closed.
    #[test]
    fn test_resolve_virtual_call() {
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
            concrete: class,
            methods: vec![callee],
        });

        let mut analyses = program.module_analyses();
        let resolution = analyses.resolution(&program.tree, &program.dispatch);

        assert_eq!(resolution.target(callsite), Some(callee));
    }

    /// Virtual calls do not resolve when the slot is outside the method table.
    #[test]
    fn test_leave_missing_virtual_slot_open() {
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
            concrete: class,
            methods: vec![callee],
        });

        let mut analyses = program.module_analyses();
        let resolution = analyses.resolution(&program.tree, &program.dispatch);

        assert_eq!(resolution.target(callsite), None);
    }

    /// Dynamic calls resolve when receiver and constraint tables are closed.
    #[test]
    fn test_resolve_dynamic_call() {
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

        let mut analyses = program.module_analyses();
        let resolution = analyses.resolution(&program.tree, &program.dispatch);

        assert_eq!(resolution.target(callsite), Some(callee));
    }

    /// Dynamic calls do not resolve against field-offset slots.
    #[test]
    fn test_leave_dynamic_field_slot_open() {
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

        let mut analyses = program.module_analyses();
        let resolution = analyses.resolution(&program.tree, &program.dispatch);

        assert_eq!(resolution.target(callsite), None);
    }

    /// Virtual calls do not resolve without a dispatch table.
    #[test]
    fn test_leave_missing_virtual_table_open() {
        let program = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.virtual v0, int32, 0(v0): (int32) => int32
    return v1
}
"#,
        );

        let mut analyses = program.module_analyses();
        let resolution = analyses.resolution(&program.tree, &program.dispatch);

        assert!(
            resolution
                .target(first_virtual_call(&program, program.entry_function_id()).0)
                .is_none()
        );
    }

    /// Return the first virtual callsite and class type.
    fn first_virtual_call(
        program: &TestProgram,
        function: mir::FunctionId,
    ) -> (mir::Point, mir::TypeId) {
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
                    return (mir::Point::Instruction(instruction_id), *class);
                }
            }
        }

        panic!("missing virtual call");
    }

    /// Return the first dynamic callsite, concrete receiver type, and constraint.
    fn first_dynamic_call(
        program: &TestProgram,
        function: mir::FunctionId,
    ) -> (mir::Point, mir::TypeId, mir::TypeId) {
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
                        mir::Point::Instruction(instruction_id),
                        concrete,
                        *constraint,
                    );
                }
            }
        }

        panic!("missing dynamic call");
    }
}

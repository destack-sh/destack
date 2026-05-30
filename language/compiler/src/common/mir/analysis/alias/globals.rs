use std::collections::{HashMap, HashSet};

use destack_mir as mir;

use crate::common::mir::MemoryLocation;

use super::common::FunctionAA;
use super::result::{AliasResult, ModRefInfo};

/// Global variable alias analysis.
///
/// Tracks which functions access which global variables, enabling precise
/// mod/ref analysis for calls. If a function doesn't access a global,
/// calls to it cannot affect locations derived from that global.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct GlobalsAA {
    /// Globals that are read by the function.
    reads: HashSet<mir::LocalNodeId<mir::Global>>,
    /// Globals that are written by the function.
    writes: HashSet<mir::LocalNodeId<mir::Global>>,
    /// Globals whose address is taken (stored somewhere, passed to calls).
    address_taken: HashSet<mir::LocalNodeId<mir::Global>>,
    /// Common function information (constants, definitions, parameters).
    function: FunctionAA,
}

impl GlobalsAA {
    /// Build GlobalsAA for a function.
    pub(super) fn build(function: &mir::Function, tree: &mir::Tree) -> Self {
        let info = FunctionAA::collect(function, tree);

        let mut reads = HashSet::new();
        let mut writes = HashSet::new();
        let mut address_taken = HashSet::new();

        // imports have no body
        if function.entry.is_none() {
            return Self {
                reads,
                writes,
                address_taken,
                function: info,
            };
        }

        // analyze global accesses
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for &instruction_id in &block.instructions {
                let inst = tree.get(instruction_id);

                match inst {
                    // direct global access
                    mir::Instruction::GlobalAddr { global, .. } => {
                        let Some(global) = global.global() else {
                            continue;
                        };

                        // conservatively mark address as taken
                        // (a more precise analysis could track uses and only mark escaped ones)
                        address_taken.insert(global);
                    }

                    // loads through global addresses
                    mir::Instruction::Load { pointer, .. } => {
                        if let Some(global) = pointer.value().and_then(|pointer| {
                            Self::get_global_base(pointer, &info.definitions, tree)
                        }) {
                            reads.insert(global);
                        }
                    }

                    // stores through global addresses
                    mir::Instruction::Store { pointer, value } => {
                        if let Some(global) = pointer.value().and_then(|pointer| {
                            Self::get_global_base(pointer, &info.definitions, tree)
                        }) {
                            writes.insert(global);
                        }

                        // storing a global address somewhere makes it escape
                        if let Some(global) = value
                            .value()
                            .and_then(|value| Self::get_global_base(value, &info.definitions, tree))
                        {
                            address_taken.insert(global);
                        }
                    }

                    // calls may access any address-taken global
                    mir::Instruction::Call { call, .. }
                    | mir::Instruction::CallVirtual { call, .. }
                    | mir::Instruction::CallDynamic { call, .. } => {
                        let args = tree.get_arguments(call.arguments);
                        for arg in args.iter().copied().filter_map(|value| value.value()) {
                            if let Some(global) =
                                Self::get_global_base(arg, &info.definitions, tree)
                            {
                                address_taken.insert(global);
                            }
                        }
                    }
                    mir::Instruction::CallIndirect { call, .. } => {
                        let args = tree.get_arguments(call.arguments);
                        for arg in args.iter().copied().filter_map(|value| value.value()) {
                            if let Some(global) =
                                Self::get_global_base(arg, &info.definitions, tree)
                            {
                                address_taken.insert(global);
                            }
                        }
                    }

                    _ => {}
                }
            }
        }

        Self {
            reads,
            writes,
            address_taken,
            function: info,
        }
    }

    /// Get the global base of a pointer if it's derived from a global.
    fn get_global_base(
        ptr: mir::Value,
        definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
        tree: &mir::Tree,
    ) -> Option<mir::LocalNodeId<mir::Global>> {
        let mut current = ptr;
        let mut visited = HashSet::new();

        loop {
            if !visited.insert(current) {
                return None; // cycle
            }

            let &instruction_id = definitions.get(&current)?;
            let inst = tree.get(instruction_id);
            match inst {
                mir::Instruction::GlobalAddr { global, .. } => return global.global(),

                // follow through address computations
                mir::Instruction::FieldAddr { aggregate, .. }
                | mir::Instruction::ElementAddr {
                    array: aggregate, ..
                } => {
                    current = aggregate.value()?;
                }

                // follow casts
                mir::Instruction::Cast { argument, .. } => {
                    current = argument.value()?;
                }

                _ => return None,
            }
        }
    }

    /// Query if two memory locations may alias.
    ///
    /// GlobalsAA can prove NoAlias when:
    /// - Both are from different globals
    /// - One is from a global the function doesn't access
    pub(super) fn alias(
        &self,
        loc_a: &MemoryLocation,
        loc_b: &MemoryLocation,
        tree: &mir::Tree,
    ) -> AliasResult {
        let global_a = Self::get_global_base(loc_a.ptr, &self.function.definitions, tree);
        let global_b = Self::get_global_base(loc_b.ptr, &self.function.definitions, tree);

        match (global_a, global_b) {
            // different globals don't alias
            (Some(a), Some(b)) if a != b => AliasResult::NoAlias,

            // same global may alias
            (Some(_), Some(_)) => AliasResult::MayAlias,

            // one global, one non-global
            (Some(g), None) | (None, Some(g)) => {
                // if the global's address isn't taken, non-global pointers can't alias it
                if !self.address_taken.contains(&g) {
                    AliasResult::NoAlias
                } else {
                    AliasResult::MayAlias
                }
            }

            // neither is a global
            (None, None) => AliasResult::MayAlias,
        }
    }

    /// Get mod/ref info for a call relative to a memory location.
    ///
    /// If the location is a global that the callee doesn't access,
    /// the call cannot affect it.
    pub(super) fn get_call_mod_ref(
        &self,
        _call: &mir::Instruction,
        loc: &MemoryLocation,
        tree: &mir::Tree,
    ) -> ModRefInfo {
        let Some(global) = Self::get_global_base(loc.ptr, &self.function.definitions, tree) else {
            // not a global location, we can't help
            return ModRefInfo::MOD_REF;
        };

        // if global's address is taken, calls may do anything
        if self.address_taken.contains(&global) {
            return ModRefInfo::MOD_REF;
        }

        // check if this function reads/writes this global
        let reads = self.reads.contains(&global);
        let writes = self.writes.contains(&global);

        ModRefInfo::from_flags(reads, writes)
    }

    /// Check if the function reads a specific global.
    #[allow(dead_code)]
    pub(super) fn reads_global(&self, global: mir::LocalNodeId<mir::Global>) -> bool {
        self.reads.contains(&global)
    }

    /// Check if the function writes a specific global.
    #[allow(dead_code)]
    pub(super) fn writes_global(&self, global: mir::LocalNodeId<mir::Global>) -> bool {
        self.writes.contains(&global)
    }

    /// Check if a global's address is taken.
    #[allow(dead_code)]
    pub(super) fn is_address_taken(&self, global: mir::LocalNodeId<mir::Global>) -> bool {
        self.address_taken.contains(&global)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_different_globals_no_alias() {
        let program = TestProgram::new(
            r#"
global g1: int32 = 0int32
global g2: int32 = 0int32
function test(): void {
b0:
    v0: ref<int32, raw, space(static)> = global.address g1
    v1: ref<int32, raw, space(static)> = global.address g2
    v2: int32 = 1int32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = GlobalsAA::build(function, &program.tree);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_same_global_may_alias() {
        let program = TestProgram::new(
            r#"
global g: int32 = 0int32
function test(): void {
b0:
    v0: ref<int32, raw, space(static)> = global.address g
    v1: ref<int32, raw, space(static)> = global.address g
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = GlobalsAA::build(function, &program.tree);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::MayAlias);
    }

    #[test]
    fn test_global_read_tracking() {
        let program = TestProgram::new(
            r#"
global g: int32 = 0int32
function test(): int32 {
b0:
    v0: ref<int32, raw, space(static)> = global.address g
    v1: int32 = load v0
    return v1
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = GlobalsAA::build(function, &program.tree);

        let global_id = program.tree.iter_nodes::<mir::Global>().next().unwrap().0;

        assert!(aa.reads_global(global_id));
        assert!(!aa.writes_global(global_id));
    }

    #[test]
    fn test_global_write_tracking() {
        let program = TestProgram::new(
            r#"
global g: int32 = 0int32
function test(): void {
b0:
    v0: ref<int32, raw, space(static)> = global.address g
    v1: int32 = 42int32
    store v0, v1
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = GlobalsAA::build(function, &program.tree);

        let global_id = program.tree.iter_nodes::<mir::Global>().next().unwrap().0;

        assert!(aa.writes_global(global_id));
    }

    #[test]
    fn test_global_address_taken_via_call() {
        // global address passed to call makes it escape
        let program = TestProgram::new(
            r#"
global g: int32 = 0int32
external function external(ref<int32, raw>): void
function test(): void {
b0:
    v0: ref<int32, raw, space(static)> = global.address g
    call external(v0): (ref<int32, raw>) -> void
    return
}"#,
        );

        let function_id = program
            .tree
            .iter_nodes::<mir::Function>()
            .find(|(_, f)| f.entry.is_some())
            .unwrap()
            .0;
        let function = program.tree.get(function_id);
        let aa = GlobalsAA::build(function, &program.tree);

        let global_id = program.tree.iter_nodes::<mir::Global>().next().unwrap().0;

        // address was passed to call, so it's taken
        assert!(aa.is_address_taken(global_id));
    }

    #[test]
    fn test_global_address_taken_via_store() {
        // global address stored to memory makes it escape
        let program = TestProgram::new(
            r#"
global g: int32 = 0int32
function test(v0: ref<ref<int32, raw>, raw>): void {
b0(v0: ref<ref<int32, raw>, raw>):
    v1: ref<int32, raw, space(static)> = global.address g
    store v0, v1
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = GlobalsAA::build(function, &program.tree);

        let global_id = program.tree.iter_nodes::<mir::Global>().next().unwrap().0;

        // address was stored, so it's taken
        assert!(aa.is_address_taken(global_id));
    }

    #[test]
    fn test_global_address_taken_may_alias_non_global() {
        // when global's address is taken, non-global pointer may alias it
        let program = TestProgram::new(
            r#"
global g: int32 = 0int32
function test(v0: ref<int32, raw>): void {
b0(v0: ref<int32, raw>):
    v1: ref<int32, raw, space(static)> = global.address g
    v2: int32 = 42int32
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = GlobalsAA::build(function, &program.tree);

        let loc_param = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc_global = MemoryLocation::from_ptr(mir::Value::new(1));

        // global address is taken (via global.address), so param may alias it
        assert_eq!(
            aa.alias(&loc_param, &loc_global, &program.tree),
            AliasResult::MayAlias
        );
    }

    #[test]
    fn test_global_field_access() {
        // accessing field of global should still track it
        let program = TestProgram::new(
            r#"
type Point { int32, int32 }
global g: Point = { 0int32, 0int32 }
function test(): void {
b0:
    v0: ref<Point, raw, space(static)> = global.address g
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: int32 = 42int32
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = GlobalsAA::build(function, &program.tree);

        let global_id = program.tree.iter_nodes::<mir::Global>().next().unwrap().0;

        // writing through field.address of global.address counts as write
        assert!(aa.writes_global(global_id));
    }
}

use std::collections::HashSet;

use crate::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    AliasAnalysis, AliasResult, ConstantPropagation, MemoryAccess, MemoryAccessEffect,
    MemoryAccessId, MemoryAccessLocation, MemoryDef, MemorySSA,
};
use crate::optimize::common::{
    ValueEquivalence, address_spaces_may_alias, alias_scopes_may_alias,
    build_instruction_block_map, build_value_definition_map, effect_is_trackable,
    effects_match_location, instruction_has_atomic_ordering, space_sets_may_alias,
    type_alias_tags_may_alias,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Remove redundant memory stores.
    ///
    /// Eliminates stores that write the same value as the last clobbering definition of the same location.
    /// This is distinct from dead store elimination:
    ///  the store can be removed even if the value is later read,
    ///  because the memory contents are unchanged.
    ///
    /// ```mir
    /// function before(): int32 {
    /// b0:
    ///     v0 = stack.alloc int32
    ///     v1 = 7int32
    ///     store v0, v1
    ///     store v0, v1
    ///     v2 = load v0
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(): int32 {
    /// b0:
    ///     v0 = stack.alloc int32
    ///     v1 = 7int32
    ///     store v0, v1
    ///     v2 = load v0
    ///     return v2
    /// }
    /// ```
    #[pass(id = "mem-cse", requires(call_effects, memory_access_metadata))]
    pub MemCse,
    "Eliminate redundant memory stores"
}

impl FunctionPass for MemCse {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip empty functions
        let _entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get analyses
        let (alias, memory_ssa, constants) = {
            let analyses = ctx.function_analyses(function, tree);
            (
                analyses.get::<AliasAnalysis>().clone(),
                analyses.get::<MemorySSA>(),
                analyses.get::<ConstantPropagation>(),
            )
        };

        // run memory cse
        let changed = run_mem_cse(
            function,
            tree,
            memory_ssa.as_ref(),
            &alias,
            constants.as_ref(),
        );

        // report analysis preservation
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "MemCse"
    }

    fn id(&self) -> &'static str {
        "mem-cse"
    }
}

/// Classified store like definitions for memory cse.
#[derive(Clone)]
enum DefKind {
    /// Store of a value to memory.
    Store {
        /// The stored value.
        value: mir::Value,
    },
    /// Store of a value to a local slot.
    LocalSet {
        /// The stored value.
        value: mir::Value,
    },
    /// Memset of a value and size.
    Memset {
        /// The byte value stored by the memset.
        value: mir::Value,
        /// The size of the memset in bytes.
        size: mir::Value,
    },
    /// Memcpy of a source pointer and size.
    Memcpy {
        /// The source pointer value.
        source: mir::Value,
        /// The size of the copy in bytes.
        size: mir::Value,
    },
    /// Memmove of a source pointer and size.
    Memmove {
        /// The source pointer value.
        source: mir::Value,
        /// The size of the move in bytes.
        size: mir::Value,
    },
}

/// Candidate memory definition for redundancy elimination.
#[derive(Clone)]
struct DefCandidate {
    /// The instruction that writes memory.
    instruction: mir::LocalNodeId<mir::Instruction>,
    /// The memory ssa access for the write.
    access: MemoryAccessId,
    /// The memory effect for the write.
    effect: MemoryAccessEffect,
    /// The classified def kind for value comparison.
    kind: DefKind,
}

/// Source access for memory copy operations.
#[derive(Clone)]
struct SourceAccess {
    /// The memory SSA access for the read.
    access: MemoryAccessId,
    /// The memory effect for the read.
    effect: MemoryAccessEffect,
}

/// Run memory common subexpression elimination.
fn run_mem_cse(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
    constants: &ConstantPropagation,
) -> bool {
    // collect candidate definitions
    let candidates = collect_candidates(function, tree, memory_ssa);

    // exit early when there is nothing to do
    if candidates.is_empty() {
        return false;
    }

    // build definition map for value equivalence checks
    let definitions = build_value_definition_map(function, tree);

    // build instruction block map for constant lookups
    let instruction_blocks = build_instruction_block_map(function, tree);

    // prepare value equivalence
    let mut equivalence =
        ValueEquivalence::new_with_constants(tree, &definitions, constants, &instruction_blocks);

    // collect redundant stores
    let mut redundant = HashSet::new();

    // evaluate candidates for redundancy
    for candidate in candidates {
        if candidate_is_redundant(&candidate, memory_ssa, alias, tree, &mut equivalence) {
            redundant.insert(candidate.instruction);
        }
    }

    // stop when no instructions are removed
    if redundant.is_empty() {
        return false;
    }

    // remove redundant instructions
    for &block_id in &function.blocks {
        let block = tree.get_mut(block_id);
        block.instructions.retain(|id| !redundant.contains(id));
    }

    true
}

/// Collect candidate memory definitions.
fn collect_candidates(
    function: &mir::Function,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
) -> Vec<DefCandidate> {
    let mut candidates = Vec::new();

    // scan blocks for store like instructions
    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // scan instructions for candidates
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            let Some(kind) = def_kind_for_instruction(tree, instruction) else {
                continue;
            };

            let Some(access_id) = def_access_for_instruction(memory_ssa, instruction_id) else {
                continue;
            };

            let MemoryAccess::Def(def_access) = memory_ssa.access(access_id) else {
                continue;
            };

            // require trackable effects
            if !effect_is_trackable(&def_access.effect) {
                continue;
            }

            candidates.push(DefCandidate {
                instruction: instruction_id,
                access: access_id,
                effect: def_access.effect.clone(),
                kind,
            });
        }
    }

    candidates
}

/// Classify store like instructions for redundancy checks.
fn def_kind_for_instruction(tree: &mir::Tree, instruction: &mir::Instruction) -> Option<DefKind> {
    match instruction {
        mir::Instruction::Store { value, .. } => Some(DefKind::Store {
            value: value.value()?,
        }),
        mir::Instruction::LocalSet { value, .. } => Some(DefKind::LocalSet {
            value: value.value()?,
        }),
        mir::Instruction::Intrinsic {
            intrinsic: mir::Intrinsic::Memset,
            arguments,
            ..
        } => {
            let args = tree.get_arguments(*arguments);
            let value = args.get(1)?.value()?;
            let size = args.get(2)?.value()?;
            Some(DefKind::Memset { value, size })
        }
        mir::Instruction::Intrinsic {
            intrinsic: mir::Intrinsic::Memcpy,
            arguments,
            ..
        } => {
            let args = tree.get_arguments(*arguments);
            let source = args.get(1)?.value()?;
            let size = args.get(2)?.value()?;
            Some(DefKind::Memcpy { source, size })
        }
        mir::Instruction::Intrinsic {
            intrinsic: mir::Intrinsic::Memmove,
            arguments,
            ..
        } => {
            let args = tree.get_arguments(*arguments);
            let source = args.get(1)?.value()?;
            let size = args.get(2)?.value()?;
            Some(DefKind::Memmove { source, size })
        }
        _ => None,
    }
}

/// Find the single memory def access for an instruction.
fn def_access_for_instruction(
    memory_ssa: &MemorySSA,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
) -> Option<MemoryAccessId> {
    // locate the single memory def access for this instruction
    let accesses = memory_ssa.accesses_for_instruction(instruction_id)?;
    let mut def_access = None;

    for access_id in accesses {
        if matches!(memory_ssa.access(*access_id), MemoryAccess::Def(_)) {
            if def_access.is_some() {
                return None;
            }
            def_access = Some(*access_id);
        }
    }

    def_access
}

/// Determine whether the candidate definition is redundant.
fn candidate_is_redundant(
    candidate: &DefCandidate,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
    tree: &mir::Tree,
    equivalence: &mut ValueEquivalence<'_>,
) -> bool {
    // skip untrackable candidates
    if !effect_is_trackable(&candidate.effect) {
        return false;
    }

    // skip atomically ordered candidates
    if instruction_has_atomic_ordering(tree, candidate.instruction) {
        return false;
    }

    // resolve the clobbering access for this definition
    let clobber = memory_ssa.clobbering_access_for_def(candidate.access, alias, tree);
    let Some(clobber_defs) = clobber_def_accesses(memory_ssa, clobber) else {
        return false;
    };

    // check redundancy against every incoming clobber
    for clobber_def in clobber_defs {
        // skip untrackable clobbers
        if !effect_is_trackable(&clobber_def.effect) {
            return false;
        }

        // confirm both defs touch the same location
        if !effects_match_location(tree, alias, &candidate.effect, &clobber_def.effect) {
            return false;
        }

        // classify the clobbering definition
        let Some(clobber_kind) = def_kind_for_instruction(tree, tree.get(clobber_def.instruction))
        else {
            return false;
        };

        // compare candidate value and clobber value
        if !def_kinds_equivalent(
            candidate,
            &clobber_kind,
            clobber_def,
            memory_ssa,
            alias,
            tree,
            equivalence,
        ) {
            return false;
        }
    }

    true
}

/// Collect clobbering def accesses for a MemorySSA clobber id.
fn clobber_def_accesses(
    memory_ssa: &MemorySSA,
    clobber: MemoryAccessId,
) -> Option<Vec<&MemoryDef>> {
    // resolve the clobber access kind
    match memory_ssa.access(clobber) {
        MemoryAccess::Def(def_access) => Some(vec![def_access]),
        MemoryAccess::Phi(phi) => {
            // collect incoming defs for a phi
            let mut defs = Vec::new();

            for (_, incoming) in &phi.incoming {
                let MemoryAccess::Def(def_access) = memory_ssa.access(*incoming) else {
                    return None;
                };
                defs.push(def_access);
            }

            // return None when no incoming defs are available
            if defs.is_empty() { None } else { Some(defs) }
        }
        _ => None,
    }
}

/// Check if the candidate and clobber kinds are equivalent.
fn def_kinds_equivalent(
    candidate: &DefCandidate,
    clobber_kind: &DefKind,
    clobber_def: &MemoryDef,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
    tree: &mir::Tree,
    equivalence: &mut ValueEquivalence<'_>,
) -> bool {
    // compare candidate and clobber kinds
    match (&candidate.kind, clobber_kind) {
        (DefKind::Store { value }, DefKind::Store { value: other })
        | (DefKind::LocalSet { value }, DefKind::LocalSet { value: other }) => {
            // compare scalar values
            equivalence.equivalent(*value, *other)
        }
        (
            DefKind::Memset { value, size },
            DefKind::Memset {
                value: other,
                size: other_size,
            },
        ) => {
            // compare memset value and size
            equivalence.equivalent(*value, *other) && equivalence.equivalent(*size, *other_size)
        }
        (
            DefKind::Memcpy { source, size },
            DefKind::Memcpy {
                source: other_source,
                size: other_size,
            },
        )
        | (
            DefKind::Memmove { source, size },
            DefKind::Memmove {
                source: other_source,
                size: other_size,
            },
        ) => {
            // compare memop source and size
            if !equivalence.equivalent(*source, *other_source)
                || !equivalence.equivalent(*size, *other_size)
            {
                return false;
            }

            // resolve source effects for both memops
            let Some(candidate_source) = memop_source_access(memory_ssa, candidate.instruction)
            else {
                return false;
            };

            let Some(clobber_source) = memop_source_access(memory_ssa, clobber_def.instruction)
            else {
                return false;
            };

            // compute alias relationship between source and destination
            let allow_must_alias = matches!(candidate.kind, DefKind::Memmove { .. });
            let alias_result =
                memop_alias_result(&candidate.effect, &candidate_source.effect, alias, tree);

            // allow memmove with identical source and destination
            if allow_must_alias && alias_result == AliasResult::MustAlias {
                return true;
            }

            // require the source memory to be stable
            if !memop_source_is_stable(&candidate_source, &clobber_source, memory_ssa, alias, tree)
            {
                return false;
            }

            // require no overlap between source and destination
            memop_sources_do_not_overlap(alias_result, allow_must_alias)
        }
        _ => false,
    }
}

/// Return true when a memory access is atomically ordered.
/// Return the source memory access for a memcpy or memmove instruction.
fn memop_source_access(
    memory_ssa: &MemorySSA,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
) -> Option<SourceAccess> {
    // collect access ids for the instruction
    let accesses = memory_ssa.accesses_for_instruction(instruction_id)?;
    let mut source_access = None;

    // locate the single read access
    for access_id in accesses {
        let MemoryAccess::Use(use_access) = memory_ssa.access(*access_id) else {
            continue;
        };

        if matches!(use_access.effect.location, MemoryAccessLocation::Pointer(_)) {
            if source_access.is_some() {
                return None;
            }

            source_access = Some(SourceAccess {
                access: *access_id,
                effect: use_access.effect.clone(),
            });
        }
    }

    source_access
}

/// Check whether the memop source is unchanged between two uses.
fn memop_source_is_stable(
    candidate: &SourceAccess,
    clobber: &SourceAccess,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
    tree: &mir::Tree,
) -> bool {
    // compare clobbering accesses for the source
    let candidate_clobber = memory_ssa.clobbering_access_for_use(candidate.access, alias, tree);
    let clobber_clobber = memory_ssa.clobbering_access_for_use(clobber.access, alias, tree);

    candidate_clobber == clobber_clobber
}

/// Check whether the memop source and destination do not overlap.
fn memop_sources_do_not_overlap(alias_result: AliasResult, allow_must_alias: bool) -> bool {
    // evaluate alias relationship between source and destination
    match alias_result {
        AliasResult::NoAlias => true,
        AliasResult::MustAlias => allow_must_alias,
        _ => false,
    }
}

/// Compute alias results for memop source and destination.
fn memop_alias_result(
    dest_effect: &MemoryAccessEffect,
    source_effect: &MemoryAccessEffect,
    alias: &AliasAnalysis,
    tree: &mir::Tree,
) -> AliasResult {
    // apply alias scopes
    if !alias_scopes_may_alias(
        &dest_effect.alias_scopes,
        &dest_effect.noalias_scopes,
        &source_effect.alias_scopes,
        &source_effect.noalias_scopes,
    ) {
        return AliasResult::NoAlias;
    }

    // apply location sets
    if !space_sets_may_alias(dest_effect.space_set, source_effect.space_set) {
        return AliasResult::NoAlias;
    }

    // apply address spaces
    if !address_spaces_may_alias(&dest_effect.address_spaces, &source_effect.address_spaces) {
        return AliasResult::NoAlias;
    }

    // apply type-alias metadata
    if !type_alias_tags_may_alias(
        &tree.metadata.memory.type_alias,
        dest_effect.type_alias_tag,
        source_effect.type_alias_tag,
    ) {
        return AliasResult::NoAlias;
    }

    // ask alias analysis for pointer locations
    match (&dest_effect.location, &source_effect.location) {
        (MemoryAccessLocation::Pointer(dest), MemoryAccessLocation::Pointer(source)) => {
            alias.alias(dest, source)
        }
        _ => AliasResult::MayAlias,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Redundant store of the same value is removed.
    #[test]
    fn test_remove_redundant_store() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    store v0, v1
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_output(expected);
    }

    /// Redundant store with equivalent constants is removed.
    #[test]
    fn test_remove_redundant_store_with_equal_constants() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    v2: int32 = 7int32
    store v0, v1
    store v0, v2
    v3: int32 = load v0
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    v2: int32 = 7int32
    store v0, v1
    v3: int32 = load v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_output(expected);
    }

    /// Redundant store with equivalent binary value is removed.
    #[test]
    fn test_remove_redundant_store_with_equivalent_binary() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    v3: int32 = int.add v1, v2
    store v0, v3
    v4: int32 = int.add v1, v2
    store v0, v4
    v5: int32 = load v0
    return v5
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    v3: int32 = int.add v1, v2
    store v0, v3
    v4: int32 = int.add v1, v2
    v5: int32 = load v0
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_output(expected);
    }

    /// Redundant store with commuted binary value is removed.
    #[test]
    fn test_remove_redundant_store_with_commuted_binary() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    v3: int32 = int.add v1, v2
    store v0, v3
    v4: int32 = int.add v2, v1
    store v0, v4
    v5: int32 = load v0
    return v5
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    v3: int32 = int.add v1, v2
    store v0, v3
    v4: int32 = int.add v2, v1
    v5: int32 = load v0
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_output(expected);
    }

    /// Redundant store with constant propagated value is removed.
    #[test]
    fn test_remove_redundant_store_with_constant_propagation() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    v3: int32 = int.add v1, v2
    v4: int32 = 5int32
    store v0, v3
    store v0, v4
    v5: int32 = load v0
    return v5
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    v3: int32 = int.add v1, v2
    v4: int32 = 5int32
    store v0, v3
    v5: int32 = load v0
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_output(expected);
    }

    /// Redundant store across a read only call is removed.
    #[test]
    fn test_remove_redundant_store_across_read_only_call() {
        let input = r#"
function callee(): void {
b0:
    return
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    store v0, v1
    call callee(): () -> void
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;
        let expected = r#"
function callee(): void {
b0:
    return
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    store v0, v1
    call callee(): () -> void
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.function_id_by_name("test");
        let (_, callee) = test.first_call_in_entry(function_id);
        test.tree.get_mut(callee).memory_effect =
            mir::MemoryEffect::read_only(mir::MemorySpaceSet::ANY);

        test.run_pass(&MemCse);
        test.assert_output(expected);
    }

    /// Store across a write call is preserved.
    #[test]
    fn test_preserve_store_across_write_call() {
        let input = r#"
function callee(): void {
b0:
    return
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    store v0, v1
    call callee(): () -> void
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.function_id_by_name("test");
        let (_, callee) = test.first_call_in_entry(function_id);
        test.tree.get_mut(callee).memory_effect =
            mir::MemoryEffect::read_write(mir::MemorySpaceSet::ANY);

        test.run_pass(&MemCse);
        test.assert_unchanged(input);
    }

    /// Redundant store across heap only call is removed.
    #[test]
    fn test_remove_redundant_store_across_heap_only_call() {
        let input = r#"
function callee(): void {
b0:
    return
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    store v0, v1
    call callee(): () -> void
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;
        let expected = r#"
function callee(): void {
b0:
    return
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    store v0, v1
    call callee(): () -> void
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.function_id_by_name("test");
        let (_, callee) = test.first_call_in_entry(function_id);
        test.tree.get_mut(callee).memory_effect =
            mir::MemoryEffect::write_only(mir::MemorySpaceSet::RAW_HEAP);

        test.run_pass(&MemCse);
        test.assert_output(expected);
    }

    /// Redundant store across disjoint address space call is removed.
    #[test]
    fn test_remove_redundant_store_across_address_space_call() {
        let input = r#"
function callee(): void {
b0:
    return
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    store v0, v1
    call callee(): () -> void
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;
        let expected = r#"
function callee(): void {
b0:
    return
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    store v0, v1
    call callee(): () -> void
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.function_id_by_name("test");
        let (_, callee) = test.first_call_in_entry(function_id);
        test.tree.get_mut(callee).memory_effect =
            mir::MemoryEffect::write_only(mir::MemorySpaceSet::ANY)
                .with_address_spaces(mir::AddressSpaceSet::new(vec![mir::AddressSpace::Shared]));

        test.run_pass(&MemCse);
        test.assert_output(expected);
    }

    /// Distinct store values are preserved.
    #[test]
    fn test_preserve_store_with_different_value() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    v2: int32 = 9int32
    store v0, v1
    store v0, v2
    v3: int32 = load v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_unchanged(input);
    }

    /// Store after an intervening clobber is preserved.
    #[test]
    fn test_preserve_store_after_clobber() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 1int32
    v2: int32 = 2int32
    store v0, v1
    store v0, v2
    store v0, v1
    v3: int32 = load v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_unchanged(input);
    }

    /// Volatile stores are never removed.
    #[test]
    fn test_preserve_volatile_store() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    store v0, v1
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let block = test.tree.get(function.blocks[0]);
        let pointer = *test
            .stack_alloc_destinations_in_entry(function_id)
            .first()
            .expect("missing stack allocation");

        let volatile_store = block.instructions[3];
        test.insert_pointer_access_with_options(
            volatile_store,
            mir::MemoryAccessKind::Write,
            pointer,
            None,
            Vec::new(),
            Vec::new(),
            None,
            true,
            None,
        );

        test.run_pass(&MemCse);
        test.assert_unchanged(input);
    }

    /// Atomic stores are never removed.
    #[test]
    fn test_preserve_atomic_store() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    store v0, v1
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let block = test.tree.get(function.blocks[0]);
        let pointer = *test
            .stack_alloc_destinations_in_entry(function_id)
            .first()
            .expect("missing stack allocation");

        let ordered_store = block.instructions[3];
        test.insert_pointer_access_with_options(
            ordered_store,
            mir::MemoryAccessKind::Write,
            pointer,
            None,
            Vec::new(),
            Vec::new(),
            None,
            false,
            Some(mir::MemoryOrdering::SequentiallyConsistent),
        );

        test.run_pass(&MemCse);
        test.assert_unchanged(input);
    }

    /// Redundant store after identical incoming stores is removed.
    #[test]
    fn test_remove_redundant_store_after_phi() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 7int32
    branch v0, b1, b2
b1:
    store v1, v2
    jump b3
b2:
    store v1, v2
    jump b3
b3:
    store v1, v2
    v3: int32 = load v1
    return v3
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 7int32
    branch v0, b1, b2
b1:
    store v1, v2
    jump b3
b2:
    store v1, v2
    jump b3
b3:
    v3: int32 = load v1
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_output(expected);
    }

    /// Store after divergent incoming values is preserved.
    #[test]
    fn test_preserve_store_after_phi_with_different_values() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 7int32
    v3: int32 = 9int32
    branch v0, b1, b2
b1:
    store v1, v2
    jump b3
b2:
    store v1, v3
    jump b3
b3:
    store v1, v2
    v4: int32 = load v1
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_unchanged(input);
    }

    /// Redundant local sets are removed.
    #[test]
    fn test_remove_redundant_local_set() {
        let input = r#"
function test(): int32 {
    local local0: int32, owned
b0:
    v0: int32 = 1int32
    local.set local0, v0
    local.set local0, v0
    v1: int32 = local.get local0
    return v1
}"#;
        let expected = r#"
function test(): int32 {
    local local0: int32, owned
b0:
    v0: int32 = 1int32
    local.set local0, v0
    v1: int32 = local.get local0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_output(expected);
    }

    /// Redundant memset is removed.
    #[test]
    fn test_remove_redundant_memset() {
        let input = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int8 = 0int8
    v2: int64 = 4int64
    intrinsic.memset(v0, v1, v2)
    intrinsic.memset(v0, v1, v2)
    return
}"#;
        let expected = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int8 = 0int8
    v2: int64 = 4int64
    intrinsic.memset(v0, v1, v2)
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_output(expected);
    }

    /// Redundant memcpy is removed.
    #[test]
    fn test_remove_redundant_memcpy() {
        let input = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int64 = 4int64
    intrinsic.memcpy(v0, v1, v2)
    intrinsic.memcpy(v0, v1, v2)
    return
}"#;
        let expected = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int64 = 4int64
    intrinsic.memcpy(v0, v1, v2)
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_output(expected);
    }

    /// Memcpy with differing size is preserved.
    #[test]
    fn test_preserve_memcpy_with_different_size() {
        let input = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int64 = 4int64
    v3: int64 = 8int64
    intrinsic.memcpy(v0, v1, v2)
    intrinsic.memcpy(v0, v1, v3)
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_unchanged(input);
    }

    /// Memcpy with source changes is preserved.
    #[test]
    fn test_preserve_memcpy_with_source_change() {
        let input = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int64 = 4int64
    v3: int32 = 7int32
    intrinsic.memcpy(v0, v1, v2)
    store v1, v3
    intrinsic.memcpy(v0, v1, v2)
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_unchanged(input);
    }

    /// Redundant memmove is removed when source is stable.
    #[test]
    fn test_remove_redundant_memmove() {
        let input = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int64 = 4int64
    intrinsic.memmove(v0, v1, v2)
    intrinsic.memmove(v0, v1, v2)
    return
}"#;
        let expected = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int64 = 4int64
    intrinsic.memmove(v0, v1, v2)
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_output(expected);
    }

    /// Memmove with overlapping spaces is preserved.
    #[test]
    fn test_preserve_overlapping_memmove() {
        let input = r#"
type Bytes int8[12]
function test(): void {
b0:
    v0: ref<Bytes, raw, space(stack)> = stack.alloc Bytes
    v1: int64 = 0int64
    v2: int64 = 4int64
    v3: ref<int8, borrowed> = element.address v0, v1
    v4: ref<int8, borrowed> = element.address v0, v2
    v5: int64 = 8int64
    intrinsic.memmove(v4, v3, v5)
    intrinsic.memmove(v4, v3, v5)
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MemCse);
        test.assert_unchanged(input);
    }
}

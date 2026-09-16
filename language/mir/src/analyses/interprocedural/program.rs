use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use destack_core::{FxIndexMap, StableHasher};
use destack_serde::{Error, Reflect, hash_into};
use serde::{Deserialize, Serialize};

use crate::{
    CallComponentTable, ControlTable, EffectBody, EffectTable, EscapeBody, EscapeEffect,
    FunctionEffect, FunctionId, Linkage, ResolutionTable, Symbol, Tree,
};

/// Local effects and pointer flows extracted from one function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionEffectBody {
    /// Whether this contribution contains a function definition.
    is_defined: bool,
    /// Whether the linker can select an equivalent definition from another module.
    is_shared: bool,
    /// Local effects and dependencies on callee effects.
    effects: EffectBody,
    /// Pointer flows and dependencies on callee escape paths.
    escape: EscapeBody,
    /// The fingerprint of these extracted effects and pointer flows.
    fingerprint: u128,
}

/// Derived behavior and pointer flow for one function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionEffectResult {
    /// Memory and execution effects visible to callers.
    pub effect: FunctionEffect,
    /// Parameter retention, return paths, and external result pointers.
    pub escape: EscapeEffect,
}

/// Reusable interprocedural results indexed by persistent function symbols.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct ProgramEffectTable {
    /// Function symbols and extracted input fingerprints in symbol order.
    functions: Vec<(Symbol, u128)>,
    /// Derived results in the same function order.
    results: Vec<Arc<FunctionEffectResult>>,
    /// Recursive components in callee first order.
    components: CallComponentTable,
}

/// Conflicting declarations or definitions of one program function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramEffectError {
    /// Multiple definitions cannot share this symbol.
    ConflictingDefinition(Symbol),
    /// External declarations disagree for this symbol.
    ConflictingDeclaration(Symbol),
    /// A callee has no declaration or definition in the program.
    MissingFunction(Symbol),
}

impl Display for ProgramEffectError {
    /// Describe the conflicting or missing program function.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingDefinition(symbol) => {
                write!(formatter, "conflicting definitions for {symbol:?}")
            }
            Self::ConflictingDeclaration(symbol) => {
                write!(formatter, "conflicting declarations for {symbol:?}")
            }
            Self::MissingFunction(symbol) => write!(formatter, "missing function {symbol:?}"),
        }
    }
}

impl std::error::Error for ProgramEffectError {}

impl FunctionEffectBody {
    /// Return whether this contribution contains a function definition.
    pub fn is_defined(&self) -> bool {
        self.is_defined
    }

    /// Return whether other modules can define the same specialization.
    pub fn is_shared(&self) -> bool {
        self.is_shared
    }

    /// Return the pointer graph extracted from this function's own MIR nodes.
    pub fn escape(&self) -> &EscapeBody {
        &self.escape
    }

    /// Iterate the known callees whose effects this function consumes.
    pub fn callees(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.effects
            .calls
            .iter()
            .flat_map(|call| call.targets.iter().copied())
    }

    /// Extract the local inputs consumed by interprocedural analysis.
    pub fn analyse(
        function: FunctionId,
        resolution: &ResolutionTable,
        effects: &EffectTable,
        tree: &Tree,
    ) -> Result<Self, Error> {
        // extract both analyses using the same control flow graph
        let declaration = tree.get(function);
        let graph = Arc::new(ControlTable::analyse(declaration, tree));
        let is_defined = declaration.is_defined();
        let is_shared = declaration.linkage == Linkage::Shared;
        let effect = EffectBody::analyse(function, &graph, resolution, effects, tree);
        let escape = EscapeBody::analyse(function, graph, resolution, effects, tree);

        // fingerprint the extracted inputs and linkage
        let mut hasher = StableHasher::new();
        hash_into(&(is_defined, is_shared, &effect, &escape), &mut hasher)?;
        let fingerprint = hasher.finish_u128();

        Ok(Self {
            is_defined,
            is_shared,
            effects: effect,
            escape,
            fingerprint,
        })
    }
}

impl ProgramEffectTable {
    /// Solve recursive components and reuse results whose inputs and callee effects are equal.
    pub fn analyse(
        mut functions: Vec<(Symbol, Arc<FunctionEffectBody>)>,
        previous: Option<&Self>,
    ) -> Result<Self, ProgramEffectError> {
        // group declarations and definitions while preserving shared definition order
        functions.sort_by_key(|(symbol, _)| *symbol);
        let mut selected: Vec<(Symbol, Arc<FunctionEffectBody>)> = Vec::new();
        for (symbol, incoming) in functions {
            match selected
                .last_mut()
                .filter(|(current, _)| *current == symbol)
            {
                // retain the selected definition and reject conflicting definitions
                Some((_, current)) if current.is_defined() => {
                    if incoming.is_defined() && !(current.is_shared() && incoming.is_shared()) {
                        return Err(ProgramEffectError::ConflictingDefinition(symbol));
                    }
                }
                // require external declarations to agree
                Some((_, current)) if !incoming.is_defined() => {
                    if current != &incoming {
                        return Err(ProgramEffectError::ConflictingDeclaration(symbol));
                    }
                }
                // replace the declaration with its definition
                Some((_, current)) => *current = incoming,
                // retain the first occurrence of this symbol
                None => selected.push((symbol, incoming)),
            }
        }
        let functions = selected;

        let indices = functions
            .iter()
            .enumerate()
            .map(|(index, (symbol, _))| (*symbol, index))
            .collect::<FxIndexMap<_, _>>();
        let mut offsets = vec![0];
        let mut targets = Vec::new();

        // build the component graph from named callees in each extracted body
        let mut outgoing = Vec::new();
        for (_, function) in &functions {
            outgoing.clear();
            for target in function.callees() {
                let index = indices
                    .get(&target)
                    .ok_or(ProgramEffectError::MissingFunction(target))?;
                outgoing.push(*index as u32);
            }

            // retain each distinct callee in the function's edge range
            outgoing.sort_unstable();
            outgoing.dedup();
            targets.extend_from_slice(&outgoing);
            offsets.push(targets.len() as u32);
        }
        let components = CallComponentTable::analyse(&offsets, &targets);
        let mut effects = FxIndexMap::default();
        let mut escapes = FxIndexMap::default();

        // initialize every function before solving callees, including recursive declarations
        for (symbol, function) in &functions {
            effects.insert(*symbol, function.effects.local.clone());
            escapes.insert(*symbol, function.escape.initial_effect());
        }

        // solve callees before deciding whether their callers can reuse previous results
        let mut results = vec![None; functions.len()];
        for members in components.components() {
            let reused = previous.filter(|previous| {
                previous.can_reuse(members, &functions, &offsets, &targets, &effects, &escapes)
            });

            // reuse complete results when the component and its external callees are unchanged
            if let Some(previous) = reused {
                for &member in members {
                    let symbol = functions[member as usize].0;
                    let result = previous
                        .function(symbol)
                        .unwrap_or_else(|| unreachable!("reused function has no result"))
                        .clone();
                    effects.insert(symbol, result.effect.clone());
                    escapes.insert(symbol, result.escape.clone());
                    results[member as usize] = Some(result);
                }
            }
            // propagate finite effect sets and minimum pointer depths within this component
            else {
                loop {
                    let mut changed = false;
                    for &member in members {
                        let (symbol, function) = &functions[member as usize];
                        let effect = function.effects.analyse_calls(&effects);
                        let escape = function.escape.analyse_calls(&escapes).effect;
                        changed |= effects[symbol] != effect || escapes[symbol] != escape;
                        effects.insert(*symbol, effect);
                        escapes.insert(*symbol, escape);
                    }

                    // stop after one acyclic visit or a stable recursive round
                    if !changed || !components.is_recursive(members[0] as usize) {
                        break;
                    }
                }

                // allocate published results after the complete component stabilizes
                for &member in members {
                    let symbol = functions[member as usize].0;
                    results[member as usize] = Some(Arc::new(FunctionEffectResult {
                        effect: effects[&symbol].clone(),
                        escape: escapes[&symbol].clone(),
                    }));
                }
            }
        }

        // require a result for every declaration and definition in the graph
        let results = results
            .into_iter()
            .map(|result| {
                result.unwrap_or_else(|| unreachable!("function component was not solved"))
            })
            .collect();

        // retain only the identities and fingerprints needed for reuse
        let functions = functions
            .into_iter()
            .map(|(symbol, function)| (symbol, function.fingerprint))
            .collect();

        Ok(Self {
            functions,
            results,
            components,
        })
    }

    /// Return the derived result for a function in this program.
    pub fn function(&self, symbol: Symbol) -> Option<&Arc<FunctionEffectResult>> {
        let index = self
            .functions
            .binary_search_by_key(&symbol, |(symbol, _)| *symbol)
            .ok()?;

        Some(&self.results[index])
    }

    /// Check component membership, local inputs, and every consumed external callee result.
    fn can_reuse(
        &self,
        members: &[u32],
        functions: &[(Symbol, Arc<FunctionEffectBody>)],
        offsets: &[u32],
        targets: &[u32],
        effects: &FxIndexMap<Symbol, FunctionEffect>,
        escapes: &FxIndexMap<Symbol, EscapeEffect>,
    ) -> bool {
        // locate the previous component through its first persistent function symbol
        let first = functions[members[0] as usize].0;
        let Ok(index) = self
            .functions
            .binary_search_by_key(&first, |(symbol, _)| *symbol)
        else {
            return false;
        };
        let component = self.components.component(index) as usize;
        let previous = self.components.members(component);
        if previous.len() != members.len() {
            return false;
        }

        // compare persistent identities and extracted inputs
        for (&member, &previous) in members.iter().zip(previous) {
            let (symbol, function) = &functions[member as usize];
            let (previous_symbol, previous_function) = &self.functions[previous as usize];
            if symbol != previous_symbol || function.fingerprint != *previous_function {
                return false;
            }
            let outgoing =
                &targets[offsets[member as usize] as usize..offsets[member as usize + 1] as usize];
            for &target in outgoing {
                if members.binary_search(&target).is_ok() {
                    continue;
                }
                let symbol = functions[target as usize].0;
                let Some(previous) = self.function(symbol) else {
                    return false;
                };
                if previous.effect != effects[&symbol] || previous.escape != escapes[&symbol] {
                    return false;
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use destack_serde::{from_slice, to_vec};

    use crate::analyses::tests::TestProgram;
    use crate::{
        EscapeEffect, FunctionBehavior, FunctionEffect, FunctionEffectResult, MemoryEffect,
        ParameterEscape, ProgramEffectTable, StorageSet,
    };

    /// Reuse callers across modules when an edited callee exposes the same effects and paths.
    #[test]
    fn test_reuse_unchanged_callee_effects() {
        let leaf_source = r#"
export function leaf(v0: int32): int32 {
entry(v0: int32):
    v1: ref<int32, unique, mutable, local> = new.zeroed int32
    return v0
}
"#;
        let caller_source = r#"
external function leaf(int32): int32

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call leaf(v0): (int32) => int32
    return v1
}
"#;
        let mut program = TestProgram::new(&[leaf_source, caller_source]);
        program.import((1, "leaf"), (0, "leaf"));
        let first = program.analyse_effects(None);
        let encoded = to_vec(&first).expect("serialize program effects");
        let first: ProgramEffectTable = from_slice(&encoded).expect("deserialize program effects");
        assert_eq!(to_vec(&first).unwrap(), encoded);

        let module = &program.modules[1];
        let caller = module.tree.get(module.entry_function_id()).symbol;
        let module = &program.modules[0];
        let leaf = module.tree.get(module.function_id_by_name("leaf")).symbol;
        let second = program.analyse_effects(Some(&first));

        assert!(Arc::ptr_eq(
            first.function(caller).unwrap(),
            second.function(caller).unwrap()
        ));
        assert_eq!(
            second.function(caller).unwrap().effect,
            FunctionEffect {
                memory: MemoryEffect::write_only(StorageSet::LOCAL),
                behavior: FunctionBehavior::none().with_allocates(),
            }
        );

        // add an allocation while preserving the callee effects consumed by its caller
        let edited = leaf_source.replacen(
            "    return v0",
            "    v2: ref<int32, unique, mutable, local> = new.zeroed int32\n    return v0",
            1,
        );
        let mut program = TestProgram::new(&[&edited, caller_source]);
        program.import((1, "leaf"), (0, "leaf"));
        let third = program.analyse_effects(Some(&second));

        assert!(!Arc::ptr_eq(
            second.function(leaf).unwrap(),
            third.function(leaf).unwrap()
        ));
        assert!(Arc::ptr_eq(
            second.function(caller).unwrap(),
            third.function(caller).unwrap()
        ));
    }

    /// Remove recursive effects across modules when an edited component no longer performs them.
    #[test]
    fn test_remove_recursive_effects() {
        let caller_source = r#"
external function second(int32): int32

export function first(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call second(v0): (int32) => int32
    return v1
}
"#;
        let callee_source = r#"
external function first(int32): int32

export function second(v0: int32): int32 {
entry(v0: int32):
    poll
    v1: int32 = call first(v0): (int32) => int32
    return v0
}
"#;
        let mut program = TestProgram::new(&[caller_source, callee_source]);
        program.import((0, "second"), (1, "second"));
        program.import((1, "first"), (0, "first"));
        let first = program.analyse_effects(None);
        let module = &program.modules[0];
        let caller = module.tree.get(module.function_id_by_name("first")).symbol;
        let module = &program.modules[1];
        let callee = module.tree.get(module.function_id_by_name("second")).symbol;

        let expected = FunctionEffectResult {
            effect: FunctionEffect {
                memory: MemoryEffect::none(),
                behavior: FunctionBehavior::none().with_preserved_execution(),
            },
            escape: EscapeEffect {
                parameters: vec![ParameterEscape {
                    retained: None,
                    returned: Some(0),
                }],
                environment: None,
                external: None,
            },
        };
        for function in [caller, callee] {
            assert_eq!(first.function(function).unwrap().as_ref(), &expected);
        }

        // remove the poll and solve the recursive component from its initial state
        let edited = callee_source.replace("    poll\n", "");
        let mut program = TestProgram::new(&[caller_source, &edited]);
        program.import((0, "second"), (1, "second"));
        program.import((1, "first"), (0, "first"));
        let second = program.analyse_effects(Some(&first));

        let expected = FunctionEffectResult {
            effect: FunctionEffect::none(),
            ..expected
        };
        for function in [caller, callee] {
            assert_eq!(second.function(function).unwrap().as_ref(), &expected);
        }
        assert!(!Arc::ptr_eq(
            first.function(caller).unwrap(),
            second.function(caller).unwrap()
        ));
    }

    /// Rebuild components when edits join and separate recursive functions across modules.
    #[test]
    fn test_merge_and_split_call_components() {
        let caller_source = r#"
external function second(int32): int32

export function first(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call second(v0): (int32) => int32
    return v1
}
"#;
        let callee_source = r#"
external function first(int32): int32

export function second(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#;
        let mut program = TestProgram::new(&[caller_source, callee_source]);
        program.import((0, "second"), (1, "second"));
        program.import((1, "first"), (0, "first"));
        let first = program.analyse_effects(None);
        let module = &program.modules[0];
        let caller = module.tree.get(module.function_id_by_name("first")).symbol;
        let module = &program.modules[1];
        let callee = module.tree.get(module.function_id_by_name("second")).symbol;

        // add a backedge while preserving the result returned from the recursive component
        let joined = callee_source.replace(
            "    return v0",
            "    v1: int32 = call first(v0): (int32) => int32\n    return v0",
        );
        let mut joined = TestProgram::new(&[caller_source, &joined]);
        joined.import((0, "second"), (1, "second"));
        joined.import((1, "first"), (0, "first"));
        let second = joined.analyse_effects(Some(&first));
        assert_eq!(
            second.function(caller).unwrap().effect,
            FunctionEffect::none()
        );
        assert!(!Arc::ptr_eq(
            first.function(caller).unwrap(),
            second.function(caller).unwrap()
        ));
        assert!(!Arc::ptr_eq(
            first.function(callee).unwrap(),
            second.function(callee).unwrap()
        ));

        // remove the backedge and compare complete recomputed results
        let third = program.analyse_effects(Some(&second));
        assert_eq!(third.function(caller), first.function(caller));
        assert_eq!(third.function(callee), first.function(callee));
        assert!(!Arc::ptr_eq(
            second.function(caller).unwrap(),
            third.function(caller).unwrap()
        ));
        assert!(!Arc::ptr_eq(
            second.function(callee).unwrap(),
            third.function(callee).unwrap()
        ));
    }
}

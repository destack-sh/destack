use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{Origin, Task};
use crate::{CompilerError, CompilerResult};

/// One open inference variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct VariableState {
    /// The source that produced the variable.
    pub(in crate::check) origin: Origin,
    /// The literal widening policy applied when solving.
    pub(in crate::check) widening: Widening,
    /// Types that must be assignable to the variable.
    pub(in crate::check) lower: SmallVec<[dir::GlobalTypeId; 2]>,
    /// Types the variable must be assignable to.
    pub(in crate::check) upper: SmallVec<[dir::GlobalTypeId; 2]>,
    /// The solved type, when solving finished.
    pub(in crate::check) solution: Option<dir::GlobalTypeId>,
    /// The fallback solution applied when no bounds arrive.
    pub(in crate::check) default: Option<dir::GlobalTypeId>,
    /// The union-find representative, when aliased to another variable.
    pub(in crate::check) alias: Option<dir::TypeVariableId>,
    /// Tasks waiting for this variable to solve.
    pub(in crate::check) waiters: SmallVec<[Task; 2]>,
}

/// Literal widening policy for one solved variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum Widening {
    /// Keep literal solutions exact.
    Preserve,
    /// Widen literal solutions to their base type.
    Widen,
}

/// Dense per-module variable storage for one checked component.
#[derive(Debug)]
pub(in crate::check) struct VariableTable {
    /// Variables keyed by allocating module.
    modules: IndexMap<ModuleId, Vec<VariableState>>,
}

impl VariableTable {
    /// Create an empty variable table.
    pub(in crate::check) fn new() -> Self {
        Self {
            modules: IndexMap::new(),
        }
    }

    /// Allocate one unsolved variable.
    pub(in crate::check) fn allocate(
        &mut self,
        module: ModuleId,
        origin: Origin,
        widening: Widening,
    ) -> dir::TypeVariableId {
        let variables = self.modules.entry(module).or_default();
        let id = dir::TypeVariableId::new(module, variables.len() as u32);
        variables.push(VariableState {
            origin,
            widening,
            lower: SmallVec::new(),
            upper: SmallVec::new(),
            solution: None,
            default: None,
            alias: None,
            waiters: SmallVec::new(),
        });

        id
    }

    /// Return one variable state.
    pub(in crate::check) fn get(&self, id: dir::TypeVariableId) -> CompilerResult<&VariableState> {
        self.modules
            .get(&id.module_id)
            .and_then(|variables| variables.get(id.index as usize))
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check variable {id:?} is not allocated"),
            })
    }

    /// Return one mutable variable state.
    pub(in crate::check) fn get_mut(
        &mut self,
        id: dir::TypeVariableId,
    ) -> CompilerResult<&mut VariableState> {
        self.modules
            .get_mut(&id.module_id)
            .and_then(|variables| variables.get_mut(id.index as usize))
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check variable {id:?} is not allocated"),
            })
    }

    /// Chase alias links to the representative variable.
    pub(in crate::check) fn representative(
        &self,
        id: dir::TypeVariableId,
    ) -> CompilerResult<dir::TypeVariableId> {
        let mut current = id;

        // follow alias links to the root
        while let Some(alias) = self.get(current)?.alias {
            current = alias;
        }

        Ok(current)
    }

    /// Return the solved type of one variable through its representative.
    pub(in crate::check) fn solution(
        &self,
        id: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let representative = self.representative(id)?;

        Ok(self.get(representative)?.solution)
    }

    /// Return the number of variables allocated in one module.
    pub(in crate::check) fn count_in(&self, module: ModuleId) -> usize {
        self.modules
            .get(&module)
            .map(|variables| variables.len())
            .unwrap_or(0)
    }

    /// Return the total number of allocated variables.
    pub(in crate::check) fn count(&self) -> usize {
        self.modules.values().map(|variables| variables.len()).sum()
    }

    /// Iterate all variables with their ids.
    pub(in crate::check) fn iter(
        &self,
    ) -> impl Iterator<Item = (dir::TypeVariableId, &VariableState)> + '_ {
        self.modules.iter().flat_map(|(module, variables)| {
            variables
                .iter()
                .enumerate()
                .map(|(index, state)| (dir::TypeVariableId::new(*module, index as u32), state))
        })
    }

    /// Truncate one module's variables to a previous count.
    pub(in crate::check) fn truncate(&mut self, module: ModuleId, count: usize) {
        if let Some(variables) = self.modules.get_mut(&module) {
            variables.truncate(count);
        }
    }
}

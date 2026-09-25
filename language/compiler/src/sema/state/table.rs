use rustc_hash::FxHashMap;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{
    Bound, BoundEntry, BoundIter, BoundList, BoundSide, EMPTY, OriginId, Variable, VariableKind,
    VariableState,
};
use crate::{CompilerError, CompilerResult};

/// Committed node types in dense module columns.
#[derive(Debug, Default)]
pub(in crate::sema) struct NodeTable {
    /// One dense column per module, indexed by tree-global node id.
    columns: FxHashMap<ModuleId, Vec<Option<(dir::NodeType, dir::GlobalTypeId)>>>,
}

impl NodeTable {
    /// Return one committed node type.
    pub(in crate::sema) fn get(&self, node: &dir::GlobalNodeIdAny) -> Option<dir::GlobalTypeId> {
        let column = self.columns.get(&node.module_id)?;
        let (tag, ty) = column.get(node.local_id.id as usize).copied().flatten()?;

        (tag == node.local_id.ty).then_some(ty)
    }

    /// Drop one committed node type.
    pub(in crate::sema) fn remove(&mut self, node: dir::GlobalNodeIdAny) {
        if let Some(column) = self.columns.get_mut(&node.module_id)
            && let Some(entry) = column.get_mut(node.local_id.id as usize)
        {
            *entry = None;
        }
    }

    /// Commit one node type.
    pub(in crate::sema) fn insert(&mut self, node: dir::GlobalNodeIdAny, ty: dir::GlobalTypeId) {
        // grow this module's column to cover the node
        let column = self.columns.entry(node.module_id).or_default();
        let index = node.local_id.id as usize;
        if column.len() <= index {
            column.resize_with(index + 1, || None);
        }

        // commit the node type
        column[index] = Some((node.local_id.ty, ty));
    }

    /// Collect every committed node id.
    pub(in crate::sema) fn nodes(&self) -> Vec<dir::GlobalNodeIdAny> {
        // collect each column's tagged entries
        let mut nodes = Vec::new();
        for (module, column) in &self.columns {
            for (index, entry) in column.iter().enumerate() {
                if let Some((tag, _)) = entry {
                    nodes.push(dir::GlobalNodeIdAny {
                        module_id: *module,
                        local_id: dir::LocalNodeIdAny::new(index as u32, *tag),
                    });
                }
            }
        }

        nodes
    }
}

/// Variables and their shared bound storage, owned by one solver.
#[derive(Debug, Default)]
pub(in crate::sema) struct VariableTable {
    /// Variables indexed by variable id.
    variables: Vec<Variable>,
    /// The shared bounds linked per variable side.
    bounds: Vec<BoundEntry>,
}

impl VariableTable {
    /// Create an empty variable table.
    pub(in crate::sema) fn new() -> Self {
        Self::default()
    }

    /// Allocate one variable.
    pub(in crate::sema) fn allocate(
        &mut self,
        variable: dir::TypeVariableId,
        origin: OriginId,
        kind: VariableKind,
    ) {
        debug_assert_eq!(self.variables.len() as u32, variable.0);
        self.variables.push(Variable {
            origin,
            lower: BoundList::new(),
            upper: BoundList::new(),
            state: VariableState::Open,
            kind,
            parameter: None,
            default: None,
            is_fixed: false,
            is_dead: false,
            is_join: false,
        });
    }

    /// Return one variable.
    pub(in crate::sema) fn get(&self, id: dir::TypeVariableId) -> CompilerResult<&Variable> {
        self.variables
            .get(id.0 as usize)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check variable {id:?} is not allocated"),
            })
    }

    /// Return one variable mutably.
    pub(in crate::sema) fn get_mut(
        &mut self,
        id: dir::TypeVariableId,
    ) -> CompilerResult<&mut Variable> {
        self.variables
            .get_mut(id.0 as usize)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check variable {id:?} is not allocated"),
            })
    }

    /// Append one bound to one variable side and return whether it is new.
    pub(in crate::sema) fn push_bound(
        &mut self,
        id: dir::TypeVariableId,
        side: BoundSide,
        bound: Bound,
    ) -> CompilerResult<bool> {
        // keep the first cause of a bound already collected on this side
        if self
            .side_bounds(id, side)?
            .any(|existing| existing.ty == bound.ty && existing.relation == bound.relation)
        {
            return Ok(false);
        }

        // append the bound and link it as the new tail
        let entry = self.bounds.len() as u32;
        self.bounds.push(BoundEntry { bound, next: EMPTY });
        let list = *self.side(id, side)?;
        if list.tail != EMPTY {
            self.bounds[list.tail as usize].next = entry;
        }
        *self.side_mut(id, side)? = BoundList {
            head: if list.head == EMPTY { entry } else { list.head },
            tail: entry,
            count: list.count + 1,
        };

        Ok(true)
    }

    /// Iterate one variable side in insertion order.
    pub(in crate::sema) fn side_bounds(
        &self,
        id: dir::TypeVariableId,
        side: BoundSide,
    ) -> CompilerResult<BoundIter<'_>> {
        let list = self.side(id, side)?;

        // iterate the bounds on that side
        Ok(BoundIter {
            bounds: &self.bounds,
            current: list.head,
        })
    }

    /// Return one side list.
    fn side(&self, id: dir::TypeVariableId, side: BoundSide) -> CompilerResult<&BoundList> {
        let variable = self.get(id)?;

        // read the list each side holds
        Ok(match side {
            BoundSide::Lower => &variable.lower,
            BoundSide::Upper => &variable.upper,
        })
    }

    /// Return one mutable side list.
    fn side_mut(
        &mut self,
        id: dir::TypeVariableId,
        side: BoundSide,
    ) -> CompilerResult<&mut BoundList> {
        let variable = self.get_mut(id)?;

        // read the list each side holds
        Ok(match side {
            BoundSide::Lower => &mut variable.lower,
            BoundSide::Upper => &mut variable.upper,
        })
    }

    /// Return the number of collected bounds.
    pub(in crate::sema) fn bound_count(&self) -> usize {
        self.bounds.len()
    }

    /// Drop the bounds collected past one count.
    pub(in crate::sema) fn truncate_bounds(&mut self, count: usize) {
        self.bounds.truncate(count);
    }

    /// End both of one variable's bound lists at their tails, dropping links past them.
    pub(in crate::sema) fn seal_tails(&mut self, id: dir::TypeVariableId) -> CompilerResult<()> {
        let variable = *self.get(id)?;
        for tail in [variable.lower.tail, variable.upper.tail] {
            if tail != EMPTY
                && let Some(entry) = self.bounds.get_mut(tail as usize)
            {
                entry.next = EMPTY;
            }
        }

        Ok(())
    }

    /// Return the total number of allocated variables.
    pub(in crate::sema) fn count(&self) -> usize {
        self.variables.len()
    }

    /// Iterate all variables with their ids.
    pub(in crate::sema) fn iter(
        &self,
    ) -> impl Iterator<Item = (dir::TypeVariableId, &Variable)> + '_ {
        self.variables
            .iter()
            .enumerate()
            .map(|(index, variable)| (dir::TypeVariableId(index as u32), variable))
    }
}

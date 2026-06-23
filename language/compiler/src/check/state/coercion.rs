use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{CheckState, Mutation};
use crate::{CompilerError, CompilerResult};

/// Implicit coercions derived while solving accepted value relations.
#[derive(Debug)]
pub(in crate::check) struct CoercionTable {
    /// Coercions keyed by the value node being coerced.
    coercions: IndexMap<dir::GlobalNodeIdAny, dir::Coercion>,
}

impl CoercionTable {
    /// Create an empty coercion table.
    pub(in crate::check) fn new() -> Self {
        Self {
            coercions: IndexMap::new(),
        }
    }

    /// Return one recorded coercion.
    pub(in crate::check) fn get(&self, node: dir::GlobalNodeIdAny) -> Option<dir::Coercion> {
        self.coercions.get(&node).copied()
    }

    /// Record one coercion and return the previous entry.
    pub(in crate::check) fn insert(
        &mut self,
        node: dir::GlobalNodeIdAny,
        coercion: dir::Coercion,
    ) -> Option<dir::Coercion> {
        self.coercions.insert(node, coercion)
    }

    /// Restore one previous coercion entry.
    pub(in crate::check) fn restore(
        &mut self,
        node: dir::GlobalNodeIdAny,
        previous: Option<dir::Coercion>,
    ) {
        match previous {
            Some(coercion) => {
                self.coercions.insert(node, coercion);
            }
            None => {
                self.coercions.swap_remove(&node);
            }
        }
    }

    /// Iterate coercions owned by one module.
    pub(in crate::check) fn iter_module(
        &self,
        module: ModuleId,
    ) -> impl Iterator<Item = (dir::GlobalNodeIdAny, dir::Coercion)> + '_ {
        self.coercions
            .iter()
            .filter(move |(node, _)| node.module_id == module)
            .map(|(node, coercion)| (*node, *coercion))
    }
}

impl CheckState<'_> {
    /// Set one implicit coercion selected by the solver.
    pub(in crate::check) fn set_coercion(
        &mut self,
        node: dir::GlobalNodeIdAny,
        coercion: dir::Coercion,
    ) -> CompilerResult<()> {
        if self
            .coercions
            .get(node)
            .is_some_and(|previous| previous == coercion)
        {
            return Ok(());
        }

        let previous = self.coercions.insert(node, coercion);
        if previous.is_some_and(|previous| previous != coercion) {
            return Err(CompilerError::Internal {
                message: format!("check node {node:?} received two coercions"),
            });
        }
        self.journal
            .record(Mutation::CoercionSet { node, previous });

        Ok(())
    }

    /// Return recorded coercions owned by one module.
    pub(in crate::check) fn module_coercions(
        &self,
        module: ModuleId,
    ) -> Vec<(dir::GlobalNodeIdAny, dir::Coercion)> {
        self.coercions.iter_module(module).collect()
    }
}

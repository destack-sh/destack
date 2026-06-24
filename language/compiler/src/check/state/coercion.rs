use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{Answer, CheckState, Mutation, Origin, Relation, answer};
use crate::{CompilerError, CompilerResult};

/// Implicit coercions selected while solving assignment relations.
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
    /// Insert the implicit coercion required by one solved relation.
    pub(in crate::check) fn insert_implicit_coercion(
        &mut self,
        origin: Origin,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
        coercion_site: Option<dir::GlobalNodeIdAny>,
        predicates: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<()>> {
        let Some(node) = coercion_site else {
            return Ok(Answer::Ready(()));
        };
        if !matches!(relation, Relation::Assignable | Relation::Writable) {
            return Ok(Answer::Ready(()));
        }

        // derive the coercion under the same assumptions as the relation
        let mark = self.assume(predicates)?;
        let answer = self.implicit_coercion(origin, node, relation, left, right);
        self.release_assumptions(mark);
        let Some(coercion) = answer!(answer?) else {
            return Ok(Answer::Ready(()));
        };

        self.set_coercion(node, coercion)?;

        Ok(Answer::Ready(()))
    }

    /// Return the implicit coercion required by one solved relation.
    fn implicit_coercion(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::Coercion>>> {
        let node_type = match self.node_type_maybe(node) {
            Some(ty) => ty,
            None => {
                return Err(CompilerError::Internal {
                    message: format!("coercion site {node:?} has no input type"),
                });
            }
        };

        // require the relation source to be the node's walked type
        let node_type = self.settled_root(node_type)?;
        let source = self.settled_root(left)?;
        let source = answer!(self.evaluate_root(origin, source)?);
        if !answer!(self.decide_equal(origin, node_type, source)?) {
            return Ok(Answer::Ready(None));
        }

        // keep contextual literals implicit
        let target = self.settled_root(right)?;
        let target = answer!(self.evaluate_root(origin, target)?);
        if self.contextualizes_to(origin, source, target)? {
            return Ok(Answer::Ready(None));
        }
        if answer!(self.decide_equal(origin, source, target)?) {
            return Ok(Answer::Ready(None));
        }
        if !answer!(self.decide_relation(origin, relation, source, target)?) {
            return Err(CompilerError::Internal {
                message: format!(
                    "solved relation no longer holds for coercion site {node:?}: {} {relation:?} {}",
                    self.format_type(source),
                    self.format_type(target)
                ),
            });
        }

        let coercion = dir::Coercion::new(source, target, dir::CastOrigin::Implicit);

        Ok(Answer::Ready(Some(coercion)))
    }

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

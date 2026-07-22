use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, Constraint, ConstraintState, Relation};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return the coercions produced by completed value checks in one module.
    pub(in crate::check) fn implicit_coercions(
        &mut self,
        module: ModuleId,
        failed_applications: &FxIndexSet<dir::GlobalTypeId>,
        sealed: &mut FxIndexMap<dir::GlobalTypeId, Option<dir::GlobalTypeId>>,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::Coercion)>> {
        // decorator arguments decode to static terms and store nothing at runtime
        let decorated = self.decorator_argument_index(module)?;

        let mut collected = Vec::new();
        for (id, constraint) in self.solver.constraints.iter() {
            if self.solver.constraints.state(id)? != ConstraintState::Holds {
                continue;
            }
            let Constraint::Value(constraint) = constraint else {
                continue;
            };
            if !matches!(
                constraint.relation,
                Relation::Assignable | Relation::Writable
            ) || !constraint.use_.is_stored()
            {
                continue;
            }
            let Some(coercion) = self.solver.constraints.coercion(id)? else {
                continue;
            };

            if constraint.node.module_id != module {
                continue;
            }
            if decorated.contains(constraint.node.local_id.id) {
                continue;
            }
            collected.push((constraint.node, coercion.clone()));
        }

        let mut coercions = FxIndexMap::default();
        for (node, mut coercion) in collected {
            let mut result = Ok(());
            coercion.map_type_ids(&mut |id| {
                self.seal_or_record(id, failed_applications, sealed, &mut result)
            });
            result?;

            // every value node has one exact runtime coercion
            match coercions.get(&node) {
                None => {
                    coercions.insert(node, coercion);
                }
                Some(previous) if previous == &coercion => {}
                Some(previous) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "node {node:?} has conflicting implicit coercions {previous:?} and {coercion:?}"
                        ),
                    });
                }
            }
        }

        Ok(coercions.into_iter().collect())
    }

    /// Index the static argument subtrees of one module's decorators.
    fn decorator_argument_index(&self, module: ModuleId) -> CompilerResult<dir::NodeParentIndex> {
        let roots = self
            .decorators
            .iter()
            .filter(|application| application.owner.module_id == module)
            .flat_map(|application| application.expression.arguments.iter().copied())
            .collect::<Vec<_>>();
        let state = self
            .modules
            .get(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} is not part of this component"),
            })?;

        Ok(dir::NodeParentIndex::from_roots(
            state.source_tree(),
            &roots,
        ))
    }
}

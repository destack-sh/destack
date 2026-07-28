use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, Constraint, ConstraintState, Relation};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return the resolved coercions produced by completed value checks in one module.
    pub(in crate::check) fn resolved_coercions(
        &mut self,
        module: ModuleId,
        failed_applications: &FxIndexSet<dir::GlobalTypeId>,
        resolved_types: &mut FxIndexMap<dir::GlobalTypeId, Option<dir::GlobalTypeId>>,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::Coercion)>> {
        let mut collected = Vec::new();
        for (id, constraint) in self.solver.constraints.iter() {
            if self.solver.constraints.state(id)? != ConstraintState::Holds {
                continue;
            }
            let Constraint::Value(constraint) = constraint else {
                continue;
            };
            if constraint.relation != Relation::Assignable
                || !constraint.use_.requires_runtime_coercion()
            {
                continue;
            }
            let Some(coercion) = self.solver.constraints.coercion(id)? else {
                continue;
            };

            if constraint.node.module_id != module {
                continue;
            }
            collected.push((constraint.node, coercion.clone()));
        }

        let mut coercions = FxIndexMap::default();
        for (node, mut coercion) in collected {
            let mut ids = FxIndexSet::default();

            // collect and resolve the coercion before changing one embedded id
            coercion.map_type_ids(&mut |id| {
                ids.insert(id);

                id
            });
            let replacements = self.resolve_type_ids(ids, failed_applications, resolved_types)?;
            coercion.map_type_ids(&mut |id| replacements[&id]);
            // require one exact coercion for each value occurrence
            match coercions.get(&node) {
                None => {
                    coercions.insert(node, coercion);
                }
                Some(previous) if previous == &coercion => {}
                Some(previous) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "node {node:?} has conflicting implicit coercions: {previous:?} and {coercion:?}"
                        ),
                    });
                }
            }
        }

        Ok(coercions.into_iter().collect())
    }
}

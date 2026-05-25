use destack_dir as dir;

use crate::CompilerResult;
use crate::check::solve::Decision;
use crate::check::{CheckComponentState, Constraint, ConstraintOrigin, TypeRelation};

impl CheckComponentState<'_> {
    /// Commit accepted type changes into the checked coercion table.
    pub(super) fn commit_coercion_table(&mut self) -> CompilerResult<()> {
        let constraints = self.collect_constraints();

        // write coercions for accepted value type changes
        for constraint in constraints {
            let Constraint::RelateType {
                relation,
                left,
                right,
                origin: ConstraintOrigin::Node(node),
            } = constraint
            else {
                continue;
            };

            let origin = match relation {
                TypeRelation::Assignable => dir::CastOrigin::Implicit,
                TypeRelation::Castable => dir::CastOrigin::Explicit,
                TypeRelation::Equal
                | TypeRelation::Satisfies
                | TypeRelation::Extends
                | TypeRelation::Implements => continue,
            };
            if self.decide_type_relation(relation, left, right)? != Decision::Yes {
                continue;
            }

            let environment = self.environment.clone();
            let check_module = self.module_mut(node.module_id)?;
            let Some(source) = check_module.commit_variable_type(environment.as_ref(), left) else {
                continue;
            };
            let Some(target) = check_module.commit_variable_type(environment.as_ref(), right)
            else {
                continue;
            };
            if source == target {
                continue;
            }

            check_module
                .output
                .coercions
                .set_coercion(node, dir::Coercion::new(source, target, origin));
        }

        Ok(())
    }
}

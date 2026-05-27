use destack_dir as dir;

use crate::CompilerResult;
use crate::check::solve::Decision;
use crate::check::{CheckState, Constraint, ConstraintOrigin, TypeOperand, TypeRelation};

impl CheckState<'_> {
    /// Commit accepted type changes into the checked coercion table.
    pub(super) fn commit_coercion_table(&mut self) -> CompilerResult<()> {
        let constraints = self.collect_constraints();

        // write coercions for accepted value type changes
        for constraint in constraints {
            let Constraint::Type {
                relation,
                left,
                right,
                origin: ConstraintOrigin::Node(node),
                condition: _,
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
            let (TypeOperand::Variable(left), TypeOperand::Variable(right)) = (left, right) else {
                continue;
            };

            let environment = self.environment.clone();
            let Some(source) = self.commit_variable_type(environment.as_ref(), left) else {
                continue;
            };
            let Some(target) = self.commit_variable_type(environment.as_ref(), right) else {
                continue;
            };
            if source == target {
                continue;
            }

            self.output_mut(node.module_id)
                .coercions
                .set_coercion(node, dir::Coercion::new(source, target, origin));
        }

        Ok(())
    }
}

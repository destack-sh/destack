use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckState, Condition, Constraint, Decision, Origin, TypeOperand, TypeRelation,
};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit accepted type changes into the checked coercion table.
    pub(super) fn commit_coercion_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<()> {
        let constraints = self
            .variables
            .constraints
            .iter()
            .filter_map(|constraint| {
                let Constraint::Type {
                    relation,
                    left,
                    right,
                    origin: Origin::Node(node),
                    condition,
                } = constraint
                else {
                    return None;
                };
                if node.module_id != module {
                    return None;
                }

                Some((*relation, *left, *right, *node, condition.clone()))
            })
            .collect::<Vec<(
                TypeRelation,
                TypeOperand,
                TypeOperand,
                dir::GlobalNodeIdAny,
                Condition,
            )>>();

        // write coercions for accepted value type changes
        for (relation, left, right, node, condition) in constraints {
            // skip inactive and parametric coercions
            if self.reduce_condition_decision(&condition)? != Decision::Yes {
                continue;
            }

            // keep only relations that introduce runtime casts
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

            let source_node = node.local_id;
            let Some(source) =
                self.commit_type_operand(module, output, environment, left, source_node)
            else {
                continue;
            };
            let Some(target) =
                self.commit_type_operand(module, output, environment, right, source_node)
            else {
                continue;
            };
            if source == target {
                continue;
            }

            output
                .coercions
                .set_coercion(node, dir::Coercion::new(source, target, origin));
        }

        Ok(())
    }
}

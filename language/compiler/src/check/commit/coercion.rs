use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, Condition, Constraint, Origin, TypeOperand, TypeRelation};
use crate::{CompilerError, CompilerResult};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit checked type coercions into one DIR coercion segment.
    pub(super) fn commit_coercion_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<dir::CoercionSegment> {
        let mut table = dir::CoercionSegment::new(module);
        let constraints = self.inference.constraints().cloned().collect::<Vec<_>>();

        // record solved coercions at value boundaries
        for constraint in constraints {
            let Constraint::Type {
                relation,
                left,
                right,
                origin: Origin::Node(source),
                condition: Condition::Always,
                coercion,
            } = constraint
            else {
                continue;
            };
            let Some(origin) = coercion else {
                continue;
            };
            if source.module_id != module || source.local_id.ty != dir::NodeType::Expression {
                continue;
            }
            let Some(coercion) = self.relation_coercion(
                module,
                output,
                environment,
                relation,
                left,
                right,
                source,
                origin,
            )?
            else {
                continue;
            };

            if let Some(existing) = table.bind_coercion(source, coercion)
                && existing != coercion
            {
                let source = self.dump_in_module(module, &source);

                return Err(CompilerError::Internal {
                    message: format!(
                        "coercion source {source} received {existing:?} and {coercion:?}"
                    ),
                });
            }
        }

        Ok(table)
    }

    /// Return the coercion represented by one solved type relation.
    fn relation_coercion(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        relation: TypeRelation,
        source_type: impl Into<TypeOperand>,
        target_type: impl Into<TypeOperand>,
        source: dir::GlobalNodeIdAny,
        origin: dir::CastOrigin,
    ) -> CompilerResult<Option<dir::Coercion>> {
        if !matches!(
            relation,
            TypeRelation::Equal | TypeRelation::Assignable | TypeRelation::Castable
        ) {
            return Ok(None);
        }

        let source_type = self.commit_type_operand(
            module,
            output,
            environment,
            source_type.into(),
            source.local_id,
        )?;
        let target_type = self.commit_type_operand(
            module,
            output,
            environment,
            target_type.into(),
            source.local_id,
        )?;
        let (Some(source_type), Some(target_type)) = (source_type, target_type) else {
            return Ok(None);
        };
        if source_type == target_type {
            return Ok(None);
        }

        Ok(Some(dir::Coercion::new(source_type, target_type, origin)))
    }
}

use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::CheckState;

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit checked annotation invocations into one DIR annotation segment.
    pub(super) fn commit_annotation_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<dir::AnnotationSegment> {
        let attachments = {
            let state = self.module(module);
            let view = state.view();
            let mut attachments = Vec::new();

            // collect visible decorator attachments in owner order
            for (owner_id, decorator_ids) in view.get_all_decorators() {
                let owner = dir::LocalNodeIdAny::new(owner_id, view.get_node_type(owner_id));

                for decorator_id in decorator_ids {
                    attachments.push((owner, decorator_id));
                }
            }

            attachments
        };
        let mut annotations = dir::AnnotationSegment::new(module);

        // write checked annotation invocations
        for (owner, decorator_id) in attachments {
            let invocation = self.decorator_invocation(module, decorator_id);
            let arguments = invocation
                .arguments
                .iter()
                .map(|argument| {
                    self.commit_annotation_argument(module, output, environment, *argument)
                })
                .collect::<CompilerResult<Vec<_>>>()?;

            annotations.insert_invocation(dir::AnnotationInvocation {
                source: decorator_id.into_any().into_global(module),
                owner: owner.into_global(module),
                target: invocation.target.into_any().into_global(module),
                resolution: self.decorator_target(module, &invocation),
                arguments,
            });
        }

        Ok(annotations)
    }

    /// Commit one annotation argument.
    fn commit_annotation_argument(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        argument: dir::LocalNodeId<dir::Argument>,
    ) -> CompilerResult<dir::AnnotationArgument> {
        let source = argument.into_any().into_global(module);
        let Some(value) = self.module(module).view().get(argument).value() else {
            return Ok(dir::AnnotationArgument {
                source,
                value: None,
            });
        };
        let value = self.commit_annotation_argument_value(module, output, environment, value)?;

        Ok(dir::AnnotationArgument { source, value })
    }

    /// Commit one annotation argument value.
    fn commit_annotation_argument_value(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::GlobalStaticId>> {
        let node = value.into_global_any(module);

        // use an existing static operand
        if let Some(operand) = self.inputs.node_static(node) {
            return Ok(self.commit_static_operand(module, output, environment, operand));
        }

        // commit locally concrete literal values
        let Some(term) = self.static_expression_literal(module, value)? else {
            return Ok(None);
        };
        let value = self.intern_static(module, output, term).into_global(module);

        Ok(Some(value))
    }
}

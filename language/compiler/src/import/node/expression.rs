use destack_dir as dir;

use crate::import::state::ImportState;
use crate::{Compiler, CompilerResult};

impl Compiler {
    /// Import dependency edges declared by one expression.
    pub(in crate::import) fn collect_expression_dependencies(
        &self,
        state: &mut ImportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> CompilerResult<()> {
        match expression {
            // collect direct dependency edge
            dir::Expression::Import {
                target,
                items,
                attributes,
                ..
            } => {
                self.collect_binding_dependency(
                    state,
                    expression_id,
                    *target,
                    items.as_deref(),
                    attributes.as_ref(),
                    dir::DependencyRelation::Import,
                )?;
            }

            // collect re-export dependency edge
            dir::Expression::Export {
                target: Some(target),
                attributes,
                ..
            } => {
                self.collect_dependency(
                    state,
                    expression_id,
                    *target,
                    attributes.as_ref(),
                    dir::DependencyRelation::ReExport,
                )?;
            }

            // ignore expressions without dependency declarations
            _ => {}
        }

        Ok(())
    }
}

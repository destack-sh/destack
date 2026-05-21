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
            // scan dependency declarations inside global blocks
            dir::Expression::Declaration(declaration_id) => {
                let declaration = state.view.get(*declaration_id);
                if let dir::Declaration::Global(declaration) = declaration {
                    self.collect_dependencies(state, &declaration.expressions)?;
                }
            }

            // collect direct dependency edge
            dir::Expression::Import {
                target, attributes, ..
            } => {
                self.collect_dependency(
                    state,
                    expression_id,
                    *target,
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

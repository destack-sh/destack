use destack_dir as dir;

use crate::import::state::ImportState;
use crate::{Compiler, CompilerResult};

impl Compiler {
    /// Import module edges declared by one expression.
    pub(in crate::import) fn collect_expression_modules(
        &self,
        state: &mut ImportState<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> CompilerResult<()> {
        match expression {
            // scan module declarations inside global blocks
            dir::Expression::Declaration(declaration_id) => {
                let declaration = state.view.get(*declaration_id);
                if let dir::Declaration::Global(declaration) = declaration {
                    self.collect_modules(state, &declaration.expressions)?;
                }
            }

            // collect direct import edge
            dir::Expression::Import {
                target, attributes, ..
            } => {
                self.collect_module(
                    state,
                    expression_id,
                    *target,
                    attributes.as_ref(),
                    dir::ModuleRelation::Import,
                )?;
            }

            // collect re-export edge
            dir::Expression::Export {
                target: Some(target),
                attributes,
                ..
            } => {
                self.collect_module(
                    state,
                    expression_id,
                    *target,
                    attributes.as_ref(),
                    dir::ModuleRelation::ReExport,
                )?;
            }

            // ignore expressions without module declarations
            _ => {}
        }

        Ok(())
    }
}

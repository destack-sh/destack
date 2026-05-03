use crate::{CompilerError, CompilerResult, LowerError};
use destack_dir as dir;

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower a root expression.
    pub(crate) fn lower_root_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // read the root expression
        let expression = self.dir_tree.get(expression_id);

        // route the root expression by kind
        match expression {
            dir::Expression::Declaration(declaration) => {
                // lower declaration roots
                let declaration_id = *declaration;
                let declaration = self.dir_tree.get(declaration_id);
                self.lower_declaration(declaration_id, declaration)
            }
            dir::Expression::Let {
                mutability,
                declarators,
                ..
            } => {
                // lower module let bindings
                self.lower_module_let(expression_id, *mutability, declarators)
            }
            _ => {
                // reject unsupported root expressions
                Err(CompilerError::from(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: format!("unsupported root expression `{}`", expression.kind_name()),
                }))?
            }
        }
    }
}

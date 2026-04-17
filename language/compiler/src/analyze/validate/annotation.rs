use crate::analyze::common::TypeContext;
use crate::{AnalyzeError, Compiler};
use destack_dir::{
    Decorator, Expression, LocalNodeId, NodeTree, NodeVisitor, NodeVisitorOptions, walk_expression,
};

impl Compiler {
    /// Validate a single decorator node.
    pub(super) fn validate_annotation(
        &self,
        ctx: &TypeContext<'_>,
        annotation_id: LocalNodeId<Decorator>,
        annotation: &Decorator,
    ) {
        // destack decorators allow static arguments
        if ctx.module.language_type.is_destack() {
            return;
        }

        if self.decorator_expression_has_static_arguments(ctx.tree, annotation.expression) {
            let node = annotation_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidDecoratorStaticArguments { node });
        }
    }

    /// Return true when a decorator expression contains static arguments.
    fn decorator_expression_has_static_arguments(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let mut scanner = DecoratorStaticArgumentScanner::default();
        let expression = tree.get(expression_id);
        scanner.visit_expression(tree, expression_id, expression);
        scanner.found
    }
}

#[derive(Default)]
struct DecoratorStaticArgumentScanner {
    options: NodeVisitorOptions,
    found: bool,
}

impl DecoratorStaticArgumentScanner {
    /// Return true when the expression carries static arguments.
    fn expression_has_static_arguments(expression: &Expression) -> bool {
        match expression {
            Expression::Instantiation {
                generic_arguments, ..
            } => !generic_arguments.is_empty(),
            Expression::Call {
                generic_arguments, ..
            }
            | Expression::New {
                generic_arguments, ..
            }
            | Expression::Member {
                generic_arguments, ..
            }
            | Expression::PrivateMember {
                generic_arguments, ..
            }
            | Expression::UnresolvedPath {
                generic_arguments, ..
            }
            | Expression::LocalReference {
                generic_arguments, ..
            }
            | Expression::ModuleReference {
                generic_arguments, ..
            }
            | Expression::GlobalReference {
                generic_arguments, ..
            } => !generic_arguments.is_empty(),
            _ => false,
        }
    }
}

impl NodeVisitor for DecoratorStaticArgumentScanner {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        if self.found {
            return;
        }
        if Self::expression_has_static_arguments(expression) {
            self.found = true;
            return;
        }
        walk_expression(self, tree, id, expression);
    }
}

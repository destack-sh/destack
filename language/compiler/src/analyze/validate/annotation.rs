use crate::{AnalyzeError, Compiler};
use destack_dir::{
    Annotation, Expression, LocalNodeId, NodeTree, NodeVisitor, NodeVisitorOptions, walk_expression,
};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Validate a single annotation node.
    pub(super) fn validate_annotation(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        annotation_id: LocalNodeId<Annotation>,
        annotation: &Annotation,
    ) {
        let Annotation::Decorator { expression, .. } = annotation else {
            return;
        };

        // destack decorators allow static arguments
        if module.language_type.is_destack() {
            return;
        }

        if self.decorator_expression_has_static_arguments(tree, *expression) {
            let node = annotation_id
                .into_global_any(module.id)
                .into_anchored(Some(profile));
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
            Expression::Instantiation { .. } => true,
            Expression::Call {
                static_arguments, ..
            }
            | Expression::New {
                static_arguments, ..
            }
            | Expression::Member {
                static_arguments, ..
            }
            | Expression::PrivateMember {
                static_arguments, ..
            }
            | Expression::UnresolvedPath {
                static_arguments, ..
            }
            | Expression::LocalReference {
                static_arguments, ..
            }
            | Expression::ModuleReference {
                static_arguments, ..
            }
            | Expression::GlobalReference {
                static_arguments, ..
            }
            | Expression::TypeImport {
                static_arguments, ..
            } => static_arguments
                .as_ref()
                .is_some_and(|args| !args.is_empty()),
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

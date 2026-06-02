use crate::Parser;
use destack_dir::{Declaration, Expression, FunctionDeclaration, FunctionForm, LocalNodeId};

impl Parser {
    /// Unwrap label wrappers to get the underlying expression.
    pub(crate) fn unwrap_label_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let mut current = expression_id;
        while let Expression::Label { body, .. } = self.tree.get(current) {
            current = *body;
        }

        current
    }

    /// Return true when an expression is a lambda declaration without wrapping parentheses.
    pub(crate) fn is_unparenthesized_lambda_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        matches!(
            self.tree.get(expression_id),
            Expression::Declaration(declaration_id)
                if matches!(
                    self.tree.get(*declaration_id),
                    Declaration::Function(FunctionDeclaration { signature, .. })
                        if signature.form == FunctionForm::Lambda
                )
        )
    }

    /// Return true when an expression can be used as an unparenthesized tagged template tag.
    pub(super) fn tagged_template_tag_is_valid(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        !self.is_unparenthesized_lambda_expression(expression_id)
            && !matches!(self.tree.get(expression_id), Expression::Unary { .. })
    }
}

use tspp_dir as dir;

use crate::sema::CheckState;

impl CheckState<'_> {
    /// Return whether one block can complete normally.
    ///
    /// Example:
    /// ```tspp
    /// {
    ///     const value = 1;
    ///     value
    /// }
    /// ```
    pub(in crate::sema) fn block_can_complete_normally(
        &self,
        block: dir::LocalNodeId<dir::Block>,
    ) -> bool {
        let view = self.module(self.module_id).view();
        let block = view.get(block);
        for expression in &block.leading_expressions {
            if !self.expression_can_complete_normally(*expression) {
                return false;
            }
        }

        match block.tail_expression {
            Some(expression) => self.expression_can_complete_normally(expression),
            None => true,
        }
    }

    /// Return whether one expression can complete normally.
    ///
    /// Example:
    /// ```tspp
    /// if (condition) { value } else { return }
    /// ```
    pub(in crate::sema) fn expression_can_complete_normally(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let view = self.module(self.module_id).view();
        match view.get(id) {
            // unbound jumps already emitted diagnostics
            dir::Expression::Break { .. } | dir::Expression::Continue { .. }
                if self.flow.is_unbound_jump(id.into_any()) =>
            {
                true
            }
            // return, break, continue
            dir::Expression::Return { .. }
            | dir::Expression::Break { .. }
            | dir::Expression::Continue { .. } => false,
            // value?.member
            dir::Expression::Chain { expression } => {
                self.expression_can_complete_normally(*expression)
            }
            // { ... }
            dir::Expression::Block(block) => self.block_can_complete_normally(*block),
            // if condition { then } else { otherwise }
            dir::Expression::If {
                then_expression,
                else_expression: Some(else_expression),
                ..
            } => {
                self.expression_can_complete_normally(*then_expression)
                    || self.expression_can_complete_normally(*else_expression)
            }
            // if condition { then }
            dir::Expression::If {
                else_expression: None,
                ..
            } => true,
            // match (value) { pattern => body }
            dir::Expression::Match { arms, .. } => arms
                .iter()
                .any(|arm| self.match_arm_can_complete_normally(view.get(*arm))),
            // switch (value) { case pattern: body }
            dir::Expression::Switch { .. } => !self
                .module(self.module_id)
                .unreachable_ends
                .contains(&id.into_any()),
            // try { value } catch (error) { recover(error) }
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => {
                let body = self.expression_can_complete_normally(*body);
                let catch = catch.is_some_and(|catch| {
                    self.expression_can_complete_normally(view.get(catch).body)
                });
                let finally = if let Some(finally) = finally {
                    self.expression_can_complete_normally(*finally)
                } else {
                    true
                };

                finally && (body || catch)
            }
            // expressions completing normally by form
            dir::Expression::Declaration(_)
            | dir::Expression::Import { .. }
            | dir::Expression::Export { .. }
            | dir::Expression::Let { .. }
            | dir::Expression::LetElse { .. }
            | dir::Expression::Using { .. }
            | dir::Expression::While { .. }
            | dir::Expression::ForEach { .. }
            | dir::Expression::For { .. }
            | dir::Expression::Loop { .. }
            | dir::Expression::Await { .. }
            | dir::Expression::Yield { .. }
            | dir::Expression::Identifier { .. }
            | dir::Expression::This
            | dir::Expression::Literal(_)
            | dir::Expression::Super
            | dir::Expression::ImportMeta
            | dir::Expression::ImportSource
            | dir::Expression::Debugger
            | dir::Expression::Missing
            | dir::Expression::Error
            | dir::Expression::Infer { .. }
            | dir::Expression::RangeExpression { .. }
            | dir::Expression::TemplateExpression { .. }
            | dir::Expression::TaggedTemplateExpression { .. }
            | dir::Expression::ArrayExpression { .. }
            | dir::Expression::FixedArrayExpression { .. }
            | dir::Expression::TupleExpression { .. }
            | dir::Expression::ObjectExpression { .. }
            | dir::Expression::StructExpression { .. }
            | dir::Expression::TreeExpression { .. }
            | dir::Expression::Type { .. }
            | dir::Expression::Const { .. }
            | dir::Expression::As { .. }
            | dir::Expression::Satisfies { .. }
            | dir::Expression::Is { .. }
            | dir::Expression::InstanceOf { .. }
            | dir::Expression::Unary { .. }
            | dir::Expression::BorrowOf { .. }
            | dir::Expression::Member { .. }
            | dir::Expression::Index { .. }
            | dir::Expression::Instantiation { .. }
            | dir::Expression::Call { .. }
            | dir::Expression::New { .. }
            | dir::Expression::AwaitMaybe { .. }
            | dir::Expression::AwaitMust { .. }
            | dir::Expression::Maybe { .. }
            | dir::Expression::Must { .. }
            | dir::Expression::Binary { .. }
            | dir::Expression::Assign { .. } => true,
        }
    }

    /// Return whether one match arm can complete normally.
    ///
    /// Example:
    /// ```tspp
    /// match (value) {
    ///     0 => "zero",
    ///     _ => "other",
    /// }
    /// ```
    pub(in crate::sema) fn match_arm_can_complete_normally(&self, arm: &dir::MatchArm) -> bool {
        match arm {
            // pattern => expression
            dir::MatchArm::Expression { body, .. } => self.expression_can_complete_normally(*body),
            // pattern => { ... }
            dir::MatchArm::Block { body, .. } => self.block_can_complete_normally(*body),
        }
    }
}

use destack_dir as dir;

use crate::check::WalkState;

impl WalkState<'_, '_> {
    /// Return whether one block can complete normally.
    ///
    /// Example:
    /// ```ds
    /// {
    ///     const value = 1;
    ///     value
    /// }
    /// ```
    pub(in crate::check) fn block_can_complete_normally(&self, block: &dir::Block) -> bool {
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
    /// ```ds
    /// if (condition) { value } else { fallback }
    /// ```
    pub(in crate::check) fn expression_can_complete_normally(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        match self.tree.get(id) {
            // recovered targetless jumps complete as plain statements
            dir::Expression::Break { .. } | dir::Expression::Continue { .. }
                if self.flow().is_unbound_jump(id.into_any()) =>
            {
                true
            }
            // return, break, continue, throw
            dir::Expression::Return { .. }
            | dir::Expression::Break { .. }
            | dir::Expression::Continue { .. }
            | dir::Expression::Throw { .. } => false,
            // { ... }
            dir::Expression::Block(block) => {
                self.block_can_complete_normally(self.tree.get(*block))
            }
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
            dir::Expression::Match { cases, .. } => cases
                .iter()
                .any(|case| self.match_case_can_complete_normally(self.tree.get(*case))),
            // try { value } catch (error) { recover(error) }
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => {
                let body = self.expression_can_complete_normally(*body);
                let catch = catch.is_some_and(|catch| {
                    self.expression_can_complete_normally(self.tree.get(catch).body)
                });
                let finally = if let Some(finally) = finally {
                    self.expression_can_complete_normally(*finally)
                } else {
                    true
                };

                finally && (body || catch)
            }
            // expressions that do not force control transfer by syntax
            dir::Expression::Declaration(_)
            | dir::Expression::Label { .. }
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
            | dir::Expression::ScalarLiteral(_)
            | dir::Expression::PrivateIdentifier { .. }
            | dir::Expression::Super
            | dir::Expression::ImportMeta
            | dir::Expression::Debugger
            | dir::Expression::Missing
            | dir::Expression::Stub
            | dir::Expression::Error
            | dir::Expression::QualifiedReference { .. }
            | dir::Expression::RangeExpression { .. }
            | dir::Expression::TemplateExpression { .. }
            | dir::Expression::TaggedTemplateExpression { .. }
            | dir::Expression::ArrayExpression { .. }
            | dir::Expression::FixedArrayExpression { .. }
            | dir::Expression::TupleExpression { .. }
            | dir::Expression::SequenceExpression { .. }
            | dir::Expression::ObjectExpression { .. }
            | dir::Expression::StructExpression { .. }
            | dir::Expression::TreeExpression { .. }
            | dir::Expression::Parenthesized { .. }
            | dir::Expression::Type { .. }
            | dir::Expression::Comptime { .. }
            | dir::Expression::As { .. }
            | dir::Expression::Satisfies { .. }
            | dir::Expression::Is { .. }
            | dir::Expression::InstanceOf { .. }
            | dir::Expression::Unary { .. }
            | dir::Expression::MoveOf { .. }
            | dir::Expression::BorrowOf { .. }
            | dir::Expression::Member { .. }
            | dir::Expression::PrivateMember { .. }
            | dir::Expression::Index { .. }
            | dir::Expression::Instantiation { .. }
            | dir::Expression::Call { .. }
            | dir::Expression::New { .. }
            | dir::Expression::NewMaybe { .. }
            | dir::Expression::AwaitMaybe { .. }
            | dir::Expression::AwaitMust { .. }
            | dir::Expression::Maybe { .. }
            | dir::Expression::Must { .. }
            | dir::Expression::Binary { .. }
            | dir::Expression::Assign { .. } => true,
        }
    }

    /// Return whether one match case can complete normally.
    ///
    /// Example:
    /// ```ds
    /// match (value) {
    ///     0 => "zero",
    ///     _ => "other",
    /// }
    /// ```
    pub(in crate::check) fn match_case_can_complete_normally(&self, case: &dir::MatchCase) -> bool {
        match case {
            // pattern => expression
            dir::MatchCase::Expression { body, .. } => self.expression_can_complete_normally(*body),
            // pattern => { ... }
            dir::MatchCase::Block { body, .. } => {
                self.block_can_complete_normally(self.tree.get(*body))
            }
        }
    }
}

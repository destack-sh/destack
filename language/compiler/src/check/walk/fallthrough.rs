use destack_dir as dir;

use crate::check::WalkState;

impl WalkState<'_, '_> {
    /// Return whether one block can fall through normally.
    ///
    /// Example:
    /// ```ds
    /// {
    ///     return value;
    /// }
    /// ```
    pub(in crate::check) fn can_block_fall_through(
        &self,
        tree: &dir::Tree,
        block: &dir::Block,
    ) -> bool {
        for expression in &block.leading_expressions {
            if !self.can_expression_fall_through(tree, *expression) {
                return false;
            }
        }

        match block.tail_expression {
            Some(expression) => self.can_expression_fall_through(tree, expression),
            None => true,
        }
    }

    /// Return whether one expression can fall through normally.
    ///
    /// Example:
    /// ```ds
    /// if condition { return value; }
    /// ```
    pub(in crate::check) fn can_expression_fall_through(
        &self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        match tree.get(id) {
            // return, break, continue, throw
            dir::Expression::Return { .. }
            | dir::Expression::Break { .. }
            | dir::Expression::Continue { .. }
            | dir::Expression::Throw { .. } => false,
            // { ... }
            dir::Expression::Block(block) => self.can_block_fall_through(tree, tree.get(*block)),
            // if condition { then } else { otherwise }
            dir::Expression::If {
                then_expression,
                else_expression: Some(else_expression),
                ..
            } => {
                self.can_expression_fall_through(tree, *then_expression)
                    || self.can_expression_fall_through(tree, *else_expression)
            }
            // if condition { then }
            dir::Expression::If {
                else_expression: None,
                ..
            } => true,
            // match value { case pattern => body }
            dir::Expression::Match { cases, .. } => cases
                .iter()
                .any(|case| self.can_match_case_fall_through(tree, tree.get(*case))),
            // try body catch error finally cleanup
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => {
                let body = self.can_expression_fall_through(tree, *body);
                let catch = if let Some(catch) = catch {
                    self.can_catch_fall_through(tree, tree.get(*catch))
                } else {
                    true
                };
                let finally = if let Some(finally) = finally {
                    self.can_expression_fall_through(tree, *finally)
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

    /// Return whether one match case can fall through normally.
    ///
    /// Example:
    /// ```ds
    /// case value => return value
    /// ```
    pub(in crate::check) fn can_match_case_fall_through(
        &self,
        tree: &dir::Tree,
        case: &dir::MatchCase,
    ) -> bool {
        match case {
            // case pattern => expression
            dir::MatchCase::Expression { body, .. } => {
                self.can_expression_fall_through(tree, *body)
            }
            // case pattern => { ... }
            dir::MatchCase::Block { body, .. } => {
                self.can_block_fall_through(tree, tree.get(*body))
            }
        }
    }

    /// Return whether one catch body can fall through normally.
    ///
    /// Example:
    /// ```ds
    /// catch error => recover(error)
    /// ```
    pub(in crate::check) fn can_catch_fall_through(
        &self,
        tree: &dir::Tree,
        catch: &dir::Catch,
    ) -> bool {
        self.can_expression_fall_through(tree, catch.body)
    }
}

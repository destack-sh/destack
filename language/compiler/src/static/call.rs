use destack_dir as dir;

use super::{StaticContext, StaticError};

impl StaticContext<'_> {
    /// Evaluate one `array.includes(value)` membership call, the only static call form.
    pub(super) fn evaluate_call(
        &self,
        callee: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Result<dir::StaticTerm, StaticError> {
        let Some((receiver, value)) = self.array_includes_call(callee, arguments) else {
            return Err(StaticError::NotStatic(callee));
        };

        // test the value against an array receiver by structural equality
        let dir::StaticTerm::Array { elements } = self.evaluate_expression(receiver)? else {
            return Err(StaticError::NotStatic(receiver));
        };
        let value = self.evaluate_expression(value)?;

        Ok(dir::ScalarLiteral::Boolean(elements.contains(&value)).into())
    }

    /// Match one `array.includes(value)` call into its receiver and value expressions.
    fn array_includes_call(
        &self,
        callee: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Option<(
        dir::LocalNodeId<dir::Expression>,
        dir::LocalNodeId<dir::Expression>,
    )> {
        // the callee is a `receiver.includes` member access
        let dir::Expression::Member {
            left: receiver,
            name: Some(name),
        } = self.view.get(callee)
        else {
            return None;
        };
        if self.strings.get(*name) != "includes" {
            return None;
        }

        // the sole argument is one positional value
        let [argument] = arguments else {
            return None;
        };
        let dir::Argument::Positional { value } = self.view.get(*argument) else {
            return None;
        };

        Some((*receiver, *value))
    }
}

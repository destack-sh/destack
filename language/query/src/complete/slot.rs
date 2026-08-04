use destack_dir as dir;
use destack_source::FileId;

use crate::{ModuleQueryContext, QueryResult};

/// The structural owner for one expression slot.
#[derive(Debug, Clone, Copy)]
enum ExpressionSlotOwner {
    /// The slot is the value side of one declarator.
    DeclaratorValue(dir::LocalNodeId<dir::Declarator>),
    /// The slot expects a value expression.
    Value,
}

impl ExpressionSlotOwner {
    /// Return whether a parameter owns this expression child.
    fn parameter_owns(
        parameter: &dir::Parameter,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        match parameter {
            dir::Parameter::Named { default, .. } | dir::Parameter::Pattern { default, .. } => {
                *default == Some(expression_id)
            }
            dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. } => false,
            dir::Parameter::Error => false,
        }
    }

    /// Return whether an argument owns this expression child.
    fn argument_owns(
        argument: &dir::Argument,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        match argument {
            dir::Argument::Named { value, .. }
            | dir::Argument::Labeled { value, .. }
            | dir::Argument::Positional { value, .. }
            | dir::Argument::Spread { value, .. } => *value == expression_id,
            dir::Argument::Elision | dir::Argument::Error => false,
        }
    }

    /// Return whether a property owns this expression child.
    fn property_owns(
        property: &dir::Property,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        match property {
            dir::Property::Field { value, .. } => *value == expression_id,
            dir::Property::Method { body, .. } => *body == Some(expression_id),
            dir::Property::Spread { value, .. } => *value == expression_id,
            dir::Property::Error => false,
        }
    }

    /// Return whether a member owns this expression child.
    fn member_owns(member: &dir::Member, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        match member {
            dir::Member::AssociatedType { .. } => false,
            dir::Member::AssociatedConst { value, .. } => *value == Some(expression_id),
            dir::Member::Field { default, .. } => *default == Some(expression_id),
            dir::Member::Method { body, .. } => *body == Some(expression_id),
            dir::Member::StaticBlock { body, .. } | dir::Member::ComptimeBlock { body, .. } => {
                *body == expression_id
            }
            dir::Member::Error => false,
        }
    }

    /// Return whether an expression owns this expression child.
    fn expression_owns(
        view: dir::View<'_>,
        expression: &dir::Expression,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        match expression {
            dir::Expression::ObjectExpression { .. } => false,
            dir::Expression::Comptime { body: expression }
            | dir::Expression::Await { expression }
            | dir::Expression::Unary {
                right: expression, ..
            }
            | dir::Expression::BorrowOf {
                right: expression, ..
            } => *expression == expression_id,
            dir::Expression::Binary { left, right, .. } => {
                *left == expression_id || *right == expression_id
            }
            dir::Expression::Assign { left, right, .. } => {
                Self::assign_pattern_contains_expression(view, *left, expression_id)
                    || *right == expression_id
            }
            dir::Expression::Return { value } | dir::Expression::Yield { value, .. } => {
                *value == Some(expression_id)
            }
            dir::Expression::Index { left, index, .. } => {
                *left == expression_id || *index == Some(expression_id)
            }
            dir::Expression::Member { left, .. }
            | dir::Expression::Instantiation { left, .. }
            | dir::Expression::Call { left, .. } => *left == expression_id,
            dir::Expression::As { expression, .. }
            | dir::Expression::Satisfies { expression, .. } => *expression == expression_id,
            _ => false,
        }
    }

    /// Return whether one assign pattern contains the expression.
    fn assign_pattern_contains_expression(
        view: dir::View<'_>,
        assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let assign_pattern = view.get(assign_pattern_id);

        match assign_pattern {
            dir::AssignPattern::Place { expression: value } => *value == expression_id,
            dir::AssignPattern::Default { pattern, value } => {
                Self::assign_pattern_contains_expression(view, *pattern, expression_id)
                    || *value == expression_id
            }
            dir::AssignPattern::Sequence { fields }
            | dir::AssignPattern::Tuple { fields }
            | dir::AssignPattern::Object { fields } => fields.iter().any(|field_id| {
                Self::assign_pattern_field_contains_expression(view, *field_id, expression_id)
            }),
        }
    }

    /// Return whether one assign pattern field contains the expression.
    fn assign_pattern_field_contains_expression(
        view: dir::View<'_>,
        assign_pattern_field_id: dir::LocalNodeId<dir::AssignPatternField>,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let assign_pattern_field = view.get(assign_pattern_field_id);

        match assign_pattern_field {
            dir::AssignPatternField::Named { pattern, .. } => {
                Self::assign_pattern_contains_expression(view, *pattern, expression_id)
            }
            dir::AssignPatternField::Computed { key, pattern } => {
                *key == expression_id
                    || Self::assign_pattern_contains_expression(view, *pattern, expression_id)
            }
            dir::AssignPatternField::Positional { pattern } => {
                Self::assign_pattern_contains_expression(view, *pattern, expression_id)
            }
            dir::AssignPatternField::Rest { pattern } => pattern.is_some_and(|pattern_id| {
                Self::assign_pattern_contains_expression(view, pattern_id, expression_id)
            }),
            dir::AssignPatternField::Elision => false,
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Return whether the cursor occupies a parser-authored expression slot.
    pub(super) fn is_expression_slot_at_offset(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<bool> {
        let owner = self.expression_slot_owner(file_id, offset)?;

        Ok(owner.is_some())
    }

    /// Check whether the cursor sits in a declarator initializer hole.
    pub(super) fn is_declarator_value_hole(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<bool> {
        let view = self.view();
        let Some(declarator_id) = self.current_initializer_declarator(file_id, offset)? else {
            return Ok(false);
        };

        let declarator = view.get(declarator_id);
        let Some(value_id) = declarator.value else {
            return Ok(false);
        };

        Ok(matches!(view.get(value_id), dir::Expression::Missing))
    }

    /// Resolve the structural owner for the innermost expression slot at the cursor.
    fn expression_slot_owner(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<ExpressionSlotOwner>> {
        let Some(expression) = self.expression_hole_at_offset(file_id, offset)? else {
            return Ok(None);
        };

        Ok(expression_hole_owner(self.view(), expression))
    }
}

/// Resolve the structural context for one expression hole.
fn expression_hole_owner(
    view: dir::View<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ExpressionSlotOwner> {
    let parent_id = view.get_parent_for(expression_id)?;

    match parent_id.ty {
        dir::NodeType::Declarator => {
            let declarator_id = dir::LocalNodeId::<dir::Declarator>::new(parent_id.id);
            let declarator = view.get(declarator_id);

            if declarator.value == Some(expression_id) {
                return Some(ExpressionSlotOwner::DeclaratorValue(declarator_id));
            }

            None
        }
        dir::NodeType::Parameter => {
            let parameter = view.get(dir::LocalNodeId::<dir::Parameter>::new(parent_id.id));
            ExpressionSlotOwner::parameter_owns(parameter, expression_id)
                .then_some(ExpressionSlotOwner::Value)
        }
        dir::NodeType::Argument => {
            let argument = view.get(dir::LocalNodeId::<dir::Argument>::new(parent_id.id));
            ExpressionSlotOwner::argument_owns(argument, expression_id)
                .then_some(ExpressionSlotOwner::Value)
        }
        dir::NodeType::Property => {
            let property = view.get(dir::LocalNodeId::<dir::Property>::new(parent_id.id));
            ExpressionSlotOwner::property_owns(property, expression_id)
                .then_some(ExpressionSlotOwner::Value)
        }
        dir::NodeType::Member => {
            let member = view.get(dir::LocalNodeId::<dir::Member>::new(parent_id.id));
            ExpressionSlotOwner::member_owns(member, expression_id)
                .then_some(ExpressionSlotOwner::Value)
        }
        dir::NodeType::Expression => {
            let parent = view.get(dir::LocalNodeId::<dir::Expression>::new(parent_id.id));
            ExpressionSlotOwner::expression_owns(view, parent, expression_id)
                .then_some(ExpressionSlotOwner::Value)
        }
        _ => None,
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve the declarator that owns the initializer slot at one cursor offset.
    fn current_initializer_declarator(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<dir::LocalNodeId<dir::Declarator>>> {
        let Some(owner) = self.expression_slot_owner(file_id, offset)? else {
            return Ok(None);
        };
        let ExpressionSlotOwner::DeclaratorValue(declarator_id) = owner else {
            return Ok(None);
        };

        Ok(Some(declarator_id))
    }
}

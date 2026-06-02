use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, DecoratorInvocation};

/// Representation constraints attached to one declaration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::check) struct RepresentationConstraint {
    /// The minimum aggregate alignment in bytes.
    pub(in crate::check) minimum_alignment: Option<u32>,
    /// The maximum field alignment in bytes.
    pub(in crate::check) field_alignment_limit: Option<u32>,
}

impl RepresentationConstraint {
    /// Return the field alignment after packing constraints.
    pub(in crate::check) fn field_alignment(self, alignment: u32) -> u32 {
        match self.field_alignment_limit {
            Some(limit) => alignment.min(limit).max(1),
            None => alignment.max(1),
        }
    }

    /// Return the aggregate alignment after alignment constraints.
    pub(in crate::check) fn aggregate_alignment(self, alignment: u32) -> u32 {
        match self.minimum_alignment {
            Some(minimum) => alignment.max(minimum).max(1),
            None => alignment.max(1),
        }
    }
}

impl CheckState<'_> {
    /// Return representation constraints attached to one owner node.
    ///
    /// Examples:
    /// ```ds
    /// @repr("C", { packed: true })
    /// struct Header { tag: uint8, size: uint32 }
    /// ```
    pub(in crate::check) fn representation_constraint_for_owner(
        &self,
        module: ModuleId,
        owner: dir::LocalNodeIdAny,
    ) -> RepresentationConstraint {
        let mut constraint = RepresentationConstraint::default();

        // constrain from representation annotations in source order
        for invocation in self.decorator_invocations_for_owner(module, owner) {
            if self.decorator_item(module, &invocation) == Some(dir::LanguageItem::ReprDecorator) {
                self.constrain_representation(module, &invocation, &mut constraint);
            }
        }

        constraint
    }

    /// Constrain representation from one invocation.
    fn constrain_representation(
        &self,
        module: ModuleId,
        invocation: &DecoratorInvocation,
        constraint: &mut RepresentationConstraint,
    ) {
        let view = self.module(module).view();

        // read positional and named representation arguments
        for argument in &invocation.arguments {
            match view.get(*argument) {
                dir::Argument::Named { name, value } => {
                    self.constrain_representation_option(module, name.string(), *value, constraint)
                }
                dir::Argument::Labeled { label, value } => {
                    self.constrain_representation_option(module, *label, *value, constraint)
                }
                dir::Argument::Positional { value } => {
                    self.constrain_representation_value(module, *value, constraint)
                }
                dir::Argument::Spread { .. } | dir::Argument::Error => {}
            }
        }
    }

    /// Constrain representation from one positional value.
    fn constrain_representation_value(
        &self,
        module: ModuleId,
        value: dir::LocalNodeId<dir::Expression>,
        constraint: &mut RepresentationConstraint,
    ) {
        let view = self.module(module).view();

        // ignore representation policy and enum backing arguments
        let dir::Expression::ObjectExpression { properties } = view.get(value) else {
            return;
        };

        // read object options
        for property in properties {
            let dir::Property::Field { key, value, .. } = view.get(*property) else {
                continue;
            };
            let Some(dir::StaticKey::Name(name)) = key.static_key(view.tree()) else {
                continue;
            };

            self.constrain_representation_option(module, name, *value, constraint);
        }
    }

    /// Constrain representation from one option.
    fn constrain_representation_option(
        &self,
        module: ModuleId,
        name: dir::StringId,
        value: dir::LocalNodeId<dir::Expression>,
        constraint: &mut RepresentationConstraint,
    ) {
        let name = self.module(module).strings.get(name);

        // raise aggregate alignment
        if name == "align" {
            if let Some(alignment) = self.representation_integer_expression(module, value) {
                constraint.minimum_alignment = Some(alignment);
            }
        }
        // lower field alignment
        else if name == "packed" {
            constraint.field_alignment_limit = self
                .representation_integer_expression(module, value)
                .or_else(|| {
                    self.representation_true_expression(module, value)
                        .then_some(1)
                });
        }
    }

    /// Return one positive integer representation expression.
    fn representation_integer_expression(
        &self,
        module: ModuleId,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> Option<u32> {
        let view = self.module(module).view();
        let dir::Expression::ScalarLiteral(dir::ScalarLiteral::Integer(value)) = view.get(value)
        else {
            return None;
        };

        u32::try_from(*value).ok().filter(|value| *value > 0)
    }

    /// Return whether one expression is literal `true`.
    fn representation_true_expression(
        &self,
        module: ModuleId,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let view = self.module(module).view();

        matches!(
            view.get(value),
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::Boolean(true))
        )
    }
}

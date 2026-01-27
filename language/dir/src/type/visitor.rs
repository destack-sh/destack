use crate::{
    LocalTypeId, StaticArgument, StaticExpression, StaticProperty, Type, TypeTable,
    walk_static_argument, walk_static_expression, walk_static_property, walk_type,
};

/// Options for type visitors.
#[derive(Debug, Clone, Default)]
pub struct TypeVisitorOptions {}

/// Visit the type graph.
pub trait TypeVisitor {
    /// Return the visitor options.
    fn options(&self) -> &TypeVisitorOptions;

    /// Visit any type.
    fn visit_any(&mut self, _types: &TypeTable, _id: LocalTypeId, _ty: &Type) {
        // nothing to do
    }

    /// Visit a type id.
    fn visit_type_id(&mut self, types: &TypeTable, id: LocalTypeId) {
        let ty = types.get_type(id);
        self.visit_type(types, id, ty);
    }

    /// Visit a type.
    fn visit_type(&mut self, types: &TypeTable, id: LocalTypeId, ty: &Type) {
        destack_base::ensure_sufficient_stack(|| walk_type(self, types, id, ty));
    }

    /// Visit a static argument.
    fn visit_static_argument(&mut self, types: &TypeTable, argument: &StaticArgument) {
        walk_static_argument(self, types, argument);
    }

    /// Visit a static expression.
    fn visit_static_expression(&mut self, types: &TypeTable, expression: &StaticExpression) {
        destack_base::ensure_sufficient_stack(|| walk_static_expression(self, types, expression));
    }

    /// Visit a static property.
    fn visit_static_property(&mut self, types: &TypeTable, property: &StaticProperty) {
        walk_static_property(self, types, property);
    }
}

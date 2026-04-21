use destack_ast::{
    Declaration, Declarator, Expression, FunctionSignature, GenericArgument, GenericParameter,
    LocalNodeId, Member, NodeTree, NodeVisitor, NodeVisitorOptions, Parameter, Pattern, Property,
    TupleElement, TypeExpression, TypeMappedParameter, TypeMember, WhereClause, walk_declaration,
    walk_declarator, walk_expression, walk_generic_argument, walk_generic_parameter, walk_member,
    walk_parameter, walk_pattern, walk_property, walk_tuple_element, walk_type_expression,
    walk_type_member, walk_where_clause,
};

/// Normalize formatter-visible type wrapper nodes out of one cloned tree.
pub(super) fn normalize_formatter_tree(tree: &mut NodeTree, roots: &[LocalNodeId<Expression>]) {
    let snapshot = tree.clone();
    let mut visitor = FormatterTreeNormalizer::new(tree);

    for expression_id in roots.iter().copied() {
        let expression = snapshot.get(expression_id);
        visitor.visit_expression(&snapshot, expression_id, expression);
    }
}

/// Formatter-local visitor that rewrites retained type wrappers out of one cloned tree.
struct FormatterTreeNormalizer<'a> {
    tree: &'a mut NodeTree,
    options: NodeVisitorOptions,
}

impl<'a> FormatterTreeNormalizer<'a> {
    /// Create one formatter tree normalizer.
    fn new(tree: &'a mut NodeTree) -> Self {
        Self {
            tree,
            options: NodeVisitorOptions::default(),
        }
    }
}

impl NodeVisitor for FormatterTreeNormalizer<'_> {
    /// Return the visitor options.
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    /// Normalize one declaration and keep walking the snapshot tree.
    fn visit_declaration(
        &mut self,
        snapshot: &NodeTree,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        let mut normalized = declaration.clone();
        normalize_declaration(snapshot, &mut normalized);
        *self.tree.get_mut(id) = normalized;

        walk_declaration(self, snapshot, id, declaration);
    }

    /// Normalize one declarator and keep walking the snapshot tree.
    fn visit_declarator(
        &mut self,
        snapshot: &NodeTree,
        id: LocalNodeId<Declarator>,
        declarator: &Declarator,
    ) {
        let mut normalized = declarator.clone();
        normalize_declarator(snapshot, &mut normalized);
        *self.tree.get_mut(id) = normalized;

        walk_declarator(self, snapshot, id, declarator);
    }

    /// Normalize one property and keep walking the snapshot tree.
    fn visit_property(
        &mut self,
        snapshot: &NodeTree,
        id: LocalNodeId<Property>,
        property: &Property,
    ) {
        let mut normalized = property.clone();
        normalize_property(snapshot, &mut normalized);
        *self.tree.get_mut(id) = normalized;

        walk_property(self, snapshot, id, property);
    }

    /// Normalize one type member and keep walking the snapshot tree.
    fn visit_type_member(
        &mut self,
        snapshot: &NodeTree,
        id: LocalNodeId<TypeMember>,
        type_member: &TypeMember,
    ) {
        let mut normalized = type_member.clone();
        normalize_type_member(snapshot, &mut normalized);
        *self.tree.get_mut(id) = normalized;

        walk_type_member(self, snapshot, id, type_member);
    }

    /// Normalize one member and keep walking the snapshot tree.
    fn visit_member(&mut self, snapshot: &NodeTree, id: LocalNodeId<Member>, member: &Member) {
        let mut normalized = member.clone();
        normalize_member(snapshot, &mut normalized);
        *self.tree.get_mut(id) = normalized;

        walk_member(self, snapshot, id, member);
    }

    /// Normalize one generic parameter and keep walking the snapshot tree.
    fn visit_generic_parameter(
        &mut self,
        snapshot: &NodeTree,
        id: LocalNodeId<GenericParameter>,
        generic_parameter: &GenericParameter,
    ) {
        let mut normalized = generic_parameter.clone();
        normalize_generic_parameter(snapshot, &mut normalized);
        *self.tree.get_mut(id) = normalized;

        walk_generic_parameter(self, snapshot, id, generic_parameter);
    }

    /// Normalize one parameter and keep walking the snapshot tree.
    fn visit_parameter(
        &mut self,
        snapshot: &NodeTree,
        id: LocalNodeId<Parameter>,
        parameter: &Parameter,
    ) {
        let mut normalized = parameter.clone();
        normalize_parameter(snapshot, &mut normalized);
        *self.tree.get_mut(id) = normalized;

        walk_parameter(self, snapshot, id, parameter);
    }

    /// Normalize one generic argument and keep walking the snapshot tree.
    fn visit_generic_argument(
        &mut self,
        snapshot: &NodeTree,
        id: LocalNodeId<GenericArgument>,
        generic_argument: &GenericArgument,
    ) {
        let mut normalized = generic_argument.clone();
        normalize_generic_argument(snapshot, &mut normalized);
        *self.tree.get_mut(id) = normalized;

        walk_generic_argument(self, snapshot, id, generic_argument);
    }

    /// Normalize one tuple element and keep walking the snapshot tree.
    fn visit_tuple_element(
        &mut self,
        snapshot: &NodeTree,
        id: LocalNodeId<TupleElement>,
        tuple_element: &TupleElement,
    ) {
        let mut normalized = tuple_element.clone();
        normalize_tuple_element(snapshot, &mut normalized);
        *self.tree.get_mut(id) = normalized;

        walk_tuple_element(self, snapshot, id, tuple_element);
    }

    /// Normalize one pattern and keep walking the snapshot tree.
    fn visit_pattern(&mut self, snapshot: &NodeTree, id: LocalNodeId<Pattern>, pattern: &Pattern) {
        let mut normalized = pattern.clone();
        normalize_pattern(snapshot, &mut normalized);
        *self.tree.get_mut(id) = normalized;

        walk_pattern(self, snapshot, id, pattern);
    }

    /// Normalize one expression and keep walking the snapshot tree.
    fn visit_expression(
        &mut self,
        snapshot: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        let mut normalized = expression.clone();
        normalize_expression(snapshot, &mut normalized);
        *self.tree.get_mut(id) = normalized;

        walk_expression(self, snapshot, id, expression);
    }

    /// Normalize one type expression and keep walking the snapshot tree.
    fn visit_type_expression(
        &mut self,
        snapshot: &NodeTree,
        id: LocalNodeId<TypeExpression>,
        type_expression: &TypeExpression,
    ) {
        let mut normalized = type_expression.clone();
        normalize_type_expression(snapshot, &mut normalized);
        *self.tree.get_mut(id) = normalized;

        walk_type_expression(self, snapshot, id, type_expression);
    }

    /// Normalize one where clause and keep walking the snapshot tree.
    fn visit_where_clause(
        &mut self,
        snapshot: &NodeTree,
        id: LocalNodeId<WhereClause>,
        where_clause: &WhereClause,
    ) {
        let mut normalized = where_clause.clone();
        normalize_where_clause(snapshot, &mut normalized);
        *self.tree.get_mut(id) = normalized;

        walk_where_clause(self, snapshot, id, where_clause);
    }
}

/// Return one formatter-visible type expression id without retained wrappers.
fn normalize_type_expression_id(
    tree: &NodeTree,
    mut type_id: LocalNodeId<TypeExpression>,
) -> LocalNodeId<TypeExpression> {
    loop {
        match tree.get(type_id) {
            TypeExpression::Parenthesized { expression } => {
                type_id = *expression;
            }
            _ => return type_id,
        }
    }
}

/// Normalize one optional type expression edge.
fn normalize_optional_type_expression(
    tree: &NodeTree,
    type_id: &mut Option<LocalNodeId<TypeExpression>>,
) {
    *type_id = type_id.map(|type_id| normalize_type_expression_id(tree, type_id));
}

/// Normalize one list of type expression edges.
fn normalize_type_expression_list(
    tree: &NodeTree,
    type_ids: &mut Vec<LocalNodeId<TypeExpression>>,
) {
    for type_id in type_ids {
        *type_id = normalize_type_expression_id(tree, *type_id);
    }
}

/// Normalize one function signature.
fn normalize_function_signature(tree: &NodeTree, signature: &mut FunctionSignature) {
    normalize_optional_type_expression(tree, &mut signature.return_type);
}

/// Normalize one mapped-type parameter.
fn normalize_type_mapped_parameter(tree: &NodeTree, parameter: &mut TypeMappedParameter) {
    parameter.source_type = normalize_type_expression_id(tree, parameter.source_type);

    normalize_optional_type_expression(tree, &mut parameter.key_remap);
}

/// Normalize one declaration node.
fn normalize_declaration(tree: &NodeTree, declaration: &mut Declaration) {
    match declaration {
        Declaration::Global(_) => {}
        Declaration::Namespace(_) => {}
        Declaration::Type(declaration) => {
            declaration.value = normalize_type_expression_id(tree, declaration.value);
        }
        Declaration::ImportAlias(_) => {}
        Declaration::Struct(declaration) => {
            normalize_type_expression_list(tree, &mut declaration.implements_types);
            normalize_type_expression_list(tree, &mut declaration.embedded_types);
        }
        Declaration::Class(declaration) => {
            normalize_type_expression_list(tree, &mut declaration.implements_types);
        }
        Declaration::Enum(declaration) => {
            normalize_type_expression_list(tree, &mut declaration.implements_types);
        }
        Declaration::Interface(declaration) => {
            normalize_type_expression_list(tree, &mut declaration.extends_types);
        }
        Declaration::Extension(declaration) => {
            declaration.target_type = normalize_type_expression_id(tree, declaration.target_type);
            normalize_type_expression_list(tree, &mut declaration.implements_types);
        }
        Declaration::Function(declaration) => {
            normalize_function_signature(tree, &mut declaration.signature);
        }
    }
}

/// Normalize one declarator node.
fn normalize_declarator(tree: &NodeTree, declarator: &mut Declarator) {
    normalize_optional_type_expression(tree, &mut declarator.ty);
}

/// Normalize one property node.
fn normalize_property(tree: &NodeTree, property: &mut Property) {
    match property {
        Property::Field { .. } => {}
        Property::Method { signature, .. } => {
            normalize_function_signature(tree, signature);
        }
        Property::Spread { .. } | Property::Error => {}
    }
}

/// Normalize one type member node.
fn normalize_type_member(tree: &NodeTree, member: &mut TypeMember) {
    match member {
        TypeMember::Field { declared_type, .. } => {
            normalize_optional_type_expression(tree, declared_type);
        }
        TypeMember::Method { signature, .. } => {
            normalize_function_signature(tree, signature);
        }
        TypeMember::IndexSignature {
            key_type,
            value_type,
            ..
        } => {
            *key_type = normalize_type_expression_id(tree, *key_type);
            *value_type = normalize_type_expression_id(tree, *value_type);
        }
        TypeMember::Embed { value } => {
            *value = normalize_type_expression_id(tree, *value);
        }
        TypeMember::AssociatedType {
            constraint, value, ..
        } => {
            normalize_optional_type_expression(tree, constraint);
            normalize_optional_type_expression(tree, value);
        }
        TypeMember::AssociatedConst { declared_type, .. } => {
            normalize_optional_type_expression(tree, declared_type);
        }
        TypeMember::Error => {}
    }
}

/// Normalize one member node.
fn normalize_member(tree: &NodeTree, member: &mut Member) {
    match member {
        Member::AssociatedType {
            constraint, value, ..
        } => {
            normalize_optional_type_expression(tree, constraint);
            normalize_optional_type_expression(tree, value);
        }
        Member::AssociatedConst { declared_type, .. } => {
            normalize_optional_type_expression(tree, declared_type);
        }
        Member::Field { declared_type, .. } => {
            normalize_optional_type_expression(tree, declared_type);
        }
        Member::Method { signature, .. } => {
            normalize_function_signature(tree, signature);
        }
        Member::Embed { value, .. } => {
            *value = normalize_type_expression_id(tree, *value);
        }
        Member::StaticBlock { .. } | Member::ComptimeBlock { .. } | Member::Error => {}
    }
}

/// Normalize one generic parameter node.
fn normalize_generic_parameter(tree: &NodeTree, parameter: &mut GenericParameter) {
    match parameter {
        GenericParameter::Type {
            constraint,
            default,
            ..
        } => {
            normalize_optional_type_expression(tree, constraint);
            normalize_optional_type_expression(tree, default);
        }
        GenericParameter::Value { declared_type, .. } => {
            normalize_optional_type_expression(tree, declared_type);
        }
        GenericParameter::Error => {}
    }
}

/// Normalize one parameter node.
fn normalize_parameter(tree: &NodeTree, parameter: &mut Parameter) {
    match parameter {
        Parameter::Named { declared_type, .. }
        | Parameter::Pattern { declared_type, .. }
        | Parameter::VariadicNamed { declared_type, .. }
        | Parameter::VariadicPattern { declared_type, .. } => {
            normalize_optional_type_expression(tree, declared_type);
        }
        Parameter::Error => {}
    }
}

/// Normalize one generic argument node.
fn normalize_generic_argument(tree: &NodeTree, argument: &mut GenericArgument) {
    match argument {
        GenericArgument::Type { value } => {
            *value = normalize_type_expression_id(tree, *value);
        }
        GenericArgument::Value { .. } | GenericArgument::Error => {}
    }
}

/// Normalize one tuple element node.
fn normalize_tuple_element(tree: &NodeTree, element: &mut TupleElement) {
    match element {
        TupleElement::Element { value, .. } | TupleElement::Spread { value, .. } => {
            *value = normalize_type_expression_id(tree, *value);
        }
        TupleElement::Error => {}
    }
}

/// Normalize one pattern node.
fn normalize_pattern(tree: &NodeTree, pattern: &mut Pattern) {
    match pattern {
        Pattern::TypeExpression { value }
        | Pattern::TaggedTuple { ty: value, .. }
        | Pattern::TaggedObject { ty: value, .. } => {
            *value = normalize_type_expression_id(tree, *value);
        }
        Pattern::Assign { .. } => {}
        Pattern::Wildcard
        | Pattern::Must(_)
        | Pattern::ReferenceOf { .. }
        | Pattern::ValueOf { .. }
        | Pattern::Binding { .. }
        | Pattern::Expression { .. }
        | Pattern::Tuple { .. }
        | Pattern::Array { .. }
        | Pattern::Object { .. }
        | Pattern::Union { .. } => {}
    }
}

/// Normalize one where clause node.
fn normalize_where_clause(tree: &NodeTree, where_clause: &mut WhereClause) {
    where_clause.right = normalize_type_expression_id(tree, where_clause.right);
}

/// Normalize one expression node.
fn normalize_expression(tree: &NodeTree, expression: &mut Expression) {
    match expression {
        Expression::Try { catch_ty, .. } => {
            normalize_optional_type_expression(tree, catch_ty);
        }
        Expression::ObjectExpression { ty, .. } => {
            normalize_optional_type_expression(tree, ty);
        }
        Expression::Type { value } => {
            *value = normalize_type_expression_id(tree, *value);
        }
        Expression::As { target_type, .. }
        | Expression::Satisfies { target_type, .. }
        | Expression::Is { target_type, .. } => {
            *target_type = normalize_type_expression_id(tree, *target_type);
        }
        _ => {}
    }
}

/// Normalize one type expression node.
fn normalize_type_expression(tree: &NodeTree, expression: &mut TypeExpression) {
    match expression {
        TypeExpression::Parenthesized { expression } => {
            *expression = normalize_type_expression_id(tree, *expression);
        }
        TypeExpression::Array { element } => {
            *element = normalize_type_expression_id(tree, *element);
        }
        TypeExpression::Member { left, .. } => {
            *left = normalize_type_expression_id(tree, *left);
        }
        TypeExpression::Readonly { target_type }
        | TypeExpression::KeyOf { target_type }
        | TypeExpression::Must { target_type }
        | TypeExpression::AsComptime { target_type }
        | TypeExpression::Not { target_type }
        | TypeExpression::ValueOf { target_type, .. }
        | TypeExpression::ReferenceOf { target_type, .. }
        | TypeExpression::PointerOf { target_type, .. } => {
            *target_type = normalize_type_expression_id(tree, *target_type);
        }
        TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
            normalize_type_expression_list(tree, elements);
        }
        TypeExpression::Conditional {
            left,
            extends_type,
            then_type,
            else_type,
        } => {
            *left = normalize_type_expression_id(tree, *left);
            *extends_type = normalize_type_expression_id(tree, *extends_type);
            *then_type = normalize_type_expression_id(tree, *then_type);
            *else_type = normalize_type_expression_id(tree, *else_type);
        }
        TypeExpression::Mapped {
            parameter, value, ..
        } => {
            normalize_type_mapped_parameter(tree, parameter);
            *value = normalize_type_expression_id(tree, *value);
        }
        TypeExpression::Index { left, index } => {
            *left = normalize_type_expression_id(tree, *left);
            *index = normalize_type_expression_id(tree, *index);
        }
        TypeExpression::TemplateLiteral { spans, .. } => {
            normalize_type_expression_list(tree, spans);
        }
        TypeExpression::Infer { constraint, .. } => {
            normalize_optional_type_expression(tree, constraint);
        }
        TypeExpression::Predicate { target, .. } => {
            normalize_optional_type_expression(tree, target);
        }
        TypeExpression::ScalarLiteral { .. }
        | TypeExpression::Literal { .. }
        | TypeExpression::Intrinsic
        | TypeExpression::Tuple { .. }
        | TypeExpression::Object { .. }
        | TypeExpression::Declaration { .. }
        | TypeExpression::Reference { .. }
        | TypeExpression::Const
        | TypeExpression::This
        | TypeExpression::Import { .. }
        | TypeExpression::TypeOfValue { .. }
        | TypeExpression::Missing
        | TypeExpression::Error => {}
    }
}

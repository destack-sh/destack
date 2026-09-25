use tspp_dir as dir;

/// One ordered DIR node sequence containing a pattern marker.
pub(crate) struct Sequence {
    /// The nodes in source order.
    nodes: Vec<dir::LocalNodeIdAny>,
}

impl Sequence {
    /// Find the ordered sequence containing a node.
    pub(crate) fn find(tree: &dir::Tree, node: dir::LocalNodeIdAny) -> Option<Self> {
        let parent = tree.get_parent(node.id)?;
        if node.ty == dir::NodeType::Decorator {
            return Self::containing(tree.get_decorators_ref(parent.id), node);
        }

        match parent.ty {
            dir::NodeType::Expression => Self::in_expression(tree, parent.id, node),
            dir::NodeType::TypeExpression => Self::in_type_expression(tree, parent.id, node),
            dir::NodeType::Block => Self::in_block(tree, parent.id, node),
            dir::NodeType::Declaration => Self::in_declaration(tree, parent.id, node),
            dir::NodeType::Property => Self::in_property(tree, parent.id, node),
            dir::NodeType::TypeMember => Self::in_type_member(tree, parent.id, node),
            dir::NodeType::Member => Self::in_member(tree, parent.id, node),
            dir::NodeType::Pattern => Self::in_pattern(tree, parent.id, node),
            dir::NodeType::AssignPattern => Self::in_assign_pattern(tree, parent.id, node),
            _ => None,
        }
    }

    /// Return the nodes in source order.
    pub(crate) fn nodes(&self) -> &[dir::LocalNodeIdAny] {
        &self.nodes
    }

    /// Return the preceding and following nodes around one member.
    pub(crate) fn neighbors(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<(Option<dir::LocalNodeIdAny>, Option<dir::LocalNodeIdAny>)> {
        let index = self.nodes.iter().position(|candidate| *candidate == node)?;
        let previous = index
            .checked_sub(1)
            .and_then(|index| self.nodes.get(index).copied());
        let next = self.nodes.get(index + 1).copied();

        Some((previous, next))
    }

    /// Select a typed node slice when it contains the requested node.
    fn containing<T: dir::Node>(
        nodes: &[dir::LocalNodeId<T>],
        node: dir::LocalNodeIdAny,
    ) -> Option<Self> {
        if !nodes.iter().any(|candidate| candidate.id == node.id) {
            return None;
        }
        let nodes = nodes.iter().map(|node| node.into_any()).collect();

        Some(Self { nodes })
    }

    /// Find a sequence owned by an expression.
    fn in_expression(tree: &dir::Tree, parent: u32, node: dir::LocalNodeIdAny) -> Option<Self> {
        let expression = tree.get(dir::LocalNodeId::<dir::Expression>::new(parent));

        match expression {
            dir::Expression::Import {
                items: Some(items), ..
            }
            | dir::Expression::Export { items, .. }
                if node.ty == dir::NodeType::DependencyItem =>
            {
                Self::containing(items, node)
            }
            dir::Expression::Let { declarators, .. }
            | dir::Expression::Using { declarators, .. }
                if node.ty == dir::NodeType::Declarator =>
            {
                Self::containing(declarators, node)
            }
            dir::Expression::Match { arms, .. } if node.ty == dir::NodeType::MatchArm => {
                Self::containing(arms, node)
            }
            dir::Expression::Switch { cases, .. } if node.ty == dir::NodeType::SwitchCase => {
                Self::containing(cases, node)
            }
            dir::Expression::TemplateExpression { value } if node.ty == dir::NodeType::Argument => {
                Self::in_template(value, node)
            }
            dir::Expression::TaggedTemplateExpression {
                generic_arguments,
                value,
                ..
            } => {
                if node.ty == dir::NodeType::GenericArgument {
                    Self::containing(generic_arguments, node)
                } else if node.ty == dir::NodeType::Argument {
                    Self::in_template(value, node)
                } else {
                    None
                }
            }
            dir::Expression::ArrayExpression { elements }
            | dir::Expression::TupleExpression { elements }
                if node.ty == dir::NodeType::Argument =>
            {
                Self::containing(elements, node)
            }
            dir::Expression::ObjectExpression { properties }
            | dir::Expression::StructExpression { properties, .. }
                if node.ty == dir::NodeType::Property =>
            {
                Self::containing(properties, node)
            }
            dir::Expression::TreeExpression {
                generic_arguments,
                attributes,
                children,
                ..
            } => {
                if node.ty == dir::NodeType::GenericArgument {
                    Self::containing(generic_arguments, node)
                } else if node.ty == dir::NodeType::TreeAttribute {
                    Self::containing(attributes.as_deref()?, node)
                } else if node.ty == dir::NodeType::TreeChild {
                    Self::containing(children.as_deref()?, node)
                } else {
                    None
                }
            }
            dir::Expression::Instantiation {
                generic_arguments, ..
            } if node.ty == dir::NodeType::GenericArgument => {
                Self::containing(generic_arguments, node)
            }
            dir::Expression::Call {
                generic_arguments,
                arguments,
                ..
            } => {
                if node.ty == dir::NodeType::GenericArgument {
                    Self::containing(generic_arguments, node)
                } else if node.ty == dir::NodeType::Argument {
                    Self::containing(arguments, node)
                } else {
                    None
                }
            }
            dir::Expression::New { arguments, .. } if node.ty == dir::NodeType::Argument => {
                Self::containing(arguments, node)
            }
            _ => None,
        }
    }

    /// Find an interpolated template argument sequence.
    fn in_template(template: &dir::TemplateLiteral, node: dir::LocalNodeIdAny) -> Option<Self> {
        let dir::TemplateLiteral::InterpolatedString { arguments, .. } = template else {
            return None;
        };

        Self::containing(arguments, node)
    }

    /// Find a sequence owned by a type expression.
    fn in_type_expression(
        tree: &dir::Tree,
        parent: u32,
        node: dir::LocalNodeIdAny,
    ) -> Option<Self> {
        let expression = tree.get(dir::LocalNodeId::<dir::TypeExpression>::new(parent));

        match expression {
            dir::TypeExpression::Tuple { elements, .. }
                if node.ty == dir::NodeType::TupleElement =>
            {
                Self::containing(elements, node)
            }
            dir::TypeExpression::Object { members } if node.ty == dir::NodeType::TypeMember => {
                Self::containing(members, node)
            }
            dir::TypeExpression::Function(signature) => Self::in_function_type(signature, node),
            dir::TypeExpression::Constructor(signature) => {
                Self::in_constructor_type(signature, node)
            }
            dir::TypeExpression::Reference {
                generic_arguments, ..
            }
            | dir::TypeExpression::Member {
                generic_arguments, ..
            } if node.ty == dir::NodeType::GenericArgument => {
                Self::containing(generic_arguments, node)
            }
            dir::TypeExpression::Union { elements }
            | dir::TypeExpression::Intersection { elements }
                if node.ty == dir::NodeType::TypeExpression =>
            {
                Self::containing(elements, node)
            }
            dir::TypeExpression::TemplateLiteral { spans, .. }
                if node.ty == dir::NodeType::TypeExpression =>
            {
                Self::containing(spans, node)
            }
            _ => None,
        }
    }

    /// Find a sequence owned by a function type.
    fn in_function_type(
        signature: &dir::FunctionTypeExpression,
        node: dir::LocalNodeIdAny,
    ) -> Option<Self> {
        match node.ty {
            dir::NodeType::GenericParameter => {
                Self::containing(&signature.generic_parameters, node)
            }
            dir::NodeType::WhereClause => Self::containing(&signature.where_clauses, node),
            dir::NodeType::Parameter => Self::containing(&signature.parameters, node),
            _ => None,
        }
    }

    /// Find a sequence owned by a constructor type.
    fn in_constructor_type(
        signature: &dir::ConstructorType,
        node: dir::LocalNodeIdAny,
    ) -> Option<Self> {
        match node.ty {
            dir::NodeType::GenericParameter => {
                Self::containing(&signature.generic_parameters, node)
            }
            dir::NodeType::WhereClause => Self::containing(&signature.where_clauses, node),
            dir::NodeType::Parameter => Self::containing(&signature.parameters, node),
            _ => None,
        }
    }

    /// Find a sequence owned by a block.
    fn in_block(tree: &dir::Tree, parent: u32, node: dir::LocalNodeIdAny) -> Option<Self> {
        let block = tree.get(dir::LocalNodeId::<dir::Block>::new(parent));
        if node.ty != dir::NodeType::Expression {
            return None;
        }
        let mut nodes = block
            .leading_expressions
            .iter()
            .map(|expression| expression.into_any())
            .collect::<Vec<_>>();
        if let Some(expression) = block.tail_expression {
            nodes.push(expression.into_any());
        }
        if !nodes.contains(&node) {
            return None;
        }

        Some(Self { nodes })
    }

    /// Find a sequence owned by a declaration.
    fn in_declaration(tree: &dir::Tree, parent: u32, node: dir::LocalNodeIdAny) -> Option<Self> {
        let declaration = tree.get(dir::LocalNodeId::<dir::Declaration>::new(parent));

        match declaration {
            dir::Declaration::Global(declaration) => {
                Self::containing(&declaration.expressions, node)
            }
            dir::Declaration::Module(declaration) => {
                Self::containing(&declaration.expressions, node)
            }
            dir::Declaration::Type(declaration) => {
                if node.ty == dir::NodeType::GenericParameter {
                    Self::containing(&declaration.generic_parameters, node)
                } else if node.ty == dir::NodeType::WhereClause {
                    Self::containing(&declaration.where_clauses, node)
                } else {
                    None
                }
            }
            dir::Declaration::Struct(declaration) => {
                if node.ty == dir::NodeType::GenericParameter {
                    Self::containing(&declaration.generic_parameters, node)
                } else if node.ty == dir::NodeType::WhereClause {
                    Self::containing(&declaration.where_clauses, node)
                } else if node.ty == dir::NodeType::TypeExpression {
                    Self::containing(&declaration.implements_types, node)
                } else if node.ty == dir::NodeType::Member {
                    Self::containing(&declaration.members, node)
                } else {
                    None
                }
            }
            dir::Declaration::Class(declaration) => {
                if node.ty == dir::NodeType::GenericParameter {
                    Self::containing(&declaration.generic_parameters, node)
                } else if node.ty == dir::NodeType::WhereClause {
                    Self::containing(&declaration.where_clauses, node)
                } else if node.ty == dir::NodeType::TypeExpression {
                    Self::containing(&declaration.implements_types, node)
                } else if node.ty == dir::NodeType::Member {
                    Self::containing(&declaration.members, node)
                } else {
                    None
                }
            }
            dir::Declaration::Enum(declaration) => {
                if node.ty == dir::NodeType::GenericParameter {
                    Self::containing(&declaration.generic_parameters, node)
                } else if node.ty == dir::NodeType::WhereClause {
                    Self::containing(&declaration.where_clauses, node)
                } else if node.ty == dir::NodeType::TypeExpression {
                    Self::containing(&declaration.implements_types, node)
                } else if node.ty == dir::NodeType::EnumField {
                    Self::containing(&declaration.fields, node)
                } else if node.ty == dir::NodeType::Member {
                    Self::containing(&declaration.members, node)
                } else {
                    None
                }
            }
            dir::Declaration::Interface(declaration) => {
                if node.ty == dir::NodeType::GenericParameter {
                    Self::containing(&declaration.generic_parameters, node)
                } else if node.ty == dir::NodeType::WhereClause {
                    Self::containing(&declaration.where_clauses, node)
                } else if node.ty == dir::NodeType::TypeExpression {
                    Self::containing(&declaration.extends_types, node)
                } else if node.ty == dir::NodeType::TypeMember {
                    Self::containing(&declaration.members, node)
                } else {
                    None
                }
            }
            dir::Declaration::Extension(declaration) => {
                if node.ty == dir::NodeType::GenericParameter {
                    Self::containing(&declaration.generic_parameters, node)
                } else if node.ty == dir::NodeType::WhereClause {
                    Self::containing(&declaration.where_clauses, node)
                } else if node.ty == dir::NodeType::TypeExpression {
                    Self::containing(&declaration.implements_types, node)
                } else if node.ty == dir::NodeType::Member {
                    Self::containing(&declaration.members, node)
                } else {
                    None
                }
            }
            dir::Declaration::Function(declaration) => {
                Self::in_function_signature(&declaration.signature, node)
            }
        }
    }

    /// Find a sequence owned by an object property.
    fn in_property(tree: &dir::Tree, parent: u32, node: dir::LocalNodeIdAny) -> Option<Self> {
        let property = tree.get(dir::LocalNodeId::<dir::Property>::new(parent));
        let dir::Property::Method { signature, .. } = property else {
            return None;
        };

        Self::in_function_signature(signature, node)
    }

    /// Find a sequence owned by a declaration member.
    fn in_member(tree: &dir::Tree, parent: u32, node: dir::LocalNodeIdAny) -> Option<Self> {
        let member = tree.get(dir::LocalNodeId::<dir::Member>::new(parent));

        match member {
            dir::Member::AssociatedType {
                generic_parameters,
                where_clauses,
                ..
            } => {
                if node.ty == dir::NodeType::GenericParameter {
                    Self::containing(generic_parameters, node)
                } else if node.ty == dir::NodeType::WhereClause {
                    Self::containing(where_clauses, node)
                } else {
                    None
                }
            }
            dir::Member::Method { signature, .. } => Self::in_function_signature(signature, node),
            _ => None,
        }
    }

    /// Find a sequence owned by a type member.
    fn in_type_member(tree: &dir::Tree, parent: u32, node: dir::LocalNodeIdAny) -> Option<Self> {
        let member = tree.get(dir::LocalNodeId::<dir::TypeMember>::new(parent));

        match member {
            dir::TypeMember::Method { signature, .. } => {
                Self::in_function_signature(signature, node)
            }
            dir::TypeMember::CallSignature { signature } => Self::in_function_type(signature, node),
            dir::TypeMember::ConstructSignature { signature } => {
                Self::in_constructor_type(signature, node)
            }
            dir::TypeMember::AssociatedType {
                generic_parameters,
                where_clauses,
                ..
            } => {
                if node.ty == dir::NodeType::GenericParameter {
                    Self::containing(generic_parameters, node)
                } else if node.ty == dir::NodeType::WhereClause {
                    Self::containing(where_clauses, node)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Find a sequence owned by a function signature.
    fn in_function_signature(
        signature: &dir::FunctionSignature,
        node: dir::LocalNodeIdAny,
    ) -> Option<Self> {
        match node.ty {
            dir::NodeType::GenericParameter => {
                Self::containing(&signature.generic_parameters, node)
            }
            dir::NodeType::WhereClause => Self::containing(&signature.where_clauses, node),
            dir::NodeType::Parameter => Self::containing(&signature.parameters, node),
            _ => None,
        }
    }

    /// Find a sequence owned by a destructuring pattern.
    fn in_pattern(tree: &dir::Tree, parent: u32, node: dir::LocalNodeIdAny) -> Option<Self> {
        let pattern = tree.get(dir::LocalNodeId::<dir::Pattern>::new(parent));

        match pattern {
            dir::Pattern::Tuple { fields }
            | dir::Pattern::NominalTuple { fields, .. }
            | dir::Pattern::Sequence { fields }
            | dir::Pattern::Object { fields }
            | dir::Pattern::NominalObject { fields, .. } => Self::containing(fields, node),
            dir::Pattern::Union { patterns } => Self::containing(patterns, node),
            _ => None,
        }
    }

    /// Find a sequence owned by an assignment pattern.
    fn in_assign_pattern(tree: &dir::Tree, parent: u32, node: dir::LocalNodeIdAny) -> Option<Self> {
        let pattern = tree.get(dir::LocalNodeId::<dir::AssignPattern>::new(parent));

        match pattern {
            dir::AssignPattern::Sequence { fields }
            | dir::AssignPattern::Tuple { fields }
            | dir::AssignPattern::Object { fields } => Self::containing(fields, node),
            _ => None,
        }
    }
}

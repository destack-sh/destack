use crate::{
    Annotation, Argument, Block, Definition, Expression, MatchCase, NodeId, NodeTree, NodeType,
    NodeVisitor, Parameter, Pattern, PatternField, Type, UseItem, Variant, VariantField,
    WhereClause, WithClause,
};

/// Walk any node.
pub fn walk_any<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    node_type: NodeType,
    node_id: u32,
) {
    let local_idx = tree.local_id_by_node[node_id as usize];
    match node_type {
        // --------------------------------------------------------------------
        // Groupings
        // --------------------------------------------------------------------
        NodeType::Expression => {
            let expression = tree.expressions.get(local_idx);
            walk_expression(visitor, tree, NodeId::new(node_id), expression);
        }
        NodeType::Block => {
            let block = tree.blocks.get(local_idx);
            walk_block(visitor, tree, NodeId::new(node_id), block);
        }
        // --------------------------------------------------------------------
        // Declarations
        // --------------------------------------------------------------------
        NodeType::Definition => {
            let definition = tree.definitions.get(local_idx);
            walk_definition(visitor, tree, NodeId::new(node_id), definition);
        }
        // --------------------------------------------------------------------
        // Types
        // --------------------------------------------------------------------
        NodeType::Type => {
            let ty = tree.types.get(local_idx);
            walk_type(visitor, tree, NodeId::new(node_id), ty);
        }
        NodeType::Variant => {
            let variant = tree.variants.get(local_idx);
            walk_variant(visitor, tree, NodeId::new(node_id), variant);
        }
        NodeType::VariantField => {
            let variant_field = tree.variant_fields.get(local_idx);
            walk_variant_field(visitor, tree, NodeId::new(node_id), variant_field);
        }
        NodeType::WhereClause => {
            let where_clause = tree.where_clauses.get(local_idx);
            walk_where_clause(visitor, tree, NodeId::new(node_id), where_clause);
        }
        // --------------------------------------------------------------------
        // Context
        // --------------------------------------------------------------------
        NodeType::WithClause => {
            let with_clause = tree.with_clauses.get(local_idx);
            walk_with_clause(visitor, tree, NodeId::new(node_id), with_clause);
        }
        NodeType::UseItem => {
            let use_item = tree.use_items.get(local_idx);
            walk_use_item(visitor, tree, NodeId::new(node_id), use_item);
        }
        // --------------------------------------------------------------------
        // Bindings
        // --------------------------------------------------------------------
        NodeType::Parameter => {
            let parameter = tree.parameters.get(local_idx);
            walk_parameter(visitor, tree, NodeId::new(node_id), parameter);
        }
        NodeType::Argument => {
            let argument = tree.arguments.get(local_idx);
            walk_argument(visitor, tree, NodeId::new(node_id), argument);
        }
        // --------------------------------------------------------------------
        // Matching
        // --------------------------------------------------------------------
        NodeType::Pattern => {
            let pattern = tree.patterns.get(local_idx);
            walk_pattern(visitor, tree, NodeId::new(node_id), pattern);
        }
        NodeType::PatternField => {
            let pattern_field = tree.pattern_fields.get(local_idx);
            walk_pattern_field(visitor, tree, NodeId::new(node_id), pattern_field);
        }
        NodeType::MatchCase => {
            let match_case = tree.match_cases.get(local_idx);
            walk_match_case(visitor, tree, NodeId::new(node_id), match_case);
        }
        // --------------------------------------------------------------------
        // Annotations
        // --------------------------------------------------------------------
        NodeType::Annotation => {
            let annotation = tree.annotations.get(local_idx);
            walk_annotation(visitor, tree, NodeId::new(node_id), annotation);
        }
    }
}

// ----------------------------------------------------------------------------
// Groupings
// ----------------------------------------------------------------------------

/// Walk the Expression.
pub fn walk_expression<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Expression>,
    expression: &Expression,
) {
    visitor.visit_any(tree, NodeType::Expression, id.id);
    match expression {
        Expression::Definition(definition_id) => {
            let definition = tree.get(*definition_id);
            walk_definition(visitor, tree, *definition_id, definition);
        }
        Expression::Block(block_id) => {
            let block = tree.get(*block_id);
            walk_block(visitor, tree, *block_id, block);
        }
        Expression::With { clauses, body } => {
            for clause_id in clauses {
                let clause = tree.get(*clause_id);
                walk_with_clause(visitor, tree, *clause_id, clause);
            }
            if let Some(body_id) = body {
                let block = tree.get(*body_id);
                walk_block(visitor, tree, *body_id, block);
            }
        }
        Expression::Use { items, body } => {
            for item_id in items {
                let item = tree.get(*item_id);
                walk_use_item(visitor, tree, *item_id, item);
            }
            if let Some(body_id) = body {
                let block = tree.get(*body_id);
                walk_block(visitor, tree, *body_id, block);
            }
        }
        Expression::Unary { operator: _, right }
        | Expression::Reference { right }
        | Expression::Dereference { right } => {
            let right_expression = tree.get(*right);
            walk_expression(visitor, tree, *right, right_expression);
        }
        Expression::Binary {
            left,
            operator: _,
            right,
        }
        | Expression::AssignDirect { left, right }
        | Expression::AssignBinary {
            left,
            operator: _,
            right,
        } => {
            let left_expression = tree.get(*left);
            walk_expression(visitor, tree, *left, left_expression);
            let right_expression = tree.get(*right);
            walk_expression(visitor, tree, *right, right_expression);
        }
        Expression::Drop => {}
        Expression::Member { left, path: _ } => {
            let left_expression = tree.get(*left);
            walk_expression(visitor, tree, *left, left_expression);
        }
        Expression::Call {
            left,
            dynamic_arguments,
        } => {
            let left_expression = tree.get(*left);
            walk_expression(visitor, tree, *left, left_expression);
            for argument_id in dynamic_arguments {
                let argument = tree.get(*argument_id);
                walk_argument(visitor, tree, *argument_id, argument);
            }
        }
        Expression::Index { left, right } => {
            let left_expression = tree.get(*left);
            walk_expression(visitor, tree, *left, left_expression);
            let right_expression = tree.get(*right);
            walk_expression(visitor, tree, *right, right_expression);
        }
        Expression::Cast { value, ty } => {
            let value_expression = tree.get(*value);
            walk_expression(visitor, tree, *value, value_expression);
            let ty_node = tree.get(*ty);
            walk_type(visitor, tree, *ty, ty_node);
        }
        Expression::ScalarLiteral => {}
        Expression::StructLiteral { ty, fields } => {
            let ty_node = tree.get(*ty);
            walk_type(visitor, tree, *ty, ty_node);
            for field_id in fields {
                let argument = tree.get(*field_id);
                walk_argument(visitor, tree, *field_id, argument);
            }
        }
        Expression::TupleLiteral { elements } => {
            for argument_id in elements {
                let argument = tree.get(*argument_id);
                walk_argument(visitor, tree, *argument_id, argument);
            }
        }
        Expression::ArrayLiteral { elements } => {
            for element_id in elements {
                let element = tree.get(*element_id);
                walk_expression(visitor, tree, *element_id, element);
            }
        }
        Expression::If {
            condition,
            then_block,
            else_block,
        } => {
            let condition_expression = tree.get(*condition);
            walk_expression(visitor, tree, *condition, condition_expression);
            let then_block_node = tree.get(*then_block);
            walk_block(visitor, tree, *then_block, then_block_node);
            if let Some(else_block_id) = else_block {
                let else_expression = tree.get(*else_block_id);
                walk_expression(visitor, tree, *else_block_id, else_expression);
            }
        }
        Expression::Loop {
            condition,
            body,
            source: _,
        } => {
            let condition_expression = tree.get(*condition);
            walk_expression(visitor, tree, *condition, condition_expression);
            let body_block = tree.get(*body);
            walk_block(visitor, tree, *body, body_block);
        }
        Expression::Match { value, cases } => {
            let value_expression = tree.get(*value);
            walk_expression(visitor, tree, *value, value_expression);
            for case_id in cases {
                let case = tree.get(*case_id);
                walk_match_case(visitor, tree, *case_id, case);
            }
        }
        Expression::Break { label: _, value } => {
            if let Some(value_id) = value {
                let value_expression = tree.get(*value_id);
                walk_expression(visitor, tree, *value_id, value_expression);
            }
        }
        Expression::Continue { label: _ } => {}
        Expression::Defer { body } => {
            let body_expression = tree.get(*body);
            walk_expression(visitor, tree, *body, body_expression);
        }
        Expression::Return { value } => {
            if let Some(value_id) = value {
                let value_expression = tree.get(*value_id);
                walk_expression(visitor, tree, *value_id, value_expression);
            }
        }
        Expression::Error => {}
    }
}

/// Walk the Block.
pub fn walk_block<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Block>,
    block: &Block,
) {
    visitor.visit_any(tree, NodeType::Block, id.id);
    for expression_id in &block.expressions {
        let expression = tree.get(*expression_id);
        walk_expression(visitor, tree, *expression_id, expression);
    }
}

// ----------------------------------------------------------------------------
// Declarations
// ----------------------------------------------------------------------------

/// Walk the Definition.
pub fn walk_definition<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Definition>,
    definition: &Definition,
) {
    visitor.visit_any(tree, NodeType::Definition, id.id);
    match definition {
        Definition::Module {
            name: _,
            visibility: _,
            with_clauses,
            where_clauses,
            definitions,
        } => {
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_with_clause(visitor, tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_where_clause(visitor, tree, *clause_id, clause);
                }
            }
            for definition_id in definitions {
                let child_definition = tree.get(*definition_id);
                walk_definition(visitor, tree, *definition_id, child_definition);
            }
        }
        Definition::Struct {
            name: _,
            visibility: _,
            super_types,
            variant,
            with_clauses,
            where_clauses,
            definitions,
        } => {
            if let Some(super_types) = super_types {
                for super_type_id in super_types.iter() {
                    let super_type = tree.get(*super_type_id);
                    walk_type(visitor, tree, *super_type_id, super_type);
                }
            }
            let variant_node = tree.get(*variant);
            walk_variant(visitor, tree, *variant, variant_node);
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_with_clause(visitor, tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_where_clause(visitor, tree, *clause_id, clause);
                }
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                walk_definition(visitor, tree, *definition_id, child_definition);
            }
        }
        Definition::Enum {
            name: _,
            visibility: _,
            super_types,
            variant,
            with_clauses,
            where_clauses,
            definitions,
        } => {
            if let Some(super_types) = super_types {
                for super_type_id in super_types.iter() {
                    let super_type = tree.get(*super_type_id);
                    walk_type(visitor, tree, *super_type_id, super_type);
                }
            }
            let variant_node = tree.get(*variant);
            walk_variant(visitor, tree, *variant, variant_node);
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_with_clause(visitor, tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_where_clause(visitor, tree, *clause_id, clause);
                }
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                walk_definition(visitor, tree, *definition_id, child_definition);
            }
        }
        Definition::Union {
            name: _,
            visibility: _,
            super_types,
            variants,
            with_clauses,
            where_clauses,
            definitions,
        } => {
            if let Some(super_types) = super_types {
                for super_type_id in super_types.iter() {
                    let super_type = tree.get(*super_type_id);
                    walk_type(visitor, tree, *super_type_id, super_type);
                }
            }
            for variant_id in variants.iter() {
                let variant_node = tree.get(*variant_id);
                walk_variant(visitor, tree, *variant_id, variant_node);
            }
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_with_clause(visitor, tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_where_clause(visitor, tree, *clause_id, clause);
                }
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                walk_definition(visitor, tree, *definition_id, child_definition);
            }
        }
        Definition::Trait {
            name: _,
            visibility: _,
            super_types,
            with_clauses,
            where_clauses,
            definitions,
        } => {
            if let Some(super_types) = super_types {
                for super_type_id in super_types.iter() {
                    let super_type = tree.get(*super_type_id);
                    walk_type(visitor, tree, *super_type_id, super_type);
                }
            }
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_with_clause(visitor, tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_where_clause(visitor, tree, *clause_id, clause);
                }
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                walk_definition(visitor, tree, *definition_id, child_definition);
            }
        }
        Definition::Function {
            name: _,
            visibility: _,
            with_clauses,
            where_clauses,
            definitions,
        } => {
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_with_clause(visitor, tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_where_clause(visitor, tree, *clause_id, clause);
                }
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                walk_definition(visitor, tree, *definition_id, child_definition);
            }
        }
        Definition::Implement {
            name: _,
            visibility: _,
            for_type,
            with_clauses,
            where_clauses,
            definitions,
        } => {
            if let Some(for_type_id) = for_type {
                let implement_type = tree.get(*for_type_id);
                walk_type(visitor, tree, *for_type_id, implement_type);
            }
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_with_clause(visitor, tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    walk_where_clause(visitor, tree, *clause_id, clause);
                }
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                walk_definition(visitor, tree, *definition_id, child_definition);
            }
        }
        Definition::Let {
            name: _,
            visibility: _,
        } => {}
    }
}

// ----------------------------------------------------------------------------
// Types
// ----------------------------------------------------------------------------

/// Walk the Type.
pub fn walk_type<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Type>,
    ty: &Type,
) {
    visitor.visit_any(tree, NodeType::Type, id.id);
    match ty {
        Type::Infer | Type::Never | Type::TypeLiteral(_) | Type::ScalarLiteral(_) | Type::Self_ => {
        }
        Type::Maybe(inner) | Type::Not(inner) | Type::Virtual(inner) | Type::Variadic(inner) => {
            let inner_type = tree.get(*inner);
            walk_type(visitor, tree, *inner, inner_type);
        }
        Type::Reference {
            mutability: _,
            target,
        } => {
            let target_type = tree.get(*target);
            walk_type(visitor, tree, *target, target_type);
        }
        Type::Expression(expression_id) => {
            let expression = tree.get(*expression_id);
            walk_expression(visitor, tree, *expression_id, expression);
        }
        Type::Path {
            path: _,
            static_arguments,
        } => {
            if let Some(arguments) = static_arguments {
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    walk_argument(visitor, tree, *argument_id, argument);
                }
            }
        }
        Type::Variant(variant_id) => {
            let variant = tree.get(*variant_id);
            walk_variant(visitor, tree, *variant_id, variant);
        }
        Type::Array { element, count } => {
            let element_type = tree.get(*element);
            walk_type(visitor, tree, *element, element_type);
            let count_expression = tree.get(*count);
            walk_expression(visitor, tree, *count, count_expression);
        }
        Type::Slice { element } => {
            let element_type = tree.get(*element);
            walk_type(visitor, tree, *element, element_type);
        }
        Type::Tuple(elements) | Type::Union(elements) | Type::Intersection(elements) => {
            for element_id in elements {
                let element_type = tree.get(*element_id);
                walk_type(visitor, tree, *element_id, element_type);
            }
        }
    }
}

/// Walk the Variant.
pub fn walk_variant<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Variant>,
    variant: &Variant,
) {
    visitor.visit_any(tree, NodeType::Variant, id.id);
    match variant {
        Variant::Struct {
            name: _,
            representation_type,
            fields,
        }
        | Variant::Tuple {
            name: _,
            representation_type,
            fields,
        } => {
            if let Some(representation_type) = representation_type {
                let representation_type_node = tree.get(*representation_type);
                walk_type(
                    visitor,
                    tree,
                    *representation_type,
                    representation_type_node,
                );
            }
            for field_id in fields.iter() {
                let field = tree.get(*field_id);
                walk_variant_field(visitor, tree, *field_id, field);
            }
        }
        Variant::Unit { name: _ } => {}
    }
}

/// Walk the VariantField.
pub fn walk_variant_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<VariantField>,
    variant_field: &VariantField,
) {
    visitor.visit_any(tree, NodeType::VariantField, id.id);
    match variant_field {
        VariantField::Named { name: _, ty } | VariantField::Positional { ty } => {
            let ty_node = tree.get(*ty);
            walk_type(visitor, tree, *ty, ty_node);
        }
    }
}

/// Walk the WhereClause.
pub fn walk_where_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<WhereClause>,
    where_clause: &WhereClause,
) {
    visitor.visit_any(tree, NodeType::WhereClause, id.id);
    match where_clause {
        WhereClause::Assertion { left: _, right } => {
            let right_expression = tree.get(*right);
            walk_expression(visitor, tree, *right, right_expression);
        }
        WhereClause::Guard { guard } => {
            let guard_expression = tree.get(*guard);
            walk_expression(visitor, tree, *guard, guard_expression);
        }
    }
}

// ----------------------------------------------------------------------------
// Context
// ----------------------------------------------------------------------------

/// Walk the WithClause.
pub fn walk_with_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<WithClause>,
    with_clause: &WithClause,
) {
    visitor.visit_any(tree, NodeType::WithClause, id.id);
    let right_expression = tree.get(with_clause.right);
    walk_expression(visitor, tree, with_clause.right, right_expression);
}

/// Walk the UseItem.
pub fn walk_use_item<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<UseItem>,
    _use_item: &UseItem,
) {
    visitor.visit_any(tree, NodeType::UseItem, id.id);
}

// ----------------------------------------------------------------------------
// Bindings
// ----------------------------------------------------------------------------

/// Walk the Parameter.
pub fn walk_parameter<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Parameter>,
    parameter: &Parameter,
) {
    visitor.visit_any(tree, NodeType::Parameter, id.id);
    if let Some(ty_id) = parameter.ty {
        let ty_node = tree.get(ty_id);
        walk_type(visitor, tree, ty_id, ty_node);
    }
    if let Some(default_id) = parameter.default {
        let default_expression = tree.get(default_id);
        walk_expression(visitor, tree, default_id, default_expression);
    }
}

/// Walk the Argument.
pub fn walk_argument<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Argument>,
    argument: &Argument,
) {
    visitor.visit_any(tree, NodeType::Argument, id.id);
    match argument {
        Argument::Named { name: _, value } | Argument::Positional(value) => {
            let value_expression = tree.get(*value);
            walk_expression(visitor, tree, *value, value_expression);
        }
    }
}

// ----------------------------------------------------------------------------
// Matching
// ----------------------------------------------------------------------------

/// Walk the Pattern.
pub fn walk_pattern<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Pattern>,
    pattern: &Pattern,
) {
    visitor.visit_any(tree, NodeType::Pattern, id.id);
    match pattern {
        Pattern::Wildcard
        | Pattern::Rest
        | Pattern::ScalarLiteral(_)
        | Pattern::Binding { name: _ }
        | Pattern::Path(_) => {}
        Pattern::Maybe(inner) => {
            let inner_pattern = tree.get(*inner);
            walk_pattern(visitor, tree, *inner, inner_pattern);
        }
        Pattern::Reference {
            target,
            mutability: _,
        } => {
            let target_pattern = tree.get(*target);
            walk_pattern(visitor, tree, *target, target_pattern);
        }
        Pattern::Range {
            start,
            end,
            is_inclusive: _,
        } => {
            if let Some(start_id) = start {
                let start_pattern = tree.get(*start_id);
                walk_pattern(visitor, tree, *start_id, start_pattern);
            }
            if let Some(end_id) = end {
                let end_pattern = tree.get(*end_id);
                walk_pattern(visitor, tree, *end_id, end_pattern);
            }
        }
        Pattern::Tuple { path: _, fields } | Pattern::Slice { fields } => {
            for field_id in fields {
                let field = tree.get(*field_id);
                walk_pattern_field(visitor, tree, *field_id, field);
            }
        }
        Pattern::Struct { ty, fields } => {
            let ty_expression = tree.get(*ty);
            walk_expression(visitor, tree, *ty, ty_expression);
            for field_id in fields {
                let field = tree.get(*field_id);
                walk_pattern_field(visitor, tree, *field_id, field);
            }
        }
        Pattern::Union { fields } => {
            for pattern_id in fields {
                let union_pattern = tree.get(*pattern_id);
                walk_pattern(visitor, tree, *pattern_id, union_pattern);
            }
        }
    }
}

/// Walk the PatternField.
pub fn walk_pattern_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<PatternField>,
    pattern_field: &PatternField,
) {
    visitor.visit_any(tree, NodeType::PatternField, id.id);
    match pattern_field {
        PatternField::Named {
            name: _,
            pattern,
            mutability: _,
        } => {
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                walk_pattern(visitor, tree, *pattern_id, pattern_node);
            }
        }
        PatternField::NamedAlias {
            name: _,
            alias: _,
            mutability: _,
        } => {}
        PatternField::Positional { pattern } => {
            let pattern_node = tree.get(*pattern);
            walk_pattern(visitor, tree, *pattern, pattern_node);
        }
    }
}

/// Walk the MatchCase.
pub fn walk_match_case<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<MatchCase>,
    match_case: &MatchCase,
) {
    visitor.visit_any(tree, NodeType::MatchCase, id.id);
    match match_case {
        MatchCase::Expression {
            pattern,
            body,
            guard,
        } => {
            let pattern_node = tree.get(*pattern);
            walk_pattern(visitor, tree, *pattern, pattern_node);
            let body_expression = tree.get(*body);
            walk_expression(visitor, tree, *body, body_expression);
            if let Some(guard_id) = guard {
                let guard_expression = tree.get(*guard_id);
                walk_expression(visitor, tree, *guard_id, guard_expression);
            }
        }
        MatchCase::Block {
            pattern,
            body,
            guard,
        } => {
            let pattern_node = tree.get(*pattern);
            walk_pattern(visitor, tree, *pattern, pattern_node);
            let body_block = tree.get(*body);
            walk_block(visitor, tree, *body, body_block);
            if let Some(guard_id) = guard {
                let guard_expression = tree.get(*guard_id);
                walk_expression(visitor, tree, *guard_id, guard_expression);
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Annotations
// ----------------------------------------------------------------------------

/// Walk the Annotation.
pub fn walk_annotation<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Annotation>,
    annotation: &Annotation,
) {
    visitor.visit_any(tree, NodeType::Annotation, id.id);
    match annotation {
        Annotation::Blank {
            position: _,
            lines: _,
        }
        | Annotation::Doc {
            position: _,
            string: _,
        }
        | Annotation::Comment {
            position: _,
            string: _,
        } => {}
        Annotation::Tag {
            position: _,
            receiver: _,
            arguments,
        }
        | Annotation::Decorator {
            position: _,
            receiver: _,
            arguments,
        } => {
            if let Some(arguments) = arguments {
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    walk_argument(visitor, tree, *argument_id, argument);
                }
            }
        }
    }
}

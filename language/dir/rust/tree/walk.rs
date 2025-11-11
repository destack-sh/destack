use crate::{
    Annotation, Argument, ArgumentSlot, Block, Definition, DependencyItem, Expression, Field,
    FunctionSignature, Generics, MatchCase, NodeId, NodeTree, NodeType, NodeVisitor, Parameter,
    Pattern, PatternField, TemplateLiteral, Type, Variant, WhereClause, WithClause,
};

/// Walk any node.
pub fn walk_any<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    node_type: NodeType,
    node_id: u32,
) {
    let local_idx = tree.local_id_by_node_id[node_id as usize];
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
        NodeType::Field => {
            let field = tree.fields.get(local_idx);
            walk_field(visitor, tree, NodeId::new(node_id), field);
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
        NodeType::DependencyItem => {
            let dependency_item = tree.dependency_items.get(local_idx);
            walk_dependency_item(visitor, tree, NodeId::new(node_id), dependency_item);
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

/// Walk the Generics.
fn walk_generics<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, generics: &Generics) {
    if let Some(static_parameters) = generics.static_parameters.as_ref() {
        for parameter_id in static_parameters.iter() {
            let parameter = tree.get(*parameter_id);
            visitor.visit_parameter(tree, *parameter_id, parameter);
        }
    }
    if let Some(with_clauses) = generics.with_clauses.as_ref() {
        for clause_id in with_clauses.iter() {
            let clause = tree.get(*clause_id);
            visitor.visit_with_clause(tree, *clause_id, clause);
        }
    }
    if let Some(where_clauses) = generics.where_clauses.as_ref() {
        for clause_id in where_clauses.iter() {
            let clause = tree.get(*clause_id);
            visitor.visit_where_clause(tree, *clause_id, clause);
        }
    }
}

/// Walk the FunctionSignature.
fn walk_function_signature<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    signature: &FunctionSignature,
) {
    if let Some(self_parameter) = &signature.self_parameter
        && let Some(ty) = &self_parameter.ty
    {
        let ty_node = tree.get(*ty);
        visitor.visit_type(tree, *ty, ty_node);
    }
    for parameter_id in signature.dynamic_parameters.iter() {
        let parameter = tree.get(*parameter_id);
        visitor.visit_parameter(tree, *parameter_id, parameter);
    }
    if let Some(return_type) = signature.return_type {
        let return_type_node = tree.get(return_type);
        visitor.visit_type(tree, return_type, return_type_node);
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
        Expression::Block { block: block_id } => {
            let block = tree.get(*block_id);
            visitor.visit_block(tree, *block_id, block);
        }
        Expression::Definition {
            definition: definition_id,
        } => {
            let definition = tree.get(*definition_id);
            visitor.visit_definition(tree, *definition_id, definition);
        }
        Expression::With { clauses, body } => {
            for clause_id in clauses {
                let clause = tree.get(*clause_id);
                visitor.visit_with_clause(tree, *clause_id, clause);
            }
            if let Some(body_id) = body {
                let block = tree.get(*body_id);
                visitor.visit_block(tree, *body_id, block);
            }
        }
        Expression::Import {
            kind: _,
            asynchrony: _,
            items,
            arguments,
        } => {
            for item_id in items {
                let item = tree.get(*item_id);
                visitor.visit_dependency_item(tree, *item_id, item);
            }
            if let Some(arguments) = arguments {
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
        }
        Expression::Export {
            mode: _,
            kind: _,
            items,
            value,
        } => {
            if let Some(items) = items {
                for item_id in items {
                    let item = tree.get(*item_id);
                    visitor.visit_dependency_item(tree, *item_id, item);
                }
            }
            if let Some(value_id) = value {
                let value_expr = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_expr);
            }
        }
        Expression::Let {
            mutability: _,
            pattern: pattern_id,
            ty: ty_id,
            value: value_id,
        } => {
            let pattern = tree.get(*pattern_id);
            visitor.visit_pattern(tree, *pattern_id, pattern);
            if let Some(ty_id) = ty_id {
                let ty = tree.get(*ty_id);
                visitor.visit_type(tree, *ty_id, ty);
            }
            if let Some(value_id) = value_id {
                let value = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value);
            }
        }
        Expression::LetType {
            mutability: _,
            name: _,
            static_parameters,
            value,
        } => {
            if let Some(static_parameters) = static_parameters {
                for parameter_id in static_parameters {
                    let parameter = tree.get(*parameter_id);
                    visitor.visit_parameter(tree, *parameter_id, parameter);
                }
            }
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Expression::Unary {
            operator: _,
            right,
        }
        | Expression::ValueOf {
            mutability: _,
            variance: _,
            right,
        }
        | Expression::ReferenceOf {
            mutability: _,
            variance: _,
            right,
        }
        | Expression::TypeUnary {
            operator: _,
            right,
        } => {
            let right_expression = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expression);
        }
        Expression::Binary {
            left,
            operator: _,
            right,
        }
        | Expression::TypeBinary {
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
            visitor.visit_expression(tree, *left, left_expression);
            let right_expression = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expression);
        }
        Expression::Member {
            left,
            path: _,
            static_arguments,
        } => {
            let left_expression = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expression);
            if let Some(static_arguments) = static_arguments {
                for argument_id in static_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
        }
        Expression::Call {
            left,
            dynamic_arguments,
        } => {
            let left_expression = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expression);
            for argument_id in dynamic_arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }
        }
        Expression::New {
            left: _,
            static_arguments,
            dynamic_arguments,
        } => {
            if let Some(static_arguments) = static_arguments {
                for argument_id in static_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
            for argument_id in dynamic_arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }
        }
        Expression::Delete { value } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Expression::Index { left, right } => {
            let left_expression = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expression);
            if let Some(right_id) = right {
                let right_expression = tree.get(*right_id);
                visitor.visit_expression(tree, *right_id, right_expression);
            }
        }
        Expression::Maybe { left } => {
            let left_expression = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expression);
        }
        Expression::Must { left } => {
            let left_expression = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expression);
        }
        Expression::Path {
            path: _,
            static_arguments,
        } => {
            if let Some(static_arguments) = static_arguments {
                for argument_id in static_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
        }
        Expression::ScalarLiteral { value: _ } => {
            // nothing to do
        }
        Expression::TemplateLiteral { value } => {
            match value {
                TemplateLiteral::String { .. } | TemplateLiteral::TaggedString { .. } => {
                    // nothing to do
                }
                TemplateLiteral::InterpolatedString { arguments, .. }
                | TemplateLiteral::TaggedInterpolatedString { arguments, .. } => {
                    for argument_id in arguments {
                        let argument = tree.get(*argument_id);
                        visitor.visit_argument(tree, *argument_id, argument);
                    }
                }
            }
        }
        Expression::TypeLiteral { value: _ } => {
            // nothing to do
        }
        Expression::RangeLiteral {
            start,
            end,
            is_inclusive: _,
        } => {
            let start_expression = tree.get(*start);
            visitor.visit_expression(tree, *start, start_expression);
            let end_expression = tree.get(*end);
            visitor.visit_expression(tree, *end, end_expression);
        }
        Expression::ArrayLiteral { elements } => {
            for element_id in elements {
                let element = tree.get(*element_id);
                visitor.visit_argument(tree, *element_id, element);
            }
        }
        Expression::TupleLiteral { ty, elements } => {
            if let Some(ty_id) = ty {
                let ty_node = tree.get(*ty_id);
                visitor.visit_type(tree, *ty_id, ty_node);
            }
            for argument_id in elements {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }
        }
        Expression::StructLiteral { ty, fields } => {
            if let Some(ty_id) = ty {
                let ty_node = tree.get(*ty_id);
                visitor.visit_type(tree, *ty_id, ty_node);
            }
            for field_id in fields {
                let argument = tree.get(*field_id);
                visitor.visit_argument(tree, *field_id, argument);
            }
        }
        Expression::TreeLiteral {
            path: _,
            arguments,
            elements,
        } => {
            if let Some(arguments) = arguments {
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
            if let Some(elements) = elements {
                for element_id in elements {
                    let element = tree.get(*element_id);
                    visitor.visit_argument(tree, *element_id, element);
                }
            }
        }
        Expression::If {
            kind: _,
            condition,
            then_expression,
            else_expression,
        } => {
            let condition_expression = tree.get(*condition);
            visitor.visit_expression(tree, *condition, condition_expression);
            let then_expression_node = tree.get(*then_expression);
            visitor.visit_expression(tree, *then_expression, then_expression_node);
            if let Some(else_expression_id) = else_expression {
                let else_expression = tree.get(*else_expression_id);
                visitor.visit_expression(tree, *else_expression_id, else_expression);
            }
        }
        Expression::Loop {
            condition,
            body,
            source: _,
        } => {
            let condition_expression = tree.get(*condition);
            visitor.visit_expression(tree, *condition, condition_expression);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }
        Expression::ForEach {
            asynchrony: _,
            kind: _,
            pattern,
            iterator,
            body,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            let iterator_expression = tree.get(*iterator);
            visitor.visit_expression(tree, *iterator, iterator_expression);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }
        Expression::For {
            initialization,
            condition,
            increment,
            body,
        } => {
            if let Some(initialization_id) = initialization {
                let initialization_expression = tree.get(*initialization_id);
                visitor.visit_expression(tree, *initialization_id, initialization_expression);
            }
            if let Some(condition_id) = condition {
                let condition_expression = tree.get(*condition_id);
                visitor.visit_expression(tree, *condition_id, condition_expression);
            }
            if let Some(increment_id) = increment {
                let increment_expression = tree.get(*increment_id);
                visitor.visit_expression(tree, *increment_id, increment_expression);
            }
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }
        Expression::Match {
            value,
            cases,
            source: _,
        } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
            for case_id in cases {
                let case = tree.get(*case_id);
                visitor.visit_match_case(tree, *case_id, case);
            }
        }
        Expression::Break { target: _, value } => {
            if let Some(value_id) = value {
                let value_expression = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_expression);
            }
        }
        Expression::Continue { target: _ } => {}
        Expression::Defer { expression } => {
            let body_expression = tree.get(*expression);
            visitor.visit_expression(tree, *expression, body_expression);
        }
        Expression::Throw { value } => {
            if let Some(value_id) = value {
                let value_expression = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_expression);
            }
        }
        Expression::Return { value } => {
            if let Some(value_id) = value {
                let value_expression = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_expression);
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
        visitor.visit_expression(tree, *expression_id, expression);
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
        Definition::Intrinsic { intrinsic: _ } => {
            // nothing to do
        }
        Definition::Import {
            kind: _,
            asynchrony: _,
            items,
            arguments,
        } => {
            for item_id in items {
                let item = tree.get(*item_id);
                visitor.visit_dependency_item(tree, *item_id, item);
            }
            if let Some(arguments) = arguments {
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
        }
        Definition::Export {
            mode: _,
            kind: _,
            items,
        } => {
            for item_id in items {
                let item = tree.get(*item_id);
                visitor.visit_dependency_item(tree, *item_id, item);
            }
        }
        Definition::Let { meta: _, value } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Definition::Type {
            meta: _,
            generics,
            value,
        } => {
            if let Some(generics) = generics.as_ref() {
                walk_generics(visitor, tree, generics);
            }
            let value_type = tree.get(*value);
            visitor.visit_type(tree, *value, value_type);
        }
        Definition::Namespace {
            meta: _,
            generics,
            definitions,
        } => {
            if let Some(generics) = generics.as_ref() {
                walk_generics(visitor, tree, generics);
            }
            for definition_id in definitions {
                let child_definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, child_definition);
            }
        }
        Definition::Struct {
            meta: _,
            kind: _,
            generics,
            embedded_definitions,
            variant,
            definitions,
        } => {
            if let Some(generics) = generics.as_ref() {
                walk_generics(visitor, tree, generics);
            }
            for embedded_definition in embedded_definitions.iter() {
                let ty_id = embedded_definition.ty();
                let ty = tree.get(ty_id);
                visitor.visit_type(tree, ty_id, ty);
            }
            let variant_node = tree.get(*variant);
            visitor.visit_variant(tree, *variant, variant_node);
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, child_definition);
            }
        }
        Definition::Enum {
            meta: _,
            generics,
            embedded_definitions,
            variants,
            definitions,
        } => {
            if let Some(generics) = generics.as_ref() {
                walk_generics(visitor, tree, generics);
            }
            for embedded_definition in embedded_definitions.iter() {
                let ty_id = embedded_definition.ty();
                let ty = tree.get(ty_id);
                visitor.visit_type(tree, ty_id, ty);
            }
            for variant_id in variants.iter() {
                let variant_node = tree.get(*variant_id);
                visitor.visit_variant(tree, *variant_id, variant_node);
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, child_definition);
            }
        }
        Definition::Union {
            meta: _,
            generics,
            embedded_definitions,
            variants,
            definitions,
        } => {
            if let Some(generics) = generics.as_ref() {
                walk_generics(visitor, tree, generics);
            }
            for embedded_definition in embedded_definitions.iter() {
                let ty_id = embedded_definition.ty();
                let ty = tree.get(ty_id);
                visitor.visit_type(tree, ty_id, ty);
            }
            for variant_id in variants.iter() {
                let variant_node = tree.get(*variant_id);
                visitor.visit_variant(tree, *variant_id, variant_node);
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, child_definition);
            }
        }
        Definition::Interface {
            meta: _,
            generics,
            embedded_definitions,
            fields,
            definitions,
        } => {
            if let Some(generics) = generics.as_ref() {
                walk_generics(visitor, tree, generics);
            }
            for embedded_definition in embedded_definitions.iter() {
                let ty_id = embedded_definition.ty();
                let ty = tree.get(ty_id);
                visitor.visit_type(tree, ty_id, ty);
            }
            for field_id in fields.iter() {
                let field = tree.get(*field_id);
                visitor.visit_field(tree, *field_id, field);
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, child_definition);
            }
        }
        Definition::Function {
            meta: _,
            generics,
            signature,
            definitions,
            body,
        } => {
            if let Some(generics) = generics.as_ref() {
                walk_generics(visitor, tree, generics);
            }
            walk_function_signature(visitor, tree, signature);
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, child_definition);
            }
            if let Some(body) = body {
                let body_expression = tree.get(*body);
                visitor.visit_expression(tree, *body, body_expression);
            }
        }
        Definition::Extension {
            meta: _,
            generics,
            target_type,
            implements_types,
            definitions,
        } => {
            if let Some(generics) = generics.as_ref() {
                walk_generics(visitor, tree, generics);
            }
            let target_type_expr = tree.get(*target_type);
            visitor.visit_type(tree, *target_type, target_type_expr);
            if let Some(implements_types) = implements_types {
                for implements_type in implements_types {
                    let implements_type_expr = tree.get(*implements_type);
                    visitor.visit_type(tree, *implements_type, implements_type_expr);
                }
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, child_definition);
            }
        }
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
        Type::Scalar(_) => {}
        Type::Definition(definition_id) => {
            if visitor.options().visit_indirect {
                let definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, definition);
            }
        }

        Type::Unary { operator: _, right }
        | Type::Mutable {
            mutability: _,
            right,
        }
        | Type::ValueOf {
            mutability: _,
            variance: _,
            right,
        }
        | Type::ReferenceOf {
            mutability: _,
            variance: _,
            right,
        } => {
            let target_type = tree.get(*right);
            visitor.visit_type(tree, *right, target_type);
        }
        Type::Binary {
            left,
            operator: _,
            right,
        } => {
            let left_type = tree.get(*left);
            visitor.visit_type(tree, *left, left_type);
            let right_type = tree.get(*right);
            visitor.visit_type(tree, *right, right_type);
        }

        Type::Range {
            start,
            end,
            is_inclusive: _,
        } => {
            let start_type = tree.get(*start);
            visitor.visit_type(tree, *start, start_type);
            let end_type = tree.get(*end);
            visitor.visit_type(tree, *end, end_type);
        }
        Type::ArrayStatic { element, count } => {
            let element_type = tree.get(*element);
            visitor.visit_type(tree, *element, element_type);
            let count_expression = tree.get(*count);
            visitor.visit_expression(tree, *count, count_expression);
        }
        Type::ArraySlice { element } => {
            let element_type = tree.get(*element);
            visitor.visit_type(tree, *element, element_type);
        }
        Type::ArrayDynamic { elements } => {
            for element_id in elements {
                let element_type = tree.get(*element_id);
                visitor.visit_type(tree, *element_id, element_type);
            }
        }
        Type::Tuple(elements) | Type::Intersection(elements) => {
            for element_id in elements {
                let element_type = tree.get(*element_id);
                visitor.visit_type(tree, *element_id, element_type);
            }
        }

        Type::UnevaluatedExpression(expression_id) => {
            let expression = tree.get(*expression_id);
            visitor.visit_expression(tree, *expression_id, expression);
        }
        Type::UnevaluatedSelf => {}
        Type::Error => {}
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
            ty,
            fields,
            value,
        }
        | Variant::Tuple {
            name: _,
            ty,
            fields,
            value,
        } => {
            if let Some(representation_type) = ty {
                let representation_type_node = tree.get(*representation_type);
                visitor.visit_type(tree, *representation_type, representation_type_node);
            }
            for field_id in fields.iter() {
                let field = tree.get(*field_id);
                visitor.visit_field(tree, *field_id, field);
            }
            if let Some(value) = value {
                let value_expression = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expression);
            }
        }
        Variant::Unit { name: _, ty, value } => {
            if let Some(ty_id) = ty {
                let ty = tree.get(*ty_id);
                visitor.visit_type(tree, *ty_id, ty);
            }
            if let Some(value_id) = value {
                let value = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value);
            }
        }
    }
}

/// Walk the Field.
pub fn walk_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Field>,
    field: &Field,
) {
    visitor.visit_any(tree, NodeType::Field, id.id);
    match field {
        Field::Named {
            modifiers: _,
            name: _,
            ty,
            default,
        }
        | Field::Positional {
            modifiers: _,
            ty,
            default,
        } => {
            let ty_node = tree.get(*ty);
            visitor.visit_type(tree, *ty, ty_node);
            if let Some(default) = default {
                let default_expression = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expression);
            }
        }
        Field::Dynamic {
            modifiers: _,
            name: _,
            ty,
            key,
            default,
        } => {
            let ty_node = tree.get(*ty);
            visitor.visit_type(tree, *ty, ty_node);
            let key_node = tree.get(*key);
            visitor.visit_type(tree, *key, key_node);
            if let Some(default) = default {
                let default_expression = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expression);
            }
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
            visitor.visit_expression(tree, *right, right_expression);
        }
        WhereClause::Guard { guard } => {
            let guard_expression = tree.get(*guard);
            visitor.visit_expression(tree, *guard, guard_expression);
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
    visitor.visit_expression(tree, with_clause.right, right_expression);
}

/// Walk the DependencyItem.
pub fn walk_dependency_item<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<DependencyItem>,
    _dependency_item: &DependencyItem,
) {
    visitor.visit_any(tree, NodeType::DependencyItem, id.id);
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
    match parameter {
        Parameter::Named {
            modifiers: _,
            name: _,
            ty,
            default,
        } => {
            if let Some(ty) = ty {
                let type_node = tree.get(*ty);
                visitor.visit_type(tree, *ty, type_node);
            }
            if let Some(default) = default {
                let default_expression = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expression);
            }
        }
        Parameter::Pattern {
            modifiers: _,
            pattern,
            ty,
            default,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            if let Some(ty) = ty {
                let type_node = tree.get(*ty);
                visitor.visit_type(tree, *ty, type_node);
            }
            if let Some(default) = default {
                let default_expression = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expression);
            }
        }
        Parameter::Variadic {
            modifiers: _,
            name: _,
            ty,
        } => {
            if let Some(ty) = ty {
                let type_node = tree.get(*ty);
                visitor.visit_type(tree, *ty, type_node);
            }
        }
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
        Argument::UnevaluatedNamed {
            modifiers: _,
            name: _,
            value,
        }
        | Argument::UnevaluatedPositional {
            modifiers: _,
            value,
        }
        | Argument::UnevaluatedSpread {
            modifiers: _,
            name: _,
            value,
        } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Argument::UnevaluatedDynamic {
            modifiers: _,
            name: _,
            key,
            value,
        } => {
            let key_expression = tree.get(*key);
            visitor.visit_expression(tree, *key, key_expression);
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Argument::Direct {
            modifiers: _,
            name: _,
            slot,
            value,
        }
        | Argument::Spread {
            modifiers: _,
            slot,
            value,
        } => {
            match slot {
                ArgumentSlot::Parameter { parameter } => {
                    let parameter_node = tree.get(*parameter);
                    visitor.visit_parameter(tree, *parameter, parameter_node);
                }
                ArgumentSlot::Field { field } => {
                    let field_node = tree.get(*field);
                    visitor.visit_field(tree, *field, field_node);
                }
            }
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Argument::Dynamic {
            modifiers: _,
            name: _,
            key,
            value,
        } => {
            let key_expression = tree.get(*key);
            visitor.visit_expression(tree, *key, key_expression);
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
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
        Pattern::Wildcard | Pattern::Rest { name: _ } => {}
        Pattern::Maybe(inner) => {
            let inner_pattern = tree.get(*inner);
            visitor.visit_pattern(tree, *inner, inner_pattern);
        }
        Pattern::Reference {
            right: target,
            mutability: _,
        } => {
            let target_pattern = tree.get(*target);
            visitor.visit_pattern(tree, *target, target_pattern);
        }
        Pattern::Binding {
            mutability: _,
            name: _,
            pattern,
        } => {
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, pattern_node);
            }
        }
        Pattern::Expression { value } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Pattern::Range {
            start,
            end,
            is_inclusive: _,
        } => {
            if let Some(start_id) = start {
                let start_pattern = tree.get(*start_id);
                visitor.visit_pattern(tree, *start_id, start_pattern);
            }
            if let Some(end_id) = end {
                let end_pattern = tree.get(*end_id);
                visitor.visit_pattern(tree, *end_id, end_pattern);
            }
        }
        Pattern::Tuple { ty, fields } => {
            if let Some(ty_id) = ty {
                let ty_type = tree.get(*ty_id);
                visitor.visit_type(tree, *ty_id, ty_type);
            }
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_pattern_field(tree, *field_id, field);
            }
        }
        Pattern::Slice { fields } => {
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_pattern_field(tree, *field_id, field);
            }
        }
        Pattern::Struct { ty, fields } => {
            if let Some(ty_id) = ty {
                let ty_type = tree.get(*ty_id);
                visitor.visit_type(tree, *ty_id, ty_type);
            }
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_pattern_field(tree, *field_id, field);
            }
        }
        Pattern::Union { patterns } => {
            for pattern_id in patterns {
                let union_pattern = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, union_pattern);
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
            mutability: _,
            name: _,
            pattern,
            default,
        } => {
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, pattern_node);
            }
            if let Some(default) = default {
                let default_expression = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expression);
            }
        }
        PatternField::Alias {
            mutability: _,
            name: _,
            alias: _,
            default,
        } => {
            if let Some(default) = default {
                let default_expression = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expression);
            }
        }
        PatternField::Positional { pattern } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
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
            visitor.visit_pattern(tree, *pattern, pattern_node);
            let body_expression = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expression);
            if let Some(guard_id) = guard {
                let guard_expression = tree.get(*guard_id);
                visitor.visit_expression(tree, *guard_id, guard_expression);
            }
        }
        MatchCase::Block {
            pattern,
            body,
            guard,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
            if let Some(guard_id) = guard {
                let guard_expression = tree.get(*guard_id);
                visitor.visit_expression(tree, *guard_id, guard_expression);
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
        Annotation::Doc {
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
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
        }
    }
}

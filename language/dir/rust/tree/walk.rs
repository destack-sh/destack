use crate::{
    Annotation, Argument, ArgumentSlot, Block, Definition, Expression, ImportItem, MatchCase,
    NodeId, NodeTree, NodeType, NodeVisitor, Parameter, Pattern, PatternField, TemplateLiteral,
    Type, Variant, VariantField, WhereClause, WithClause,
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
        NodeType::ImportItem => {
            let import_item = tree.import_items.get(local_idx);
            walk_import_item(visitor, tree, NodeId::new(node_id), import_item);
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
        Expression::Import { items } => {
            for item_id in items {
                let item = tree.get(*item_id);
                visitor.visit_import_item(tree, *item_id, item);
            }
        }
        Expression::Export { mode: _, items } => {
            for item_id in items {
                let item = tree.get(*item_id);
                visitor.visit_import_item(tree, *item_id, item);
            }
        }
        Expression::Let {
            mutability: _,
            visibility: _,
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
            visibility: _,
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
        Expression::Type {
            mutability: _,
            value,
        } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Expression::Unary {
            operator: _,
            expression: right,
        }
        | Expression::Reference {
            mutability: _,
            right,
        } => {
            let right_expression = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expression);
        }
        Expression::Dynamic {
            mutability: _,
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
        Expression::Member { left, path: _ } => {
            let left_expression = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expression);
        }
        Expression::Call {
            runtime: _,
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
        Expression::Path { path: _ } => {
            // nothing to do
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
                visitor.visit_expression(tree, *element_id, element);
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
            runtime: _,
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
            runtime: _,
            condition,
            body,
            source: _,
        } => {
            let condition_expression = tree.get(*condition);
            visitor.visit_expression(tree, *condition, condition_expression);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }
        Expression::Match {
            runtime: _,
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
        Expression::Break {
            destination: _,
            value,
        } => {
            if let Some(value_id) = value {
                let value_expression = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_expression);
            }
        }
        Expression::Continue { destination: _ } => {}
        Expression::Defer { expression } => {
            let body_expression = tree.get(*expression);
            visitor.visit_expression(tree, *expression, body_expression);
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
        Definition::Import { items } => {
            for item_id in items {
                let item = tree.get(*item_id);
                visitor.visit_import_item(tree, *item_id, item);
            }
        }
        Definition::Export { mode: _, items } => {
            for item_id in items {
                let item = tree.get(*item_id);
                visitor.visit_import_item(tree, *item_id, item);
            }
        }
        Definition::Let {
            name: _,
            export: _,
            visibility: _,
        } => {}
        Definition::Type {
            name: _,
            export: _,
            visibility: _,
            value,
        } => {
            let value_type = tree.get(*value);
            visitor.visit_type(tree, *value, value_type);
        }
        Definition::Module {
            kind: _,
            name: _,
            export: _,
            visibility: _,
            with_clauses,
            where_clauses,
            definitions,
        } => {
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_with_clause(tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_where_clause(tree, *clause_id, clause);
                }
            }
            for definition_id in definitions {
                let child_definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, child_definition);
            }
        }
        Definition::Struct {
            kind: _,
            name: _,
            export: _,
            visibility: _,
            static_parameters,
            embedded_definitions,
            variant,
            with_clauses,
            where_clauses,
            definitions,
        } => {
            if let Some(static_parameters) = static_parameters {
                for parameter_id in static_parameters.iter() {
                    let parameter = tree.get(*parameter_id);
                    visitor.visit_parameter(tree, *parameter_id, parameter);
                }
            }
            for embedded_definition in embedded_definitions.iter() {
                let ty_id = embedded_definition.ty();
                let ty = tree.get(ty_id);
                visitor.visit_type(tree, ty_id, ty);
            }
            let variant_node = tree.get(*variant);
            visitor.visit_variant(tree, *variant, variant_node);
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_with_clause(tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_where_clause(tree, *clause_id, clause);
                }
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, child_definition);
            }
        }
        Definition::Enum {
            kind: _,
            name: _,
            export: _,
            visibility: _,
            static_parameters,
            embedded_definitions,
            variants,
            with_clauses,
            where_clauses,
            definitions,
        } => {
            if let Some(static_parameters) = static_parameters {
                for parameter_id in static_parameters.iter() {
                    let parameter = tree.get(*parameter_id);
                    visitor.visit_parameter(tree, *parameter_id, parameter);
                }
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
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_with_clause(tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_where_clause(tree, *clause_id, clause);
                }
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, child_definition);
            }
        }
        Definition::Union {
            kind: _,
            name: _,
            export: _,
            visibility: _,
            static_parameters,
            embedded_definitions,
            variants,
            with_clauses,
            where_clauses,
            definitions,
        } => {
            if let Some(static_parameters) = static_parameters {
                for parameter_id in static_parameters.iter() {
                    let parameter = tree.get(*parameter_id);
                    visitor.visit_parameter(tree, *parameter_id, parameter);
                }
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
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_with_clause(tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_where_clause(tree, *clause_id, clause);
                }
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, child_definition);
            }
        }
        Definition::Interface {
            kind: _,
            name: _,
            export: _,
            visibility: _,
            static_parameters,
            embedded_definitions,
            with_clauses,
            where_clauses,
            fields,
            definitions,
        } => {
            if let Some(static_parameters) = static_parameters {
                for parameter_id in static_parameters.iter() {
                    let parameter = tree.get(*parameter_id);
                    visitor.visit_parameter(tree, *parameter_id, parameter);
                }
            }
            for embedded_definition in embedded_definitions.iter() {
                let ty_id = embedded_definition.ty();
                let ty = tree.get(ty_id);
                visitor.visit_type(tree, ty_id, ty);
            }
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_with_clause(tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_where_clause(tree, *clause_id, clause);
                }
            }
            for field_id in fields.iter() {
                let field = tree.get(*field_id);
                visitor.visit_variant_field(tree, *field_id, field);
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, child_definition);
            }
        }
        Definition::Function {
            kind: _,
            name: _,
            export: _,
            visibility: _,
            runtime: _,
            cardinality: _,
            accessor: _,
            style: _,
            static_parameters,
            self_parameter: _,
            dynamic_parameters,
            return_type,
            with_clauses,
            where_clauses,
            definitions,
            body,
        } => {
            if let Some(static_parameters) = static_parameters {
                for parameter_id in static_parameters.iter() {
                    let parameter = tree.get(*parameter_id);
                    visitor.visit_parameter(tree, *parameter_id, parameter);
                }
            }
            for parameter_id in dynamic_parameters.iter() {
                let parameter = tree.get(*parameter_id);
                visitor.visit_parameter(tree, *parameter_id, parameter);
            }
            if let Some(return_type) = return_type {
                let return_type_node = tree.get(*return_type);
                visitor.visit_type(tree, *return_type, return_type_node);
            }
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_with_clause(tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_where_clause(tree, *clause_id, clause);
                }
            }
            for definition_id in definitions.iter() {
                let child_definition = tree.get(*definition_id);
                visitor.visit_definition(tree, *definition_id, child_definition);
            }
            if let Some(body) = body {
                let body_expression = tree.get(*body);
                visitor.visit_expression(tree, *body, body_expression);
            }
        }
        Definition::Implement {
            kind: _,
            export: _,
            visibility: _,
            static_parameters,
            target_type,
            super_types,
            with_clauses,
            where_clauses,
            definitions,
        } => {
            if let Some(static_parameters) = static_parameters {
                for parameter_id in static_parameters.iter() {
                    let parameter = tree.get(*parameter_id);
                    visitor.visit_parameter(tree, *parameter_id, parameter);
                }
            }
            let target_type_expr = tree.get(*target_type);
            visitor.visit_type(tree, *target_type, target_type_expr);
            if let Some(super_types) = super_types {
                for super_type in super_types {
                    let super_type_expr = tree.get(*super_type);
                    visitor.visit_type(tree, *super_type, super_type_expr);
                }
            }
            if let Some(with_clauses) = with_clauses {
                for clause_id in with_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_with_clause(tree, *clause_id, clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for clause_id in where_clauses.iter() {
                    let clause = tree.get(*clause_id);
                    visitor.visit_where_clause(tree, *clause_id, clause);
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
        Type::Maybe(right)
        | Type::Not(right)
        | Type::Must(right)
        | Type::Mutable {
            mutability: _,
            target: right,
        }
        | Type::Reference {
            mutability: _,
            target: right,
        }
        | Type::Dynamic {
            mutability: _,
            target: right,
        } => {
            let target_type = tree.get(*right);
            visitor.visit_type(tree, *right, target_type);
        }
        Type::Definition(definition_id) => {
            let definition = tree.get(*definition_id);
            visitor.visit_definition(tree, *definition_id, definition);
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
        Type::Array { element, count } => {
            let element_type = tree.get(*element);
            visitor.visit_type(tree, *element, element_type);
            let count_expression = tree.get(*count);
            visitor.visit_expression(tree, *count, count_expression);
        }
        Type::Slice { element } => {
            let element_type = tree.get(*element);
            visitor.visit_type(tree, *element, element_type);
        }
        Type::Tuple(elements) | Type::Intersection(elements) => {
            for element_id in elements {
                let element_type = tree.get(*element_id);
                visitor.visit_type(tree, *element_id, element_type);
            }
        }

        Type::UnresolvedExpression(expression_id) => {
            let expression = tree.get(*expression_id);
            visitor.visit_expression(tree, *expression_id, expression);
        }
        Type::UnresolvedSelf => {}
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
            ty: representation_type,
            fields,
            value,
        }
        | Variant::Tuple {
            name: _,
            ty: representation_type,
            fields,
            value,
        } => {
            if let Some(representation_type) = representation_type {
                let representation_type_node = tree.get(*representation_type);
                visitor.visit_type(tree, *representation_type, representation_type_node);
            }
            for field_id in fields.iter() {
                let field = tree.get(*field_id);
                visitor.visit_variant_field(tree, *field_id, field);
            }
            if let Some(value) = value {
                let value_expression = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expression);
            }
        }
        Variant::Unit {
            name: _,
            ty: representation_type,
            value,
        } => {
            if let Some(representation_type_id) = representation_type {
                let representation_type = tree.get(*representation_type_id);
                visitor.visit_type(tree, *representation_type_id, representation_type);
            }
            if let Some(value) = value {
                let value_expression = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expression);
            }
        }
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
        VariantField::Named {
            visibility: _,
            mutability: _,
            name: _,
            ty,
            default,
        }
        | VariantField::Positional {
            visibility: _,
            mutability: _,
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
        VariantField::Dynamic {
            visibility: _,
            mutability: _,
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

/// Walk the UseItem.
pub fn walk_import_item<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<ImportItem>,
    _import_item: &ImportItem,
) {
    visitor.visit_any(tree, NodeType::ImportItem, id.id);
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
        Parameter::Variadic { name: _, ty } => {
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
        Argument::UnresolvedNamed { name: _, value }
        | Argument::UnresolvedPositional { value }
        | Argument::UnresolvedSpread { value } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Argument::UnresolvedDynamic {
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
            name: _,
            slot,
            value,
        }
        | Argument::Spread { slot, value } => {
            match slot {
                ArgumentSlot::Parameter { parameter } => {
                    let parameter_node = tree.get(*parameter);
                    visitor.visit_parameter(tree, *parameter, parameter_node);
                }
                ArgumentSlot::Field { field } => {
                    let field_node = tree.get(*field);
                    visitor.visit_variant_field(tree, *field, field_node);
                }
            }
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Argument::Dynamic {
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
        PatternField::NamedAlias {
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

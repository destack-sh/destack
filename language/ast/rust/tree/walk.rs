use crate::{
    Annotation, Argument, Blank, Block, Comment, Decorator, Definition, Doc, EnumField, Expression,
    ImportClause, ImportItem, MatchCase, NodeId, NodeTree, NodeType, NodeVisitor, Parameter,
    Pattern, PatternField, Tag, UnionField, VariantField, WhereClause, WithClause,
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
        NodeType::VariantField => {
            let variant_field = tree.variant_fields.get(local_idx);
            walk_variant_field(visitor, tree, NodeId::new(node_id), variant_field);
        }
        NodeType::EnumField => {
            let enum_field = tree.enum_fields.get(local_idx);
            walk_enum_field(visitor, tree, NodeId::new(node_id), enum_field);
        }
        NodeType::UnionField => {
            let union_field = tree.union_fields.get(local_idx);
            walk_union_field(visitor, tree, NodeId::new(node_id), union_field);
        }
        // --------------------------------------------------------------------
        // Context
        // --------------------------------------------------------------------
        NodeType::WithClause => {
            let with_clause = tree.with_clauses.get(local_idx);
            walk_with_clause(visitor, tree, NodeId::new(node_id), with_clause);
        }
        NodeType::WhereClause => {
            let where_clause = tree.where_clauses.get(local_idx);
            walk_where_clause(visitor, tree, NodeId::new(node_id), where_clause);
        }
        NodeType::ImportClause => {
            let import_clause = tree.import_clauses.get(local_idx);
            walk_import_clause(visitor, tree, NodeId::new(node_id), import_clause);
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
        NodeType::Blank => {
            let blank = tree.blanks.get(local_idx);
            walk_blank(visitor, tree, NodeId::new(node_id), blank);
        }
        NodeType::Doc => {
            let doc = tree.docs.get(local_idx);
            walk_doc(visitor, tree, NodeId::new(node_id), doc);
        }
        NodeType::Comment => {
            let comment = tree.comments.get(local_idx);
            walk_comment(visitor, tree, NodeId::new(node_id), comment);
        }
        NodeType::Tag => {
            let tag = tree.tags.get(local_idx);
            walk_tag(visitor, tree, NodeId::new(node_id), tag);
        }
        NodeType::Decorator => {
            let decorator = tree.decorators.get(local_idx);
            walk_decorator(visitor, tree, NodeId::new(node_id), decorator);
        }
    }
}

// ----------------------------------------------------------------------------
// Groupings
// ----------------------------------------------------------------------------

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

/// Walk the Expression.
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
            visitor.visit_definition(tree, *definition_id, definition);
        }

        Expression::Block(block_id) => {
            let block = tree.get(*block_id);
            visitor.visit_block(tree, *block_id, block);
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

        Expression::Import { clauses } => {
            for clause_id in clauses {
                let clause = tree.get(*clause_id);
                visitor.visit_import_clause(tree, *clause_id, clause);
            }
        }

        Expression::Export { mode: _, clauses } => {
            for clause_id in clauses {
                let clause = tree.get(*clause_id);
                visitor.visit_import_clause(tree, *clause_id, clause);
            }
        }

        Expression::Let {
            mutability: _,
            visibility: _,
            export: _,
            pattern,
            ty,
            value,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            if let Some(type_id) = ty {
                let type_expr = tree.get(*type_id);
                visitor.visit_expression(tree, *type_id, type_expr);
            }
            if let Some(value_id) = value {
                let value_expr = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_expr);
            }
        }

        Expression::Type {
            name: _,
            static_parameters,
            visibility: _,
            export: _,
            value,
        } => {
            if let Some(static_parameters) = static_parameters {
                for parameter_id in static_parameters {
                    let parameter = tree.get(*parameter_id);
                    visitor.visit_parameter(tree, *parameter_id, parameter);
                }
            }
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }

        Expression::If {
            runtime: _,
            style: _,
            condition,
            then_expression,
            else_expression,
        } => {
            let cond_expr = tree.get(*condition);
            visitor.visit_expression(tree, *condition, cond_expr);
            let then_expression_node = tree.get(*then_expression);
            visitor.visit_expression(tree, *then_expression, then_expression_node);
            if let Some(else_expression_id) = else_expression {
                let else_expression_node = tree.get(*else_expression_id);
                visitor.visit_expression(tree, *else_expression_id, else_expression_node);
            }
        }

        Expression::While {
            runtime: _,
            condition,
            body,
        } => {
            let cond_expr = tree.get(*condition);
            visitor.visit_expression(tree, *condition, cond_expr);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }

        Expression::For {
            runtime: _,
            pattern,
            iterator,
            body,
        } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
            let iterator_expr = tree.get(*iterator);
            visitor.visit_expression(tree, *iterator, iterator_expr);
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }

        Expression::Loop { runtime: _, body } => {
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }

        Expression::Try {
            runtime: _,
            try_block: r#try,
            catch_block: catch,
        } => {
            let try_expr_node = tree.get(*r#try);
            visitor.visit_expression(tree, *r#try, try_expr_node);
            if let Some(catch_id) = catch {
                let catch_expr = tree.get(*catch_id);
                visitor.visit_expression(tree, *catch_id, catch_expr);
            }
        }

        Expression::Match {
            runtime: _,
            value,
            cases,
        } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
            for case_id in cases {
                let case = tree.get(*case_id);
                visitor.visit_match_case(tree, *case_id, case);
            }
        }

        Expression::Break { label: _, value } => {
            if let Some(value_id) = value {
                let value_expr = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_expr);
            }
        }

        Expression::Continue { label: _ } => {}

        Expression::Defer { expression, catch } => {
            if let Some(expr_id) = expression {
                let expr = tree.get(*expr_id);
                visitor.visit_expression(tree, *expr_id, expr);
            }
            if let Some(catch_id) = catch {
                let catch_expr = tree.get(*catch_id);
                visitor.visit_expression(tree, *catch_id, catch_expr);
            }
        }

        Expression::Return { value } => {
            if let Some(value_id) = value {
                let value_expr = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_expr);
            }
        }

        Expression::Path {
            path: _,
            static_arguments,
        } => {
            if let Some(arguments) = static_arguments {
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
        }

        Expression::ScalarLiteral(_) => {
            // no child nodes to visit
        }

        Expression::TypeLiteral(_) => {
            // no child nodes to visit
        }

        Expression::RangeLiteral {
            start,
            end,
            is_inclusive: _,
        } => {
            let start_expr = tree.get(*start);
            visitor.visit_expression(tree, *start, start_expr);
            let end_expr = tree.get(*end);
            visitor.visit_expression(tree, *end, end_expr);
        }

        Expression::ArrayLiteral { elements } => {
            for element_id in elements {
                let element_expr = tree.get(*element_id);
                visitor.visit_expression(tree, *element_id, element_expr);
            }
        }

        Expression::TupleLiteral { elements } => {
            for argument_id in elements {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }
        }

        Expression::StructLiteral { ty, fields } => {
            if let Some(type_id) = ty {
                let type_expr = tree.get(*type_id);
                visitor.visit_expression(tree, *type_id, type_expr);
            }
            for field_id in fields {
                let field_arg = tree.get(*field_id);
                visitor.visit_argument(tree, *field_id, field_arg);
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

        Expression::Parenthesized { expression } => {
            let expr = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expr);
        }

        Expression::Unary { operator: _, right } => {
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::Reference {
            mutability: _,
            right,
        } => {
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::Member { receiver, path: _ } => {
            let receiver_expr = tree.get(*receiver);
            visitor.visit_expression(tree, *receiver, receiver_expr);
        }

        Expression::Index { receiver, index } => {
            let receiver_expr = tree.get(*receiver);
            visitor.visit_expression(tree, *receiver, receiver_expr);
            if let Some(index_id) = index {
                let index_expr = tree.get(*index_id);
                visitor.visit_expression(tree, *index_id, index_expr);
            }
        }

        Expression::Call {
            runtime: _,
            receiver,
            dynamic_arguments,
        } => {
            let receiver_expr = tree.get(*receiver);
            visitor.visit_expression(tree, *receiver, receiver_expr);
            for argument_id in dynamic_arguments {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }
        }

        Expression::Maybe(expr_id) => {
            let expr = tree.get(*expr_id);
            visitor.visit_expression(tree, *expr_id, expr);
        }

        Expression::Must(expr_id) => {
            let expr = tree.get(*expr_id);
            visitor.visit_expression(tree, *expr_id, expr);
        }

        Expression::Binary {
            left,
            operator: _,
            right,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::Assign {
            left,
            operator: _,
            right,
        } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::Error => {}
    }
}

// ----------------------------------------------------------------------------
// Declarations
// ----------------------------------------------------------------------------

/// Walk the Definition and visit all child nodes.
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
            export: _,
            format: _,
            with_clauses,
            where_clauses,
            expressions,
        } => {
            for expr_id in expressions {
                let expr = tree.get(*expr_id);
                visitor.visit_expression(tree, *expr_id, expr);
            }
            if let Some(with_clauses) = with_clauses {
                for with_id in with_clauses {
                    let with_clause = tree.get(*with_id);
                    visitor.visit_with_clause(tree, *with_id, with_clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for where_id in where_clauses {
                    let where_clause = tree.get(*where_id);
                    visitor.visit_where_clause(tree, *where_id, where_clause);
                }
            }
        }
        Definition::Struct {
            name: _,
            visibility: _,
            style: _,
            export: _,
            super_types,
            representation_type,
            static_parameters,
            with_clauses,
            where_clauses,
            fields,
            expressions,
        } => {
            if let Some(super_types) = super_types {
                for super_type_id in super_types {
                    let expr = tree.get(*super_type_id);
                    visitor.visit_expression(tree, *super_type_id, expr);
                }
            }
            if let Some(representation_type) = representation_type {
                let expr = tree.get(*representation_type);
                visitor.visit_expression(tree, *representation_type, expr);
            }
            if let Some(static_parameters) = static_parameters {
                for param_id in static_parameters {
                    let param = tree.get(*param_id);
                    visitor.visit_parameter(tree, *param_id, param);
                }
            }
            if let Some(with_clauses) = with_clauses {
                for with_id in with_clauses {
                    let with_clause = tree.get(*with_id);
                    visitor.visit_with_clause(tree, *with_id, with_clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for where_id in where_clauses {
                    let where_clause = tree.get(*where_id);
                    visitor.visit_where_clause(tree, *where_id, where_clause);
                }
            }
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_variant_field(tree, *field_id, field);
            }
            for expr_id in expressions {
                let expr = tree.get(*expr_id);
                visitor.visit_expression(tree, *expr_id, expr);
            }
        }
        Definition::Enum {
            name: _,
            visibility: _,
            export: _,
            tag_type,
            static_parameters,
            super_types,
            with_clauses,
            where_clauses,
            fields,
            expressions,
        } => {
            if let Some(tag_type) = tag_type {
                let expr = tree.get(*tag_type);
                visitor.visit_expression(tree, *tag_type, expr);
            }
            if let Some(static_parameters) = static_parameters {
                for param_id in static_parameters {
                    let param = tree.get(*param_id);
                    visitor.visit_parameter(tree, *param_id, param);
                }
            }
            if let Some(super_types) = super_types {
                for super_type_id in super_types {
                    let expr = tree.get(*super_type_id);
                    visitor.visit_expression(tree, *super_type_id, expr);
                }
            }
            if let Some(with_clauses) = with_clauses {
                for with_id in with_clauses {
                    let with_clause = tree.get(*with_id);
                    visitor.visit_with_clause(tree, *with_id, with_clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for where_id in where_clauses {
                    let where_clause = tree.get(*where_id);
                    visitor.visit_where_clause(tree, *where_id, where_clause);
                }
            }
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_enum_field(tree, *field_id, field);
            }
            for expr_id in expressions {
                let expr = tree.get(*expr_id);
                visitor.visit_expression(tree, *expr_id, expr);
            }
        }
        Definition::Union {
            name: _,
            visibility: _,
            export: _,
            tag_type,
            representation_type,
            static_parameters,
            super_types,
            with_clauses: with,
            where_clauses,
            fields,
            expressions,
        } => {
            if let Some(tag_type) = tag_type {
                let expr = tree.get(*tag_type);
                visitor.visit_expression(tree, *tag_type, expr);
            }
            if let Some(representation_type) = representation_type {
                let expr = tree.get(*representation_type);
                visitor.visit_expression(tree, *representation_type, expr);
            }
            if let Some(static_parameters) = static_parameters {
                for param_id in static_parameters {
                    let param = tree.get(*param_id);
                    visitor.visit_parameter(tree, *param_id, param);
                }
            }
            if let Some(super_types) = super_types {
                for super_type_id in super_types {
                    let expr = tree.get(*super_type_id);
                    visitor.visit_expression(tree, *super_type_id, expr);
                }
            }
            if let Some(with_clauses) = with {
                for with_id in with_clauses {
                    let with_clause = tree.get(*with_id);
                    visitor.visit_with_clause(tree, *with_id, with_clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for where_id in where_clauses {
                    let where_clause = tree.get(*where_id);
                    visitor.visit_where_clause(tree, *where_id, where_clause);
                }
            }
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_union_field(tree, *field_id, field);
            }
            for expr_id in expressions {
                let expr = tree.get(*expr_id);
                visitor.visit_expression(tree, *expr_id, expr);
            }
        }
        Definition::Interface {
            name: _,
            visibility: _,
            export: _,
            super_types,
            static_parameters,
            with_clauses,
            where_clauses,
            expressions,
        } => {
            if let Some(super_types) = super_types {
                for super_type_id in super_types {
                    let expr = tree.get(*super_type_id);
                    visitor.visit_expression(tree, *super_type_id, expr);
                }
            }
            if let Some(static_parameters) = static_parameters {
                for param_id in static_parameters {
                    let param = tree.get(*param_id);
                    visitor.visit_parameter(tree, *param_id, param);
                }
            }
            if let Some(with_clauses) = with_clauses {
                for with_id in with_clauses {
                    let with_clause = tree.get(*with_id);
                    visitor.visit_with_clause(tree, *with_id, with_clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for where_id in where_clauses {
                    let where_clause = tree.get(*where_id);
                    visitor.visit_where_clause(tree, *where_id, where_clause);
                }
            }
            for expr_id in expressions {
                let expr = tree.get(*expr_id);
                visitor.visit_expression(tree, *expr_id, expr);
            }
        }
        Definition::Implement {
            static_parameters,
            target_type,
            super_type,
            with_clauses,
            where_clauses,
            expressions,
        } => {
            if let Some(static_parameters) = static_parameters {
                for argument_id in static_parameters {
                    let argument = tree.get(*argument_id);
                    visitor.visit_parameter(tree, *argument_id, argument);
                }
            }
            let target_type_expr = tree.get(*target_type);
            visitor.visit_expression(tree, *target_type, target_type_expr);
            if let Some(super_type) = super_type {
                let super_type_expr = tree.get(*super_type);
                visitor.visit_expression(tree, *super_type, super_type_expr);
            }
            if let Some(with_clauses) = with_clauses {
                for with_id in with_clauses {
                    let with_clause = tree.get(*with_id);
                    visitor.visit_with_clause(tree, *with_id, with_clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for where_id in where_clauses {
                    let where_clause = tree.get(*where_id);
                    visitor.visit_where_clause(tree, *where_id, where_clause);
                }
            }
            for expr_id in expressions {
                let expr = tree.get(*expr_id);
                visitor.visit_expression(tree, *expr_id, expr);
            }
        }
        Definition::Function {
            name: _,
            visibility: _,
            export: _,
            runtime: _,
            style: _,
            static_parameters,
            self_parameter: _,
            dynamic_parameters,
            return_type,
            with_clauses,
            where_clauses,
            body,
        } => {
            if let Some(static_parameters) = static_parameters {
                for param_id in static_parameters {
                    let param = tree.get(*param_id);
                    visitor.visit_parameter(tree, *param_id, param);
                }
            }
            for param_id in dynamic_parameters {
                let param = tree.get(*param_id);
                visitor.visit_parameter(tree, *param_id, param);
            }
            if let Some(return_type) = return_type {
                let expr = tree.get(*return_type);
                visitor.visit_expression(tree, *return_type, expr);
            }
            if let Some(with_clauses) = with_clauses {
                for with_id in with_clauses {
                    let with_clause = tree.get(*with_id);
                    visitor.visit_with_clause(tree, *with_id, with_clause);
                }
            }
            if let Some(where_clauses) = where_clauses {
                for where_id in where_clauses {
                    let where_clause = tree.get(*where_id);
                    visitor.visit_where_clause(tree, *where_id, where_clause);
                }
            }
            if let Some(body_id) = body {
                let expression = tree.get(*body_id);
                visitor.visit_expression(tree, *body_id, expression);
            }
        }
    }
}

/// Walk the VariantField.
pub fn walk_variant_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<VariantField>,
    field: &VariantField,
) {
    visitor.visit_any(tree, NodeType::VariantField, id.id);
    let type_node = tree.get(field.ty);
    visitor.visit_expression(tree, field.ty, type_node);

    if let Some(default) = &field.default {
        let expression = tree.get(*default);
        visitor.visit_expression(tree, *default, expression);
    }
}

/// Walk the EnumField.
pub fn walk_enum_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<EnumField>,
    field: &EnumField,
) {
    visitor.visit_any(tree, NodeType::EnumField, id.id);
    if let Some(value) = &field.value {
        let expression = tree.get(*value);
        visitor.visit_expression(tree, *value, expression);
    }
}

/// Walk the UnionField.
pub fn walk_union_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<UnionField>,
    field: &UnionField,
) {
    visitor.visit_any(tree, NodeType::UnionField, id.id);
    match field {
        UnionField::Unit { name: _, value } => {
            if let Some(value) = value {
                let expression = tree.get(*value);
                visitor.visit_expression(tree, *value, expression);
            }
        }
        UnionField::Tuple {
            name: _,
            fields,
            value,
        } => {
            for field in fields {
                let field_node = tree.get(*field);
                visitor.visit_variant_field(tree, *field, field_node);
            }
            if let Some(value) = value {
                let expression = tree.get(*value);
                visitor.visit_expression(tree, *value, expression);
            }
        }
        UnionField::Struct {
            name: _,
            fields,
            value,
        } => {
            for field in fields {
                let field_node = tree.get(*field);
                visitor.visit_variant_field(tree, *field, field_node);
            }
            if let Some(value) = value {
                let expression = tree.get(*value);
                visitor.visit_expression(tree, *value, expression);
            }
        }
    };
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
    let right = tree.get(with_clause.right);
    visitor.visit_expression(tree, with_clause.right, right);
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

/// Walk the UseClause.
pub fn walk_import_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<ImportClause>,
    import_clause: &ImportClause,
) {
    visitor.visit_any(tree, NodeType::ImportClause, id.id);
    if let Some(items) = &import_clause.items {
        for item_id in items {
            let item = tree.get(*item_id);
            visitor.visit_import_item(tree, *item_id, item);
        }
    }
}

/// Walk the UseItem.
pub fn walk_import_item<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    _tree: &NodeTree,
    id: NodeId<ImportItem>,
    _import_item: &ImportItem,
) {
    visitor.visit_any(_tree, NodeType::ImportItem, id.id);
    // UseItem has no child nodes to visit (only StringId fields)
}

/// Walk the Parameter.
pub fn walk_parameter<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Parameter>,
    parameter: &Parameter,
) {
    visitor.visit_any(tree, NodeType::Parameter, id.id);
    match parameter {
        Parameter::Scalar {
            name: _,
            ty,
            default,
        } => {
            if let Some(ty) = ty {
                let type_node = tree.get(*ty);
                visitor.visit_expression(tree, *ty, type_node);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        Parameter::Variadic { name: _, ty } => {
            if let Some(ty) = ty {
                let type_node = tree.get(*ty);
                visitor.visit_expression(tree, *ty, type_node);
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
        Argument::Named { name: _, value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Argument::NamedShorthand { name: _ } => {
            // no child nodes to visit
        }
        Argument::Positional { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Argument::Spread { value } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
    }
}

// ----------------------------------------------------------------------------
// Patterns
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
        Pattern::Wildcard => {
            // no child nodes to visit
        }
        Pattern::Rest => {
            // no child nodes to visit
        }
        Pattern::Maybe(unwrap) => {
            let unwrap_pattern = tree.get(*unwrap);
            visitor.visit_pattern(tree, *unwrap, unwrap_pattern);
        }
        Pattern::Reference {
            right: target,
            mutability: _,
        } => {
            let target_pattern = tree.get(*target);
            visitor.visit_pattern(tree, *target, target_pattern);
        }
        Pattern::Binding { name: _, pattern } => {
            if let Some(pattern_id) = pattern {
                let pattern_node = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, pattern_node);
            }
        }
        Pattern::Expression { value } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Pattern::Range {
            start,
            end,
            is_inclusive: _,
        } => {
            if let Some(start_pattern) = start {
                let start_node = tree.get(*start_pattern);
                visitor.visit_pattern(tree, *start_pattern, start_node);
            }
            if let Some(end_pattern) = end {
                let end_node = tree.get(*end_pattern);
                visitor.visit_pattern(tree, *end_pattern, end_node);
            }
        }
        Pattern::Tuple { ty, fields } => {
            if let Some(ty_id) = ty {
                let ty_expression = tree.get(*ty_id);
                visitor.visit_expression(tree, *ty_id, ty_expression);
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
            if let Some(type_id) = ty {
                let type_node = tree.get(*type_id);
                visitor.visit_expression(tree, *type_id, type_node);
            }
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_pattern_field(tree, *field_id, field);
            }
        }
        Pattern::Union { patterns } => {
            for pattern_id in patterns {
                let pattern = tree.get(*pattern_id);
                visitor.visit_pattern(tree, *pattern_id, pattern);
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
                visitor.visit_pattern(tree, *pattern_id, pattern_node);
            }
        }
        PatternField::NamedAlias {
            name: _,
            alias: _,
            mutability: _,
        } => {
            // no child nodes to visit
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
            let body_expr = tree.get(*body);
            visitor.visit_expression(tree, *body, body_expr);
            if let Some(guard_expr) = guard {
                let guard_node = tree.get(*guard_expr);
                visitor.visit_expression(tree, *guard_expr, guard_node);
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
            if let Some(guard_expr) = guard {
                let guard_node = tree.get(*guard_expr);
                visitor.visit_expression(tree, *guard_expr, guard_node);
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
        Annotation::Blank { node, position: _ } => {
            visitor.visit_blank(tree, *node, tree.get(*node));
        }
        Annotation::Doc { node, position: _ } => {
            visitor.visit_doc(tree, *node, tree.get(*node));
        }
        Annotation::Comment { node, position: _ } => {
            visitor.visit_comment(tree, *node, tree.get(*node));
        }
        Annotation::Tag { node, position: _ } => {
            visitor.visit_tag(tree, *node, tree.get(*node));
        }
        Annotation::Decorator { node, position: _ } => {
            visitor.visit_decorator(tree, *node, tree.get(*node));
        }
    }
}

/// Walk the Blank.
pub fn walk_blank<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Blank>,
    _blank: &Blank,
) {
    visitor.visit_any(tree, NodeType::Blank, id.id);
}

/// Walk the Doc.
pub fn walk_doc<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Doc>,
    _doc: &Doc,
) {
    visitor.visit_any(tree, NodeType::Doc, id.id);
}

/// Walk the Comment.
pub fn walk_comment<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Comment>,
    _comment: &Comment,
) {
    visitor.visit_any(tree, NodeType::Comment, id.id);
}

/// Walk the Tag.
pub fn walk_tag<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Tag>,
    tag: &Tag,
) {
    visitor.visit_any(tree, NodeType::Tag, id.id);
    if let Some(arguments) = &tag.arguments {
        for argument in arguments {
            let argument_node = tree.get(*argument);
            visitor.visit_argument(tree, *argument, argument_node);
        }
    }
}

/// Walk the Decorator.
pub fn walk_decorator<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: NodeId<Decorator>,
    decorator: &Decorator,
) {
    visitor.visit_any(tree, NodeType::Decorator, id.id);
    if let Some(arguments) = &decorator.arguments {
        for argument in arguments {
            let argument_node = tree.get(*argument);
            visitor.visit_argument(tree, *argument, argument_node);
        }
    }
}

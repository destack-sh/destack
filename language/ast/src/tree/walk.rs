use crate::{
    Annotation, Argument, Blank, Block, Comment, Decorator, Definition, DefinitionMeta,
    DependencyItem, Doc, EnumField, Expression, Key, MatchCase, MutableNodeTree, NodeId, NodeType,
    NodeVisitor, Parameter, Pattern, PatternField, Property, Tag, TemplateLiteral, WhereClause,
    WithClause,
};

/// Walk any node.
pub fn walk_any<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &MutableNodeTree,
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
        NodeType::Property => {
            let property = tree.properties.get(local_idx);
            walk_property(visitor, tree, NodeId::new(node_id), property);
        }
        NodeType::EnumField => {
            let enum_field = tree.enum_fields.get(local_idx);
            walk_enum_field(visitor, tree, NodeId::new(node_id), enum_field);
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
    tree: &MutableNodeTree,
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
    tree: &MutableNodeTree,
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

        Expression::Statement(statement_id) => {
            let statement = tree.get(*statement_id);
            visitor.visit_expression(tree, *statement_id, statement);
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
            target: _,
            alias: _,
            items,
            arguments,
        } => {
            if let Some(items) = items {
                for item_id in items {
                    let item = tree.get(*item_id);
                    visitor.visit_dependency_item(tree, *item_id, item);
                }
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
            target: _,
            alias: _,
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
            meta: _,
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

        Expression::LetType {
            kind: _,
            mutability: _,
            meta: _,
            static_parameters,
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
            kind: _,
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
            kind: _,
            condition,
            body,
        } => {
            let cond_expr = tree.get(*condition);
            visitor.visit_expression(tree, *condition, cond_expr);
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
            let iterator_expr = tree.get(*iterator);
            visitor.visit_expression(tree, *iterator, iterator_expr);
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
                let initialization_expr = tree.get(*initialization_id);
                visitor.visit_expression(tree, *initialization_id, initialization_expr);
            }
            if let Some(condition_id) = condition {
                let condition_expr = tree.get(*condition_id);
                visitor.visit_expression(tree, *condition_id, condition_expr);
            }
            if let Some(increment_id) = increment {
                let increment_expr = tree.get(*increment_id);
                visitor.visit_expression(tree, *increment_id, increment_expr);
            }
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }

        Expression::Loop { body } => {
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }

        Expression::Try {
            try_expression,
            catch_pattern,
            catch_expression,
            finally_expression,
        } => {
            let try_expr_node = tree.get(*try_expression);
            visitor.visit_expression(tree, *try_expression, try_expr_node);
            if let Some(catch_pattern_id) = catch_pattern {
                let catch_pattern_node = tree.get(*catch_pattern_id);
                visitor.visit_pattern(tree, *catch_pattern_id, catch_pattern_node);
            }
            if let Some(catch_id) = catch_expression {
                let catch_expr = tree.get(*catch_id);
                visitor.visit_expression(tree, *catch_id, catch_expr);
            }
            if let Some(finally_id) = finally_expression {
                let finally_expr = tree.get(*finally_id);
                visitor.visit_expression(tree, *finally_id, finally_expr);
            }
        }

        Expression::Match {
            kind: _,
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

        Expression::Defer { expression } => {
            if let Some(expr_id) = expression {
                let expr = tree.get(*expr_id);
                visitor.visit_expression(tree, *expr_id, expr);
            }
        }

        Expression::Await { expression } => {
            let expression_node = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_node);
        }

        Expression::Yield {
            cardinality: _,
            value,
        } => {
            let value_node = tree.get(*value);
            visitor.visit_expression(tree, *value, value_node);
        }

        Expression::Throw { value } => {
            if let Some(value_id) = value {
                let value_expr = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_expr);
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

        Expression::TemplateLiteral(template_literal) => match template_literal {
            TemplateLiteral::String { .. } | TemplateLiteral::TaggedString { .. } => {}
            TemplateLiteral::InterpolatedString { arguments, .. }
            | TemplateLiteral::TaggedInterpolatedString { arguments, .. } => {
                for argument_id in arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
        },

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
            for argument_id in elements {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }
        }

        Expression::TupleLiteral { elements } => {
            for argument_id in elements {
                let argument = tree.get(*argument_id);
                visitor.visit_argument(tree, *argument_id, argument);
            }
        }

        Expression::StructLiteral { ty, properties } => {
            if let Some(type_id) = ty {
                let type_expr = tree.get(*type_id);
                visitor.visit_expression(tree, *type_id, type_expr);
            }
            for property_id in properties {
                let property = tree.get(*property_id);
                visitor.visit_property(tree, *property_id, property);
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

        Expression::TypeUnary { operator: _, right } => {
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::ValueOf {
            mutability: _,
            variance: _,
            right,
        } => {
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::ReferenceOf {
            mutability: _,
            variance: _,
            right,
        } => {
            let right_expr = tree.get(*right);
            visitor.visit_expression(tree, *right, right_expr);
        }

        Expression::Member {
            left: receiver,
            path: _,
            static_arguments,
        } => {
            let receiver_expr = tree.get(*receiver);
            visitor.visit_expression(tree, *receiver, receiver_expr);
            if let Some(static_arguments) = static_arguments {
                for argument_id in static_arguments {
                    let argument = tree.get(*argument_id);
                    visitor.visit_argument(tree, *argument_id, argument);
                }
            }
        }

        Expression::Index {
            position: _,
            left: receiver,
            index,
        } => {
            let receiver_expr = tree.get(*receiver);
            visitor.visit_expression(tree, *receiver, receiver_expr);
            if let Some(index_id) = index {
                let index_expr = tree.get(*index_id);
                visitor.visit_expression(tree, *index_id, index_expr);
            }
        }

        Expression::Call {
            position: _,
            left,
            static_arguments,
            dynamic_arguments,
        } => {
            let receiver_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, receiver_expr);
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
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }

        Expression::Maybe { position: _, left } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
        }

        Expression::Must { position: _, left } => {
            let left_expr = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expr);
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

        Expression::TypeBinary {
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

fn walk_definition_meta<V: NodeVisitor + ?Sized>(
    _visitor: &mut V,
    _tree: &MutableNodeTree,
    _meta: &DefinitionMeta,
) {
    // nothing to do
}

/// Walk the Definition and visit all child nodes.
pub fn walk_definition<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &MutableNodeTree,
    id: NodeId<Definition>,
    definition: &Definition,
) {
    visitor.visit_any(tree, NodeType::Definition, id.id);

    match definition {
        Definition::Namespace {
            meta,
            with_clauses,
            where_clauses,
            expressions,
        } => {
            walk_definition_meta(visitor, tree, meta);
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
            meta,
            kind: _,
            extends_types,
            implements_types,
            static_parameters,
            with_clauses,
            where_clauses,
            properties,
        } => {
            walk_definition_meta(visitor, tree, meta);
            if let Some(ext_types) = extends_types {
                for extends_type_id in ext_types {
                    let expr = tree.get(*extends_type_id);
                    visitor.visit_expression(tree, *extends_type_id, expr);
                }
            }
            if let Some(impl_types) = implements_types {
                for implements_type_id in impl_types {
                    let expr = tree.get(*implements_type_id);
                    visitor.visit_expression(tree, *implements_type_id, expr);
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
            for property_id in properties {
                let property = tree.get(*property_id);
                visitor.visit_property(tree, *property_id, property);
            }
        }
        Definition::Enum {
            meta,
            static_parameters,
            extends_types,
            implements_types,
            with_clauses,
            where_clauses,
            fields,
            properties,
        } => {
            walk_definition_meta(visitor, tree, meta);
            if let Some(static_parameters) = static_parameters {
                for param_id in static_parameters {
                    let param = tree.get(*param_id);
                    visitor.visit_parameter(tree, *param_id, param);
                }
            }
            if let Some(ext_types) = extends_types {
                for extends_type_id in ext_types {
                    let expr = tree.get(*extends_type_id);
                    visitor.visit_expression(tree, *extends_type_id, expr);
                }
            }
            if let Some(impl_types) = implements_types {
                for implements_type_id in impl_types {
                    let expr = tree.get(*implements_type_id);
                    visitor.visit_expression(tree, *implements_type_id, expr);
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
            for property_id in properties {
                let property = tree.get(*property_id);
                visitor.visit_property(tree, *property_id, property);
            }
        }
        Definition::Interface {
            meta,
            extends_types,
            static_parameters,
            with_clauses,
            where_clauses,
            properties,
        } => {
            walk_definition_meta(visitor, tree, meta);
            if let Some(extends_types) = extends_types {
                for extends_type_id in extends_types {
                    let expr = tree.get(*extends_type_id);
                    visitor.visit_expression(tree, *extends_type_id, expr);
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
            for property_id in properties {
                let property = tree.get(*property_id);
                visitor.visit_property(tree, *property_id, property);
            }
        }
        Definition::Implement {
            meta,
            static_parameters,
            target_type,
            implements_types,
            with_clauses,
            where_clauses,
            properties,
        } => {
            walk_definition_meta(visitor, tree, meta);
            if let Some(static_parameters) = static_parameters {
                for argument_id in static_parameters {
                    let argument = tree.get(*argument_id);
                    visitor.visit_parameter(tree, *argument_id, argument);
                }
            }
            let target_type_expr = tree.get(*target_type);
            visitor.visit_expression(tree, *target_type, target_type_expr);
            if let Some(impl_types) = implements_types {
                for implements_type_id in impl_types {
                    let implements_type_expr = tree.get(*implements_type_id);
                    visitor.visit_expression(tree, *implements_type_id, implements_type_expr);
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
            for property_id in properties {
                let property = tree.get(*property_id);
                visitor.visit_property(tree, *property_id, property);
            }
        }
        Definition::Function {
            meta,
            asynchrony: _,
            cardinality: _,
            kind: _,
            mode: _,
            static_parameters,
            dynamic_parameters,
            return_type,
            with_clauses,
            where_clauses,
            body,
        } => {
            walk_definition_meta(visitor, tree, meta);
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

/// Walk a Key.
pub fn walk_key<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &MutableNodeTree, key: &Key) {
    match key {
        Key::Name(_name) => {
            // nothing to do
        }
        Key::Expression(dynamic_key) => {
            let dynamic_key_expr = tree.get(*dynamic_key);
            visitor.visit_expression(tree, *dynamic_key, dynamic_key_expr);
        }
        Key::NamedExpression { name: _, key } => {
            let key_expr = tree.get(*key);
            visitor.visit_expression(tree, *key, key_expr);
        }
    }
}

/// Walk the Property.
pub fn walk_property<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &MutableNodeTree,
    id: NodeId<Property>,
    property: &Property,
) {
    visitor.visit_any(tree, NodeType::Property, id.id);
    match property {
        Property::Field {
            modifiers: _,
            key,
            value,
            default,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }
            if let Some(value) = value {
                let value_expr = tree.get(*value);
                visitor.visit_expression(tree, *value, value_expr);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        Property::Method {
            modifiers: _,
            key,
            asynchrony: _,
            abstraction: _,
            cardinality: _,
            mode: _,
            static_parameters,
            dynamic_parameters,
            return_type,
            with_clauses,
            where_clauses,
            body,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }
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
        Property::Spread {
            modifiers: _,
            value,
        } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
    }
}

/// Walk the EnumField.
pub fn walk_enum_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &MutableNodeTree,
    id: NodeId<EnumField>,
    field: &EnumField,
) {
    visitor.visit_any(tree, NodeType::EnumField, id.id);
    if let Some(value) = &field.value {
        let expression = tree.get(*value);
        visitor.visit_expression(tree, *value, expression);
    }
}

// ----------------------------------------------------------------------------
// Context
// ----------------------------------------------------------------------------

/// Walk the WithClause.
pub fn walk_with_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &MutableNodeTree,
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
    tree: &MutableNodeTree,
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
/// Walk the DependencyItem.
pub fn walk_dependency_item<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    _tree: &MutableNodeTree,
    id: NodeId<DependencyItem>,
    _dependency_item: &DependencyItem,
) {
    visitor.visit_any(_tree, NodeType::DependencyItem, id.id);
    // DependencyItem has no child nodes to visit (only StringId fields)
}

/// Walk the Parameter.
pub fn walk_parameter<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &MutableNodeTree,
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
                visitor.visit_expression(tree, *ty, type_node);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
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
                visitor.visit_expression(tree, *ty, type_node);
            }
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        Parameter::Variadic {
            modifiers: _,
            name: _,
            ty,
        } => {
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
    tree: &MutableNodeTree,
    id: NodeId<Argument>,
    argument: &Argument,
) {
    visitor.visit_any(tree, NodeType::Argument, id.id);
    match argument {
        Argument::Named {
            modifiers: _,
            name: _,
            value,
        } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Argument::Shorthand {
            modifiers: _,
            name: _,
        } => {
            // no child nodes to visit
        }
        Argument::Positional {
            modifiers: _,
            value,
        } => {
            let value_expr = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expr);
        }
        Argument::Spread {
            modifiers: _,
            name: _,
            value,
        } => {
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
    tree: &MutableNodeTree,
    id: NodeId<Pattern>,
    pattern: &Pattern,
) {
    visitor.visit_any(tree, NodeType::Pattern, id.id);
    match pattern {
        Pattern::Wildcard => {
            // no child nodes to visit
        }
        Pattern::Rest { name: _ } => {
            // no child nodes to visit
        }
        Pattern::Maybe(unwrap) => {
            let unwrap_pattern = tree.get(*unwrap);
            visitor.visit_pattern(tree, *unwrap, unwrap_pattern);
        }
        Pattern::ReferenceOf {
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
    tree: &MutableNodeTree,
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
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
            }
        }
        PatternField::Alias {
            mutability: _,
            name: _,
            alias: _,
            default,
        } => {
            if let Some(default) = default {
                let default_expr = tree.get(*default);
                visitor.visit_expression(tree, *default, default_expr);
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
    tree: &MutableNodeTree,
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
    tree: &MutableNodeTree,
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
    tree: &MutableNodeTree,
    id: NodeId<Blank>,
    _blank: &Blank,
) {
    visitor.visit_any(tree, NodeType::Blank, id.id);
}

/// Walk the Doc.
pub fn walk_doc<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &MutableNodeTree,
    id: NodeId<Doc>,
    _doc: &Doc,
) {
    visitor.visit_any(tree, NodeType::Doc, id.id);
}

/// Walk the Comment.
pub fn walk_comment<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &MutableNodeTree,
    id: NodeId<Comment>,
    _comment: &Comment,
) {
    visitor.visit_any(tree, NodeType::Comment, id.id);
}

/// Walk the Tag.
pub fn walk_tag<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &MutableNodeTree,
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
    tree: &MutableNodeTree,
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

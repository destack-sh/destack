use crate::{
    Annotation, Argument, Block, Declaration, DependencyItem, EnumField, Expression,
    FunctionSignature, Generics, Heritage, Key, LocalNodeId, MatchCase, NodeTree, NodeType,
    NodeVisitor, Parameter, Pattern, PatternField, Property, TemplateLiteral, Type, TypeField,
    WhereClause, WithClause,
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
        NodeType::Expression => {
            let expression = tree.expressions.get(local_idx);
            walk_expression(visitor, tree, LocalNodeId::new(node_id), expression);
        }
        NodeType::Block => {
            let block = tree.blocks.get(local_idx);
            walk_block(visitor, tree, LocalNodeId::new(node_id), block);
        }
        NodeType::Declaration => {
            let declaration = tree.declarations.get(local_idx);
            walk_declaration(visitor, tree, LocalNodeId::new(node_id), declaration);
        }
        NodeType::Type => {
            let ty = tree.types.get(local_idx);
            walk_type(visitor, tree, LocalNodeId::new(node_id), ty);
        }
        NodeType::TypeField => {
            let attribute = tree.type_fields.get(local_idx);
            walk_type_field(visitor, tree, LocalNodeId::new(node_id), attribute);
        }
        NodeType::Property => {
            let property = tree.properties.get(local_idx);
            walk_property(visitor, tree, LocalNodeId::new(node_id), property);
        }
        NodeType::EnumField => {
            let enum_field = tree.enum_fields.get(local_idx);
            walk_enum_field(visitor, tree, LocalNodeId::new(node_id), enum_field);
        }
        NodeType::WhereClause => {
            let where_clause = tree.where_clauses.get(local_idx);
            walk_where_clause(visitor, tree, LocalNodeId::new(node_id), where_clause);
        }
        NodeType::WithClause => {
            let with_clause = tree.with_clauses.get(local_idx);
            walk_with_clause(visitor, tree, LocalNodeId::new(node_id), with_clause);
        }
        NodeType::DependencyItem => {
            let dependency_item = tree.dependency_items.get(local_idx);
            walk_dependency_item(visitor, tree, LocalNodeId::new(node_id), dependency_item);
        }
        NodeType::Parameter => {
            let parameter = tree.parameters.get(local_idx);
            walk_parameter(visitor, tree, LocalNodeId::new(node_id), parameter);
        }
        NodeType::Argument => {
            let argument = tree.arguments.get(local_idx);
            walk_argument(visitor, tree, LocalNodeId::new(node_id), argument);
        }
        NodeType::Pattern => {
            let pattern = tree.patterns.get(local_idx);
            walk_pattern(visitor, tree, LocalNodeId::new(node_id), pattern);
        }
        NodeType::PatternField => {
            let pattern_field = tree.pattern_fields.get(local_idx);
            walk_pattern_field(visitor, tree, LocalNodeId::new(node_id), pattern_field);
        }
        NodeType::MatchCase => {
            let match_case = tree.match_cases.get(local_idx);
            walk_match_case(visitor, tree, LocalNodeId::new(node_id), match_case);
        }
        NodeType::Annotation => {
            let annotation = tree.annotations.get(local_idx);
            walk_annotation(visitor, tree, LocalNodeId::new(node_id), annotation);
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

/// Walk the Heritage.
fn walk_heritage<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, heritage: &Heritage) {
    if let Some(extends_types) = heritage.extends_types.as_ref() {
        for extends_type_id in extends_types.iter() {
            let extends_type = tree.get(*extends_type_id);
            visitor.visit_type(tree, *extends_type_id, extends_type);
        }
    }
    if let Some(implements_types) = heritage.implements_types.as_ref() {
        for implements_type_id in implements_types.iter() {
            let implements_type = tree.get(*implements_type_id);
            visitor.visit_type(tree, *implements_type_id, implements_type);
        }
    }
    if let Some(embedded_types) = heritage.embedded_types.as_ref() {
        for embedded_type_id in embedded_types.iter() {
            let embedded_type = tree.get(*embedded_type_id);
            visitor.visit_type(tree, *embedded_type_id, embedded_type);
        }
    }
}

/// Walk the FunctionSignature.
fn walk_function_signature<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    signature: &FunctionSignature,
) {
    if let Some(generics) = signature.generics.as_ref() {
        walk_generics(visitor, tree, generics);
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

/// Walk the Expression.
pub fn walk_expression<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Expression>,
    expression: &Expression,
) {
    visitor.visit_any(tree, NodeType::Expression, id.id);
    match expression {
        Expression::Declaration {
            declaration: declaration_id,
        } => {
            let declaration = tree.get(*declaration_id);
            visitor.visit_declaration(tree, *declaration_id, declaration);
        }
        Expression::Block { block: block_id } => {
            let block = tree.get(*block_id);
            visitor.visit_block(tree, *block_id, block);
        }
        Expression::Statement {
            statement: statement_id,
        } => {
            let statement = tree.get(*statement_id);
            visitor.visit_expression(tree, *statement_id, statement);
        }
        Expression::With {
            clauses,
            body,
            scope: _,
            symbol: _,
        } => {
            for clause_id in clauses {
                let clause = tree.get(*clause_id);
                visitor.visit_with_clause(tree, *clause_id, clause);
            }
            if let Some(body_id) = body {
                let block = tree.get(*body_id);
                visitor.visit_block(tree, *body_id, block);
            }
        }
        Expression::UnresolvedImport {
            kind: _,
            target: _,
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
        Expression::Import {
            kind: _,
            target: _,
            module: _,
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
        Expression::UnresolvedReExport {
            mode: _,
            target: _,
            kind: _,
            items,
        } => {
            for item_id in items {
                let item = tree.get(*item_id);
                visitor.visit_dependency_item(tree, *item_id, item);
            }
        }
        Expression::ReExport {
            mode: _,
            target: _,
            module: _,
            kind: _,
            items,
        } => {
            for item_id in items {
                let item = tree.get(*item_id);
                visitor.visit_dependency_item(tree, *item_id, item);
            }
        }
        Expression::Export {
            mode: _,
            kind: _,
            items,
        } => {
            for item_id in items {
                let item = tree.get(*item_id);
                visitor.visit_dependency_item(tree, *item_id, item);
            }
        }
        Expression::Let {
            mutability: _,
            pattern: pattern_id,
            ty: ty_id,
            value: value_id,
            symbol: _,
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
            kind: _,
            mutability: _,
            name: _,
            static_parameters,
            value,
            symbol: _,
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
        Expression::Unary { operator: _, right }
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
        | Expression::TypeUnary { operator: _, right } => {
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
        | Expression::Assign { left, right }
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
            name: _,
            symbol: _,
            static_arguments,
        }
        | Expression::UnresolvedMember {
            left,
            name: _,
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
            static_arguments,
            dynamic_arguments,
        } => {
            let left_expression = tree.get(*left);
            visitor.visit_expression(tree, *left, left_expression);
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
        Expression::UnresolvedPath {
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
                TemplateLiteral::String { .. } => {
                    // nothing to do
                }
                TemplateLiteral::InterpolatedString { arguments, .. } => {
                    for argument_id in arguments {
                        let argument = tree.get(*argument_id);
                        visitor.visit_argument(tree, *argument_id, argument);
                    }
                }
            }
        }
        Expression::TaggedTemplateLiteral { tag, value } => {
            let tag_expression = tree.get(*tag);
            visitor.visit_expression(tree, *tag, tag_expression);
            match value {
                TemplateLiteral::String { .. } => {
                    // nothing to do
                }
                TemplateLiteral::InterpolatedString { arguments, .. } => {
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
        Expression::StructLiteral { ty, properties } => {
            if let Some(ty_id) = ty {
                let ty_node = tree.get(*ty_id);
                visitor.visit_type(tree, *ty_id, ty_node);
            }
            for property_id in properties {
                let property = tree.get(*property_id);
                visitor.visit_property(tree, *property_id, property);
            }
        }
        Expression::TreeLiteral {
            left,
            arguments,
            elements,
        } => {
            if let Some(left_id) = left {
                let left_expression = tree.get(*left_id);
                visitor.visit_expression(tree, *left_id, left_expression);
            }
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
            let expression_node = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_node);
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
            kind: _,
            condition,
            body,
            scope: _,
            symbol: _,
        } => {
            if let Some(condition_id) = condition {
                let condition_expression = tree.get(*condition_id);
                visitor.visit_expression(tree, *condition_id, condition_expression);
            }
            let body_block = tree.get(*body);
            visitor.visit_block(tree, *body, body_block);
        }
        Expression::ForEach {
            asynchrony: _,
            kind: _,
            pattern,
            iterator,
            body,
            scope: _,
            symbol: _,
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
            scope: _,
            symbol: _,
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
        Expression::Try {
            try_expression,
            catch_pattern,
            catch_expression,
            finally_expression,
            scope: _,
            symbol: _,
        } => {
            let try_expression_node = tree.get(*try_expression);
            visitor.visit_expression(tree, *try_expression, try_expression_node);
            if let Some(catch_pattern_id) = catch_pattern {
                let catch_pattern_node = tree.get(*catch_pattern_id);
                visitor.visit_pattern(tree, *catch_pattern_id, catch_pattern_node);
            }
            if let Some(catch_expression_id) = catch_expression {
                let catch_expression_node = tree.get(*catch_expression_id);
                visitor.visit_expression(tree, *catch_expression_id, catch_expression_node);
            }
            if let Some(finally_expression_id) = finally_expression {
                let finally_expression_node = tree.get(*finally_expression_id);
                visitor.visit_expression(tree, *finally_expression_id, finally_expression_node);
            }
        }
        Expression::Match {
            value,
            cases,
            source: _,
            scope: _,
            symbol: _,
        } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
            for case_id in cases {
                let case = tree.get(*case_id);
                visitor.visit_match_case(tree, *case_id, case);
            }
        }
        Expression::Break { target: _, value }
        | Expression::UnresolvedBreak { target: _, value } => {
            if let Some(value_id) = value {
                let value_expression = tree.get(*value_id);
                visitor.visit_expression(tree, *value_id, value_expression);
            }
        }
        Expression::Continue { target: _ } | Expression::UnresolvedContinue { target: _ } => {}
        Expression::Defer { expression } => {
            let body_expression = tree.get(*expression);
            visitor.visit_expression(tree, *expression, body_expression);
        }
        Expression::Await { expression } => {
            let expression_node = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_node);
        }
        Expression::Yield {
            cardinality: _,
            value,
        } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
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
    id: LocalNodeId<Block>,
    block: &Block,
) {
    visitor.visit_any(tree, NodeType::Block, id.id);
    for expression_id in &block.expressions {
        let expression = tree.get(*expression_id);
        visitor.visit_expression(tree, *expression_id, expression);
    }
}

/// Walk the Declaration.
pub fn walk_declaration<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Declaration>,
    declaration: &Declaration,
) {
    visitor.visit_any(tree, NodeType::Declaration, id.id);
    match declaration {
        Declaration::Namespace {
            descriptor: _,
            generics,
            declarations,
            scope: _,
        } => {
            walk_generics(visitor, tree, generics);
            for declaration_id in declarations {
                let child_declaration = tree.get(*declaration_id);
                visitor.visit_declaration(tree, *declaration_id, child_declaration);
            }
        }
        Declaration::Struct {
            descriptor: _,
            kind: _,
            generics,
            heritage,
            properties,
            scope: _,
        } => {
            walk_generics(visitor, tree, generics);
            walk_heritage(visitor, tree, heritage);
            for property_id in properties {
                let property = tree.get(*property_id);
                visitor.visit_property(tree, *property_id, property);
            }
        }
        Declaration::Enum {
            descriptor: _,
            generics,
            heritage,
            fields,
            properties,
            scope: _,
        } => {
            walk_generics(visitor, tree, generics);
            walk_heritage(visitor, tree, heritage);
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_enum_field(tree, *field_id, field);
            }
            for property_id in properties {
                let property = tree.get(*property_id);
                visitor.visit_property(tree, *property_id, property);
            }
        }
        Declaration::Interface {
            descriptor: _,
            generics,
            heritage,
            properties,
            scope: _,
        } => {
            walk_generics(visitor, tree, generics);
            walk_heritage(visitor, tree, heritage);
            for property_id in properties {
                let property = tree.get(*property_id);
                visitor.visit_property(tree, *property_id, property);
            }
        }
        Declaration::Function {
            descriptor: _,
            signature,
            declarations,
            body,
            scope: _,
        } => {
            walk_function_signature(visitor, tree, signature);
            for declaration_id in declarations.iter() {
                let child_declaration = tree.get(*declaration_id);
                visitor.visit_declaration(tree, *declaration_id, child_declaration);
            }
            if let Some(body) = body {
                let body_expression = tree.get(*body);
                visitor.visit_expression(tree, *body, body_expression);
            }
        }
        Declaration::Implement {
            descriptor: _,
            generics,
            target_type,
            heritage,
            properties,
            scope: _,
        } => {
            walk_generics(visitor, tree, generics);
            let target_type_expr = tree.get(*target_type);
            visitor.visit_type(tree, *target_type, target_type_expr);
            walk_heritage(visitor, tree, heritage);
            for property_id in properties {
                let property = tree.get(*property_id);
                visitor.visit_property(tree, *property_id, property);
            }
        }
    }
}

/// Walk the Type.
pub fn walk_type<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Type>,
    ty: &Type,
) {
    visitor.visit_any(tree, NodeType::Type, id.id);
    match ty {
        Type::Scalar(_) => {}
        Type::Symbol(_) => {}

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
        Type::ArraySized { element, count } => {
            let element_type = tree.get(*element);
            visitor.visit_type(tree, *element, element_type);
            let count_expression = tree.get(*count);
            visitor.visit_expression(tree, *count, count_expression);
        }
        Type::Array { element } => {
            if let Some(element_id) = element {
                let element_type = tree.get(*element_id);
                visitor.visit_type(tree, *element_id, element_type);
            }
        }
        Type::Tuple { elements } => {
            for element_id in elements {
                let element_type = tree.get(*element_id);
                visitor.visit_type(tree, *element_id, element_type);
            }
        }
        Type::Struct { attributes } => {
            for attribute_id in attributes {
                let attribute = tree.get(*attribute_id);
                visitor.visit_type_field(tree, *attribute_id, attribute);
            }
        }
        Type::Union { elements } => {
            for element_id in elements {
                let element_type = tree.get(*element_id);
                visitor.visit_type(tree, *element_id, element_type);
            }
        }
        Type::Intersection { elements } => {
            for element_id in elements {
                let element_type = tree.get(*element_id);
                visitor.visit_type(tree, *element_id, element_type);
            }
        }
        Type::Function { signature } => {
            walk_function_signature(visitor, tree, signature);
        }

        Type::UnresolvedExpression(expression_id) => {
            let expression = tree.get(*expression_id);
            visitor.visit_expression(tree, *expression_id, expression);
        }

        Type::Error => {}
    }
}

/// Walk the TypeField.
pub fn walk_type_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<TypeField>,
    attribute: &TypeField,
) {
    visitor.visit_any(tree, NodeType::TypeField, id.id);
    match attribute {
        TypeField::Field {
            modifiers: _,
            key,
            ty,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }
            let ty_type = tree.get(*ty);
            visitor.visit_type(tree, *ty, ty_type);
        }
        TypeField::Method {
            modifiers: _,
            key,
            signature,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }
            walk_function_signature(visitor, tree, signature);
        }
    }
}

/// Walk the Key.
pub fn walk_key<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &NodeTree, key: &Key) {
    match key {
        Key::Name(_) => {}
        Key::Expression(expression) => {
            let expression_expr = tree.get(*expression);
            visitor.visit_expression(tree, *expression, expression_expr);
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
    tree: &NodeTree,
    id: LocalNodeId<Property>,
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
            signature,
            body,
        } => {
            if let Some(key) = key {
                walk_key(visitor, tree, key);
            }
            walk_function_signature(visitor, tree, signature);
            if let Some(body) = body {
                let body_expr = tree.get(*body);
                visitor.visit_expression(tree, *body, body_expr);
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
    tree: &NodeTree,
    id: LocalNodeId<EnumField>,
    enum_field: &EnumField,
) {
    visitor.visit_any(tree, NodeType::EnumField, id.id);
    if let Some(value) = enum_field.value {
        let value_expression = tree.get(value);
        visitor.visit_expression(tree, value, value_expression);
    }
}

/// Walk the WhereClause.
pub fn walk_where_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<WhereClause>,
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

/// Walk the WithClause.
pub fn walk_with_clause<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<WithClause>,
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
    id: LocalNodeId<DependencyItem>,
    dependency_item: &DependencyItem,
) {
    visitor.visit_any(tree, NodeType::DependencyItem, id.id);
    match dependency_item {
        DependencyItem::UnresolvedDefault {
            kind: _,
            alias: _,
            symbol: _,
        } => {
            // nothing to do
        }
        DependencyItem::UnresolvedItem {
            kind: _,
            name: _,
            alias: _,
            symbol: _,
        } => {
            // nothing to do
        }
        DependencyItem::Value { value } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        DependencyItem::Local { symbol: _ } => {
            // nothing to do
        }
        DependencyItem::Remote {
            symbol: _,
            remote_symbol: _,
            module: _,
        } => {
            // nothing to do
        }
    }
}

/// Walk the Parameter.
pub fn walk_parameter<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Parameter>,
    parameter: &Parameter,
) {
    visitor.visit_any(tree, NodeType::Parameter, id.id);
    match parameter {
        Parameter::Named {
            modifiers: _,
            name: _,
            ty,
            default,
            symbol: _,
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
            symbol: _,
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
            symbol: _,
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
    id: LocalNodeId<Argument>,
    argument: &Argument,
) {
    visitor.visit_any(tree, NodeType::Argument, id.id);
    match argument {
        Argument::UnresolvedNamed {
            modifiers: _,
            name: _,
            value,
        }
        | Argument::UnresolvedPositional {
            modifiers: _,
            value,
        }
        | Argument::UnresolvedSpread {
            modifiers: _,
            name: _,
            value,
        } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Argument::UnresolvedDynamic {
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
            symbol: _,
            value,
        }
        | Argument::Spread {
            modifiers: _,
            name: _,
            symbol: _,
            value,
        } => {
            let value_expression = tree.get(*value);
            visitor.visit_expression(tree, *value, value_expression);
        }
        Argument::Dynamic {
            modifiers: _,
            name: _,
            symbol: _,
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

/// Walk the Pattern.
pub fn walk_pattern<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Pattern>,
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
    id: LocalNodeId<PatternField>,
    pattern_field: &PatternField,
) {
    visitor.visit_any(tree, NodeType::PatternField, id.id);
    match pattern_field {
        PatternField::Named {
            mutability: _,
            name: _,
            symbol: _,
            pattern,
            default,
        }
        | PatternField::UnresolvedNamed {
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
            symbol: _,
            default,
        }
        | PatternField::UnresolvedAlias {
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
        PatternField::Positional { pattern, symbol: _ }
        | PatternField::UnresolvedPositional { pattern } => {
            let pattern_node = tree.get(*pattern);
            visitor.visit_pattern(tree, *pattern, pattern_node);
        }
    }
}

/// Walk the MatchCase.
pub fn walk_match_case<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<MatchCase>,
    match_case: &MatchCase,
) {
    visitor.visit_any(tree, NodeType::MatchCase, id.id);
    match match_case {
        MatchCase::Expression {
            pattern,
            body,
            guard,
            scope: _,
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
            scope: _,
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

/// Walk the Annotation.
pub fn walk_annotation<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Annotation>,
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
            left: _,
            symbol: _,
            arguments,
        }
        | Annotation::UnresolvedTag {
            position: _,
            left: _,
            arguments,
        }
        | Annotation::Decorator {
            position: _,
            left: _,
            symbol: _,
            arguments,
        }
        | Annotation::UnresolvedDecorator {
            position: _,
            left: _,
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

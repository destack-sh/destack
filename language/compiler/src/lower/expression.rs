use dyst_ast::{self as ast};
use dyst_dir::{
    DependencyItem, DependencySource, Expression, ForEachKind, IfKind, LoopKind, MatchSource,
    Module, NodeId, Path, PathBase, Runtime, YieldCardinality,
};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower runtime into a DIR runtime.
    #[inline]
    pub fn lower_runtime(&self, runtime: ast::Runtime) -> Runtime {
        match runtime {
            ast::Runtime::Dynamic => Runtime::Dynamic,
            ast::Runtime::Static => Runtime::Static,
        }
    }

    /// Lower if kind into a DIR if kind.
    #[inline]
    pub fn lower_if_kind(&self, kind: ast::IfKind) -> IfKind {
        match kind {
            ast::IfKind::If => IfKind::If,
            ast::IfKind::Ternary => IfKind::Ternary,
        }
    }

    /// Lower an expression to a DIR expression.
    pub fn lower_expression(
        &mut self,
        module: &Module,
        expression_id: ast::NodeId<ast::Expression>,
    ) -> NodeId<Expression> {
        let expression = module.get(expression_id);
        let expression = match expression {
            ast::Expression::Block(block_id) => {
                let block_id = self.lower_block(module, *block_id);
                Expression::Block { block: block_id }
            }
            ast::Expression::Definition(definition_id) => {
                let definition_id = self.lower_definition(module, *definition_id);
                Expression::Definition {
                    definition: definition_id,
                }
            }
            ast::Expression::Statement(expression_id) => {
                let expression_id = self.lower_expression(module, *expression_id);
                Expression::Statement {
                    statement: expression_id,
                }
            }

            ast::Expression::With { clauses, body } => {
                let clauses = clauses
                    .iter()
                    .map(|clause| self.lower_with_clause(module, *clause))
                    .collect();
                let body = body.map(|body| self.lower_block(module, body));
                Expression::With { clauses, body }
            }
            ast::Expression::Import {
                kind,
                target,
                alias,
                items,
                arguments,
            } => {
                let target = self.session.strings.intern_from(&module.strings, *target);
                // items
                let mut items: Vec<_> = items
                    .as_ref()
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| self.lower_dependency_item(module, *kind, *item))
                            .collect()
                    })
                    .unwrap_or_default();
                // default item
                if let Some(alias) = alias {
                    let alias = self.session.strings.intern_from(&module.strings, *alias);
                    let item = DependencyItem::UnresolvedDefault {
                        kind: self.lower_dependency_kind(*kind),
                        alias,
                    };
                    items.push(
                        self.session
                            .tree
                            .insert_from_ast(item, module.id, expression_id),
                    );
                }
                // arguments
                let arguments = arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(module, *argument))
                        .collect()
                });
                let kind = self.lower_dependency_kind(*kind);
                // import
                Expression::UnresolvedImport {
                    kind,
                    target,
                    source: DependencySource::ImportStatement,
                    items,
                    arguments,
                }
            }
            ast::Expression::Export {
                mode,
                kind,
                target,
                alias,
                items,
                value,
            } => {
                let mode = self.lower_export_type(*mode);
                let target =
                    target.map(|target| self.session.strings.intern_from(&module.strings, target));
                // re-export from import
                if let Some(target) = target {
                    // items
                    let mut items: Vec<_> = items
                        .as_ref()
                        .map(|items| {
                            items
                                .iter()
                                .map(|item| self.lower_dependency_item(module, *kind, *item))
                                .collect()
                        })
                        .unwrap_or_default();
                    // default item
                    if let Some(alias) = alias {
                        let alias = self.session.strings.intern_from(&module.strings, *alias);
                        let item = DependencyItem::UnresolvedDefault {
                            kind: self.lower_dependency_kind(*kind),
                            alias,
                        };
                        items.push(self.session.tree.insert_from_ast(
                            item,
                            module.id,
                            expression_id,
                        ));
                    }
                    let kind = self.lower_dependency_kind(*kind);
                    // re-export
                    Expression::UnresolvedReExport {
                        mode,
                        target,
                        kind,
                        source: DependencySource::ReExportStatement,
                        items,
                    }
                }
                // export from module
                else {
                    // export = value
                    let items = {
                        if let Some(value_id) = value {
                            let value = self.lower_expression(module, *value_id);
                            let item = DependencyItem::Value { value };
                            let item_id = self
                                .session
                                .tree
                                .insert_from_ast(item, module.id, *value_id);
                            vec![item_id]
                        }
                        // export items
                        else {
                            items
                                .as_ref()
                                .map(|items| {
                                    items
                                        .iter()
                                        .map(|item| {
                                            self.lower_dependency_item(module, *kind, *item)
                                        })
                                        .collect()
                                })
                                .unwrap_or_default()
                        }
                    };
                    let kind = self.lower_dependency_kind(*kind);
                    Expression::Export {
                        mode,
                        kind,
                        source: DependencySource::ReExportStatement,
                        items,
                    }
                }
            }
            ast::Expression::Let {
                descriptor: _,
                mutability,
                pattern,
                ty,
                value,
            } => {
                let mutability = self.lower_mutability(*mutability);
                let pattern = self.lower_pattern(module, *pattern);
                let ty = ty.map(|ty| self.lower_expression_to_type(module, ty));
                let value = value.map(|value| self.lower_expression(module, value));
                Expression::Let {
                    mutability,
                    pattern,
                    ty,
                    value,
                }
            }
            ast::Expression::LetType {
                kind,
                descriptor,
                mutability,
                static_parameters,
                value,
            } => {
                let kind = self.lower_type_kind(*kind);
                let name = self.session.strings.intern_from(
                    &module.strings,
                    descriptor.name.expect("LetType must have a name").string(),
                );
                let mutability = mutability.map(|mutability| self.lower_mutability(mutability));
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.lower_parameter(module, *param))
                        .collect()
                });
                let value = self.lower_expression(module, *value);
                Expression::LetType {
                    kind,
                    mutability,
                    name,
                    static_parameters,
                    value,
                }
            }

            ast::Expression::Unary { operator, right } => {
                let right = self.lower_expression(module, *right);
                let operator = self.lower_unary_operator(*operator);
                Expression::UnresolvedUnary { operator, right }
            }

            ast::Expression::TypeUnary { operator, right } => {
                let right = self.lower_expression(module, *right);
                let operator = self.lower_type_unary_operator(*operator);
                Expression::TypeUnary { operator, right }
            }

            ast::Expression::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let mutability = mutability.map(|mutability| self.lower_mutability(mutability));
                let variance = variance.map(|variance| self.lower_variance_bound(variance));
                let right = self.lower_expression(module, *right);
                Expression::ValueOf {
                    mutability,
                    variance,
                    right,
                }
            }
            ast::Expression::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let mutability = mutability.map(|mutability| self.lower_mutability(mutability));
                let variance = variance.map(|variance| self.lower_variance_bound(variance));
                let right = self.lower_expression(module, *right);
                Expression::ReferenceOf {
                    mutability,
                    variance,
                    right,
                }
            }
            ast::Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.lower_expression(module, *left);
                let right = self.lower_expression(module, *right);
                let operator = self.lower_binary_operator(*operator);
                Expression::UnresolvedBinary {
                    left,
                    operator,
                    right,
                }
            }
            ast::Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                let left = self.lower_expression(module, *left);
                let right = self.lower_expression(module, *right);
                let operator = self.lower_type_binary_operator(*operator);
                Expression::TypeBinary {
                    left,
                    operator,
                    right,
                }
            }
            ast::Expression::Assign {
                left,
                operator,
                right,
            } => {
                let left = self.lower_expression(module, *left);
                let right = self.lower_expression(module, *right);
                let operator = self.lower_assign_operator(*operator);
                if let Some(operator) = operator {
                    Expression::UnresolvedAssignBinary {
                        left,
                        operator,
                        right,
                    }
                } else {
                    Expression::Assign { left, right }
                }
            }

            ast::Expression::Member {
                left,
                path,
                static_arguments,
            } => {
                let left = self.lower_expression(module, *left);
                let path = self.lower_path(module, path);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(module, *argument))
                        .collect()
                });
                Expression::Member {
                    left,
                    path,
                    static_arguments,
                }
            }
            ast::Expression::Call {
                position: _,
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                let left = self.lower_expression(module, *left);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(module, *argument))
                        .collect()
                });
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| self.lower_argument(module, *argument))
                    .collect();
                Expression::Call {
                    left,
                    static_arguments,
                    dynamic_arguments,
                }
            }
            ast::Expression::New {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                let left = self.lower_path(module, left);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(module, *argument))
                        .collect()
                });
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| self.lower_argument(module, *argument))
                    .collect();
                Expression::New {
                    left,
                    static_arguments,
                    dynamic_arguments,
                }
            }
            ast::Expression::Delete { value } => {
                let value = self.lower_expression(module, *value);
                Expression::Delete { value }
            }
            ast::Expression::Index {
                position: _,
                left,
                index,
            } => {
                let left = self.lower_expression(module, *left);
                let index = index.map(|index| self.lower_expression(module, index));
                Expression::Index { left, right: index }
            }
            ast::Expression::Maybe { position: _, left } => {
                let left = self.lower_expression(module, *left);
                Expression::Maybe { left }
            }
            ast::Expression::Must { position: _, left } => {
                let left = self.lower_expression(module, *left);
                Expression::Must { left }
            }

            ast::Expression::Path {
                path,
                static_arguments,
            } => {
                let path = self.lower_path(module, path);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(module, *argument))
                        .collect()
                });
                Expression::Path {
                    path,
                    static_arguments,
                }
            }
            ast::Expression::ScalarLiteral(value) => {
                let value = self.lower_scalar_literal(module, value);
                Expression::ScalarLiteral { value }
            }
            ast::Expression::TemplateLiteral(value) => {
                let value = self.lower_template_literal(module, value);
                Expression::TemplateLiteral { value }
            }
            ast::Expression::TypeLiteral(value) => {
                if *value == ast::TypeLiteral::Self_ {
                    Expression::Path {
                        path: Path::UnresolvedBase {
                            base: PathBase::SelfType,
                        },
                        static_arguments: None,
                    }
                } else {
                    let value = self.lower_type_literal(value);
                    Expression::TypeLiteral { value }
                }
            }
            ast::Expression::RangeLiteral {
                start,
                end,
                is_inclusive,
            } => {
                let start = self.lower_expression(module, *start);
                let end = self.lower_expression(module, *end);
                Expression::RangeLiteral {
                    start,
                    end,
                    is_inclusive: *is_inclusive,
                }
            }
            ast::Expression::StructLiteral { ty, properties } => {
                let ty = ty.map(|ty| self.lower_expression_to_type(module, ty));
                let properties = properties
                    .iter()
                    .map(|property| self.lower_property(module, *property))
                    .collect();
                Expression::StructLiteral { ty, properties }
            }
            ast::Expression::TupleLiteral { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.lower_argument(module, *element))
                    .collect();
                Expression::TupleLiteral { ty: None, elements }
            }
            ast::Expression::ArrayLiteral { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.lower_argument(module, *element))
                    .collect();
                Expression::ArrayLiteral { elements }
            }
            ast::Expression::TreeLiteral {
                path,
                arguments,
                elements,
            } => {
                let path = path.as_ref().map(|path| self.lower_path(module, path));
                let arguments = arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(module, *argument))
                        .collect()
                });
                let elements = elements.as_ref().map(|elements| {
                    elements
                        .iter()
                        .map(|element| self.lower_argument(module, *element))
                        .collect()
                });
                Expression::TreeLiteral {
                    path,
                    arguments,
                    elements,
                }
            }
            ast::Expression::Parenthesized { expression } => {
                let expression = self.lower_expression(module, *expression);
                Expression::Parenthesized { expression }
            }

            ast::Expression::If {
                kind,
                condition,
                then_expression,
                else_expression,
            } => {
                let kind = self.lower_if_kind(*kind);
                let condition = self.lower_expression(module, *condition);
                let then_expression = self.lower_expression(module, *then_expression);
                let else_expression = else_expression
                    .map(|else_expression| self.lower_expression(module, else_expression));
                Expression::If {
                    kind,
                    condition,
                    then_expression,
                    else_expression,
                }
            }
            ast::Expression::While {
                kind,
                condition,
                body,
            } => {
                let kind = match *kind {
                    ast::WhileKind::While => LoopKind::PreTest,
                    ast::WhileKind::DoWhile => LoopKind::PostTest,
                };
                let condition = self.lower_expression(module, *condition);
                let body = self.lower_block(module, *body);
                Expression::Loop {
                    kind,
                    condition: Some(condition),
                    body,
                }
            }
            ast::Expression::ForEach {
                asynchrony,
                kind,
                pattern,
                iterator,
                body,
            } => {
                let asynchrony = self.lower_asynchrony(*asynchrony);
                let kind = match *kind {
                    ast::ForEachKind::In => ForEachKind::In,
                    ast::ForEachKind::Of => ForEachKind::Of,
                };
                let pattern = self.lower_pattern(module, *pattern);
                let iterator = self.lower_expression(module, *iterator);
                let body = self.lower_block(module, *body);
                Expression::ForEach {
                    asynchrony,
                    kind,
                    pattern,
                    iterator,
                    body,
                }
            }
            ast::Expression::For {
                initialization,
                condition,
                increment,
                body,
            } => {
                let initialization = initialization
                    .map(|initialization| self.lower_expression(module, initialization));
                let condition = condition.map(|condition| self.lower_expression(module, condition));
                let increment = increment.map(|increment| self.lower_expression(module, increment));
                let body = self.lower_block(module, *body);
                Expression::For {
                    initialization,
                    condition,
                    increment,
                    body,
                }
            }
            ast::Expression::Loop { body } => {
                let body = self.lower_block(module, *body);
                Expression::Loop {
                    kind: LoopKind::NoTest,
                    condition: None,
                    body,
                }
            }
            ast::Expression::Try {
                try_expression,
                catch_pattern,
                catch_expression,
                finally_expression,
            } => {
                let try_expression = self.lower_expression(module, *try_expression);
                let catch_pattern =
                    catch_pattern.map(|catch_pattern| self.lower_pattern(module, catch_pattern));
                let catch_expression = catch_expression
                    .map(|catch_expression| self.lower_expression(module, catch_expression));
                let finally_expression = finally_expression
                    .map(|finally_expression| self.lower_expression(module, finally_expression));
                Expression::Try {
                    try_expression,
                    catch_pattern,
                    catch_expression,
                    finally_expression,
                }
            }
            ast::Expression::Match {
                kind: _,
                value,
                cases,
            } => {
                let value = self.lower_expression(module, *value);
                let cases = cases
                    .iter()
                    .map(|case| self.lower_match_case(module, *case))
                    .collect();
                Expression::Match {
                    value,
                    cases,
                    source: MatchSource::Match,
                }
            }

            ast::Expression::Break { label, value } => {
                let target = label.map(|label| self.lower_label(module, label));
                let value = value.map(|value| self.lower_expression(module, value));
                Expression::Break { target, value }
            }
            ast::Expression::Continue { label } => {
                let target = label.map(|label| self.lower_label(module, label));
                Expression::Continue { target }
            }
            ast::Expression::Return { value } => {
                let value = value.map(|value| self.lower_expression(module, value));
                Expression::Return { value }
            }
            ast::Expression::Defer { expression } => {
                let expression = self.lower_expression(module, *expression);
                Expression::Defer { expression }
            }
            ast::Expression::Await { expression } => {
                let expression = self.lower_expression(module, *expression);
                Expression::Await { expression }
            }
            ast::Expression::Yield { cardinality, value } => {
                let cardinality = match *cardinality {
                    ast::YieldCardinality::Generator => YieldCardinality::Generator,
                    ast::YieldCardinality::Scalar => YieldCardinality::Scalar,
                };
                let value = self.lower_expression(module, *value);
                Expression::Yield { cardinality, value }
            }
            ast::Expression::Throw { value } => {
                let value = value.map(|value| self.lower_expression(module, value));
                Expression::Throw { value }
            }

            ast::Expression::Error => Expression::Error,
        };
        self.session
            .tree
            .insert_from_ast(expression, module.id, expression_id)
    }
}

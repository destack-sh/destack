use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;
use smallvec::smallvec;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR type id into an AST type expression.
    pub(super) fn unbind_type_expression(
        &self,
        module: &Module,
        type_id: dir::LocalTypeId,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Expression> {
        // span for synthesized nodes
        let span = self.unbind_span(module, types.get_type_source(type_id));

        // unwrap unevaluated expressions directly
        if let dir::Type::Unevaluated(expression_id) = types.get_type(type_id) {
            return self.unbind_expression(
                module,
                *expression_id,
                tree,
                symbols,
                ast_tree,
                ast_strings,
                context,
            );
        }

        // lower union types to elementwise or chains
        if let dir::Type::Union { elements } = types.get_type(type_id) {
            // seed the left operand
            let mut element_ids = elements.iter();
            let Some(first) = element_ids.next() else {
                return ast_tree.insert(ast::Expression::Error, span);
            };

            // build the first element
            let mut left_id = self.unbind_type_expression(
                module,
                *first,
                tree,
                symbols,
                types,
                ast_tree,
                ast_strings,
                context,
            );

            // chain remaining elements
            for element in element_ids {
                let right_id = self.unbind_type_expression(
                    module,
                    *element,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let expression = ast::Expression::Binary {
                    left: left_id,
                    operator: ast::BinaryOperator::ElementwiseOr,
                    right: right_id,
                };
                left_id = ast_tree.insert(expression, span);
            }

            // return the chained union
            return left_id;
        }

        // lower intersection types to elementwise and chains
        if let dir::Type::Intersection { elements } = types.get_type(type_id) {
            // seed the left operand
            let mut element_ids = elements.iter();
            let Some(first) = element_ids.next() else {
                return ast_tree.insert(ast::Expression::Error, span);
            };

            // build the first element
            let mut left_id = self.unbind_type_expression(
                module,
                *first,
                tree,
                symbols,
                types,
                ast_tree,
                ast_strings,
                context,
            );

            // chain remaining elements
            for element in element_ids {
                let right_id = self.unbind_type_expression(
                    module,
                    *element,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let expression = ast::Expression::Binary {
                    left: left_id,
                    operator: ast::BinaryOperator::ElementwiseAnd,
                    right: right_id,
                };
                left_id = ast_tree.insert(expression, span);
            }

            // return the chained intersection
            return left_id;
        }

        // build a type expression for the remaining cases
        let ast_expression = match types.get_type(type_id) {
            dir::Type::TypeLiteral { value } => {
                // type literal
                let value = self.unbind_type_literal(value, context);
                ast::Expression::TypeLiteral(value)
            }
            dir::Type::InferVar { .. } => {
                // infer placeholder
                ast::Expression::TypeLiteral(ast::TypeLiteral::Infer)
            }
            dir::Type::Value { value } => {
                // type value expression
                let right = self.unbind_type_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let operator = ast::TypeUnaryOperator::Type;
                ast::Expression::TypeUnary { operator, right }
            }
            dir::Type::This => ast::Expression::This,
            dir::Type::Reference {
                symbol,
                static_arguments,
            } => {
                // reference path with static arguments
                let name = self.unbind_symbol_name(*symbol, module, symbols, ast_strings);
                let path = ast::Path {
                    segments: smallvec![name],
                };
                let static_arguments = self.unbind_static_arguments(
                    module,
                    static_arguments.as_deref(),
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Expression::Path {
                    path,
                    static_arguments,
                }
            }
            dir::Type::Conditional {
                distributive_symbol: _,
                left,
                right,
                then_type,
                else_type,
            } => {
                // conditional type expression
                let left = self.unbind_type_expression(
                    module,
                    *left,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let right = self.unbind_type_expression(
                    module,
                    *right,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let then_type = self.unbind_type_expression(
                    module,
                    *then_type,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let else_type = self.unbind_type_expression(
                    module,
                    *else_type,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Expression::TypeConditional {
                    left,
                    right,
                    then_type,
                    else_type,
                }
            }
            dir::Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                // mapped type expression
                let name = ast_strings.intern_from(&self.program.strings, parameter.name);
                let constraint = self.unbind_type_expression(
                    module,
                    parameter.constraint,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let key_remap = parameter.key_remap.map(|key_remap| {
                    self.unbind_type_expression(
                        module,
                        key_remap,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                let parameter = ast::TypeMappedParameter {
                    name,
                    constraint,
                    key_remap,
                };
                let modifiers = self.unbind_type_mapped_modifiers(context, *modifiers);
                let value = self.unbind_type_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Expression::TypeMapped {
                    parameter,
                    modifiers,
                    value,
                }
            }
            dir::Type::Index { left, index } => {
                // indexed type expression
                let left = self.unbind_type_expression(
                    module,
                    *left,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let index = self.unbind_type_expression(
                    module,
                    *index,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Expression::TypeIndex { left, index }
            }
            dir::Type::TemplateLiteral { strings, spans } => {
                // template literal type expression
                let strings = strings
                    .iter()
                    .map(|string| ast_strings.intern_from(&self.program.strings, *string))
                    .collect();
                let spans = spans
                    .iter()
                    .map(|span| {
                        self.unbind_type_expression(
                            module,
                            *span,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::Expression::TypeTemplateLiteral { strings, spans }
            }
            dir::Type::Import {
                target,
                qualifier,
                static_arguments,
            } => {
                // type import expression
                let target = ast_strings.intern_from(&self.program.strings, *target);
                let target_expression_id = ast_tree.insert(
                    ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(target)),
                    span,
                );
                let target_argument_id = ast_tree.insert(
                    ast::Argument::Positional {
                        modifiers: None,
                        value: target_expression_id,
                    },
                    span,
                );
                let arguments = vec![target_argument_id];
                let qualifier = qualifier
                    .as_ref()
                    .map(|qualifier| self.unbind_path(qualifier, ast_strings, context));
                let static_arguments = self.unbind_static_arguments(
                    module,
                    static_arguments.as_deref(),
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Expression::TypeImport {
                    target: target_expression_id,
                    arguments,
                    qualifier,
                    static_arguments,
                }
            }
            dir::Type::Infer { name, constraint } => {
                // infer type expression
                let name = ast_strings.intern_from(&self.program.strings, *name);
                let constraint = constraint.map(|constraint| {
                    self.unbind_type_expression(
                        module,
                        constraint,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                ast::Expression::TypeInfer { name, constraint }
            }
            dir::Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                // type predicate expression
                let subject = self.unbind_type_predicate_subject(
                    *subject,
                    module,
                    symbols,
                    ast_strings,
                    context,
                );
                let target = target.map(|target| {
                    self.unbind_type_expression(
                        module,
                        target,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                ast::Expression::TypePredicate {
                    asserts: *asserts,
                    subject,
                    target,
                }
            }
            dir::Type::Unary { operator, right } => {
                // unary type expression
                let operator = self.unbind_type_unary_operator(context, *operator);
                let right = self.unbind_type_expression(
                    module,
                    *right,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Expression::TypeUnary { operator, right }
            }
            dir::Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                // value of type expression
                let mutability = (*mutability).map(|m| self.unbind_mutability(context, m));
                let variance = (*variance).map(|v| self.unbind_variance_bound(context, v));
                let right = self.unbind_type_expression(
                    module,
                    *right,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Expression::ValueOf {
                    mutability,
                    variance,
                    right,
                }
            }
            dir::Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                // reference type expression
                let mutability = (*mutability).map(|m| self.unbind_mutability(context, m));
                let variance = (*variance).map(|v| self.unbind_variance_bound(context, v));
                let right = self.unbind_type_expression(
                    module,
                    *right,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Expression::ReferenceOf {
                    mutability,
                    variance,
                    right,
                }
            }
            dir::Type::PointerOf { mutability, right } => {
                // pointer type expression
                let mutability = (*mutability).map(|m| self.unbind_mutability(context, m));
                let right = self.unbind_type_expression(
                    module,
                    *right,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Expression::PointerOf { mutability, right }
            }
            dir::Type::Binary {
                left,
                operator,
                right,
            } => {
                // binary type expression
                let left = self.unbind_type_expression(
                    module,
                    *left,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let operator = self.unbind_type_binary_operator(context, *operator);
                let right = self.unbind_type_expression(
                    module,
                    *right,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Expression::TypeBinary {
                    left,
                    operator,
                    right,
                }
            }
            dir::Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
                // sized array expression
                let left = self.unbind_type_expression(
                    module,
                    *element,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let index = self.unbind_type_expression(
                    module,
                    *count,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let array_expr = ast::Expression::Index {
                    position: ast::PostfixPosition::Direct,
                    left,
                    index: Some(index),
                };
                if *is_readonly {
                    let operator =
                        self.unbind_type_unary_operator(context, dir::TypeUnaryOperator::Readonly);
                    let right = ast_tree.insert(array_expr, span);
                    ast::Expression::TypeUnary { operator, right }
                } else {
                    array_expr
                }
            }
            dir::Type::Array {
                element,
                is_readonly,
            } => {
                // slice expression
                let element = *element;
                let left = element.map(|element| {
                    self.unbind_type_expression(
                        module,
                        element,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                let array_expr = match left {
                    Some(left) => ast::Expression::Index {
                        position: ast::PostfixPosition::Direct,
                        left,
                        index: None,
                    },
                    None => ast::Expression::ArrayExpression {
                        elements: Vec::new(),
                    },
                };
                if *is_readonly {
                    let operator =
                        self.unbind_type_unary_operator(context, dir::TypeUnaryOperator::Readonly);
                    let right = ast_tree.insert(array_expr, span);
                    ast::Expression::TypeUnary { operator, right }
                } else {
                    array_expr
                }
            }
            dir::Type::Tuple {
                elements,
                is_readonly,
            } => {
                // tuple expression
                let elements = elements
                    .iter()
                    .map(|element| {
                        // tuple element argument
                        let value = self.unbind_type_expression(
                            module,
                            element.ty,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        );
                        let label = element
                            .label
                            .map(|label| ast_strings.intern_from(&self.program.strings, label));
                        let modifiers = self.unbind_type_element_modifiers(element, context);
                        let argument = if element.is_rest {
                            ast::Argument::Spread {
                                modifiers,
                                label,
                                value,
                            }
                        } else if let Some(label) = label {
                            ast::Argument::Labeled {
                                modifiers,
                                label,
                                value,
                            }
                        } else {
                            ast::Argument::Positional { modifiers, value }
                        };
                        ast_tree.insert(argument, span)
                    })
                    .collect();
                let tuple_expr = ast::Expression::TupleExpression { elements };
                if *is_readonly {
                    let operator =
                        self.unbind_type_unary_operator(context, dir::TypeUnaryOperator::Readonly);
                    let right = ast_tree.insert(tuple_expr, span);
                    ast::Expression::TypeUnary { operator, right }
                } else {
                    tuple_expr
                }
            }
            dir::Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                // object type members
                let mut properties = Vec::new();

                // field members
                for field in fields {
                    let key = self.unbind_type_field_key(
                        field.key,
                        module,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let modifiers = self.unbind_type_field_modifiers(field, context);
                    let value = self.unbind_type_expression(
                        module,
                        field.ty,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let property = ast::Property::Field {
                        modifiers,
                        key: Some(key),
                        value: Some(value),
                        default: None,
                    };
                    properties.push(ast_tree.insert(property, span));
                }

                // call signature members
                for signature_id in call_signatures {
                    let signature = self.unbind_type_function_signature(
                        module,
                        *signature_id,
                        ast::FunctionKind::Function,
                        Some(ast::FunctionMode::Call),
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let property = ast::Property::Method {
                        modifiers: None,
                        key: None,
                        signature,
                        body: None,
                    };
                    properties.push(ast_tree.insert(property, span));
                }

                // construct signature members
                for signature_id in construct_signatures {
                    let signature = self.unbind_type_function_signature(
                        module,
                        *signature_id,
                        ast::FunctionKind::Function,
                        Some(ast::FunctionMode::New),
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let property = ast::Property::Method {
                        modifiers: None,
                        key: None,
                        signature,
                        body: None,
                    };
                    properties.push(ast_tree.insert(property, span));
                }

                // index signature members
                for signature in index_signatures {
                    let key = self.unbind_type_index_signature_key(
                        module,
                        signature,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let modifiers = self.unbind_type_index_signature_modifiers(signature, context);
                    let value = self.unbind_type_expression(
                        module,
                        signature.value_type,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let property = ast::Property::Field {
                        modifiers,
                        key: Some(key),
                        value: Some(value),
                        default: None,
                    };
                    properties.push(ast_tree.insert(property, span));
                }

                ast::Expression::ObjectExpression {
                    ty: None,
                    properties,
                }
            }
            dir::Type::Function { .. } => {
                // function type as a declaration expression
                let signature = self.unbind_type_function_signature(
                    module,
                    type_id,
                    ast::FunctionKind::Lambda,
                    None,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let declaration = ast::Declaration::Function {
                    descriptor: ast::DeclarationDescriptor::default(),
                    signature,
                    body: None,
                };
                let declaration_id = ast_tree.insert(declaration, span);
                ast::Expression::Declaration(declaration_id)
            }
            dir::Type::Error => ast::Expression::Error,
            dir::Type::Unevaluated(_) => unreachable!("handled above"),
            dir::Type::Union { .. } | dir::Type::Intersection { .. } => {
                unreachable!("handled above")
            }
        };

        // insert the synthesized expression

        ast_tree.insert(ast_expression, span)
    }

    /// Unbind static arguments into AST arguments.
    fn unbind_static_arguments(
        &self,
        module: &Module,
        static_arguments: Option<&[dir::StaticArgument]>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> Option<Vec<ast::LocalNodeId<ast::Argument>>> {
        // unwrap static arguments when present
        let static_arguments = static_arguments?;

        // map each static argument into an AST argument
        let arguments = static_arguments
            .iter()
            .map(|argument| {
                self.unbind_static_argument(
                    module,
                    argument,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                )
            })
            .collect();

        // return the argument list
        Some(arguments)
    }

    /// Unbind a static argument into an AST argument.
    fn unbind_static_argument(
        &self,
        module: &Module,
        argument: &dir::StaticArgument,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Argument> {
        // reuse unevaluated arguments directly
        if let dir::StaticArgument::Unevaluated { node } = argument {
            let argument_id = node
                .try_into_local_typed::<dir::Argument>()
                .expect("static unevaluated argument should reference an argument node");
            if node.module_id == module.id {
                return self.unbind_argument(
                    module,
                    argument_id,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
            }

            let argument_module = self.program.modules.get(node.module_id);
            let argument_module = argument_module.as_ref();
            if let Some(dir) = self
                .program
                .artifacts
                .dir_patched(node.module_id, context.profile)
            {
                return self.unbind_argument(
                    &argument_module,
                    argument_id,
                    &dir.tree,
                    &dir.symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
            }
            if let Some(dir) = self
                .program
                .artifacts
                .dir_elaborated(node.module_id, context.profile)
            {
                return self.unbind_argument(
                    &argument_module,
                    argument_id,
                    &dir.tree,
                    &dir.symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
            }
            if let Some(dir) = self
                .program
                .artifacts
                .dir_analyzed(node.module_id, context.profile)
            {
                return self.unbind_argument(
                    &argument_module,
                    argument_id,
                    &dir.tree,
                    &dir.symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
            }
            if let Some(dir) = self
                .program
                .artifacts
                .dir_interface(node.module_id, context.profile)
            {
                return self.unbind_argument(
                    &argument_module,
                    argument_id,
                    &dir.tree,
                    &dir.symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
            }
            if let Some(dir) = self
                .program
                .artifacts
                .dir_declared(node.module_id, context.profile)
            {
                return self.unbind_argument(
                    &argument_module,
                    argument_id,
                    &dir.tree,
                    &dir.symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
            }
            if let Some(dir) = self
                .program
                .artifacts
                .dir_resolved(node.module_id, context.profile)
            {
                return self.unbind_argument(
                    &argument_module,
                    argument_id,
                    &dir.tree,
                    &dir.symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
            }
            if let Some(dir) = self
                .program
                .artifacts
                .dir_prepared(node.module_id, context.profile)
            {
                return self.unbind_argument(
                    &argument_module,
                    argument_id,
                    &dir.tree,
                    &dir.symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
            }
            if let Some(dir) = self.program.artifacts.dir_base(node.module_id) {
                return self.unbind_argument(
                    &argument_module,
                    argument_id,
                    &dir.tree,
                    &dir.symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
            }

            panic!(
                "missing dir artifact for static argument module {:?}",
                node.module_id
            );
        }

        // build an evaluated argument from a static value
        let dir::StaticArgument::Evaluated { name, value } = argument else {
            unreachable!("handled above");
        };

        // choose a span for the static value
        let span = match value {
            dir::StaticExpression::Type { ty } => {
                self.unbind_span(module, types.get_type_source(*ty))
            }
            _ => self.unbind_span(module, context.fallback_node),
        };

        // lower the static value expression
        let value_id = self.unbind_static_expression(
            module,
            value,
            tree,
            symbols,
            types,
            ast_tree,
            ast_strings,
            context,
        );

        // assemble the argument node
        let modifiers = None;
        let argument = if let Some(name) = name {
            let name = ast_strings.intern_from(&self.program.strings, *name);
            let name = ast::Name::Identifier(name);
            ast::Argument::Named {
                modifiers,
                name,
                value: value_id,
            }
        } else {
            ast::Argument::Positional {
                modifiers,
                value: value_id,
            }
        };

        // return the argument node
        ast_tree.insert(argument, span)
    }

    /// Unbind a static expression into an AST expression.
    fn unbind_static_expression(
        &self,
        module: &Module,
        value: &dir::StaticExpression,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Expression> {
        let span = self.unbind_span(module, context.fallback_node);

        // reuse type expressions when possible
        if let dir::StaticExpression::Type { ty } = value {
            return self.unbind_type_expression(
                module,
                *ty,
                tree,
                symbols,
                types,
                ast_tree,
                ast_strings,
                context,
            );
        }

        // lower static literals into AST expressions
        let ast_expression = match value {
            dir::StaticExpression::ScalarLiteral { value } => {
                let value = self.unbind_scalar_literal(value, ast_strings, context);
                ast::Expression::ScalarLiteral(value)
            }
            dir::StaticExpression::TypeLiteral { value } => {
                let value = self.unbind_type_literal(value, context);
                ast::Expression::TypeLiteral(value)
            }
            dir::StaticExpression::ArrayExpression { elements } => {
                // lower elements into positional arguments
                let elements = elements
                    .iter()
                    .map(|element| {
                        let value = self.unbind_static_expression(
                            module,
                            element,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        );
                        let argument = ast::Argument::Positional {
                            modifiers: None,
                            value,
                        };
                        ast_tree.insert(argument, span)
                    })
                    .collect();
                ast::Expression::ArrayExpression { elements }
            }
            dir::StaticExpression::TupleExpression { elements } => {
                // lower elements into positional arguments
                let elements = elements
                    .iter()
                    .map(|element| {
                        let value = self.unbind_static_expression(
                            module,
                            element,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        );
                        let argument = ast::Argument::Positional {
                            modifiers: None,
                            value,
                        };
                        ast_tree.insert(argument, span)
                    })
                    .collect();
                ast::Expression::TupleExpression { elements }
            }
            dir::StaticExpression::ObjectExpression { .. }
            | dir::StaticExpression::Declaration { .. }
            | dir::StaticExpression::Unevaluated { .. }
            | dir::StaticExpression::Type { .. } => ast::Expression::Error,
        };

        ast_tree.insert(ast_expression, span)
    }

    /// Unbind type element modifiers into binding modifiers.
    fn unbind_type_element_modifiers(
        &self,
        element: &dir::TypeElement,
        _context: &mut UnbindContext,
    ) -> Option<ast::BindingModifier> {
        // seed the default modifiers
        let mut modifiers = ast::BindingModifier::default();

        // apply optional and readonly modifiers
        if element.is_optional {
            modifiers.kind = Some(ast::BindingKind::Maybe);
        }
        if element.is_readonly {
            modifiers.mutability = Some(ast::Mutability::Immutable);
        }

        // return the modifier when present
        if modifiers == ast::BindingModifier::default() {
            None
        } else {
            Some(modifiers)
        }
    }

    /// Unbind type field modifiers into binding modifiers.
    fn unbind_type_field_modifiers(
        &self,
        field: &dir::TypeField,
        _context: &mut UnbindContext,
    ) -> Option<ast::BindingModifier> {
        // seed the default modifiers
        let mut modifiers = ast::BindingModifier::default();

        // apply optional and readonly modifiers
        if field.is_optional {
            modifiers.kind = Some(ast::BindingKind::Maybe);
        }
        if field.is_readonly {
            modifiers.mutability = Some(ast::Mutability::Immutable);
        }

        // return the modifier when present
        if modifiers == ast::BindingModifier::default() {
            None
        } else {
            Some(modifiers)
        }
    }

    /// Unbind a type field key into an AST key.
    fn unbind_type_field_key(
        &self,
        key: dir::StaticKey,
        module: &Module,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::Key {
        // map the static key into an AST key
        match key {
            dir::StaticKey::Name(name) => {
                let name = ast_strings.intern_from(&self.program.strings, name);
                ast::Key::Name(ast::Name::Identifier(name))
            }
            dir::StaticKey::Number(name) => {
                let name = ast_strings.intern_from(&self.program.strings, name);
                ast::Key::Name(ast::Name::Number(name))
            }
            dir::StaticKey::Symbol(symbol) => {
                let expression = self.unbind_symbol_key_expression(
                    module,
                    symbols,
                    symbol,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Key::Expression(expression)
            }
        }
    }

    /// Unbind a type index signature key into an AST key.
    fn unbind_type_index_signature_key(
        &self,
        module: &Module,
        signature: &dir::TypeIndexSignature,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::Key {
        // bind the key name
        let name = ast_strings.intern_from(&self.program.strings, signature.name);

        // bind the key type expression
        let key = self.unbind_type_expression(
            module,
            signature.key_type,
            tree,
            symbols,
            types,
            ast_tree,
            ast_strings,
            context,
        );

        // return the named expression key
        ast::Key::NamedExpression { name, key }
    }

    /// Unbind type index signature modifiers into binding modifiers.
    fn unbind_type_index_signature_modifiers(
        &self,
        signature: &dir::TypeIndexSignature,
        _context: &mut UnbindContext,
    ) -> Option<ast::BindingModifier> {
        // seed the default modifiers
        let mut modifiers = ast::BindingModifier::default();

        // apply readonly modifiers
        if signature.is_readonly {
            modifiers.mutability = Some(ast::Mutability::Immutable);
        }

        // return the modifier when present
        if modifiers == ast::BindingModifier::default() {
            None
        } else {
            Some(modifiers)
        }
    }

    /// Unbind a type function signature into an AST function signature.
    fn unbind_type_function_signature(
        &self,
        module: &Module,
        signature_id: dir::LocalTypeId,
        kind: ast::FunctionKind,
        mode: Option<ast::FunctionMode>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::FunctionSignature {
        // read the function signature from the type table
        let dir::Type::Function {
            asynchrony,
            cardinality,
            static_parameters: _,
            this_parameter,
            dynamic_parameters,
            return_type,
        } = types.get_type(signature_id)
        else {
            return ast::FunctionSignature {
                abstraction: ast::FunctionAbstraction::Concrete,
                asynchrony: ast::Asynchrony::Sync,
                cardinality: ast::FunctionCardinality::Scalar,
                mode,
                kind,
                generics: None,
                this_parameter: None,
                dynamic_parameters: Vec::new(),
                return_type: None,
            };
        };

        // keep the shared fields in locals
        let this_parameter = *this_parameter;
        let return_type = *return_type;

        // build the optional this parameter
        let this_parameter = this_parameter.map(|this_type| {
            let name = ast_strings.intern("this");
            let ty = Some(self.unbind_type_expression(
                module,
                this_type,
                tree,
                symbols,
                types,
                ast_tree,
                ast_strings,
                context,
            ));
            let parameter = ast::Parameter::Named {
                modifiers: None,
                name,
                ty,
                default: None,
            };
            ast_tree.insert(
                parameter,
                self.unbind_span(module, types.get_type_source(signature_id)),
            )
        });

        // build dynamic parameters with synthetic names
        let parameter_types = dynamic_parameters.as_slice();
        let mut dynamic_parameters = Vec::with_capacity(parameter_types.len());
        for (index, parameter_type_id) in parameter_types.iter().enumerate() {
            let name = format!("arg{index}");
            let name = ast_strings.intern(&name);
            let parameter_type = self.unbind_type_expression(
                module,
                *parameter_type_id,
                tree,
                symbols,
                types,
                ast_tree,
                ast_strings,
                context,
            );
            let parameter = ast::Parameter::Named {
                modifiers: None,
                name,
                ty: Some(parameter_type),
                default: None,
            };
            let parameter_id = ast_tree.insert(
                parameter,
                self.unbind_span(module, types.get_type_source(*parameter_type_id)),
            );
            dynamic_parameters.push(parameter_id);
        }

        // build the return type expression
        let return_type = return_type.map(|return_type| {
            self.unbind_type_expression(
                module,
                return_type,
                tree,
                symbols,
                types,
                ast_tree,
                ast_strings,
                context,
            )
        });

        // map asynchrony and cardinality
        let asynchrony = self.unbind_asynchrony(context, *asynchrony);
        let cardinality = match cardinality {
            dir::FunctionCardinality::Scalar => ast::FunctionCardinality::Scalar,
            dir::FunctionCardinality::Generator => ast::FunctionCardinality::Generator,
        };

        // assemble the signature
        ast::FunctionSignature {
            abstraction: ast::FunctionAbstraction::Concrete,
            asynchrony,
            cardinality,
            mode,
            kind,
            generics: None,
            this_parameter,
            dynamic_parameters,
            return_type,
        }
    }

    /// Unbind a symbol id into a name for type paths.
    fn unbind_symbol_name(
        &self,
        symbol_id: dir::GlobalSymbolId,
        module: &Module,
        symbols: &dir::SymbolTable,
        ast_strings: &mut StringPool,
    ) -> ast::StringId {
        // load the symbol key name from the owning module
        let name = if symbol_id.module_id == module.id {
            symbols
                .get_symbol(symbol_id.into_local())
                .key
                .and_then(|key| match key {
                    dir::StaticKey::Name(name) | dir::StaticKey::Number(name) => Some(name),
                    _ => None,
                })
        } else {
            self.artifact_dir_base(symbol_id.module_id).and_then(|dir| {
                dir.symbols
                    .get_symbol(symbol_id.into_local())
                    .key
                    .and_then(|key| match key {
                        dir::StaticKey::Name(name) | dir::StaticKey::Number(name) => Some(name),
                        _ => None,
                    })
            })
        };

        // return the interned name with a fallback

        name.map(|name| ast_strings.intern_from(&self.program.strings, name))
            .unwrap_or_else(|| ast_strings.intern("_"))
    }

    /// Unbind a symbol key into an AST key expression.
    fn unbind_symbol_key_expression(
        &self,
        module: &Module,
        _symbols: &dir::SymbolTable,
        symbol: dir::SymbolKey,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Expression> {
        let span = self.unbind_span(module, context.fallback_node);

        // handle well known symbol keys as Symbol.<member>
        if let dir::SymbolKey::WellKnown(key) = symbol {
            let symbol_name = ast_strings.intern("Symbol");
            let member_name = ast_strings.intern(key.member_name());
            let path = ast::Path {
                segments: smallvec![symbol_name, member_name],
            };
            return ast_tree.insert(
                ast::Expression::Path {
                    path,
                    static_arguments: None,
                },
                span,
            );
        }

        // fall back to an error expression for uncommon keys
        ast_tree.insert(ast::Expression::Error, span)
    }

    /// Unbind a DIR mutability to an AST mutability.
    #[inline]
    pub(super) fn unbind_mutability(
        &self,
        _context: &mut UnbindContext,
        mutability: dir::Mutability,
    ) -> ast::Mutability {
        match mutability {
            dir::Mutability::Immutable => ast::Mutability::Immutable,
            dir::Mutability::Mutable => ast::Mutability::Mutable,
        }
    }

    /// Unbind a DIR variance bound to an AST variance bound.
    #[inline]
    pub(super) fn unbind_variance_bound(
        &self,
        _context: &mut UnbindContext,
        variance: dir::VarianceBound,
    ) -> ast::VarianceBound {
        match variance {
            dir::VarianceBound::Implements => ast::VarianceBound::Implements,
            dir::VarianceBound::Extends => ast::VarianceBound::Extends,
            dir::VarianceBound::Super => ast::VarianceBound::Super,
        }
    }

    /// Unbind a DIR type modifier to an AST type modifier.
    #[inline]
    pub(super) fn unbind_type_modifier(
        &self,
        _context: &mut UnbindContext,
        modifier: dir::TypeModifier,
    ) -> ast::TypeModifier {
        match modifier {
            dir::TypeModifier::Add => ast::TypeModifier::Add,
            dir::TypeModifier::Remove => ast::TypeModifier::Remove,
            dir::TypeModifier::None => ast::TypeModifier::None,
        }
    }

    /// Unbind DIR mapped type modifiers to AST mapped type modifiers.
    #[inline]
    pub(super) fn unbind_type_mapped_modifiers(
        &self,
        _context: &mut UnbindContext,
        modifiers: dir::TypeMappedModifiers,
    ) -> ast::TypeMappedModifiers {
        ast::TypeMappedModifiers {
            readonly: self.unbind_type_modifier(_context, modifiers.readonly),
            optional: self.unbind_type_modifier(_context, modifiers.optional),
        }
    }

    /// Unbind a DIR asynchrony to an AST asynchrony.
    #[inline]
    pub(super) fn unbind_asynchrony(
        &self,
        _context: &mut UnbindContext,
        asynchrony: dir::Asynchrony,
    ) -> ast::Asynchrony {
        match asynchrony {
            dir::Asynchrony::Sync => ast::Asynchrony::Sync,
            dir::Asynchrony::Async => ast::Asynchrony::Async,
        }
    }

    /// Unbind a DIR type kind to an AST type kind.
    #[inline]
    pub(super) fn unbind_type_kind(
        &self,
        _context: &mut UnbindContext,
        kind: dir::TypeKind,
    ) -> ast::TypeKind {
        match kind {
            dir::TypeKind::Structural => ast::TypeKind::Structural,
            dir::TypeKind::Nominal => ast::TypeKind::Nominal,
        }
    }

    /// Unbind DIR generics to AST generics.
    pub(super) fn unbind_generics(
        &self,
        module: &Module,
        generics: &dir::Generics,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::Generics {
        let static_parameters = generics
            .static_parameters
            .as_ref()
            .map(|static_parameters| {
                static_parameters
                    .iter()
                    .map(|static_parameter| {
                        self.unbind_parameter(
                            module,
                            *static_parameter,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect()
            });
        let where_clauses = generics.where_clauses.as_ref().map(|where_clauses| {
            where_clauses
                .iter()
                .map(|where_clause| {
                    self.unbind_where_clause(
                        module,
                        *where_clause,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                })
                .collect()
        });
        ast::Generics {
            static_parameters,
            where_clauses,
        }
    }

    /// Unbind DIR heritage to AST heritage.
    pub(super) fn unbind_heritage(
        &self,
        module: &Module,
        heritage: &dir::Heritage,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::Heritage {
        let extends_types = heritage.extends_types.as_ref().map(|extends_types| {
            extends_types
                .iter()
                .map(|extends_type| {
                    self.unbind_expression(
                        module,
                        *extends_type,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                })
                .collect()
        });
        let implements_types = heritage.implements_types.as_ref().map(|implements_types| {
            implements_types
                .iter()
                .map(|implements_type| {
                    self.unbind_expression(
                        module,
                        *implements_type,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                })
                .collect()
        });
        ast::Heritage {
            extends_types,
            implements_types,
        }
    }
}

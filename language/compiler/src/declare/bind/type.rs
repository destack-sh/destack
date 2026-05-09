use destack_artifact::Ast;
use destack_ast::{self as ast, StringId};
use destack_dir::{
    BindingTable, ConstructorTypeDeclaration, DeclaredModule, FunctionTypeDeclaration,
    GenericArgument, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, MappedTypeModifier,
    Mutability, NodeType, ScopeKind, StaticKey, SymbolBinding, SymbolForm, SymbolRole, SymbolSpace,
    Tree, TupleElement, TypeExpression, TypeMappedParameter, TypeMember, TypePredicateSubject,
    TypeTable, VarianceBound,
};
use destack_workspace::Module;

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a generic argument into a DIR generic argument node.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn bind_generic_argument(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_argument_id: ast::LocalNodeId<ast::GenericArgument>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut BindingTable,
        types: &mut TypeTable,
        space: SymbolSpace,
    ) -> LocalNodeId<GenericArgument> {
        let ast_argument = ast.tree.get(ast_argument_id);
        let argument_id = tree.reserve_from_source(
            NodeType::GenericArgument,
            ast_argument_id.id,
            scope,
            parent_id,
        );

        match ast_argument {
            ast::GenericArgument::Type { value } => {
                let value = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(argument_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(argument_id, GenericArgument::Type { value })
            }
            ast::GenericArgument::Value { value } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(argument_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(argument_id, GenericArgument::Value { value })
            }
            ast::GenericArgument::Error => tree.insert(argument_id, GenericArgument::Error),
        }
    }

    /// Bind a tuple element into a DIR tuple element node.
    #[allow(clippy::too_many_arguments)]
    fn bind_tuple_element(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_element_id: ast::LocalNodeId<ast::TupleElement>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut BindingTable,
        types: &mut TypeTable,
        space: SymbolSpace,
    ) -> LocalNodeId<TupleElement> {
        let ast_element = ast.tree.get(ast_element_id);
        let element_id =
            tree.reserve_from_source(NodeType::TupleElement, ast_element_id.id, scope, parent_id);

        match ast_element {
            ast::TupleElement::Element {
                label,
                value,
                is_optional,
                is_readonly,
            } => {
                let label = label.map(|label| label);
                let value = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(element_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(
                    element_id,
                    TupleElement::Element {
                        label,
                        value,
                        is_optional: *is_optional,
                        is_readonly: *is_readonly,
                    },
                )
            }
            ast::TupleElement::Spread { label, value } => {
                let label = label.map(|label| label);
                let value = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(element_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(element_id, TupleElement::Spread { label, value })
            }
            ast::TupleElement::Error => tree.insert(element_id, TupleElement::Error),
        }
    }

    /// Bind a type member into a DIR type member node.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn bind_type_member(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_member_id: ast::LocalNodeId<ast::TypeMember>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut BindingTable,
        types: &mut TypeTable,
        space: SymbolSpace,
    ) -> LocalNodeId<TypeMember> {
        let ast_member = ast.tree.get(ast_member_id);
        let member_id =
            tree.reserve_from_source(NodeType::TypeMember, ast_member_id.id, scope, parent_id);

        match ast_member {
            ast::TypeMember::Field {
                is_static,
                is_optional,
                is_readonly,
                key,
                declared_type,
            } => {
                // member symbol
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);

                // member syntax
                let key = self.bind_key(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *key,
                    Some(member_id.into()),
                    tree,
                    symbols,
                    types,
                );
                let declared_type = declared_type.map(|declared_type| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        declared_type,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });

                let member_id = tree.insert(
                    member_id,
                    TypeMember::Field {
                        is_static: *is_static,
                        is_optional: *is_optional,
                        is_readonly: *is_readonly,
                        key,
                        declared_type,
                        symbol: symbol_id,
                    },
                );
                symbols.declare_symbol(symbol_id, member_id);
                member_id
            }
            ast::TypeMember::Method {
                is_static,
                is_optional,
                key,
                signature,
                body,
            } => {
                // member symbol and scope
                let (symbol_id, method_scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Function,
                    scope,
                    None,
                    symbols,
                );
                let member_id = tree.reserve_from_source(
                    NodeType::TypeMember,
                    ast_member_id.id,
                    (method_scope_id, LocalScopeMark::end()),
                    parent_id,
                );

                // bind the implicit this local for method bodies
                let this_name = StringId::for_text("this");
                let method_scope = (method_scope_id, symbols.get_scope_mark(method_scope_id));
                self.bind_named_local(
                    module,
                    ast,
                    SymbolSpace::Value,
                    StaticKey::Name(this_name),
                    method_scope,
                    symbols,
                );

                // member syntax
                let key = self.bind_key(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    method_scope,
                    *key,
                    Some(member_id.into()),
                    tree,
                    symbols,
                    types,
                );
                let signature = self.bind_function_signature(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    method_scope,
                    signature,
                    Some(member_id.into()),
                    tree,
                    symbols,
                    types,
                );
                let body_scope = (method_scope.0, symbols.get_scope_mark(method_scope.0));
                let body = body.map(|body| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        body_scope,
                        body,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Type,
                    )
                });

                let member_id = tree.insert(
                    member_id,
                    TypeMember::Method {
                        is_static: *is_static,
                        is_optional: *is_optional,
                        key,
                        signature,
                        body,
                        symbol: symbol_id,
                    },
                );
                symbols.declare_symbol(symbol_id, member_id);
                member_id
            }
            ast::TypeMember::CallSignature { signature } => {
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let signature_scope_id =
                    symbols.insert_scope(ScopeKind::Type, Some(scope), Some(symbol_id));
                let mut signature_scope = (
                    signature_scope_id,
                    symbols.get_scope_mark(signature_scope_id),
                );

                let generic_parameters = signature
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_generic_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            signature_scope,
                            *parameter,
                            Some(member_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                signature_scope = (
                    signature_scope_id,
                    symbols.get_scope_mark(signature_scope_id),
                );

                let this_parameter = signature.this_parameter.map(|parameter| {
                    self.bind_parameter(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        signature_scope,
                        SymbolSpace::Type,
                        parameter,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                    )
                });
                let parameters = signature
                    .parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            signature_scope,
                            SymbolSpace::Type,
                            *parameter,
                            Some(member_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                signature_scope = (
                    signature_scope_id,
                    symbols.get_scope_mark(signature_scope_id),
                );

                let return_type = signature.return_type.map(|return_type| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        signature_scope,
                        return_type,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                let where_clauses = signature
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.bind_where_clause(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            signature_scope,
                            *where_clause,
                            Some(member_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                let signature = FunctionTypeDeclaration {
                    generic_parameters,
                    where_clauses,
                    this_parameter,
                    parameters,
                    return_type,
                };

                let member_id = tree.insert(
                    member_id,
                    TypeMember::CallSignature {
                        signature,
                        symbol: symbol_id,
                    },
                );
                symbols.declare_symbol(symbol_id, member_id);
                member_id
            }
            ast::TypeMember::ConstructSignature { signature } => {
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let signature_scope_id =
                    symbols.insert_scope(ScopeKind::Type, Some(scope), Some(symbol_id));
                let mut signature_scope = (
                    signature_scope_id,
                    symbols.get_scope_mark(signature_scope_id),
                );

                let is_abstract = signature.is_abstract;
                let generic_parameters = signature
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_generic_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            signature_scope,
                            *parameter,
                            Some(member_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                signature_scope = (
                    signature_scope_id,
                    symbols.get_scope_mark(signature_scope_id),
                );

                let parameters = signature
                    .parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            signature_scope,
                            SymbolSpace::Type,
                            *parameter,
                            Some(member_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                signature_scope = (
                    signature_scope_id,
                    symbols.get_scope_mark(signature_scope_id),
                );

                let return_type = signature.return_type.map(|return_type| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        signature_scope,
                        return_type,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                let where_clauses = signature
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.bind_where_clause(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            signature_scope,
                            *where_clause,
                            Some(member_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                let signature = ConstructorTypeDeclaration {
                    is_abstract,
                    generic_parameters,
                    where_clauses,
                    parameters,
                    return_type,
                };

                let member_id = tree.insert(
                    member_id,
                    TypeMember::ConstructSignature {
                        signature,
                        symbol: symbol_id,
                    },
                );
                symbols.declare_symbol(symbol_id, member_id);
                member_id
            }
            ast::TypeMember::IndexSignature {
                is_optional,
                is_readonly,
                name,
                key_type,
                value_type,
            } => {
                // member symbol
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);

                // member syntax
                let name = *name;
                let key_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *key_type,
                    Some(member_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let value_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value_type,
                    Some(member_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                let member_id = tree.insert(
                    member_id,
                    TypeMember::IndexSignature {
                        is_optional: *is_optional,
                        is_readonly: *is_readonly,
                        name,
                        key_type,
                        value_type,
                        symbol: symbol_id,
                    },
                );
                symbols.declare_symbol(symbol_id, member_id);
                member_id
            }
            ast::TypeMember::Embed { value } => {
                // member symbol
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);

                // member syntax
                let value = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(member_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                let member_id = tree.insert(
                    member_id,
                    TypeMember::Embed {
                        value,
                        symbol: symbol_id,
                    },
                );
                symbols.declare_symbol(symbol_id, member_id);
                member_id
            }
            ast::TypeMember::AssociatedType {
                name,
                generic_parameters,
                where_clauses,
                constraint,
                value,
            } => {
                // member name
                let name = *name;

                // member syntax
                let generic_parameters = generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_generic_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *parameter,
                            Some(member_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                let scope = (scope.0, symbols.get_scope_mark(scope.0));
                let where_clauses = where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.bind_where_clause(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *where_clause,
                            Some(member_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                let constraint = constraint.map(|constraint| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        constraint,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Type,
                    )
                });
                let value = value.map(|value| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        value,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Type,
                    )
                });

                // member symbol
                let (symbol_id, _) = symbols.insert_symbol(
                    SymbolRole::Item,
                    SymbolForm::TypeAlias,
                    SymbolSpace::Type,
                    SymbolBinding::Runtime,
                    Some(StaticKey::Name(name)),
                    scope,
                    None,
                );
                let member_id = tree.insert(
                    member_id,
                    TypeMember::AssociatedType {
                        name,
                        generic_parameters,
                        where_clauses,
                        constraint,
                        value,
                        symbol: symbol_id,
                    },
                );
                symbols.declare_symbol(symbol_id, member_id);
                member_id
            }
            ast::TypeMember::AssociatedConst {
                name,
                declared_type,
                value,
            } => {
                // member name
                let name = *name;

                // member syntax
                let declared_type = declared_type.map(|declared_type| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        declared_type,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Type,
                    )
                });
                let value = value.map(|value| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        value,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Type,
                    )
                });

                // member symbol
                let (symbol_id, _) = symbols.insert_symbol(
                    SymbolRole::Item,
                    SymbolForm::Variable,
                    SymbolSpace::Value,
                    SymbolBinding::Runtime,
                    Some(StaticKey::Name(name)),
                    scope,
                    None,
                );
                let member_id = tree.insert(
                    member_id,
                    TypeMember::AssociatedConst {
                        name,
                        declared_type,
                        value,
                        symbol: symbol_id,
                    },
                );
                symbols.declare_symbol(symbol_id, member_id);
                member_id
            }
            ast::TypeMember::Error => {
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let member_id = tree.insert(member_id, TypeMember::Error { symbol: symbol_id });
                symbols.declare_symbol(symbol_id, member_id);
                member_id
            }
        }
    }

    /// Bind a type expression into DIR type-expression space.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn bind_type_expression(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_type_expression_id: ast::LocalNodeId<ast::TypeExpression>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut BindingTable,
        types: &mut TypeTable,
        space: SymbolSpace,
    ) -> LocalNodeId<TypeExpression> {
        let ast_type_expression = ast.tree.get(ast_type_expression_id);
        let type_expression_id = tree.reserve_from_source(
            NodeType::TypeExpression,
            ast_type_expression_id.id,
            scope,
            parent_id,
        );

        match ast_type_expression {
            ast::TypeExpression::Parenthesized { expression } => {
                let expression = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *expression,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(
                    type_expression_id,
                    TypeExpression::Parenthesized { expression },
                )
            }
            ast::TypeExpression::ScalarLiteral { value } => {
                let value = self.bind_scalar_literal(module, ast, value);
                tree.insert(type_expression_id, TypeExpression::ScalarLiteral { value })
            }
            ast::TypeExpression::Literal { value } => {
                let value = self.bind_type_literal(value);
                tree.insert(type_expression_id, TypeExpression::Literal { value })
            }
            ast::TypeExpression::Intrinsic => {
                tree.insert(type_expression_id, TypeExpression::Intrinsic)
            }
            ast::TypeExpression::Tuple { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.bind_tuple_element(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *element,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                            space,
                        )
                    })
                    .collect();

                tree.insert(type_expression_id, TypeExpression::Tuple { elements })
            }
            ast::TypeExpression::ArrayTuple { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.bind_tuple_element(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *element,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                            space,
                        )
                    })
                    .collect();

                tree.insert(type_expression_id, TypeExpression::ArrayTuple { elements })
            }
            ast::TypeExpression::Array { element } => {
                let element = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *element,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(type_expression_id, TypeExpression::Array { element })
            }
            ast::TypeExpression::Slice { element } => {
                let element = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *element,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(type_expression_id, TypeExpression::Slice { element })
            }
            ast::TypeExpression::FixedArray { element, length } => {
                let element = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *element,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let length = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *length,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(
                    type_expression_id,
                    TypeExpression::FixedArray { element, length },
                )
            }
            ast::TypeExpression::Object { members } => {
                let members = members
                    .iter()
                    .map(|member| {
                        self.bind_type_member(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *member,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                            space,
                        )
                    })
                    .collect();

                tree.insert(type_expression_id, TypeExpression::Object { members })
            }
            ast::TypeExpression::Declaration { declaration } => {
                let declaration = self.bind_declaration(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *declaration,
                    false,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                );

                tree.insert(
                    type_expression_id,
                    TypeExpression::Declaration { declaration },
                )
            }
            ast::TypeExpression::FunctionTypeDeclaration(function) => {
                let signature_scope_id = symbols.insert_scope(ScopeKind::Type, Some(scope), None);
                let mut signature_scope = (
                    signature_scope_id,
                    symbols.get_scope_mark(signature_scope_id),
                );

                let generic_parameters = function
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_generic_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            signature_scope,
                            *parameter,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                signature_scope = (
                    signature_scope_id,
                    symbols.get_scope_mark(signature_scope_id),
                );

                let this_parameter = function.this_parameter.map(|parameter| {
                    self.bind_parameter(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        signature_scope,
                        SymbolSpace::Type,
                        parameter,
                        Some(type_expression_id.into()),
                        tree,
                        symbols,
                        types,
                    )
                });
                let parameters = function
                    .parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            signature_scope,
                            SymbolSpace::Type,
                            *parameter,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                signature_scope = (
                    signature_scope_id,
                    symbols.get_scope_mark(signature_scope_id),
                );

                let return_type = function.return_type.map(|return_type| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        signature_scope,
                        return_type,
                        Some(type_expression_id.into()),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                let where_clauses = function
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.bind_where_clause(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            signature_scope,
                            *where_clause,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                tree.insert(
                    type_expression_id,
                    TypeExpression::FunctionTypeDeclaration(FunctionTypeDeclaration {
                        generic_parameters,
                        where_clauses,
                        this_parameter,
                        parameters,
                        return_type,
                    }),
                )
            }
            ast::TypeExpression::ConstructorTypeDeclaration(function) => {
                let signature_scope_id = symbols.insert_scope(ScopeKind::Type, Some(scope), None);
                let mut signature_scope = (
                    signature_scope_id,
                    symbols.get_scope_mark(signature_scope_id),
                );

                let generic_parameters = function
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_generic_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            signature_scope,
                            *parameter,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                signature_scope = (
                    signature_scope_id,
                    symbols.get_scope_mark(signature_scope_id),
                );

                let parameters = function
                    .parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            signature_scope,
                            SymbolSpace::Type,
                            *parameter,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                signature_scope = (
                    signature_scope_id,
                    symbols.get_scope_mark(signature_scope_id),
                );

                let return_type = function.return_type.map(|return_type| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        signature_scope,
                        return_type,
                        Some(type_expression_id.into()),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                let where_clauses = function
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.bind_where_clause(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            signature_scope,
                            *where_clause,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                tree.insert(
                    type_expression_id,
                    TypeExpression::ConstructorTypeDeclaration(ConstructorTypeDeclaration {
                        is_abstract: function.is_abstract,
                        generic_parameters,
                        where_clauses,
                        parameters,
                        return_type,
                    }),
                )
            }
            ast::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                let path = self.bind_path(module, ast, path);
                let generic_arguments = generic_arguments
                    .iter()
                    .map(|argument| {
                        self.bind_generic_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *argument,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                            space,
                        )
                    })
                    .collect();

                tree.insert(
                    type_expression_id,
                    TypeExpression::Reference {
                        path,
                        generic_arguments,
                    },
                )
            }
            ast::TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let left = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *left,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let name = *name;
                let generic_arguments = generic_arguments
                    .iter()
                    .map(|argument| {
                        self.bind_generic_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *argument,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                            space,
                        )
                    })
                    .collect();

                tree.insert(
                    type_expression_id,
                    TypeExpression::Member {
                        left,
                        name,
                        generic_arguments,
                    },
                )
            }
            ast::TypeExpression::Const => tree.insert(type_expression_id, TypeExpression::Const),
            ast::TypeExpression::This => tree.insert(type_expression_id, TypeExpression::This),
            ast::TypeExpression::Readonly { target_type } => {
                let target_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *target_type,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(type_expression_id, TypeExpression::Readonly { target_type })
            }
            ast::TypeExpression::KeyOf { target_type } => {
                let target_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *target_type,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(type_expression_id, TypeExpression::KeyOf { target_type })
            }
            ast::TypeExpression::TypeOfValue { value } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *value,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                );

                tree.insert(type_expression_id, TypeExpression::TypeOfValue { value })
            }
            ast::TypeExpression::Must { target_type } => {
                let target_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *target_type,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(type_expression_id, TypeExpression::Must { target_type })
            }
            ast::TypeExpression::AsComptime { target_type } => {
                let target_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *target_type,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(
                    type_expression_id,
                    TypeExpression::AsComptime { target_type },
                )
            }
            ast::TypeExpression::Not { target_type } => {
                let target_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *target_type,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(type_expression_id, TypeExpression::Not { target_type })
            }
            ast::TypeExpression::OwnedOf {
                mutability,
                variance,
                target_type,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let variance = variance.map(|variance| self.bind_variance_bound(variance));
                let target_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *target_type,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(
                    type_expression_id,
                    TypeExpression::OwnedOf {
                        mutability,
                        variance,
                        target_type,
                    },
                )
            }
            ast::TypeExpression::BorrowedOf {
                mutability,
                variance,
                target_type,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let variance = variance.map(|variance| self.bind_variance_bound(variance));
                let target_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *target_type,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(
                    type_expression_id,
                    TypeExpression::BorrowedOf {
                        mutability,
                        variance,
                        target_type,
                    },
                )
            }
            ast::TypeExpression::PointerOf {
                mutability,
                target_type,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let target_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *target_type,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(
                    type_expression_id,
                    TypeExpression::PointerOf {
                        mutability,
                        target_type,
                    },
                )
            }
            ast::TypeExpression::Union { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.bind_type_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *element,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                            space,
                        )
                    })
                    .collect();

                tree.insert(type_expression_id, TypeExpression::Union { elements })
            }
            ast::TypeExpression::Intersection { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.bind_type_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *element,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                            space,
                        )
                    })
                    .collect();

                tree.insert(
                    type_expression_id,
                    TypeExpression::Intersection { elements },
                )
            }
            ast::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                let left = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *left,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                let conditional_scope_id =
                    symbols.insert_scope(ScopeKind::TypeConditional, Some(scope), None);
                let conditional_scope = (
                    conditional_scope_id,
                    symbols.get_scope_mark(conditional_scope_id),
                );
                let extends_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    conditional_scope,
                    *extends_type,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                let conditional_scope = (
                    conditional_scope_id,
                    symbols.get_scope_mark(conditional_scope_id),
                );
                let then_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    conditional_scope,
                    *then_type,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let else_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *else_type,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(
                    type_expression_id,
                    TypeExpression::Conditional {
                        left,
                        extends_type,
                        then_type,
                        else_type,
                    },
                )
            }
            ast::TypeExpression::Mapped {
                parameter,
                readonly,
                optional,
                value,
            } => {
                let name = parameter.name;
                let source_type = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    parameter.source_type,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                let parameter_scope_id = symbols.insert_scope(ScopeKind::Type, Some(scope), None);
                let parameter_scope = (
                    parameter_scope_id,
                    symbols.get_scope_mark(parameter_scope_id),
                );
                let (symbol, _) = self.bind_named_local(
                    module,
                    ast,
                    SymbolSpace::Type,
                    StaticKey::Name(name),
                    parameter_scope,
                    symbols,
                );

                let parameter_scope = (
                    parameter_scope_id,
                    symbols.get_scope_mark(parameter_scope_id),
                );
                let key_remap = parameter.key_remap.map(|key_remap| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        parameter_scope,
                        key_remap,
                        Some(type_expression_id.into()),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });
                let parameter = TypeMappedParameter {
                    name,
                    symbol,
                    source_type,
                    key_remap,
                };

                let parameter_scope = (
                    parameter_scope_id,
                    symbols.get_scope_mark(parameter_scope_id),
                );
                let value = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    parameter_scope,
                    *value,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(
                    type_expression_id,
                    TypeExpression::Mapped {
                        parameter,
                        readonly: self.bind_type_modifier(*readonly),
                        optional: self.bind_type_modifier(*optional),
                        value,
                    },
                )
            }
            ast::TypeExpression::Index { left, index } => {
                let left = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *left,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );
                let index = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    *index,
                    Some(type_expression_id.into()),
                    tree,
                    symbols,
                    types,
                    space,
                );

                tree.insert(type_expression_id, TypeExpression::Index { left, index })
            }
            ast::TypeExpression::TemplateLiteral { strings, spans } => {
                let strings = strings.iter().map(|string| *string).collect();
                let spans = spans
                    .iter()
                    .map(|span| {
                        self.bind_type_expression(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *span,
                            Some(type_expression_id.into()),
                            tree,
                            symbols,
                            types,
                            space,
                        )
                    })
                    .collect();

                tree.insert(
                    type_expression_id,
                    TypeExpression::TemplateLiteral { strings, spans },
                )
            }
            ast::TypeExpression::Infer { name, constraint } => {
                let name = *name;
                let constraint = constraint.map(|constraint| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        constraint,
                        Some(type_expression_id.into()),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });

                if let Some(infer_scope) =
                    self.find_nearest_scope_of_kind(symbols, scope, ScopeKind::TypeConditional)
                {
                    let _ = self.bind_named_local(
                        module,
                        ast,
                        SymbolSpace::Type,
                        StaticKey::Name(name),
                        infer_scope,
                        symbols,
                    );
                }

                tree.insert(
                    type_expression_id,
                    TypeExpression::Infer { name, constraint },
                )
            }
            ast::TypeExpression::Predicate {
                asserts,
                subject,
                target,
            } => {
                let subject = match subject {
                    ast::TypePredicateSubject::Identifier(name) => {
                        let name = *name;
                        TypePredicateSubject::Identifier(name)
                    }
                    ast::TypePredicateSubject::This => TypePredicateSubject::This,
                };
                let target = target.map(|target| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_scope,
                        declared_modules,
                        scope,
                        target,
                        Some(type_expression_id.into()),
                        tree,
                        symbols,
                        types,
                        space,
                    )
                });

                tree.insert(
                    type_expression_id,
                    TypeExpression::Predicate {
                        asserts: *asserts,
                        subject,
                        target,
                    },
                )
            }
            ast::TypeExpression::Missing => {
                tree.insert(type_expression_id, TypeExpression::Missing)
            }
            ast::TypeExpression::Error => tree.insert(type_expression_id, TypeExpression::Error),
        }
    }

    /// Bind mutability into a DIR mutability.
    #[inline]
    pub(super) fn bind_mutability(&self, mutability: ast::Mutability) -> Mutability {
        match mutability {
            ast::Mutability::Immutable => Mutability::Immutable,
            ast::Mutability::Mutable => Mutability::Mutable,
        }
    }

    /// Bind a variance bound into a DIR variance bound.
    pub(super) fn bind_variance_bound(&self, bound: ast::VarianceBound) -> VarianceBound {
        match bound {
            ast::VarianceBound::Implements => VarianceBound::Implements,
            ast::VarianceBound::Extends => VarianceBound::Extends,
            ast::VarianceBound::Super => VarianceBound::Super,
        }
    }

    /// Bind a type modifier into a DIR type modifier.
    #[inline]
    pub(super) fn bind_type_modifier(
        &self,
        modifier: ast::MappedTypeModifier,
    ) -> MappedTypeModifier {
        match modifier {
            ast::MappedTypeModifier::Present => MappedTypeModifier::Present,
            ast::MappedTypeModifier::Add => MappedTypeModifier::Add,
            ast::MappedTypeModifier::Remove => MappedTypeModifier::Remove,
            ast::MappedTypeModifier::None => MappedTypeModifier::None,
        }
    }
}

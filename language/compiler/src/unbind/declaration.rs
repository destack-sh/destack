use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR namespace form to an AST namespace form.
    #[inline]
    fn unbind_namespace_form(&self, form: dir::NamespaceForm) -> ast::NamespaceForm {
        match form {
            dir::NamespaceForm::Namespace => ast::NamespaceForm::Namespace,
            dir::NamespaceForm::Module => ast::NamespaceForm::Module,
        }
    }

    /// Unbind a DIR enum kind to an AST enum kind.
    #[inline]
    fn unbind_enum_kind(&self, kind: dir::EnumKind) -> ast::EnumKind {
        match kind {
            dir::EnumKind::Enum => ast::EnumKind::Enum,
            dir::EnumKind::Const => ast::EnumKind::Const,
        }
    }

    /// Unbind a DIR declarator to an AST declarator.
    pub(super) fn unbind_declarator(
        &self,
        module: &Module,
        declarator_id: dir::LocalNodeId<dir::Declarator>,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Declarator> {
        let declarator = tree.get(declarator_id);
        let span = self.unbind_span(module, declarator_id.into());

        // pattern and initializer
        let pattern = self.unbind_pattern(
            module,
            declarator.pattern,
            tree,
            symbols,
            types,
            ast_tree,
            ast_strings,
            context,
        );
        let value = declarator.value.map(|value| {
            self.unbind_expression(
                module,
                value,
                tree,
                symbols,
                types,
                ast_tree,
                ast_strings,
                context,
            )
        });

        let ast_declarator = ast::Declarator {
            pattern,
            ty: None,
            value,
        };
        let ast_declarator_id = ast_tree.insert(ast_declarator, span);
        context.map(declarator_id.into_any(), ast_declarator_id.into_any());

        ast_declarator_id
    }

    /// Unbind a DIR declaration to an AST declaration.
    pub(super) fn unbind_declaration(
        &self,
        module: &Module,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Declaration> {
        let declaration = tree.get(declaration_id);
        let span = self.unbind_span(module, declaration_id.into());

        let ast_declaration = match declaration {
            dir::Declaration::Global(declaration) => {
                // body
                let is_ambient = self.unbind_ambientness(declaration.is_ambient, context);
                let expressions = declaration
                    .expressions
                    .iter()
                    .map(|expression| {
                        self.unbind_expression(
                            module,
                            *expression,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::Declaration::Global(ast::GlobalDeclaration {
                    is_ambient,
                    expressions,
                })
            }
            dir::Declaration::Module(declaration) => {
                // body
                let expressions = declaration
                    .expressions
                    .iter()
                    .map(|expression| {
                        self.unbind_expression(
                            module,
                            *expression,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::Declaration::Module(ast::ModuleDeclaration { expressions })
            }
            dir::Declaration::Namespace(declaration) => {
                // declaration header
                let name = self.unbind_name(ast_strings, declaration.name);
                let export = declaration
                    .export
                    .map(|export| self.unbind_export_kind(export));
                let is_ambient = self.unbind_ambientness(declaration.is_ambient, context);
                let form = self.unbind_namespace_form(declaration.form);

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.unbind_generic_parameter(
                            module,
                            *parameter,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.unbind_where_clause(
                            module,
                            *where_clause,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                // body
                let expressions = declaration
                    .expressions
                    .iter()
                    .map(|expression| {
                        self.unbind_expression(
                            module,
                            *expression,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::Declaration::Namespace(ast::NamespaceDeclaration {
                    name,
                    export,
                    is_ambient,
                    form,
                    generic_parameters,
                    where_clauses,
                    expressions,
                })
            }
            dir::Declaration::Type(declaration) => {
                // declaration header
                let name = self.unbind_name(ast_strings, declaration.name);
                let export = declaration
                    .export
                    .map(|export| self.unbind_export_kind(export));
                let is_ambient = self.unbind_ambientness(declaration.is_ambient, context);
                let mutability = declaration
                    .mutability
                    .map(|mutability| self.unbind_mutability(context, mutability));

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.unbind_generic_parameter(
                            module,
                            *parameter,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.unbind_where_clause(
                            module,
                            *where_clause,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                // value
                let value = self.unbind_type_expression(
                    module,
                    declaration.value,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );

                ast::Declaration::Type(ast::TypeDeclaration {
                    name,
                    export,
                    is_ambient,
                    is_nominal: declaration.is_nominal,
                    mutability,
                    generic_parameters,
                    where_clauses,
                    value,
                })
            }
            dir::Declaration::Struct(declaration) => {
                // declaration header
                let name = self.unbind_name(ast_strings, declaration.name);
                let export = declaration
                    .export
                    .map(|export| self.unbind_export_kind(export));
                let is_ambient = self.unbind_ambientness(declaration.is_ambient, context);

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.unbind_generic_parameter(
                            module,
                            *parameter,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.unbind_where_clause(
                            module,
                            *where_clause,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                // relations and body
                let implements_types = declaration
                    .implements_types
                    .iter()
                    .map(|ty| {
                        self.unbind_type_expression(
                            module,
                            *ty,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let members = declaration
                    .members
                    .iter()
                    .map(|member| {
                        self.unbind_member(
                            module,
                            *member,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::Declaration::Struct(ast::StructDeclaration {
                    name,
                    export,
                    is_ambient,
                    generic_parameters,
                    where_clauses,
                    implements_types,
                    members,
                })
            }
            dir::Declaration::Class(declaration) => {
                // declaration header
                let name = declaration
                    .name
                    .map(|name| self.unbind_name(ast_strings, name));
                let export = declaration
                    .export
                    .map(|export| self.unbind_export_kind(export));
                let is_ambient = self.unbind_ambientness(declaration.is_ambient, context);

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.unbind_generic_parameter(
                            module,
                            *parameter,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.unbind_where_clause(
                            module,
                            *where_clause,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                // relations and body
                let extends_expression = declaration.extends_expression.map(|expression| {
                    self.unbind_expression(
                        module,
                        expression,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                let extends_generic_arguments = declaration
                    .extends_generic_arguments
                    .iter()
                    .map(|argument| {
                        self.unbind_generic_argument(
                            module,
                            *argument,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let implements_types = declaration
                    .implements_types
                    .iter()
                    .map(|ty| {
                        self.unbind_type_expression(
                            module,
                            *ty,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let members = declaration
                    .members
                    .iter()
                    .map(|member| {
                        self.unbind_member(
                            module,
                            *member,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::Declaration::Class(ast::ClassDeclaration {
                    name,
                    export,
                    is_ambient,
                    is_abstract: declaration.is_abstract,
                    is_final: declaration.is_final,
                    generic_parameters,
                    where_clauses,
                    extends_expression,
                    extends_generic_arguments,
                    implements_types,
                    members,
                })
            }
            dir::Declaration::Enum(declaration) => {
                // declaration header
                let name = declaration
                    .name
                    .map(|name| self.unbind_name(ast_strings, name));
                let export = declaration
                    .export
                    .map(|export| self.unbind_export_kind(export));
                let is_ambient = self.unbind_ambientness(declaration.is_ambient, context);
                let kind = self.unbind_enum_kind(declaration.kind);

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.unbind_generic_parameter(
                            module,
                            *parameter,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.unbind_where_clause(
                            module,
                            *where_clause,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                // relations and body
                let implements_types = declaration
                    .implements_types
                    .iter()
                    .map(|ty| {
                        self.unbind_type_expression(
                            module,
                            *ty,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let fields = declaration
                    .fields
                    .iter()
                    .map(|field| {
                        self.unbind_enum_field(
                            module,
                            *field,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let members = declaration
                    .members
                    .iter()
                    .map(|member| {
                        self.unbind_member(
                            module,
                            *member,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::Declaration::Enum(ast::EnumDeclaration {
                    name,
                    export,
                    is_ambient,
                    kind,
                    generic_parameters,
                    where_clauses,
                    implements_types,
                    fields,
                    members,
                })
            }
            dir::Declaration::Interface(declaration) => {
                // declaration header
                let name = declaration
                    .name
                    .map(|name| self.unbind_name(ast_strings, name));
                let export = declaration
                    .export
                    .map(|export| self.unbind_export_kind(export));
                let is_ambient = self.unbind_ambientness(declaration.is_ambient, context);

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.unbind_generic_parameter(
                            module,
                            *parameter,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.unbind_where_clause(
                            module,
                            *where_clause,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                // relations and body
                let extends = declaration
                    .extends
                    .iter()
                    .map(|heritage| {
                        let expression = self.unbind_expression(
                            module,
                            heritage.expression,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        );
                        let generic_arguments = heritage
                            .generic_arguments
                            .iter()
                            .map(|argument| {
                                self.unbind_generic_argument(
                                    module,
                                    *argument,
                                    tree,
                                    symbols,
                                    types,
                                    ast_tree,
                                    ast_strings,
                                    context,
                                )
                            })
                            .collect();

                        ast::InterfaceHeritage {
                            expression,
                            generic_arguments,
                        }
                    })
                    .collect();
                let members = declaration
                    .members
                    .iter()
                    .map(|member| {
                        self.unbind_type_member(
                            module,
                            *member,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::Declaration::Interface(ast::InterfaceDeclaration {
                    name,
                    export,
                    is_ambient,
                    is_nominal: declaration.is_nominal,
                    generic_parameters,
                    where_clauses,
                    extends,
                    members,
                })
            }
            dir::Declaration::Extension(declaration) => {
                // declaration header
                let name = declaration
                    .name
                    .map(|name| self.unbind_name(ast_strings, name));
                let export = declaration
                    .export
                    .map(|export| self.unbind_export_kind(export));
                let is_ambient = self.unbind_ambientness(declaration.is_ambient, context);

                // polymorphism
                let generic_parameters = declaration
                    .generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.unbind_generic_parameter(
                            module,
                            *parameter,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let where_clauses = declaration
                    .where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.unbind_where_clause(
                            module,
                            *where_clause,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                // relations and body
                let target_type = self.unbind_type_expression(
                    module,
                    declaration.target_type,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let implements_types = declaration
                    .implements_types
                    .iter()
                    .map(|ty| {
                        self.unbind_type_expression(
                            module,
                            *ty,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let members = declaration
                    .members
                    .iter()
                    .map(|member| {
                        self.unbind_member(
                            module,
                            *member,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::Declaration::Extension(ast::ExtensionDeclaration {
                    name,
                    export,
                    is_ambient,
                    generic_parameters,
                    where_clauses,
                    target_type,
                    implements_types,
                    members,
                })
            }
            dir::Declaration::Function(declaration) => {
                // declaration header
                let name = declaration
                    .name
                    .map(|name| self.unbind_name(ast_strings, name));
                let export = declaration
                    .export
                    .map(|export| self.unbind_export_kind(export));
                let is_ambient = self.unbind_ambientness(declaration.is_ambient, context);

                // signature and body
                let signature = self.unbind_function_signature(
                    module,
                    &declaration.signature,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let body = declaration.body.map(|body| {
                    self.unbind_expression(
                        module,
                        body,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });

                ast::Declaration::Function(ast::FunctionDeclaration {
                    name,
                    export,
                    is_ambient,
                    signature,
                    body,
                })
            }
        };

        let ast_declaration_id = ast_tree.insert(ast_declaration, span);
        context.map(declaration_id.into_any(), ast_declaration_id.into_any());

        ast_declaration_id
    }

    /// Unbind a DIR enum field to an AST enum field.
    pub(super) fn unbind_enum_field(
        &self,
        module: &Module,
        field_id: dir::LocalNodeId<dir::EnumField>,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::EnumField> {
        let field = tree.get(field_id);
        let span = self.unbind_span(module, field_id.into());

        // field payload
        let name = self.unbind_name(ast_strings, field.name);
        let value = field.value.map(|value| {
            self.unbind_expression(
                module,
                value,
                tree,
                symbols,
                types,
                ast_tree,
                ast_strings,
                context,
            )
        });

        let ast_field = ast::EnumField { name, value };
        let ast_field_id = ast_tree.insert(ast_field, span);
        context.map(field_id.into_any(), ast_field_id.into_any());

        ast_field_id
    }
}

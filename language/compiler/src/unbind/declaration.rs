use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR declaration kind to an AST declaration kind.
    #[inline]
    pub(super) fn unbind_declaration_kind(
        &self,
        _context: &mut UnbindContext,
        kind: dir::DeclarationKind,
    ) -> ast::DeclarationKind {
        match kind {
            dir::DeclarationKind::Declaration => ast::DeclarationKind::Declaration,
            dir::DeclarationKind::Definition => ast::DeclarationKind::Definition,
        }
    }

    /// Unbind a DIR binding anchor to an AST binding anchor.
    #[inline]
    pub(super) fn unbind_binding_anchor(
        &self,
        _context: &mut UnbindContext,
        anchor: dir::BindingAnchor,
    ) -> ast::BindingAnchor {
        match anchor {
            dir::BindingAnchor::Static => ast::BindingAnchor::Static,
            dir::BindingAnchor::Instance => ast::BindingAnchor::Instance,
        }
    }

    /// Unbind a DIR declaration abstraction to an AST declaration abstraction.
    #[inline]
    pub(super) fn unbind_declaration_abstraction(
        &self,
        _context: &mut UnbindContext,
        abstraction: dir::DeclarationAbstraction,
    ) -> ast::DeclarationAbstraction {
        match abstraction {
            dir::DeclarationAbstraction::Abstract => ast::DeclarationAbstraction::Abstract,
            dir::DeclarationAbstraction::Concrete => ast::DeclarationAbstraction::Concrete,
        }
    }

    /// Unbind a DIR enum kind to an AST enum kind.
    #[inline]
    pub(super) fn unbind_enum_kind(
        &self,
        _context: &mut UnbindContext,
        kind: dir::EnumKind,
    ) -> ast::EnumKind {
        match kind {
            dir::EnumKind::Enum => ast::EnumKind::Enum,
            dir::EnumKind::Const => ast::EnumKind::Const,
        }
    }

    /// Unbind a DIR declaration descriptor to an AST declaration descriptor.
    pub(super) fn unbind_declaration_descriptor(
        &self,
        descriptor: &dir::DeclarationDescriptor,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::DeclarationDescriptor {
        let kind = self.unbind_declaration_kind(context, descriptor.kind);
        let abstraction = self.unbind_declaration_abstraction(context, descriptor.abstraction);
        let anchor = self.unbind_binding_anchor(context, descriptor.anchor);
        let name = descriptor.name.map(|name| {
            ast::Name::Identifier(ast_strings.intern_from(&self.program.strings, name))
        });
        let export = descriptor
            .export
            .map(|export| self.unbind_dependency_mode(context, export));
        ast::DeclarationDescriptor {
            kind,
            abstraction,
            anchor,
            name,
            export,
        }
    }

    /// Unbind a DIR declarator to an AST declarator.
    pub(super) fn unbind_declarator(
        &self,
        module: &Module,
        declarator_id: dir::LocalNodeId<dir::Declarator>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Declarator> {
        let declarator = tree.get(declarator_id);
        let span = self.unbind_span(module, declarator_id.into());
        let pattern = self.unbind_pattern(
            module,
            declarator.pattern,
            tree,
            symbols,
            ast_tree,
            ast_strings,
            context,
        );
        let value = declarator.value.map(|value| {
            self.unbind_expression(module, value, tree, symbols, ast_tree, ast_strings, context)
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
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Declaration> {
        let declaration = tree.get(declaration_id);
        let span = self.unbind_span(module, declaration_id.into());
        let ast_declaration = match declaration {
            dir::Declaration::Namespace {
                descriptor,
                generics,
                expressions,
                ..
            } => {
                let descriptor =
                    self.unbind_declaration_descriptor(descriptor, ast_strings, context);
                let generics = self.unbind_generics(
                    module,
                    generics,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let expressions = expressions
                    .iter()
                    .map(|expression| {
                        self.unbind_expression(
                            module,
                            *expression,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::Declaration::Namespace {
                    descriptor,
                    generics,
                    expressions,
                }
            }
            dir::Declaration::Type {
                descriptor,
                kind,
                mutability,
                static_parameters,
                value,
            } => {
                let descriptor =
                    self.unbind_declaration_descriptor(descriptor, ast_strings, context);
                let kind = self.unbind_type_kind(context, *kind);
                let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| {
                            self.unbind_parameter(
                                module,
                                *param,
                                tree,
                                symbols,
                                ast_tree,
                                ast_strings,
                                context,
                            )
                        })
                        .collect()
                });
                let value = self.unbind_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Declaration::Type {
                    descriptor,
                    kind,
                    mutability,
                    static_parameters,
                    value,
                }
            }
            dir::Declaration::Struct {
                descriptor,
                generics,
                heritage,
                members,
                ..
            } => {
                let descriptor =
                    self.unbind_declaration_descriptor(descriptor, ast_strings, context);
                let generics = self.unbind_generics(
                    module,
                    generics,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let heritage = self.unbind_heritage(
                    module,
                    heritage,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let members = members
                    .iter()
                    .map(|member| {
                        self.unbind_member(
                            module,
                            *member,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::Declaration::Struct {
                    descriptor,
                    generics,
                    heritage,
                    members,
                }
            }
            dir::Declaration::Class {
                descriptor,
                generics,
                heritage,
                members,
                ..
            } => {
                let descriptor =
                    self.unbind_declaration_descriptor(descriptor, ast_strings, context);
                let generics = self.unbind_generics(
                    module,
                    generics,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let heritage = self.unbind_heritage(
                    module,
                    heritage,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let members = members
                    .iter()
                    .map(|member| {
                        self.unbind_member(
                            module,
                            *member,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::Declaration::Class {
                    descriptor,
                    generics,
                    heritage,
                    members,
                }
            }
            dir::Declaration::Enum {
                descriptor,
                kind,
                generics,
                heritage,
                fields,
                members,
                ..
            } => {
                let descriptor =
                    self.unbind_declaration_descriptor(descriptor, ast_strings, context);
                let kind = self.unbind_enum_kind(context, *kind);
                let generics = self.unbind_generics(
                    module,
                    generics,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let heritage = self.unbind_heritage(
                    module,
                    heritage,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.unbind_enum_field(
                            module,
                            *field,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let members = members
                    .iter()
                    .map(|member| {
                        self.unbind_member(
                            module,
                            *member,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::Declaration::Enum {
                    descriptor,
                    kind,
                    generics,
                    heritage,
                    fields,
                    members,
                }
            }
            dir::Declaration::Interface {
                descriptor,
                kind,
                generics,
                heritage,
                members,
                ..
            } => {
                let descriptor =
                    self.unbind_declaration_descriptor(descriptor, ast_strings, context);
                let kind = self.unbind_type_kind(context, *kind);
                let generics = self.unbind_generics(
                    module,
                    generics,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let heritage = self.unbind_heritage(
                    module,
                    heritage,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let members = members
                    .iter()
                    .map(|member| {
                        self.unbind_member(
                            module,
                            *member,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::Declaration::Interface {
                    descriptor,
                    kind,
                    generics,
                    heritage,
                    members,
                }
            }
            dir::Declaration::Extension {
                descriptor,
                generics,
                target_type,
                heritage,
                members,
                ..
            } => {
                let descriptor =
                    self.unbind_declaration_descriptor(descriptor, ast_strings, context);
                let generics = self.unbind_generics(
                    module,
                    generics,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let target_type = self.unbind_expression(
                    module,
                    *target_type,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let heritage = self.unbind_heritage(
                    module,
                    heritage,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let members = members
                    .iter()
                    .map(|member| {
                        self.unbind_member(
                            module,
                            *member,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::Declaration::Extension {
                    descriptor,
                    generics,
                    target_type,
                    heritage,
                    members,
                }
            }
            dir::Declaration::Function {
                descriptor,
                signature,
                body,
                ..
            } => {
                let descriptor =
                    self.unbind_declaration_descriptor(descriptor, ast_strings, context);
                let signature = self.unbind_function_signature(
                    module,
                    signature,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let body = body.map(|body| {
                    self.unbind_expression(
                        module,
                        body,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                ast::Declaration::Function {
                    descriptor,
                    signature,
                    body,
                }
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
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::EnumField> {
        let field = tree.get(field_id);
        let span = self.unbind_span(module, field_id.into());
        let name =
            ast::Name::Identifier(ast_strings.intern_from(&self.program.strings, field.name));
        let value = field.value.map(|value| {
            self.unbind_expression(module, value, tree, symbols, ast_tree, ast_strings, context)
        });
        let ast_field = ast::EnumField { name, value };
        let ast_field_id = ast_tree.insert(ast_field, span);
        context.map(field_id.into_any(), ast_field_id.into_any());
        ast_field_id
    }
}

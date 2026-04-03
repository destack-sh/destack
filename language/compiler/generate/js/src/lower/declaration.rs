use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, CodegenJsResultExt, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Lower visibility from DIR into JS AST.
    pub fn lower_visibility(&self, visibility: dir::Visibility) -> js::Visibility {
        match visibility {
            dir::Visibility::Public => js::Visibility::Public,
            dir::Visibility::Protected => js::Visibility::Protected,
            dir::Visibility::Private => js::Visibility::Private,
        }
    }

    /// Lower a declaration kind from DIR into JS AST.
    pub fn lower_declaration_kind(
        &self,
        declaration_kind: dir::DeclarationKind,
    ) -> js::DeclarationKind {
        match declaration_kind {
            dir::DeclarationKind::Declaration => js::DeclarationKind::Declaration,
            dir::DeclarationKind::Definition => js::DeclarationKind::Definition,
        }
    }

    /// Lower a declaration abstraction from DIR into JS AST.
    pub fn lower_declaration_abstraction(
        &self,
        abstraction: dir::DeclarationAbstraction,
    ) -> js::DeclarationAbstraction {
        match abstraction {
            dir::DeclarationAbstraction::Abstract => js::DeclarationAbstraction::Abstract,
            dir::DeclarationAbstraction::Concrete => js::DeclarationAbstraction::Concrete,
        }
    }

    /// Lower an export type from DIR into JS AST.
    pub fn lower_export_type(&self, export_type: dir::DependencyMode) -> js::DependencyMode {
        match export_type {
            dir::DependencyMode::Item => js::DependencyMode::Item,
            dir::DependencyMode::Default => js::DependencyMode::Default,
            dir::DependencyMode::Namespace => js::DependencyMode::Namespace,
        }
    }

    /// Lower a binding anchor from DIR into JS AST.
    pub fn lower_binding_anchor(&self, anchor: dir::BindingAnchor) -> js::BindingAnchor {
        match anchor {
            dir::BindingAnchor::Static => js::BindingAnchor::Static,
            dir::BindingAnchor::Instance => js::BindingAnchor::Instance,
        }
    }

    /// Lower a declaration descriptor from DIR into JS AST.
    pub fn lower_declaration_descriptor(
        &mut self,
        descriptor: &dir::DeclarationDescriptor,
    ) -> js::DeclarationDescriptor {
        let kind = self.lower_declaration_kind(descriptor.kind);
        let abstraction = self.lower_declaration_abstraction(descriptor.abstraction);
        let anchor = self.lower_binding_anchor(descriptor.anchor);
        let name = descriptor.name.map(|name| self.lower_name(name));
        let export = descriptor
            .export
            .map(|export| self.lower_export_type(export));
        js::DeclarationDescriptor {
            kind,
            abstraction,
            anchor,
            name,
            export,
        }
    }

    /// Lower a declaration from DIR into JS AST.
    pub fn lower_declaration(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Declaration>> {
        let declaration = self.dir_tree.get(declaration_id);
        let declaration_symbol = match declaration {
            dir::Declaration::Global { descriptor, .. }
            | dir::Declaration::Namespace { descriptor, .. }
            | dir::Declaration::Type { descriptor, .. }
            | dir::Declaration::Struct { descriptor, .. }
            | dir::Declaration::Class { descriptor, .. }
            | dir::Declaration::Interface { descriptor, .. }
            | dir::Declaration::Enum { descriptor, .. }
            | dir::Declaration::Function { descriptor, .. } => Some(descriptor.symbol),
            _ => None,
        };
        let declaration = match declaration {
            dir::Declaration::Global {
                descriptor,
                scope: _,
                expressions,
            } => {
                let descriptor = self.lower_declaration_descriptor(descriptor);
                let statements = expressions
                    .iter()
                    .map(|expression| {
                        self.lower_expression(*expression)
                            .expect_node::<js::Statement>(
                                expression.into_global_any(self.module.id),
                                self,
                            )
                    })
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                js::Declaration::Global {
                    descriptor,
                    statements,
                }
            }
            dir::Declaration::Namespace {
                descriptor,
                kind: _,
                scope: _,
                generics: _,
                expressions,
            } => {
                let descriptor = self.lower_declaration_descriptor(descriptor);
                let statements = expressions
                    .iter()
                    .map(|expression| {
                        self.lower_expression(*expression)
                            .expect_node::<js::Statement>(
                                expression.into_global_any(self.module.id),
                                self,
                            )
                    })
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                js::Declaration::Namespace {
                    descriptor,
                    statements,
                }
            }
            dir::Declaration::Type {
                descriptor,
                kind: _,
                mutability: _,
                static_parameters,
                value,
            } => {
                let descriptor = self.lower_declaration_descriptor(descriptor);
                let static_parameters = static_parameters
                    .as_ref()
                    .map(|params| {
                        params
                            .iter()
                            .map(|param| self.lower_parameter(*param))
                            .collect::<Result<Vec<_>, CodegenJsError>>()
                    })
                    .transpose()?;
                let declared_type_id = self
                    .types
                    .get_declared_type_id(declaration_id.into_global_any(self.module.id));
                let value = declared_type_id
                    .map(|type_id| self.lower_type(type_id))
                    .transpose()?
                    .unwrap_or(self.lower_type_annotation_expression(*value)?);
                js::Declaration::Type {
                    descriptor,
                    static_parameters,
                    value,
                }
            }
            dir::Declaration::Struct {
                descriptor,
                scope: _,
                generics,
                heritage,
                members,
            } => {
                let descriptor = self.lower_declaration_descriptor(descriptor);
                let generics = self.lower_generics(generics)?;
                let heritage = self.lower_heritage(heritage)?;
                let members = members
                    .iter()
                    .map(|member| self.lower_member(*member))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                // NOTE #Incomplete: struct declarations should become just JS types + namespaces?
                js::Declaration::Class {
                    descriptor,
                    generics,
                    heritage,
                    members,
                }
            }
            dir::Declaration::Class {
                descriptor,
                self_symbol: _,
                scope: _,
                generics,
                heritage,
                members,
            } => {
                let descriptor = self.lower_declaration_descriptor(descriptor);
                let generics = self.lower_generics(generics)?;
                let heritage = self.lower_heritage(heritage)?;
                let members = members
                    .iter()
                    .map(|member| self.lower_member(*member))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                js::Declaration::Class {
                    descriptor,
                    generics,
                    heritage,
                    members,
                }
            }
            dir::Declaration::Interface {
                descriptor,
                kind: _,
                scope: _,
                generics,
                heritage,
                members,
            } => {
                let descriptor = self.lower_declaration_descriptor(descriptor);
                let generics = self.lower_generics(generics)?;
                let heritage = self.lower_heritage(heritage)?;
                let members = members
                    .iter()
                    .map(|member| self.lower_member(*member))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                js::Declaration::Interface {
                    descriptor,
                    generics,
                    heritage,
                    members,
                }
            }
            dir::Declaration::Enum {
                descriptor,
                kind: _,
                scope: _,
                generics: _,
                heritage: _,
                fields,
                members: _,
            } => {
                let descriptor = self.lower_declaration_descriptor(descriptor);
                let fields = fields
                    .iter()
                    .map(|field| self.lower_enum_field(*field))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                js::Declaration::Enum { descriptor, fields }
            }
            dir::Declaration::Function {
                descriptor,
                self_symbol: _,
                scope: _,
                signature,
                body,
            } => {
                let descriptor = self.lower_declaration_descriptor(descriptor);
                let signature = self.lower_function_signature(signature)?;
                let body = body
                    .map(|body| self.lower_expression_as_block(body))
                    .transpose()?;
                js::Declaration::Function {
                    descriptor,
                    signature,
                    body,
                }
            }
            _ => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: declaration_id.into_global_any(self.module.id),
                    message: None,
                });
            }
        };
        let declaration_id =
            self.tree
                .insert_from_source(declaration, self.module.id, declaration_id);

        if let Some(declaration_symbol) = declaration_symbol {
            self.set_source_node_symbol(declaration_id, declaration_symbol);
        }

        Ok(declaration_id)
    }

    /// Lower an enum field from DIR into JS AST.
    pub fn lower_enum_field(
        &mut self,
        field_id: dir::LocalNodeId<dir::EnumField>,
    ) -> CodegenJsResult<js::LocalNodeId<js::EnumField>> {
        let field = self.dir_tree.get(field_id);
        let name = self.strings.intern_from(self.source_strings, field.name);
        let value = field
            .value
            .as_ref()
            .map(|value_id| {
                self.lower_expression(*value_id)
                    .expect_node::<js::Expression>(value_id.into_global_any(self.module.id), self)
            })
            .transpose()?;
        let field = js::EnumField { name, value };
        let field_id = self
            .tree
            .insert_from_source(field, self.module.id, field_id);
        Ok(field_id)
    }
}

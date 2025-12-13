use crate::{
    BindingAnchor, Block, CodegenJsError, CodegenJsResult, CodegenJsResultExt, Declaration,
    DeclarationDescriptor, DeclarationKind, DependencyMode, EnumField, Expression, LocalNodeId,
    ModuleLowerer, Statement, Type, Visibility,
};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower visibility from DIR into JS AST.
    pub fn lower_visibility(&self, visibility: dir::Visibility) -> Visibility {
        match visibility {
            dir::Visibility::Public => Visibility::Public,
            dir::Visibility::Protected => Visibility::Protected,
            dir::Visibility::Private => Visibility::Private,
        }
    }

    /// Lower a declaration kind from DIR into JS AST.
    pub fn lower_declaration_kind(
        &self,
        declaration_kind: dir::DeclarationKind,
    ) -> DeclarationKind {
        match declaration_kind {
            dir::DeclarationKind::Declaration => DeclarationKind::Declaration,
            dir::DeclarationKind::Definition => DeclarationKind::Definition,
        }
    }

    /// Lower an export type from DIR into JS AST.
    pub fn lower_export_type(&self, export_type: dir::DependencyMode) -> DependencyMode {
        match export_type {
            dir::DependencyMode::Item => DependencyMode::Item,
            dir::DependencyMode::Default => DependencyMode::Default,
            dir::DependencyMode::Namespace => DependencyMode::Namespace,
        }
    }

    /// Lower a binding anchor from DIR into JS AST.
    pub fn lower_binding_anchor(&self, anchor: dir::BindingAnchor) -> BindingAnchor {
        match anchor {
            dir::BindingAnchor::Static => BindingAnchor::Static,
            dir::BindingAnchor::Instance => BindingAnchor::Instance,
        }
    }

    /// Lower a declaration descriptor from DIR into JS AST.
    pub fn lower_declaration_descriptor(
        &mut self,
        descriptor: &dir::DeclarationDescriptor,
    ) -> DeclarationDescriptor {
        let kind = self.lower_declaration_kind(descriptor.kind);
        let anchor = self.lower_binding_anchor(descriptor.anchor);
        let name = descriptor.name.map(|name| self.lower_string_to_name(name));
        let export = descriptor
            .export
            .map(|export| self.lower_export_type(export));
        DeclarationDescriptor {
            kind,
            anchor,
            name,
            export,
        }
    }

    /// Lower a declaration from DIR into JS AST.
    pub fn lower_declaration(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
    ) -> CodegenJsResult<LocalNodeId<Declaration>> {
        let declaration = self.dir_tree.get(declaration_id);
        let declaration = match declaration {
            dir::Declaration::Namespace {
                descriptor,
                scope: _,
                generics: _,
                expressions,
            } => {
                let descriptor = self.lower_declaration_descriptor(descriptor);
                let statements = expressions
                    .iter()
                    .map(|expression| {
                        self.lower_expression(*expression).expect_node::<Statement>(
                            expression.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                Declaration::Namespace {
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
                let value_expression = self
                    .lower_expression(*value)
                    .expect_node::<Expression>(value.into_global_any(self.module.id), self)?;
                let value = self.tree.insert_from_source_any(
                    Type::Expression(value_expression),
                    self.module.id,
                    value.into_any(),
                );
                Declaration::Type {
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
                Declaration::Class {
                    descriptor,
                    generics,
                    heritage,
                    members,
                }
            }
            dir::Declaration::Class {
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
                Declaration::Class {
                    descriptor,
                    generics,
                    heritage,
                    members,
                }
            }
            dir::Declaration::Interface {
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
                Declaration::Interface {
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
                Declaration::Enum { descriptor, fields }
            }
            dir::Declaration::Function {
                descriptor,
                scope: _,
                signature,
                body,
            } => {
                let descriptor = self.lower_declaration_descriptor(descriptor);
                let signature = self.lower_function_signature(signature)?;
                let body = body
                    .map(|body| {
                        self.lower_expression(body)
                            .expect_node::<Block>(body.into_global_any(self.module.id), self)
                    })
                    .transpose()?;
                Declaration::Function {
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
        Ok(declaration_id)
    }

    /// Lower an enum field from DIR into JS AST.
    pub fn lower_enum_field(
        &mut self,
        field_id: dir::LocalNodeId<dir::EnumField>,
    ) -> CodegenJsResult<LocalNodeId<EnumField>> {
        let field = self.dir_tree.get(field_id);
        let name = self
            .strings
            .intern_from(&self.module.ast.strings, field.name);
        let value = field
            .value
            .as_ref()
            .map(|value_id| {
                self.lower_expression(*value_id)
                    .expect_node::<Expression>(value_id.into_global_any(self.module.id), self)
            })
            .transpose()?;
        let field = EnumField { name, value };
        let field_id = self
            .tree
            .insert_from_source(field, self.module.id, field_id);
        Ok(field_id)
    }
}

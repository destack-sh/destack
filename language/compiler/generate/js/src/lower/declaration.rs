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

    /// Lower an export type from DIR into JS AST.
    pub fn lower_export_type(&self, export_type: dir::ExportMode) -> js::DependencyMode {
        match export_type {
            dir::ExportMode::Named => js::DependencyMode::Item,
            dir::ExportMode::Default => js::DependencyMode::Default,
        }
    }

    /// Lower one ambientness flag into a declaration kind.
    fn lower_declaration_kind(&self, ambient: dir::Ambientness) -> js::DeclarationKind {
        if ambient.is_ambient() {
            js::DeclarationKind::Declaration
        } else {
            js::DeclarationKind::Definition
        }
    }

    /// Lower one declaration descriptor from flattened DIR fields.
    pub(crate) fn lower_declaration_descriptor(
        &mut self,
        name: Option<dir::Name>,
        export: Option<dir::ExportMode>,
        ambient: dir::Ambientness,
        is_abstract: bool,
    ) -> js::DeclarationDescriptor {
        js::DeclarationDescriptor {
            kind: self.lower_declaration_kind(ambient),
            abstraction: if is_abstract {
                js::DeclarationAbstraction::Abstract
            } else {
                js::DeclarationAbstraction::Concrete
            },
            anchor: js::BindingAnchor::Instance,
            name: name.map(|name| self.lower_name(name)),
            export: export.map(|export| self.lower_export_type(export)),
        }
    }

    /// Lower a declaration from DIR into JS AST.
    pub fn lower_declaration(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Declaration>> {
        let declaration = self.dir_tree.get(declaration_id);
        let declaration_symbol = Some(declaration.symbol());
        let declaration = match declaration {
            dir::Declaration::Global(declaration) => {
                let descriptor =
                    self.lower_declaration_descriptor(None, None, declaration.ambient, false);
                let statements = declaration
                    .expressions
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
            dir::Declaration::Namespace(declaration) => {
                let descriptor = self.lower_declaration_descriptor(
                    Some(declaration.name),
                    declaration.export,
                    declaration.ambient,
                    false,
                );
                let statements = declaration
                    .expressions
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
            dir::Declaration::Type(declaration) => {
                let descriptor = self.lower_declaration_descriptor(
                    Some(declaration.name),
                    declaration.export,
                    declaration.ambient,
                    false,
                );
                let static_parameters =
                    self.lower_generic_parameters(&declaration.generic_parameters)?;
                let declared_type_id = self
                    .types
                    .get_declared_type_id(declaration_id.into_global_any(self.module.id));
                let value = declared_type_id
                    .map(|type_id| self.lower_type(type_id))
                    .transpose()?
                    .unwrap_or(self.lower_type_annotation_expression(declaration.value)?);
                js::Declaration::Type {
                    descriptor,
                    static_parameters,
                    value,
                }
            }
            dir::Declaration::Struct(declaration) => {
                let descriptor = self.lower_declaration_descriptor(
                    Some(declaration.name),
                    declaration.export,
                    declaration.ambient,
                    false,
                );
                let generics = self.lower_generic_parameters(&declaration.generic_parameters)?;
                let heritage =
                    self.lower_heritage_slice(None, None, Some(&declaration.implements_types))?;
                let members = declaration
                    .members
                    .iter()
                    .map(|member| self.lower_member(*member))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                // NOTE #Incomplete: struct declarations should become just JS types + namespaces?
                js::Declaration::Class {
                    descriptor,
                    generics: js::Generics {
                        static_parameters: generics,
                    },
                    heritage,
                    members,
                }
            }
            dir::Declaration::Class(declaration) => {
                let descriptor = self.lower_declaration_descriptor(
                    declaration.name,
                    declaration.export,
                    declaration.ambient,
                    declaration.is_abstract,
                );
                let generics = self.lower_generic_parameters(&declaration.generic_parameters)?;
                let heritage = self.lower_heritage(
                    declaration.extends_expression,
                    Some(&declaration.implements_types),
                )?;
                let members = declaration
                    .members
                    .iter()
                    .map(|member| self.lower_member(*member))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                js::Declaration::Class {
                    descriptor,
                    generics: js::Generics {
                        static_parameters: generics,
                    },
                    heritage,
                    members,
                }
            }
            dir::Declaration::Interface(declaration) => {
                let descriptor = self.lower_declaration_descriptor(
                    declaration.name,
                    declaration.export,
                    declaration.ambient,
                    false,
                );
                let generics = self.lower_generic_parameters(&declaration.generic_parameters)?;
                let heritage =
                    self.lower_heritage_slice(None, Some(&declaration.extends_types), None)?;
                let members = declaration
                    .members
                    .iter()
                    .map(|member| self.lower_type_member(*member))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                js::Declaration::Interface {
                    descriptor,
                    generics: js::Generics {
                        static_parameters: generics,
                    },
                    heritage,
                    members,
                }
            }
            dir::Declaration::Enum(declaration) => {
                let descriptor = self.lower_declaration_descriptor(
                    declaration.name,
                    declaration.export,
                    declaration.ambient,
                    false,
                );
                let fields = declaration
                    .fields
                    .iter()
                    .map(|field| self.lower_enum_field(*field))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                js::Declaration::Enum { descriptor, fields }
            }
            dir::Declaration::Function(declaration) => {
                let descriptor = self.lower_declaration_descriptor(
                    declaration.name,
                    declaration.export,
                    declaration.ambient,
                    declaration.signature.is_abstract,
                );
                let signature = self.lower_function_signature(&declaration.signature)?;
                let body = declaration
                    .body
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
        let name = self
            .strings
            .intern_from(self.source_strings, field.name.string());
        let value = field
            .value
            .map(|value_id| {
                self.lower_expression(value_id)
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

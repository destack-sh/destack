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

    /// Lower an export kind from DIR into JS AST.
    pub fn lower_export_kind(&self, export: dir::ExportKind) -> js::DependencyBinding {
        match export {
            dir::ExportKind::Named => js::DependencyBinding::Item,
            dir::ExportKind::Default => js::DependencyBinding::Default,
        }
    }

    /// Lower one interface heritage item from DIR into JS AST.
    pub(crate) fn lower_interface_heritage(
        &mut self,
        heritage: &dir::InterfaceHeritage,
    ) -> CodegenJsResult<js::InterfaceHeritage> {
        let expression = self
            .lower_expression(heritage.expression)
            .expect_node::<js::Expression>(
                heritage.expression.into_global_any(self.module.id),
                self,
            )?;
        let type_arguments = self.lower_static_type_arguments(&heritage.generic_arguments)?;

        Ok(js::InterfaceHeritage {
            expression,
            type_arguments,
        })
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
                // body
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

                let declaration = js::GlobalDeclaration {
                    is_ambient: declaration.is_ambient,
                    statements,
                };

                js::Declaration::Global(declaration)
            }
            dir::Declaration::Namespace(declaration) => {
                // body
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

                let declaration = js::NamespaceDeclaration {
                    name: Some(self.lower_name(declaration.name)),
                    export: declaration
                        .export
                        .map(|export| self.lower_export_kind(export)),
                    is_ambient: declaration.is_ambient,
                    statements,
                };

                js::Declaration::Namespace(declaration)
            }
            dir::Declaration::Type(declaration) => {
                // generic parameters
                let generic_parameters =
                    self.lower_generic_parameters(&declaration.generic_parameters)?;

                // value
                let Some(declared_type_id) = self
                    .types
                    .get_declared_type_id(declaration_id.into_global_any(self.module.id))
                else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: declaration_id.into_global_any(self.module.id),
                        message: Some(
                            "type declarations need semantic types before JS lowering".to_string(),
                        ),
                    });
                };
                let value = self.lower_type(declared_type_id)?;

                let declaration = js::TypeDeclaration {
                    name: Some(self.lower_name(declaration.name)),
                    export: declaration
                        .export
                        .map(|export| self.lower_export_kind(export)),
                    is_ambient: declaration.is_ambient,
                    generic_parameters,
                    value,
                };

                js::Declaration::Type(declaration)
            }
            dir::Declaration::Struct(declaration) => {
                // generic parameters
                let generic_parameters =
                    self.lower_generic_parameters(&declaration.generic_parameters)?;

                // implements
                let implements_types =
                    self.lower_type_annotation_expressions(&declaration.implements_types)?;

                // members
                let members = declaration
                    .members
                    .iter()
                    .map(|member| self.lower_member(*member))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;

                // struct declarations currently lower through class form

                let declaration = js::ClassDeclaration {
                    name: Some(self.lower_name(declaration.name)),
                    export: declaration
                        .export
                        .map(|export| self.lower_export_kind(export)),
                    is_ambient: declaration.is_ambient,
                    is_abstract: false,
                    generic_parameters,
                    extends_expression: None,
                    extends_generic_arguments: Vec::new(),
                    implements_types,
                    members,
                };

                js::Declaration::Class(declaration)
            }
            dir::Declaration::Class(declaration) => {
                // generic parameters
                let generic_parameters =
                    self.lower_generic_parameters(&declaration.generic_parameters)?;

                // extends
                let extends_expression = declaration
                    .extends_expression
                    .map(|expression_id| {
                        self.lower_expression(expression_id)
                            .expect_node::<js::Expression>(
                                expression_id.into_global_any(self.module.id),
                                self,
                            )
                    })
                    .transpose()?;
                let extends_generic_arguments =
                    self.lower_static_type_arguments(&declaration.extends_generic_arguments)?;

                // implements
                let implements_types =
                    self.lower_type_annotation_expressions(&declaration.implements_types)?;

                // members
                let members = declaration
                    .members
                    .iter()
                    .map(|member| self.lower_member(*member))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;

                let declaration = js::ClassDeclaration {
                    name: declaration.name.map(|name| self.lower_name(name)),
                    export: declaration
                        .export
                        .map(|export| self.lower_export_kind(export)),
                    is_ambient: declaration.is_ambient,
                    is_abstract: declaration.is_abstract,
                    generic_parameters,
                    extends_expression,
                    extends_generic_arguments,
                    implements_types,
                    members,
                };

                js::Declaration::Class(declaration)
            }
            dir::Declaration::Interface(declaration) => {
                // generic parameters
                let generic_parameters =
                    self.lower_generic_parameters(&declaration.generic_parameters)?;

                // extends
                let extends = declaration
                    .extends
                    .iter()
                    .map(|heritage| self.lower_interface_heritage(heritage))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;

                // members
                let members = declaration
                    .members
                    .iter()
                    .map(|member| self.lower_type_member(*member))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;

                let declaration = js::InterfaceDeclaration {
                    name: declaration.name.map(|name| self.lower_name(name)),
                    export: declaration
                        .export
                        .map(|export| self.lower_export_kind(export)),
                    is_ambient: declaration.is_ambient,
                    generic_parameters,
                    extends,
                    members,
                };

                js::Declaration::Interface(declaration)
            }
            dir::Declaration::Enum(declaration) => {
                // fields
                let fields = declaration
                    .fields
                    .iter()
                    .map(|field| self.lower_enum_field(*field))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;

                let declaration = js::EnumDeclaration {
                    name: declaration.name.map(|name| self.lower_name(name)),
                    export: declaration
                        .export
                        .map(|export| self.lower_export_kind(export)),
                    is_ambient: declaration.is_ambient,
                    fields,
                };

                js::Declaration::Enum(declaration)
            }
            dir::Declaration::Function(declaration) => {
                // signature
                let signature = self.lower_function_signature(&declaration.signature)?;

                // body
                let body = declaration
                    .body
                    .map(|body| self.lower_expression_as_block(body))
                    .transpose()?;

                let declaration = js::FunctionDeclaration {
                    name: declaration.name.map(|name| self.lower_name(name)),
                    export: declaration
                        .export
                        .map(|export| self.lower_export_kind(export)),
                    is_ambient: declaration.is_ambient,
                    is_abstract: declaration.signature.is_abstract,
                    signature,
                    body,
                };

                js::Declaration::Function(declaration)
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
        let name = field.name.string();
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

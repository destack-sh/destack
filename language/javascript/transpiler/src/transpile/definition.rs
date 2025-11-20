use dyst_dir::{self as dir, Module, NodeTree};
use dyst_javascript_ast::{
    BindingScope, Block, DeclarationDescriptor, DeclarationKind, Definition, EnumField, ExportType,
    Expression, NodeId, Visibility,
};

use crate::{TranspileError, TranspileResult, TranspileResultExt, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile visibility from DIR into JS AST.
    pub fn transpile_visibility(&self, visibility: dir::Visibility) -> Visibility {
        match visibility {
            dir::Visibility::Public => Visibility::Public,
            dir::Visibility::Protected => Visibility::Protected,
            dir::Visibility::Private => Visibility::Private,
        }
    }

    /// Transpile a declaration kind from DIR into JS AST.
    pub fn transpile_declaration_kind(
        &self,
        declaration_kind: dir::DeclarationKind,
    ) -> DeclarationKind {
        match declaration_kind {
            dir::DeclarationKind::Declaration => DeclarationKind::Declaration,
            dir::DeclarationKind::Definition => DeclarationKind::Definition,
        }
    }

    /// Transpile an export type from DIR into JS AST.
    pub fn transpile_export_type(&self, export_type: dir::ExportType) -> ExportType {
        match export_type {
            dir::ExportType::Item => ExportType::Item,
            dir::ExportType::Default => ExportType::Default,
            dir::ExportType::Namespace => ExportType::Namespace,
        }
    }

    /// Transpile a binding scope from DIR into JS AST.
    pub fn transpile_binding_scope(&self, scope: dir::BindingScope) -> BindingScope {
        match scope {
            dir::BindingScope::Static => BindingScope::Static,
            dir::BindingScope::Instance => BindingScope::Instance,
        }
    }

    /// Transpile a declaration descriptor from DIR into JS AST.
    pub fn transpile_declaration_descriptor(
        &self,
        module: &'a Module,
        descriptor: &dir::DeclarationDescriptor,
        unit: &mut TranspilerUnit,
    ) -> DeclarationDescriptor {
        let kind = self.transpile_declaration_kind(descriptor.kind);
        let scope = self.transpile_binding_scope(descriptor.scope);
        let name = descriptor
            .name
            .map(|name| self.transpile_string_to_name(module, name, unit));
        let export = descriptor
            .export
            .map(|export| self.transpile_export_type(export));
        DeclarationDescriptor {
            kind,
            scope,
            name,
            export,
        }
    }

    /// Transpile a definition from DIR into JS AST.
    pub fn transpile_definition(
        &self,
        module: &'a Module,
        tree: &NodeTree,
        definition_id: dir::NodeId<dir::Definition>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Definition>> {
        let definition = tree.get(definition_id);
        let definition = match definition.as_ref() {
            dir::Definition::Namespace {
                descriptor,
                scope: _,
                generics: _,
                definitions,
            } => {
                let descriptor = self.transpile_declaration_descriptor(module, descriptor, unit);
                let definitions = definitions
                    .iter()
                    .map(|definition| self.transpile_definition(module, tree, *definition, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                Definition::Namespace {
                    descriptor,
                    definitions,
                }
            }
            dir::Definition::Struct {
                descriptor,
                scope: _,
                kind: _,
                generics,
                heritage,
                properties,
            } => {
                let descriptor = self.transpile_declaration_descriptor(module, descriptor, unit);
                let generics = self.transpile_generics(module, tree, generics, unit)?;
                let heritage = self.transpile_heritage(module, tree, heritage, unit)?;
                let properties = properties
                    .iter()
                    .map(|property| self.transpile_property(module, tree, *property, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                // TODO #Broken: struct definitions should become just JS types + namespaces?
                Definition::Class {
                    descriptor,
                    generics,
                    heritage,
                    properties,
                }
            }
            dir::Definition::Interface {
                descriptor,
                scope: _,
                generics,
                heritage,
                properties,
            } => {
                let descriptor = self.transpile_declaration_descriptor(module, descriptor, unit);
                let generics = self.transpile_generics(module, tree, generics, unit)?;
                let heritage = self.transpile_heritage(module, tree, heritage, unit)?;
                let properties = properties
                    .iter()
                    .map(|property| self.transpile_property(module, tree, *property, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                Definition::Interface {
                    descriptor,
                    generics,
                    heritage,
                    properties,
                }
            }
            dir::Definition::Enum {
                descriptor,
                scope: _,
                generics: _,
                heritage: _,
                fields,
                properties: _,
            } => {
                let descriptor = self.transpile_declaration_descriptor(module, descriptor, unit);
                let fields = fields
                    .iter()
                    .map(|field| self.transpile_enum_field(module, tree, *field, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                Definition::Enum { descriptor, fields }
            }
            dir::Definition::Function {
                descriptor,
                scope: _,
                signature,
                definitions: _,
                body,
            } => {
                let descriptor = self.transpile_declaration_descriptor(module, descriptor, unit);
                let signature = self.transpile_function_signature(module, tree, signature, unit)?;
                let body = body
                    .map(|body| {
                        self.transpile_expression(module, tree, body, unit)
                            .expect_node::<Block>(body.into_any(), unit)
                    })
                    .transpose()?;
                Definition::Function {
                    descriptor,
                    signature,
                    body,
                }
            }
            _ => {
                return Err(TranspileError::UnsupportedNode {
                    node: definition_id.into_any(),
                    message: None,
                });
            }
        };
        let definition_id = unit
            .ast
            .insert_from_source(definition, module.id, definition_id);
        Ok(definition_id)
    }

    /// Transpile an enum field from DIR into JS AST.
    pub fn transpile_enum_field(
        &self,
        module: &'a Module,
        tree: &NodeTree,
        field_id: dir::NodeId<dir::EnumField>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<EnumField>> {
        let field = tree.get(field_id);
        let name = unit.strings.intern_from(&module.strings, field.name);
        let value = field
            .value
            .as_ref()
            .map(|value_id| {
                self.transpile_expression(module, tree, *value_id, unit)
                    .expect_node::<Expression>(value_id.into_any(), unit)
            })
            .transpose()?;
        let field = EnumField { name, value };
        let field_id = unit.ast.insert_from_source(field, module.id, field_id);
        Ok(field_id)
    }
}

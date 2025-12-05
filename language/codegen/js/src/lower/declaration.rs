use crate::{
    BindingAnchor, Block, Declaration, DeclarationDescriptor, DeclarationKind, DependencyMode,
    EnumField, Expression, LocalNodeId, Statement, Type, Visibility,
};
use destack_dir::{self as dir, Module, NodeTree, SymbolTable, TypeTable};

use crate::{TranspileError, TranspileResult, TranspileResultExt, Transpiler, TranspilerUnit};

impl Transpiler {
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
        &self,
        module: &Module,
        _tree: &NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
        descriptor: &dir::DeclarationDescriptor,
        unit: &mut TranspilerUnit,
    ) -> DeclarationDescriptor {
        let kind = self.lower_declaration_kind(descriptor.kind);
        let anchor = self.lower_binding_anchor(descriptor.anchor);
        let name = descriptor
            .name
            .map(|name| self.lower_string_to_name(module, name, unit));
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
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeId<Declaration>> {
        let declaration = tree.get(declaration_id);
        let declaration = match declaration {
            dir::Declaration::Namespace {
                descriptor,
                scope: _,
                generics: _,
                expressions,
            } => {
                let descriptor = self.lower_declaration_descriptor(
                    module, tree, symbols, types, descriptor, unit,
                );
                let statements = expressions
                    .iter()
                    .map(|expression| {
                        self.lower_expression(module, tree, symbols, types, *expression, unit)
                            .expect_node::<Statement>(expression.into_global_any(module.id), unit)
                    })
                    .collect::<Result<Vec<_>, TranspileError>>()?;
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
                let descriptor = self.lower_declaration_descriptor(
                    module, tree, symbols, types, descriptor, unit,
                );
                let static_parameters = static_parameters
                    .as_ref()
                    .map(|params| {
                        params
                            .iter()
                            .map(|param| {
                                self.lower_parameter(module, tree, symbols, types, *param, unit)
                            })
                            .collect::<Result<Vec<_>, TranspileError>>()
                    })
                    .transpose()?;
                let value_expression = self
                    .lower_expression(module, tree, symbols, types, *value, unit)
                    .expect_node::<Expression>(value.into_global_any(module.id), unit)?;
                let value = unit.ast.insert_from_source_any(
                    Type::Expression(value_expression),
                    module.id,
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
                properties,
            } => {
                let descriptor = self.lower_declaration_descriptor(
                    module, tree, symbols, types, descriptor, unit,
                );
                let generics =
                    self.lower_generics(module, tree, symbols, types, generics, unit)?;
                let heritage =
                    self.lower_heritage(module, tree, symbols, types, heritage, unit)?;
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.lower_property(module, tree, symbols, types, *property, unit)
                    })
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                // NOTE #Incomplete: struct declarations should become just JS types + namespaces?
                Declaration::Class {
                    descriptor,
                    generics,
                    heritage,
                    properties,
                }
            }
            dir::Declaration::Class {
                descriptor,
                scope: _,
                generics,
                heritage,
                properties,
            } => {
                let descriptor = self.lower_declaration_descriptor(
                    module, tree, symbols, types, descriptor, unit,
                );
                let generics =
                    self.lower_generics(module, tree, symbols, types, generics, unit)?;
                let heritage =
                    self.lower_heritage(module, tree, symbols, types, heritage, unit)?;
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.lower_property(module, tree, symbols, types, *property, unit)
                    })
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                Declaration::Class {
                    descriptor,
                    generics,
                    heritage,
                    properties,
                }
            }
            dir::Declaration::Interface {
                descriptor,
                scope: _,
                generics,
                heritage,
                properties,
            } => {
                let descriptor = self.lower_declaration_descriptor(
                    module, tree, symbols, types, descriptor, unit,
                );
                let generics =
                    self.lower_generics(module, tree, symbols, types, generics, unit)?;
                let heritage =
                    self.lower_heritage(module, tree, symbols, types, heritage, unit)?;
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.lower_property(module, tree, symbols, types, *property, unit)
                    })
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                Declaration::Interface {
                    descriptor,
                    generics,
                    heritage,
                    properties,
                }
            }
            dir::Declaration::Enum {
                descriptor,
                scope: _,
                generics: _,
                heritage: _,
                fields,
                properties: _,
            } => {
                let descriptor = self.lower_declaration_descriptor(
                    module, tree, symbols, types, descriptor, unit,
                );
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.lower_enum_field(module, tree, symbols, types, *field, unit)
                    })
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                Declaration::Enum { descriptor, fields }
            }
            dir::Declaration::Function {
                descriptor,
                scope: _,
                signature,
                body,
            } => {
                let descriptor = self.lower_declaration_descriptor(
                    module, tree, symbols, types, descriptor, unit,
                );
                let signature = self
                    .lower_function_signature(module, tree, symbols, types, signature, unit)?;
                let body = body
                    .map(|body| {
                        self.lower_expression(module, tree, symbols, types, body, unit)
                            .expect_node::<Block>(body.into_global_any(module.id), unit)
                    })
                    .transpose()?;
                Declaration::Function {
                    descriptor,
                    signature,
                    body,
                }
            }
            _ => {
                return Err(TranspileError::UnsupportedConstruct {
                    node: declaration_id.into_global_any(module.id),
                    message: None,
                });
            }
        };
        let declaration_id = unit
            .ast
            .insert_from_source(declaration, module.id, declaration_id);
        Ok(declaration_id)
    }

    /// Lower an enum field from DIR into JS AST.
    pub fn lower_enum_field(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        field_id: dir::LocalNodeId<dir::EnumField>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeId<EnumField>> {
        let field = tree.get(field_id);
        let name = unit.strings.intern_from(&module.ast_strings, field.name);
        let value = field
            .value
            .as_ref()
            .map(|value_id| {
                self.lower_expression(module, tree, symbols, types, *value_id, unit)
                    .expect_node::<Expression>(value_id.into_global_any(module.id), unit)
            })
            .transpose()?;
        let field = EnumField { name, value };
        let field_id = unit.ast.insert_from_source(field, module.id, field_id);
        Ok(field_id)
    }
}

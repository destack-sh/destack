use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Definition, Visibility};

use crate::{TranspileError, TranspileResult, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile visibility from DIR into JS AST.
    pub fn transpile_visibility(&self, visibility: dir::Visibility) -> Visibility {
        match visibility {
            dir::Visibility::Public => Visibility::Public,
            dir::Visibility::Protected => Visibility::Protected,
            dir::Visibility::Private => Visibility::Private,
        }
    }

    /// Transpile a definition from DIR into JS AST.
    pub fn transpile_definition(
        &self,
        module: &'a Module,
        definition_id: dir::NodeId<dir::Definition>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Definition>> {
        let definition = self.session.tree.get(definition_id);
        let definition = match definition.as_ref() {
            dir::Definition::Namespace {
                descriptor,
                generics: _,
                definitions,
            } => {
                let descriptor =
                    self.transpile_declaration_descriptor(module, *descriptor, unit)?;
                let definitions = definitions
                    .iter()
                    .map(|definition| self.transpile_definition(module, *definition, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                Definition::Namespace {
                    descriptor,
                    definitions,
                }
            }
            dir::Definition::Struct {
                descriptor,
                kind: _,
                generics,
                heritage,
                properties,
            } => {
                let descriptor =
                    self.transpile_declaration_descriptor(module, *descriptor, unit)?;
                let generics = generics
                    .as_ref()
                    .map(|generics| self.transpile_generics(module, generics, unit))
                    .transpose()?;
                let heritage = heritage
                    .as_ref()
                    .map(|heritage| self.transpile_heritage(module, heritage, unit))
                    .transpose()?;
                let properties = properties
                    .iter()
                    .map(|property| self.transpile_property(module, *property, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                // nocheckin: struct definitions should become just JS types + namespaces?
                Definition::Class {
                    descriptor,
                    generics,
                    heritage,
                    properties,
                }
            }
            dir::Definition::Interface {
                descriptor,
                generics,
                heritage,
                properties,
            } => {
                let descriptor =
                    self.transpile_declaration_descriptor(module, *descriptor, unit)?;
                let generics = generics
                    .as_ref()
                    .map(|generics| self.transpile_generics(module, generics, unit))
                    .transpose()?;
                let heritage = heritage
                    .as_ref()
                    .map(|heritage| self.transpile_heritage(module, heritage, unit))
                    .transpose()?;
                let properties = properties
                    .iter()
                    .map(|property| self.transpile_property(module, *property, unit))
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
                generics: _,
                heritage: _,
                fields,
                properties,
            } => {
                let descriptor =
                    self.transpile_declaration_descriptor(module, *descriptor, unit)?;
                let fields = fields
                    .iter()
                    .map(|field| self.transpile_enum_field(module, *field, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                let properties = properties
                    .iter()
                    .map(|property| self.transpile_property(module, *property, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                Definition::Enum { descriptor, fields }
            }
            dir::Definition::Function {
                descriptor,
                signature,
                definitions: _,
                body,
            } => {
                let descriptor =
                    self.transpile_declaration_descriptor(module, *descriptor, unit)?;
                let signature = self.transpile_function_signature(module, signature, unit)?;
                let body = body
                    .as_ref()
                    .map(|body| self.transpile_block(module, body, unit))
                    .transpose()?;
                Definition::Function {
                    descriptor,
                    signature,
                    body,
                }
            }
            dir::Definition::Implement {
                descriptor,
                generics: _,
                target_type,
                heritage: _,
                properties,
            } => {
                return Err(TranspileError::UnsupportedDefinition {
                    node: definition_id,
                });
            }
        };
        let definition_id = unit
            .ast
            .insert_from_dir(definition, module.id, definition_id);
        Ok(definition_id)
    }
}

use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    AssociatedConstDefinition, AssociatedTypeDefinition, CheckState, ClassDefinition, Definition,
    EnumDefinition, ExtensionDefinition, ExtensionTarget, FieldDefinition, InterfaceDefinition,
    MethodDefinition, NewtypeDefinition, NominalHeritage, SignatureDefinition, StructDefinition,
    TypeAliasDefinition, TypeOperand, VariantDefinition,
};
use crate::{CompilerError, CompilerResult};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit definitions into the DIR definition table.
    pub(super) fn commit_definition_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<dir::DefinitionSegment> {
        let mut table = dir::DefinitionSegment::new(module);
        let definitions = self
            .definitions
            .definitions_in(module)
            .map(|(symbol, definition)| (symbol, definition.clone()))
            .collect::<Vec<_>>();

        // commit definitions in build order
        for (symbol, definition) in definitions {
            let source = definition.source();
            let definition =
                self.commit_definition(module, output, environment, symbol, definition)?;

            table.insert_definition(symbol, source, definition);
        }

        Ok(table)
    }

    /// Commit one checked definition.
    fn commit_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        symbol: dir::GlobalSymbolId,
        definition: Definition,
    ) -> CompilerResult<dir::Definition> {
        let definition = match definition {
            Definition::TypeAlias(definition) => dir::Definition::TypeAlias(
                self.commit_type_alias_definition(module, output, environment, definition)?,
            ),
            Definition::Struct(definition) => dir::Definition::Struct(
                self.commit_struct_definition(module, output, environment, definition)?,
            ),
            Definition::Class(definition) => dir::Definition::Class(self.commit_class_definition(
                module,
                output,
                environment,
                definition,
            )?),
            Definition::Interface(definition) => dir::Definition::Interface(
                self.commit_interface_definition(module, output, environment, definition)?,
            ),
            Definition::Enum(definition) => dir::Definition::Enum(self.commit_enum_definition(
                module,
                output,
                environment,
                definition,
            )?),
            Definition::Newtype(definition) => dir::Definition::Newtype(
                self.commit_newtype_definition(module, output, environment, definition)?,
            ),
            Definition::Extension(definition) => dir::Definition::Extension(
                self.commit_extension_definition(module, output, environment, symbol, definition)?,
            ),
        };

        Ok(definition)
    }

    /// Commit one extension definition.
    fn commit_extension_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        symbol: dir::GlobalSymbolId,
        definition: ExtensionDefinition,
    ) -> CompilerResult<dir::Extension> {
        let source = definition.source.local_id;
        let target =
            self.commit_extension_target(module, output, environment, definition.target, source)?;
        let where_clauses =
            self.commit_extension_where_clauses(module, output, environment, &definition)?;
        let implements =
            self.commit_nominal_heritages(module, output, environment, definition.implements);
        let fields =
            self.commit_field_definitions(module, output, environment, definition.fields)?;
        let static_fields =
            self.commit_field_definitions(module, output, environment, definition.static_fields)?;
        let methods =
            self.commit_method_definitions(module, output, environment, definition.methods)?;
        let static_methods =
            self.commit_method_definitions(module, output, environment, definition.static_methods)?;
        let associated_types = self.commit_associated_type_definitions(
            module,
            output,
            environment,
            definition.associated_types,
        )?;
        let associated_consts = self.commit_associated_const_definitions(
            module,
            output,
            environment,
            definition.associated_consts,
        )?;

        Ok(dir::Extension::new(
            symbol,
            definition.form,
            target,
            implements,
            where_clauses,
            fields,
            static_fields,
            methods,
            static_methods,
            associated_types,
            associated_consts,
        ))
    }

    /// Commit one extension receiver target.
    fn commit_extension_target(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        target: ExtensionTarget,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::ExtensionTarget> {
        let ty = self
            .commit_type_operand(module, output, environment, target.r#type(), source)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("extension target {source:?} has no committed type"),
            })?;
        let target = match target {
            ExtensionTarget::Nominal { root, .. } => dir::ExtensionTarget::Nominal { root, ty },
            ExtensionTarget::Blanket { .. } => dir::ExtensionTarget::Blanket { ty },
        };

        Ok(target)
    }

    /// Commit extension where clauses.
    fn commit_extension_where_clauses(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: &ExtensionDefinition,
    ) -> CompilerResult<Vec<dir::ExtensionWhereClause>> {
        let mut where_clauses = Vec::with_capacity(definition.where_clauses.len());

        // commit each where clause operand pair
        for where_clause in &definition.where_clauses {
            let source = where_clause.source.local_id;
            let left = self
                .commit_type_operand(module, output, environment, where_clause.left, source)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "extension where clause {:?} has no committed left type",
                        where_clause.source
                    ),
                });
            let right = self
                .commit_type_operand(module, output, environment, where_clause.right, source)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "extension where clause {:?} has no committed right type",
                        where_clause.source
                    ),
                });

            where_clauses.push(dir::ExtensionWhereClause {
                source: where_clause.source,
                left: left?,
                right: right?,
            });
        }

        Ok(where_clauses)
    }

    /// Commit one type alias definition.
    fn commit_type_alias_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: TypeAliasDefinition,
    ) -> CompilerResult<dir::TypeAliasDefinition> {
        let source = definition.source.local_id;
        let value = self.commit_type_operand(module, output, environment, definition.value, source);
        let Some(value) = value else {
            return Err(CompilerError::Internal {
                message: format!("type alias {:?} has no committed value", definition.source),
            });
        };

        Ok(dir::TypeAliasDefinition {
            template: self.commit_nominal_template(definition.template),
            value,
        })
    }

    /// Commit one struct definition.
    fn commit_struct_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: StructDefinition,
    ) -> CompilerResult<dir::StructDefinition> {
        Ok(dir::StructDefinition {
            template: self.commit_nominal_template(definition.template),
            implements: self.commit_nominal_heritages(
                module,
                output,
                environment,
                definition.implements,
            ),
            fields: self.commit_field_definitions(
                module,
                output,
                environment,
                definition.fields,
            )?,
            static_fields: self.commit_field_definitions(
                module,
                output,
                environment,
                definition.static_fields,
            )?,
            methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.methods,
            )?,
            static_methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.static_methods,
            )?,
            associated_types: self.commit_associated_type_definitions(
                module,
                output,
                environment,
                definition.associated_types,
            )?,
            associated_consts: self.commit_associated_const_definitions(
                module,
                output,
                environment,
                definition.associated_consts,
            )?,
        })
    }

    /// Commit one class definition.
    fn commit_class_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: ClassDefinition,
    ) -> CompilerResult<dir::ClassDefinition> {
        Ok(dir::ClassDefinition {
            template: self.commit_nominal_template(definition.template),
            extends: definition.extends.map(|heritage| {
                self.commit_nominal_heritage(module, output, environment, heritage)
            }),
            implements: self.commit_nominal_heritages(
                module,
                output,
                environment,
                definition.implements,
            ),
            fields: self.commit_field_definitions(
                module,
                output,
                environment,
                definition.fields,
            )?,
            static_fields: self.commit_field_definitions(
                module,
                output,
                environment,
                definition.static_fields,
            )?,
            methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.methods,
            )?,
            static_methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.static_methods,
            )?,
            associated_types: self.commit_associated_type_definitions(
                module,
                output,
                environment,
                definition.associated_types,
            )?,
            associated_consts: self.commit_associated_const_definitions(
                module,
                output,
                environment,
                definition.associated_consts,
            )?,
        })
    }

    /// Commit one nominal interface definition.
    fn commit_interface_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: InterfaceDefinition,
    ) -> CompilerResult<dir::InterfaceDefinition> {
        Ok(dir::InterfaceDefinition {
            template: self.commit_nominal_template(definition.template),
            is_nominal: definition.is_nominal,
            extends: self.commit_nominal_heritages(module, output, environment, definition.extends),
            fields: self.commit_field_definitions(
                module,
                output,
                environment,
                definition.fields,
            )?,
            static_fields: self.commit_field_definitions(
                module,
                output,
                environment,
                definition.static_fields,
            )?,
            methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.methods,
            )?,
            static_methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.static_methods,
            )?,
            call_signatures: self.commit_signature_definitions(
                module,
                output,
                environment,
                definition.call_signatures,
            )?,
            construct_signatures: self.commit_signature_definitions(
                module,
                output,
                environment,
                definition.construct_signatures,
            )?,
            index_signatures: self.commit_signature_definitions(
                module,
                output,
                environment,
                definition.index_signatures,
            )?,
            associated_types: self.commit_associated_type_definitions(
                module,
                output,
                environment,
                definition.associated_types,
            )?,
            associated_consts: self.commit_associated_const_definitions(
                module,
                output,
                environment,
                definition.associated_consts,
            )?,
        })
    }

    /// Commit one enum definition.
    fn commit_enum_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: EnumDefinition,
    ) -> CompilerResult<dir::EnumDefinition> {
        Ok(dir::EnumDefinition {
            template: self.commit_nominal_template(definition.template),
            implements: self.commit_nominal_heritages(
                module,
                output,
                environment,
                definition.implements,
            ),
            variants: self.commit_variant_definitions(
                module,
                output,
                environment,
                definition.variants,
            ),
            static_fields: self.commit_field_definitions(
                module,
                output,
                environment,
                definition.static_fields,
            )?,
            methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.methods,
            )?,
            static_methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.static_methods,
            )?,
            associated_types: self.commit_associated_type_definitions(
                module,
                output,
                environment,
                definition.associated_types,
            )?,
            associated_consts: self.commit_associated_const_definitions(
                module,
                output,
                environment,
                definition.associated_consts,
            )?,
        })
    }

    /// Commit one newtype definition.
    fn commit_newtype_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: NewtypeDefinition,
    ) -> CompilerResult<dir::NewtypeDefinition> {
        let value = self.commit_nominal_type_operand(
            module,
            output,
            environment,
            definition.value,
            definition.source,
        )?;

        Ok(dir::NewtypeDefinition {
            template: self.commit_nominal_template(definition.template),
            value,
        })
    }

    /// Commit one checked generic template reference.
    fn commit_nominal_template(
        &self,
        template: Option<crate::check::GenericTemplateId>,
    ) -> Option<dir::LocalGenericTemplateId> {
        template.map(|template| template.local_id)
    }

    /// Commit nominal heritages.
    pub(super) fn commit_nominal_heritages(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        heritages: Vec<NominalHeritage>,
    ) -> Vec<dir::NominalHeritage> {
        heritages
            .into_iter()
            .map(|heritage| self.commit_nominal_heritage(module, output, environment, heritage))
            .collect()
    }

    /// Commit one nominal heritage.
    fn commit_nominal_heritage(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        heritage: NominalHeritage,
    ) -> dir::NominalHeritage {
        let instance = heritage.instance.as_ref().and_then(|instance| {
            self.commit_generic_instance(
                module,
                output,
                environment,
                heritage.source.local_id,
                instance,
            )
        });

        dir::NominalHeritage {
            source: heritage.source,
            symbol: heritage.symbol,
            instance,
        }
    }

    /// Commit checked field definitions.
    pub(super) fn commit_field_definitions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definitions: Vec<FieldDefinition>,
    ) -> CompilerResult<Vec<dir::FieldDefinition>> {
        definitions
            .into_iter()
            .map(|definition| {
                let ty = self.commit_nominal_type_operand(
                    module,
                    output,
                    environment,
                    definition.ty,
                    definition.source,
                )?;

                Ok(dir::FieldDefinition {
                    symbol: definition.symbol,
                    source: definition.source,
                    key: definition.key,
                    ty,
                })
            })
            .collect()
    }

    /// Commit checked method definitions.
    pub(super) fn commit_method_definitions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definitions: Vec<MethodDefinition>,
    ) -> CompilerResult<Vec<dir::MethodDefinition>> {
        definitions
            .into_iter()
            .map(|definition| {
                let ty = self.commit_nominal_type_operand(
                    module,
                    output,
                    environment,
                    definition.ty,
                    definition.source,
                )?;

                Ok(dir::MethodDefinition {
                    symbol: definition.symbol,
                    source: definition.source,
                    slot: definition.slot,
                    ty,
                })
            })
            .collect()
    }

    /// Commit checked associated type definitions.
    pub(super) fn commit_associated_type_definitions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definitions: Vec<AssociatedTypeDefinition>,
    ) -> CompilerResult<Vec<dir::AssociatedTypeDefinition>> {
        definitions
            .into_iter()
            .map(|definition| {
                let constraint = match definition.constraint {
                    Some(constraint) => Some(self.commit_nominal_type_operand(
                        module,
                        output,
                        environment,
                        constraint,
                        definition.source,
                    )?),
                    None => None,
                };
                let value = match definition.value {
                    Some(value) => Some(self.commit_nominal_type_operand(
                        module,
                        output,
                        environment,
                        value,
                        definition.source,
                    )?),
                    None => None,
                };

                Ok(dir::AssociatedTypeDefinition {
                    symbol: definition.symbol,
                    source: definition.source,
                    key: definition.key,
                    constraint,
                    value,
                })
            })
            .collect()
    }

    /// Commit checked associated const definitions.
    pub(super) fn commit_associated_const_definitions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definitions: Vec<AssociatedConstDefinition>,
    ) -> CompilerResult<Vec<dir::AssociatedConstDefinition>> {
        definitions
            .into_iter()
            .map(|definition| {
                let ty = self.commit_nominal_type_operand(
                    module,
                    output,
                    environment,
                    definition.ty,
                    definition.source,
                )?;
                let value = definition.value.and_then(|value| {
                    self.commit_static_operand(module, output, environment, value)
                });

                Ok(dir::AssociatedConstDefinition {
                    symbol: definition.symbol,
                    source: definition.source,
                    key: definition.key,
                    ty,
                    value,
                })
            })
            .collect()
    }

    /// Commit checked variant definitions.
    fn commit_variant_definitions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definitions: Vec<VariantDefinition>,
    ) -> Vec<dir::VariantDefinition> {
        definitions
            .into_iter()
            .map(|definition| dir::VariantDefinition {
                symbol: definition.symbol,
                source: definition.source,
                key: definition.key,
                value: definition.value.and_then(|value| {
                    self.commit_static_operand(module, output, environment, value)
                }),
            })
            .collect()
    }

    /// Commit checked signature definitions.
    fn commit_signature_definitions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definitions: Vec<SignatureDefinition>,
    ) -> CompilerResult<Vec<dir::SignatureDefinition>> {
        definitions
            .into_iter()
            .map(|definition| {
                let ty = self.commit_nominal_type_operand(
                    module,
                    output,
                    environment,
                    definition.ty,
                    definition.source,
                )?;

                Ok(dir::SignatureDefinition {
                    source: definition.source,
                    ty,
                })
            })
            .collect()
    }

    /// Commit one nominal type operand.
    fn commit_nominal_type_operand(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        operand: TypeOperand,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(ty) =
            self.commit_type_operand(module, output, environment, operand, source.local_id)
        else {
            return Err(CompilerError::Internal {
                message: format!("nominal source node {source:?} has unresolved type operand"),
            });
        };

        Ok(ty)
    }
}

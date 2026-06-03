use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    AssociatedConstDefinition, AssociatedTypeDefinition, CheckState, ClassDefinition,
    EnumDefinition, FieldDefinition, InterfaceDefinition, MethodDefinition, NewtypeDefinition,
    NominalDefinition, NominalHeritage, SignatureDefinition, StructDefinition, TypeOperand,
    VariantDefinition,
};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit nominal definitions into the DIR nominal table.
    pub(super) fn commit_nominal_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> dir::NominalSegment {
        let mut table = dir::NominalSegment::new(module);
        let definitions = self
            .nominals
            .definitions_in(module)
            .map(|(symbol, definition)| (symbol, definition.clone()))
            .collect::<Vec<_>>();

        // commit definitions in build order
        for (symbol, definition) in definitions {
            let source = definition.source();
            let definition =
                self.commit_nominal_definition(module, output, environment, definition);

            table.insert_definition(symbol, source, definition);
        }

        table
    }

    /// Commit one nominal definition.
    fn commit_nominal_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: NominalDefinition,
    ) -> dir::NominalDefinition {
        match definition {
            NominalDefinition::Struct(definition) => dir::NominalDefinition::Struct(
                self.commit_struct_definition(module, output, environment, definition),
            ),
            NominalDefinition::Class(definition) => dir::NominalDefinition::Class(
                self.commit_class_definition(module, output, environment, definition),
            ),
            NominalDefinition::Interface(definition) => dir::NominalDefinition::Interface(
                self.commit_interface_definition(module, output, environment, definition),
            ),
            NominalDefinition::Enum(definition) => dir::NominalDefinition::Enum(
                self.commit_enum_definition(module, output, environment, definition),
            ),
            NominalDefinition::Newtype(definition) => dir::NominalDefinition::Newtype(
                self.commit_newtype_definition(module, output, environment, definition),
            ),
        }
    }

    /// Commit one struct definition.
    fn commit_struct_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: StructDefinition,
    ) -> dir::StructDefinition {
        dir::StructDefinition {
            template: self.commit_nominal_template(output, definition.template),
            implements: self.commit_nominal_heritages(
                module,
                output,
                environment,
                definition.implements,
            ),
            fields: self.commit_field_definitions(module, output, environment, definition.fields),
            static_fields: self.commit_field_definitions(
                module,
                output,
                environment,
                definition.static_fields,
            ),
            methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.methods,
            ),
            static_methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.static_methods,
            ),
            associated_types: self.commit_associated_type_definitions(
                module,
                output,
                environment,
                definition.associated_types,
            ),
            associated_consts: self.commit_associated_const_definitions(
                module,
                output,
                environment,
                definition.associated_consts,
            ),
        }
    }

    /// Commit one class definition.
    fn commit_class_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: ClassDefinition,
    ) -> dir::ClassDefinition {
        dir::ClassDefinition {
            template: self.commit_nominal_template(output, definition.template),
            extends: definition.extends.map(|heritage| {
                self.commit_nominal_heritage(module, output, environment, heritage)
            }),
            implements: self.commit_nominal_heritages(
                module,
                output,
                environment,
                definition.implements,
            ),
            fields: self.commit_field_definitions(module, output, environment, definition.fields),
            static_fields: self.commit_field_definitions(
                module,
                output,
                environment,
                definition.static_fields,
            ),
            methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.methods,
            ),
            static_methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.static_methods,
            ),
            associated_types: self.commit_associated_type_definitions(
                module,
                output,
                environment,
                definition.associated_types,
            ),
            associated_consts: self.commit_associated_const_definitions(
                module,
                output,
                environment,
                definition.associated_consts,
            ),
        }
    }

    /// Commit one nominal interface definition.
    fn commit_interface_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: InterfaceDefinition,
    ) -> dir::InterfaceDefinition {
        dir::InterfaceDefinition {
            template: self.commit_nominal_template(output, definition.template),
            extends: self.commit_nominal_heritages(module, output, environment, definition.extends),
            fields: self.commit_field_definitions(module, output, environment, definition.fields),
            static_fields: self.commit_field_definitions(
                module,
                output,
                environment,
                definition.static_fields,
            ),
            methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.methods,
            ),
            static_methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.static_methods,
            ),
            call_signatures: self.commit_signature_definitions(
                module,
                output,
                environment,
                definition.call_signatures,
            ),
            construct_signatures: self.commit_signature_definitions(
                module,
                output,
                environment,
                definition.construct_signatures,
            ),
            index_signatures: self.commit_signature_definitions(
                module,
                output,
                environment,
                definition.index_signatures,
            ),
            associated_types: self.commit_associated_type_definitions(
                module,
                output,
                environment,
                definition.associated_types,
            ),
            associated_consts: self.commit_associated_const_definitions(
                module,
                output,
                environment,
                definition.associated_consts,
            ),
        }
    }

    /// Commit one enum definition.
    fn commit_enum_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: EnumDefinition,
    ) -> dir::EnumDefinition {
        dir::EnumDefinition {
            template: self.commit_nominal_template(output, definition.template),
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
            ),
            methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.methods,
            ),
            static_methods: self.commit_method_definitions(
                module,
                output,
                environment,
                definition.static_methods,
            ),
            associated_types: self.commit_associated_type_definitions(
                module,
                output,
                environment,
                definition.associated_types,
            ),
            associated_consts: self.commit_associated_const_definitions(
                module,
                output,
                environment,
                definition.associated_consts,
            ),
        }
    }

    /// Commit one newtype definition.
    fn commit_newtype_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: NewtypeDefinition,
    ) -> dir::NewtypeDefinition {
        let value = self.commit_nominal_type_operand(
            module,
            output,
            environment,
            definition.value,
            definition.source,
        );

        dir::NewtypeDefinition {
            template: self.commit_nominal_template(output, definition.template),
            value,
        }
    }

    /// Commit one checked generic template reference.
    fn commit_nominal_template(
        &self,
        output: &CheckModuleOutput,
        template: Option<crate::check::GenericTemplate>,
    ) -> Option<dir::LocalGenericTemplateId> {
        let template = template?;

        output
            .generics
            .iter_templates()
            .find_map(|(id, output_template)| {
                (output_template.owner == template.owner).then_some(id)
            })
    }

    /// Commit nominal heritages.
    fn commit_nominal_heritages(
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
            self.commit_generic_instance(module, output, environment, heritage.source, instance)
        });

        dir::NominalHeritage {
            source: heritage.source,
            symbol: heritage.symbol,
            instance,
        }
    }

    /// Commit checked field definitions.
    fn commit_field_definitions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definitions: Vec<FieldDefinition>,
    ) -> Vec<dir::FieldDefinition> {
        definitions
            .into_iter()
            .map(|definition| dir::FieldDefinition {
                symbol: definition.symbol,
                source: definition.source,
                key: definition.key,
                ty: self.commit_nominal_type_operand(
                    module,
                    output,
                    environment,
                    definition.ty,
                    definition.source,
                ),
            })
            .collect()
    }

    /// Commit checked method definitions.
    fn commit_method_definitions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definitions: Vec<MethodDefinition>,
    ) -> Vec<dir::MethodDefinition> {
        definitions
            .into_iter()
            .map(|definition| dir::MethodDefinition {
                symbol: definition.symbol,
                source: definition.source,
                slot: definition.slot,
                ty: self.commit_nominal_type_operand(
                    module,
                    output,
                    environment,
                    definition.ty,
                    definition.source,
                ),
            })
            .collect()
    }

    /// Commit checked associated type definitions.
    fn commit_associated_type_definitions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definitions: Vec<AssociatedTypeDefinition>,
    ) -> Vec<dir::AssociatedTypeDefinition> {
        definitions
            .into_iter()
            .map(|definition| dir::AssociatedTypeDefinition {
                symbol: definition.symbol,
                source: definition.source,
                constraint: definition.constraint.map(|constraint| {
                    self.commit_nominal_type_operand(
                        module,
                        output,
                        environment,
                        constraint,
                        definition.source,
                    )
                }),
                value: definition.value.map(|value| {
                    self.commit_nominal_type_operand(
                        module,
                        output,
                        environment,
                        value,
                        definition.source,
                    )
                }),
            })
            .collect()
    }

    /// Commit checked associated const definitions.
    fn commit_associated_const_definitions(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definitions: Vec<AssociatedConstDefinition>,
    ) -> Vec<dir::AssociatedConstDefinition> {
        definitions
            .into_iter()
            .map(|definition| dir::AssociatedConstDefinition {
                symbol: definition.symbol,
                source: definition.source,
                ty: self.commit_nominal_type_operand(
                    module,
                    output,
                    environment,
                    definition.ty,
                    definition.source,
                ),
                value: definition.value.and_then(|value| {
                    self.commit_static_operand(module, output, environment, value)
                }),
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
    ) -> Vec<dir::SignatureDefinition> {
        definitions
            .into_iter()
            .map(|definition| dir::SignatureDefinition {
                source: definition.source,
                ty: self.commit_nominal_type_operand(
                    module,
                    output,
                    environment,
                    definition.ty,
                    definition.source,
                ),
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
    ) -> dir::GlobalTypeId {
        self.commit_type_operand(module, output, environment, operand, source.local_id)
            .unwrap_or_else(|| panic!("nominal source node {source:?} has unresolved type operand"))
    }
}

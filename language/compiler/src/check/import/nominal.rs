use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    AssociatedConstDefinition, AssociatedTypeDefinition, CheckState, ClassDefinition,
    EnumDefinition, FieldDefinition, GenericApplication, GenericArgument, InterfaceDefinition,
    MethodDefinition, NewtypeDefinition, NominalDefinition, NominalHeritage, SignatureDefinition,
    StructDefinition, TypeOperand, VariantDefinition,
};

impl CheckState<'_> {
    /// Return one visible nominal definition.
    pub(in crate::check) fn nominal_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<NominalDefinition> {
        if self.is_component_module(symbol.module_id) {
            return self.nominals.definition(symbol).cloned();
        }

        let definition = self
            .dependency(symbol.module_id)
            .nominals
            .definition(symbol)?
            .clone();
        let source = self
            .dependency(symbol.module_id)
            .nominals
            .definition_source(symbol)
            .unwrap_or_else(|| panic!("dependency nominal symbol {symbol:?} has no source"));

        Some(self.import_nominal_definition(module, symbol, source, definition))
    }

    /// Return one newtype backing type operand in a component module context.
    pub(in crate::check) fn newtype_backing(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<TypeOperand> {
        match self.nominal_definition(module, symbol)? {
            NominalDefinition::Newtype(definition) => Some(definition.value),
            _ => None,
        }
    }

    /// Return nominal fields as operands in a component module context.
    pub(in crate::check) fn nominal_fields(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<Vec<(dir::StaticKey, TypeOperand)>> {
        let definition = self.nominal_definition(module, symbol)?;
        let fields = match definition {
            NominalDefinition::Struct(definition) => definition.fields,
            NominalDefinition::Class(definition) => definition.fields,
            NominalDefinition::Interface(definition) => definition.fields,
            _ => return None,
        };

        Some(
            fields
                .into_iter()
                .map(|field| (field.key, field.ty))
                .collect(),
        )
    }

    /// Return whether one nominal symbol carries reference identity.
    pub(in crate::check) fn nominal_supports_identity(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> bool {
        matches!(
            self.nominal_definition(module, symbol),
            Some(NominalDefinition::Class(_))
        )
    }

    /// Import one nominal definition.
    fn import_nominal_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
        definition: dir::NominalDefinition,
    ) -> NominalDefinition {
        match definition {
            dir::NominalDefinition::Struct(definition) => {
                NominalDefinition::Struct(StructDefinition {
                    source,
                    template: self.inference.generic_template_for_owner(symbol).cloned(),
                    implements: self.import_nominal_heritages(module, definition.implements),
                    fields: self.import_field_definitions(module, definition.fields),
                    static_fields: self.import_field_definitions(module, definition.static_fields),
                    methods: self.import_method_definitions(module, definition.methods),
                    static_methods: self
                        .import_method_definitions(module, definition.static_methods),
                    associated_types: self
                        .import_associated_type_definitions(module, definition.associated_types),
                    associated_consts: self
                        .import_associated_const_definitions(module, definition.associated_consts),
                })
            }
            dir::NominalDefinition::Class(definition) => {
                NominalDefinition::Class(ClassDefinition {
                    source,
                    template: self.inference.generic_template_for_owner(symbol).cloned(),
                    extends: definition
                        .extends
                        .map(|heritage| self.import_nominal_heritage(module, heritage)),
                    implements: self.import_nominal_heritages(module, definition.implements),
                    fields: self.import_field_definitions(module, definition.fields),
                    static_fields: self.import_field_definitions(module, definition.static_fields),
                    methods: self.import_method_definitions(module, definition.methods),
                    static_methods: self
                        .import_method_definitions(module, definition.static_methods),
                    associated_types: self
                        .import_associated_type_definitions(module, definition.associated_types),
                    associated_consts: self
                        .import_associated_const_definitions(module, definition.associated_consts),
                })
            }
            dir::NominalDefinition::Interface(definition) => {
                NominalDefinition::Interface(InterfaceDefinition {
                    source,
                    template: self.inference.generic_template_for_owner(symbol).cloned(),
                    extends: self.import_nominal_heritages(module, definition.extends),
                    fields: self.import_field_definitions(module, definition.fields),
                    static_fields: self.import_field_definitions(module, definition.static_fields),
                    methods: self.import_method_definitions(module, definition.methods),
                    static_methods: self
                        .import_method_definitions(module, definition.static_methods),
                    call_signatures: self
                        .import_signature_definitions(module, definition.call_signatures),
                    construct_signatures: self
                        .import_signature_definitions(module, definition.construct_signatures),
                    index_signatures: self
                        .import_signature_definitions(module, definition.index_signatures),
                    associated_types: self
                        .import_associated_type_definitions(module, definition.associated_types),
                    associated_consts: self
                        .import_associated_const_definitions(module, definition.associated_consts),
                })
            }
            dir::NominalDefinition::Enum(definition) => NominalDefinition::Enum(EnumDefinition {
                source,
                template: self.inference.generic_template_for_owner(symbol).cloned(),
                implements: self.import_nominal_heritages(module, definition.implements),
                variants: self.import_variant_definitions(module, definition.variants),
                static_fields: self.import_field_definitions(module, definition.static_fields),
                methods: self.import_method_definitions(module, definition.methods),
                static_methods: self.import_method_definitions(module, definition.static_methods),
                associated_types: self
                    .import_associated_type_definitions(module, definition.associated_types),
                associated_consts: self
                    .import_associated_const_definitions(module, definition.associated_consts),
            }),
            dir::NominalDefinition::Newtype(definition) => {
                NominalDefinition::Newtype(NewtypeDefinition {
                    source,
                    template: self.inference.generic_template_for_owner(symbol).cloned(),
                    value: self.import_type_operand(module, definition.value),
                })
            }
        }
    }

    /// Import nominal heritages from committed nominal metadata.
    fn import_nominal_heritages(
        &mut self,
        module: ModuleId,
        heritages: Vec<dir::NominalHeritage>,
    ) -> Vec<NominalHeritage> {
        heritages
            .into_iter()
            .map(|heritage| self.import_nominal_heritage(module, heritage))
            .collect()
    }

    /// Import one nominal heritage from committed nominal metadata.
    fn import_nominal_heritage(
        &mut self,
        module: ModuleId,
        heritage: dir::NominalHeritage,
    ) -> NominalHeritage {
        NominalHeritage {
            source: heritage.source,
            symbol: heritage.symbol,
            application: heritage.application.map(|application| {
                self.import_generic_application(module, heritage.symbol, application)
            }),
        }
    }

    /// Import one generic application.
    fn import_generic_application(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        application: dir::LocalGenericApplicationId,
    ) -> GenericApplication {
        let application = self
            .dependency(symbol.module_id)
            .generics
            .get_application(application)
            .clone();
        let template = self
            .dependency(symbol.module_id)
            .generics
            .get_template(application.template);
        let owner = template.owner;
        let arguments = application
            .arguments
            .into_iter()
            .map(|argument| {
                GenericArgument::Static(self.import_static_operand(module, argument.value))
            })
            .collect();

        GenericApplication { owner, arguments }
    }

    /// Import field definitions from committed nominal metadata.
    fn import_field_definitions(
        &mut self,
        module: ModuleId,
        fields: Vec<dir::FieldDefinition>,
    ) -> Vec<FieldDefinition> {
        fields
            .into_iter()
            .map(|field| FieldDefinition {
                symbol: field.symbol,
                source: field.source,
                key: field.key,
                ty: self.import_type_operand(module, field.ty),
            })
            .collect()
    }

    /// Import method definitions from committed nominal metadata.
    fn import_method_definitions(
        &mut self,
        module: ModuleId,
        methods: Vec<dir::MethodDefinition>,
    ) -> Vec<MethodDefinition> {
        methods
            .into_iter()
            .map(|method| MethodDefinition {
                symbol: method.symbol,
                source: method.source,
                slot: method.slot,
                ty: self.import_type_operand(module, method.ty),
            })
            .collect()
    }

    /// Import associated type definitions from committed nominal metadata.
    fn import_associated_type_definitions(
        &mut self,
        module: ModuleId,
        types: Vec<dir::AssociatedTypeDefinition>,
    ) -> Vec<AssociatedTypeDefinition> {
        types
            .into_iter()
            .map(|ty| AssociatedTypeDefinition {
                symbol: ty.symbol,
                source: ty.source,
                constraint: ty
                    .constraint
                    .map(|constraint| self.import_type_operand(module, constraint)),
                value: ty
                    .value
                    .map(|value| self.import_type_operand(module, value)),
            })
            .collect()
    }

    /// Import associated const definitions from committed nominal metadata.
    fn import_associated_const_definitions(
        &mut self,
        module: ModuleId,
        consts: Vec<dir::AssociatedConstDefinition>,
    ) -> Vec<AssociatedConstDefinition> {
        consts
            .into_iter()
            .map(|value| AssociatedConstDefinition {
                symbol: value.symbol,
                source: value.source,
                ty: self.import_type_operand(module, value.ty),
                value: value
                    .value
                    .map(|value| self.import_static_operand(module, value)),
            })
            .collect()
    }

    /// Import enum variant definitions from committed nominal metadata.
    fn import_variant_definitions(
        &mut self,
        module: ModuleId,
        variants: Vec<dir::VariantDefinition>,
    ) -> Vec<VariantDefinition> {
        variants
            .into_iter()
            .map(|variant| VariantDefinition {
                symbol: variant.symbol,
                source: variant.source,
                key: variant.key,
                value: variant
                    .value
                    .map(|value| self.import_static_operand(module, value)),
            })
            .collect()
    }

    /// Import signature definitions from committed nominal metadata.
    fn import_signature_definitions(
        &mut self,
        module: ModuleId,
        signatures: Vec<dir::SignatureDefinition>,
    ) -> Vec<SignatureDefinition> {
        signatures
            .into_iter()
            .map(|signature| SignatureDefinition {
                source: signature.source,
                ty: self.import_type_operand(module, signature.ty),
            })
            .collect()
    }
}

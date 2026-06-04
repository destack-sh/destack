use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    AssociatedConstDefinition, AssociatedTypeDefinition, CheckState, ClassDefinition,
    EnumDefinition, FieldDefinition, GenericArgument, GenericInstance, InterfaceDefinition,
    MethodDefinition, NewtypeDefinition, NominalDefinition, NominalHeritage, SignatureDefinition,
    StructDefinition, TypeOperand, VariantDefinition,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return one visible nominal definition.
    pub(in crate::check) fn nominal_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<NominalDefinition>> {
        if self.is_component_module(symbol.module_id) {
            return Ok(self.nominals.definition(symbol).cloned());
        }

        let Some(definition) = self
            .dependency(symbol.module_id)
            .nominals
            .definition(symbol)
            .cloned()
        else {
            return Ok(None);
        };
        let source = self
            .dependency(symbol.module_id)
            .nominals
            .definition_source(symbol)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("dependency nominal symbol {symbol:?} has no source"),
            })?;

        Ok(Some(self.import_nominal_definition(
            module, symbol, source, definition,
        )?))
    }

    /// Return one newtype backing type operand in a component module context.
    pub(in crate::check) fn newtype_backing(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<TypeOperand>> {
        match self.nominal_definition(module, symbol)? {
            Some(NominalDefinition::Newtype(definition)) => Ok(Some(definition.value)),
            _ => Ok(None),
        }
    }

    /// Return nominal fields as operands in a component module context.
    pub(in crate::check) fn nominal_fields(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<Vec<(dir::StaticKey, TypeOperand)>>> {
        let Some(definition) = self.nominal_definition(module, symbol)? else {
            return Ok(None);
        };
        let fields = match definition {
            NominalDefinition::Struct(definition) => definition.fields,
            NominalDefinition::Class(definition) => definition.fields,
            NominalDefinition::Interface(definition) => definition.fields,
            _ => return Ok(None),
        };

        Ok(Some(
            fields
                .into_iter()
                .map(|field| (field.key, field.ty))
                .collect(),
        ))
    }

    /// Import one nominal definition.
    fn import_nominal_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
        definition: dir::NominalDefinition,
    ) -> CompilerResult<NominalDefinition> {
        match definition {
            dir::NominalDefinition::Struct(definition) => {
                Ok(NominalDefinition::Struct(StructDefinition {
                    source,
                    template: self.inference.owner_generic_template(symbol).cloned(),
                    implements: self.import_nominal_heritages(module, definition.implements)?,
                    fields: self.import_field_definitions(module, definition.fields)?,
                    static_fields: self
                        .import_field_definitions(module, definition.static_fields)?,
                    methods: self.import_method_definitions(module, definition.methods)?,
                    static_methods: self
                        .import_method_definitions(module, definition.static_methods)?,
                    associated_types: self
                        .import_associated_type_definitions(module, definition.associated_types)?,
                    associated_consts: self.import_associated_const_definitions(
                        module,
                        definition.associated_consts,
                    )?,
                }))
            }
            dir::NominalDefinition::Class(definition) => {
                Ok(NominalDefinition::Class(ClassDefinition {
                    source,
                    template: self.inference.owner_generic_template(symbol).cloned(),
                    extends: definition
                        .extends
                        .map(|heritage| self.import_nominal_heritage(module, heritage))
                        .transpose()?,
                    implements: self.import_nominal_heritages(module, definition.implements)?,
                    fields: self.import_field_definitions(module, definition.fields)?,
                    static_fields: self
                        .import_field_definitions(module, definition.static_fields)?,
                    methods: self.import_method_definitions(module, definition.methods)?,
                    static_methods: self
                        .import_method_definitions(module, definition.static_methods)?,
                    associated_types: self
                        .import_associated_type_definitions(module, definition.associated_types)?,
                    associated_consts: self.import_associated_const_definitions(
                        module,
                        definition.associated_consts,
                    )?,
                }))
            }
            dir::NominalDefinition::Interface(definition) => {
                Ok(NominalDefinition::Interface(InterfaceDefinition {
                    source,
                    template: self.inference.owner_generic_template(symbol).cloned(),
                    extends: self.import_nominal_heritages(module, definition.extends)?,
                    fields: self.import_field_definitions(module, definition.fields)?,
                    static_fields: self
                        .import_field_definitions(module, definition.static_fields)?,
                    methods: self.import_method_definitions(module, definition.methods)?,
                    static_methods: self
                        .import_method_definitions(module, definition.static_methods)?,
                    call_signatures: self
                        .import_signature_definitions(module, definition.call_signatures)?,
                    construct_signatures: self
                        .import_signature_definitions(module, definition.construct_signatures)?,
                    index_signatures: self
                        .import_signature_definitions(module, definition.index_signatures)?,
                    associated_types: self
                        .import_associated_type_definitions(module, definition.associated_types)?,
                    associated_consts: self.import_associated_const_definitions(
                        module,
                        definition.associated_consts,
                    )?,
                }))
            }
            dir::NominalDefinition::Enum(definition) => {
                Ok(NominalDefinition::Enum(EnumDefinition {
                    source,
                    template: self.inference.owner_generic_template(symbol).cloned(),
                    implements: self.import_nominal_heritages(module, definition.implements)?,
                    variants: self.import_variant_definitions(module, definition.variants)?,
                    static_fields: self
                        .import_field_definitions(module, definition.static_fields)?,
                    methods: self.import_method_definitions(module, definition.methods)?,
                    static_methods: self
                        .import_method_definitions(module, definition.static_methods)?,
                    associated_types: self
                        .import_associated_type_definitions(module, definition.associated_types)?,
                    associated_consts: self.import_associated_const_definitions(
                        module,
                        definition.associated_consts,
                    )?,
                }))
            }
            dir::NominalDefinition::Newtype(definition) => {
                Ok(NominalDefinition::Newtype(NewtypeDefinition {
                    source,
                    template: self.inference.owner_generic_template(symbol).cloned(),
                    value: self.import_type_operand(module, definition.value)?,
                }))
            }
        }
    }

    /// Import nominal heritages from committed nominal metadata.
    fn import_nominal_heritages(
        &mut self,
        module: ModuleId,
        heritages: Vec<dir::NominalHeritage>,
    ) -> CompilerResult<Vec<NominalHeritage>> {
        let mut imported = Vec::with_capacity(heritages.len());

        for heritage in heritages {
            imported.push(self.import_nominal_heritage(module, heritage)?);
        }

        Ok(imported)
    }

    /// Import one nominal heritage from committed nominal metadata.
    fn import_nominal_heritage(
        &mut self,
        module: ModuleId,
        heritage: dir::NominalHeritage,
    ) -> CompilerResult<NominalHeritage> {
        let instance = heritage
            .instance
            .map(|instance| self.import_generic_instance(module, heritage.symbol, instance))
            .transpose()?;

        Ok(NominalHeritage {
            source: heritage.source,
            symbol: heritage.symbol,
            instance,
        })
    }

    /// Import one generic instance.
    fn import_generic_instance(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        instance: dir::LocalGenericInstanceId,
    ) -> CompilerResult<GenericInstance> {
        let instance = self
            .dependency(symbol.module_id)
            .generics
            .get_instance(instance)
            .clone();
        let template = self
            .dependency(symbol.module_id)
            .generics
            .get_template(instance.template);
        let owner = template.owner;
        let mut arguments = Vec::with_capacity(instance.arguments.len());
        for argument in instance.arguments {
            arguments.push(GenericArgument::Static(
                self.import_static_operand(module, argument.value)?,
            ));
        }

        Ok(GenericInstance {
            owner,
            arguments: arguments.into(),
        })
    }

    /// Import field definitions from committed nominal metadata.
    fn import_field_definitions(
        &mut self,
        module: ModuleId,
        fields: Vec<dir::FieldDefinition>,
    ) -> CompilerResult<Vec<FieldDefinition>> {
        let mut imported = Vec::with_capacity(fields.len());

        for field in fields {
            imported.push(FieldDefinition {
                symbol: field.symbol,
                source: field.source,
                key: field.key,
                ty: self.import_type_operand(module, field.ty)?,
            });
        }

        Ok(imported)
    }

    /// Import method definitions from committed nominal metadata.
    fn import_method_definitions(
        &mut self,
        module: ModuleId,
        methods: Vec<dir::MethodDefinition>,
    ) -> CompilerResult<Vec<MethodDefinition>> {
        let mut imported = Vec::with_capacity(methods.len());

        for method in methods {
            imported.push(MethodDefinition {
                symbol: method.symbol,
                source: method.source,
                slot: method.slot,
                ty: self.import_type_operand(module, method.ty)?,
            });
        }

        Ok(imported)
    }

    /// Import associated type definitions from committed nominal metadata.
    fn import_associated_type_definitions(
        &mut self,
        module: ModuleId,
        types: Vec<dir::AssociatedTypeDefinition>,
    ) -> CompilerResult<Vec<AssociatedTypeDefinition>> {
        let mut imported = Vec::with_capacity(types.len());

        for ty in types {
            imported.push(AssociatedTypeDefinition {
                symbol: ty.symbol,
                source: ty.source,
                constraint: ty
                    .constraint
                    .map(|constraint| self.import_type_operand(module, constraint))
                    .transpose()?,
                value: ty
                    .value
                    .map(|value| self.import_type_operand(module, value))
                    .transpose()?,
            });
        }

        Ok(imported)
    }

    /// Import associated const definitions from committed nominal metadata.
    fn import_associated_const_definitions(
        &mut self,
        module: ModuleId,
        consts: Vec<dir::AssociatedConstDefinition>,
    ) -> CompilerResult<Vec<AssociatedConstDefinition>> {
        let mut imported = Vec::with_capacity(consts.len());

        for value in consts {
            imported.push(AssociatedConstDefinition {
                symbol: value.symbol,
                source: value.source,
                ty: self.import_type_operand(module, value.ty)?,
                value: value
                    .value
                    .map(|value| self.import_static_operand(module, value))
                    .transpose()?,
            });
        }

        Ok(imported)
    }

    /// Import enum variant definitions from committed nominal metadata.
    fn import_variant_definitions(
        &mut self,
        module: ModuleId,
        variants: Vec<dir::VariantDefinition>,
    ) -> CompilerResult<Vec<VariantDefinition>> {
        let mut imported = Vec::with_capacity(variants.len());

        for variant in variants {
            imported.push(VariantDefinition {
                symbol: variant.symbol,
                source: variant.source,
                key: variant.key,
                value: variant
                    .value
                    .map(|value| self.import_static_operand(module, value))
                    .transpose()?,
            });
        }

        Ok(imported)
    }

    /// Import signature definitions from committed nominal metadata.
    fn import_signature_definitions(
        &mut self,
        module: ModuleId,
        signatures: Vec<dir::SignatureDefinition>,
    ) -> CompilerResult<Vec<SignatureDefinition>> {
        let mut imported = Vec::with_capacity(signatures.len());

        for signature in signatures {
            imported.push(SignatureDefinition {
                source: signature.source,
                ty: self.import_type_operand(module, signature.ty)?,
            });
        }

        Ok(imported)
    }
}

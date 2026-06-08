use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    AssociatedConstDefinition, AssociatedTypeDefinition, CheckState, ClassDefinition, Definition,
    EnumDefinition, ExtensionDefinition, ExtensionTarget, ExtensionWhereClause, FieldDefinition,
    GenericArgument, GenericInstance, InterfaceDefinition, MethodDefinition, NewtypeDefinition,
    NominalHeritage, SignatureDefinition, StructDefinition, TypeAliasDefinition, TypeOperand,
    VariantDefinition,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return one visible checked definition.
    pub(in crate::check) fn definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<&Definition>> {
        if self.definitions.contains_definition(symbol) {
            return Ok(self.definitions.definition(symbol));
        }
        if self.is_component_module(symbol.module_id) {
            return Ok(None);
        }

        let Some(definition) = self
            .dependency(symbol.module_id)
            .definitions
            .definition(symbol)
            .cloned()
        else {
            return Ok(None);
        };
        let source = self
            .dependency(symbol.module_id)
            .definitions
            .definition_source(symbol)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("dependency definition symbol {symbol:?} has no source"),
            })?;
        let definition = self.import_definition(module, symbol, source, definition)?;

        self.definitions.insert(symbol, definition);

        Ok(self.definitions.definition(symbol))
    }

    /// Return one newtype backing type operand in a component module context.
    pub(in crate::check) fn newtype_backing(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<TypeOperand>> {
        match self.definition(module, symbol)? {
            Some(Definition::Newtype(definition)) => Ok(Some(definition.value)),
            _ => Ok(None),
        }
    }

    /// Return nominal fields as operands in a component module context.
    pub(in crate::check) fn nominal_fields(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<Vec<(dir::StaticKey, TypeOperand)>>> {
        let Some(definition) = self.definition(module, symbol)? else {
            return Ok(None);
        };
        let fields = match definition {
            Definition::Struct(definition) => &definition.fields,
            Definition::Class(definition) => &definition.fields,
            Definition::Interface(definition) => &definition.fields,
            _ => return Ok(None),
        };

        Ok(Some(
            fields.iter().map(|field| (field.key, field.ty)).collect(),
        ))
    }

    /// Return one visible extension definition.
    pub(in crate::check) fn extension_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<ExtensionDefinition>> {
        let Some(definition) = self.definition(module, symbol)? else {
            return Ok(None);
        };
        let extension = match definition {
            Definition::Extension(extension) => Some(extension.clone()),
            _ => None,
        };

        Ok(extension)
    }

    /// Import one checked definition from committed definition metadata.
    fn import_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
        definition: dir::Definition,
    ) -> CompilerResult<Definition> {
        match definition {
            dir::Definition::TypeAlias(definition) => {
                Ok(Definition::TypeAlias(TypeAliasDefinition {
                    source,
                    template: self.import_definition_template(
                        module,
                        symbol,
                        definition.template,
                    )?,
                    value: self.import_type_operand(module, definition.value)?,
                }))
            }
            dir::Definition::Struct(definition) => Ok(Definition::Struct(StructDefinition {
                source,
                template: self.import_definition_template(module, symbol, definition.template)?,
                implements: self.import_nominal_heritages(module, definition.implements)?,
                fields: self.import_field_definitions(module, definition.fields)?,
                static_fields: self.import_field_definitions(module, definition.static_fields)?,
                methods: self.import_method_definitions(module, definition.methods)?,
                static_methods: self
                    .import_method_definitions(module, definition.static_methods)?,
                associated_types: self
                    .import_associated_type_definitions(module, definition.associated_types)?,
                associated_consts: self
                    .import_associated_const_definitions(module, definition.associated_consts)?,
            })),
            dir::Definition::Class(definition) => Ok(Definition::Class(ClassDefinition {
                source,
                template: self.import_definition_template(module, symbol, definition.template)?,
                extends: definition
                    .extends
                    .map(|heritage| self.import_nominal_heritage(module, heritage))
                    .transpose()?,
                implements: self.import_nominal_heritages(module, definition.implements)?,
                fields: self.import_field_definitions(module, definition.fields)?,
                static_fields: self.import_field_definitions(module, definition.static_fields)?,
                methods: self.import_method_definitions(module, definition.methods)?,
                static_methods: self
                    .import_method_definitions(module, definition.static_methods)?,
                associated_types: self
                    .import_associated_type_definitions(module, definition.associated_types)?,
                associated_consts: self
                    .import_associated_const_definitions(module, definition.associated_consts)?,
            })),
            dir::Definition::Interface(definition) => {
                Ok(Definition::Interface(InterfaceDefinition {
                    source,
                    template: self.import_definition_template(
                        module,
                        symbol,
                        definition.template,
                    )?,
                    is_nominal: definition.is_nominal,
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
            dir::Definition::Enum(definition) => Ok(Definition::Enum(EnumDefinition {
                source,
                template: self.import_definition_template(module, symbol, definition.template)?,
                implements: self.import_nominal_heritages(module, definition.implements)?,
                variants: self.import_variant_definitions(module, definition.variants)?,
                static_fields: self.import_field_definitions(module, definition.static_fields)?,
                methods: self.import_method_definitions(module, definition.methods)?,
                static_methods: self
                    .import_method_definitions(module, definition.static_methods)?,
                associated_types: self
                    .import_associated_type_definitions(module, definition.associated_types)?,
                associated_consts: self
                    .import_associated_const_definitions(module, definition.associated_consts)?,
            })),
            dir::Definition::Newtype(definition) => Ok(Definition::Newtype(NewtypeDefinition {
                source,
                template: self.import_definition_template(module, symbol, definition.template)?,
                value: self.import_type_operand(module, definition.value)?,
            })),
            dir::Definition::Extension(extension) => Ok(Definition::Extension(
                self.import_extension_definition(module, source, extension)?,
            )),
        }
    }

    /// Import one extension definition from committed definition metadata.
    fn import_extension_definition(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        extension: dir::Extension,
    ) -> CompilerResult<ExtensionDefinition> {
        let target = self.import_extension_target(module, extension.target)?;
        let mut where_clauses = Vec::with_capacity(extension.where_clauses.len());

        // import extension where clauses
        for where_clause in extension.where_clauses {
            where_clauses.push(ExtensionWhereClause {
                source: where_clause.source,
                left: self.import_type_operand(module, where_clause.left)?,
                right: self.import_type_operand(module, where_clause.right)?,
            });
        }

        Ok(ExtensionDefinition {
            source,
            form: extension.form,
            target,
            implements: self.import_nominal_heritages(module, extension.implements)?,
            where_clauses,
            fields: self.import_field_definitions(module, extension.fields)?,
            static_fields: self.import_field_definitions(module, extension.static_fields)?,
            methods: self.import_method_definitions(module, extension.methods)?,
            static_methods: self.import_method_definitions(module, extension.static_methods)?,
            associated_types: self
                .import_associated_type_definitions(module, extension.associated_types)?,
            associated_consts: self
                .import_associated_const_definitions(module, extension.associated_consts)?,
        })
    }

    /// Import one extension receiver target from committed definition metadata.
    fn import_extension_target(
        &mut self,
        module: ModuleId,
        target: dir::ExtensionTarget,
    ) -> CompilerResult<ExtensionTarget> {
        let target = match target {
            dir::ExtensionTarget::Nominal { root, ty } => ExtensionTarget::Nominal {
                root,
                ty: self.import_type_operand(module, ty)?,
            },
            dir::ExtensionTarget::Blanket { ty } => ExtensionTarget::Blanket {
                ty: self.import_type_operand(module, ty)?,
            },
        };

        Ok(target)
    }

    /// Import one definition template from committed definition metadata.
    fn import_definition_template(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        template: Option<dir::LocalGenericTemplateId>,
    ) -> CompilerResult<Option<dir::GlobalGenericTemplateId>> {
        let Some(template) = template else {
            return Ok(None);
        };
        let template = template.into_global(symbol.module_id);

        self.import_generic_template_id(module, template, Some(symbol))?;

        Ok(Some(template))
    }

    /// Import nominal heritages from committed nominal metadata.
    pub(super) fn import_nominal_heritages(
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
        let template = instance.template.into_global(symbol.module_id);
        let mut arguments = Vec::with_capacity(instance.arguments.len());
        for argument in instance.arguments {
            arguments.push(GenericArgument::Static(
                self.import_static_operand(module, argument.value)?,
            ));
        }

        Ok(GenericInstance::new(template, arguments.into()))
    }

    /// Import field definitions from committed nominal metadata.
    pub(super) fn import_field_definitions(
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
    pub(super) fn import_method_definitions(
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
    pub(super) fn import_associated_type_definitions(
        &mut self,
        module: ModuleId,
        types: Vec<dir::AssociatedTypeDefinition>,
    ) -> CompilerResult<Vec<AssociatedTypeDefinition>> {
        let mut imported = Vec::with_capacity(types.len());

        for ty in types {
            imported.push(AssociatedTypeDefinition {
                symbol: ty.symbol,
                source: ty.source,
                key: ty.key,
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
    pub(super) fn import_associated_const_definitions(
        &mut self,
        module: ModuleId,
        consts: Vec<dir::AssociatedConstDefinition>,
    ) -> CompilerResult<Vec<AssociatedConstDefinition>> {
        let mut imported = Vec::with_capacity(consts.len());

        for value in consts {
            imported.push(AssociatedConstDefinition {
                symbol: value.symbol,
                source: value.source,
                key: value.key,
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

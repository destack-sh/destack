use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    AssociatedConstDefinition, AssociatedTypeDefinition, CheckState, ClassDefinition, Definition,
    EnumDefinition, ExtensionDefinition, ExtensionTarget, ExtensionWhereClause, FieldDefinition,
    GenericArgument, GenericInstance, InterfaceDefinition, MethodDefinition, NewtypeDefinition,
    NominalHeritage, SignatureDefinition, StructDefinition, TypeAliasDefinition, VariantDefinition,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Import external committed definitions.
    pub(in crate::check) fn import_external_definitions(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        let mut symbols = Vec::new();

        // collect external definition symbols before mutating definitions
        for module in modules {
            symbols.extend(self.external_definition_symbols(module));
        }

        // import each external definition once
        for (module, symbol) in symbols {
            self.import_definition_symbol(module, symbol)?;
        }

        Ok(())
    }

    /// Import one committed definition into the checked definition table.
    fn import_definition_symbol(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        if self.definitions.definition(symbol).is_some() {
            return Ok(());
        }
        if self.is_component_module(symbol.module_id) {
            return Ok(());
        }

        let definition = self
            .external_module(symbol.module_id)
            .definitions
            .definition(symbol)
            .cloned();
        let Some(definition) = definition else {
            return Ok(());
        };
        let source = self
            .external_module(symbol.module_id)
            .definitions
            .definition_source(symbol)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("external definition symbol {symbol:?} has no source"),
            })?;
        let definition = self.import_definition(module, symbol, source, &definition)?;

        self.definitions.insert(symbol, definition)
    }

    /// Return definition symbols named by one component module.
    fn external_definition_symbols(
        &self,
        module: ModuleId,
    ) -> Vec<(ModuleId, dir::GlobalSymbolId)> {
        let state = self.module(module);
        let mut symbols = Vec::new();

        // collect explicitly imported definition symbols
        for (_, symbol) in state.resolved.imports.symbol_targets() {
            if self.is_component_module(symbol.module_id) {
                continue;
            }
            if self
                .external_module(symbol.module_id)
                .definitions
                .definition(symbol)
                .is_some()
            {
                symbols.push((module, symbol));
            }
        }

        // collect definitions from external modules
        for external_module in state.external_modules.iter().copied() {
            symbols.extend(
                self.external_module(external_module)
                    .definitions
                    .iter_definitions()
                    .map(|(symbol, _)| (module, symbol)),
            );
        }

        symbols
    }

    /// Import one checked definition from committed definition metadata.
    fn import_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
        definition: &dir::Definition,
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
                implements: self.import_nominal_heritages(module, &definition.implements)?,
                fields: self.import_field_definitions(module, &definition.fields)?,
                static_fields: self.import_field_definitions(module, &definition.static_fields)?,
                methods: self.import_method_definitions(module, &definition.methods)?,
                static_methods: self
                    .import_method_definitions(module, &definition.static_methods)?,
                associated_types: self
                    .import_associated_type_definitions(module, &definition.associated_types)?,
                associated_consts: self
                    .import_associated_const_definitions(module, &definition.associated_consts)?,
            })),
            dir::Definition::Class(definition) => Ok(Definition::Class(ClassDefinition {
                source,
                template: self.import_definition_template(module, symbol, definition.template)?,
                extends: definition
                    .extends
                    .as_ref()
                    .map(|heritage| self.import_nominal_heritage(module, heritage))
                    .transpose()?,
                implements: self.import_nominal_heritages(module, &definition.implements)?,
                fields: self.import_field_definitions(module, &definition.fields)?,
                static_fields: self.import_field_definitions(module, &definition.static_fields)?,
                methods: self.import_method_definitions(module, &definition.methods)?,
                static_methods: self
                    .import_method_definitions(module, &definition.static_methods)?,
                associated_types: self
                    .import_associated_type_definitions(module, &definition.associated_types)?,
                associated_consts: self
                    .import_associated_const_definitions(module, &definition.associated_consts)?,
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
                    extends: self.import_nominal_heritages(module, &definition.extends)?,
                    fields: self.import_field_definitions(module, &definition.fields)?,
                    static_fields: self
                        .import_field_definitions(module, &definition.static_fields)?,
                    methods: self.import_method_definitions(module, &definition.methods)?,
                    static_methods: self
                        .import_method_definitions(module, &definition.static_methods)?,
                    call_signatures: self
                        .import_signature_definitions(module, &definition.call_signatures)?,
                    construct_signatures: self
                        .import_signature_definitions(module, &definition.construct_signatures)?,
                    index_signatures: self
                        .import_signature_definitions(module, &definition.index_signatures)?,
                    associated_types: self
                        .import_associated_type_definitions(module, &definition.associated_types)?,
                    associated_consts: self.import_associated_const_definitions(
                        module,
                        &definition.associated_consts,
                    )?,
                }))
            }
            dir::Definition::Enum(definition) => Ok(Definition::Enum(EnumDefinition {
                source,
                template: self.import_definition_template(module, symbol, definition.template)?,
                implements: self.import_nominal_heritages(module, &definition.implements)?,
                variants: self.import_variant_definitions(module, &definition.variants)?,
                static_fields: self.import_field_definitions(module, &definition.static_fields)?,
                methods: self.import_method_definitions(module, &definition.methods)?,
                static_methods: self
                    .import_method_definitions(module, &definition.static_methods)?,
                associated_types: self
                    .import_associated_type_definitions(module, &definition.associated_types)?,
                associated_consts: self
                    .import_associated_const_definitions(module, &definition.associated_consts)?,
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
        extension: &dir::Extension,
    ) -> CompilerResult<ExtensionDefinition> {
        let target = self.import_extension_target(module, &extension.target)?;
        let mut where_clauses = Vec::with_capacity(extension.where_clauses.len());

        // import extension where clauses
        for where_clause in &extension.where_clauses {
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
            implements: self.import_nominal_heritages(module, &extension.implements)?,
            where_clauses,
            fields: self.import_field_definitions(module, &extension.fields)?,
            static_fields: self.import_field_definitions(module, &extension.static_fields)?,
            methods: self.import_method_definitions(module, &extension.methods)?,
            static_methods: self.import_method_definitions(module, &extension.static_methods)?,
            associated_types: self
                .import_associated_type_definitions(module, &extension.associated_types)?,
            associated_consts: self
                .import_associated_const_definitions(module, &extension.associated_consts)?,
        })
    }

    /// Import one extension receiver target from committed definition metadata.
    fn import_extension_target(
        &mut self,
        module: ModuleId,
        target: &dir::ExtensionTarget,
    ) -> CompilerResult<ExtensionTarget> {
        let target = match target {
            dir::ExtensionTarget::Nominal { root, ty } => ExtensionTarget::Nominal {
                root: *root,
                ty: self.import_type_operand(module, *ty)?,
            },
            dir::ExtensionTarget::Blanket { ty } => ExtensionTarget::Blanket {
                ty: self.import_type_operand(module, *ty)?,
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
        heritages: &[dir::NominalHeritage],
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
        heritage: &dir::NominalHeritage,
    ) -> CompilerResult<NominalHeritage> {
        let instance = heritage
            .instance
            .map(|instance| {
                self.import_generic_instance(module, heritage.source, heritage.symbol, instance)
            })
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
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        instance: dir::LocalGenericInstanceId,
    ) -> CompilerResult<GenericInstance> {
        let external = self.external_module(source.module_id);
        let instance = external
            .generics
            .get_instance_maybe(instance)
            .cloned()
            .ok_or_else(|| {
                let symbol_label = self.dump_in_module(module, &symbol);
                let module_label = self.dump_in_module(module, &module);
                let source_label = self.dump_in_module(module, &source);
                let instances = external
                    .generics
                    .iter_instances()
                    .map(|(instance_id, _)| format!("{instance_id:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let source_instances = external
                    .generics
                    .node_instance_ids(source)
                    .map(|instance_id| format!("{instance_id:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");

                CompilerError::Internal {
                    message: format!(
                        "definition heritage {source_label} for {symbol_label} imported by {module_label} references missing generic instance {instance:?}, source instances=[{source_instances}], available instances=[{instances}]"
                    ),
                }
            })?;
        let template = instance.template;
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
        fields: &[dir::FieldDefinition],
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
        methods: &[dir::MethodDefinition],
    ) -> CompilerResult<Vec<MethodDefinition>> {
        let mut imported = Vec::with_capacity(methods.len());

        for method in methods {
            imported.push(MethodDefinition {
                symbol: method.symbol,
                source: method.source,
                slot: method.slot,
                role: method.role,
                ty: self.import_type_operand(module, method.ty)?,
            });
        }

        Ok(imported)
    }

    /// Import associated type definitions from committed nominal metadata.
    pub(super) fn import_associated_type_definitions(
        &mut self,
        module: ModuleId,
        types: &[dir::AssociatedTypeDefinition],
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
        consts: &[dir::AssociatedConstDefinition],
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
        variants: &[dir::VariantDefinition],
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
        signatures: &[dir::SignatureDefinition],
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

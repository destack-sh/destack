use std::collections::BTreeMap;

use crate::analyze::{
    BindingCatalog, BindingField, BindingTaggedUnionVariant, BindingType, ConstantCatalog,
};

/// Named ABI types grouped by platform module.
#[derive(Debug, Default, Clone)]
pub(crate) struct ModuleAbiTypes {
    /// Newtype definitions keyed by name.
    pub newtypes: BTreeMap<String, BindingType>,
    /// Struct definitions keyed by name.
    pub structs: BTreeMap<String, BindingType>,
    /// Enum definitions keyed by name.
    pub enums: BTreeMap<String, BindingType>,
    /// Tagged union definitions keyed by name.
    pub tagged_unions: BTreeMap<String, BindingType>,
}

impl ModuleAbiTypes {
    /// Collect ABI types grouped by their owning module.
    pub(crate) fn collect_by_module(
        catalog: &BindingCatalog,
        constants: &ConstantCatalog,
        exported_types: &[BindingType],
    ) -> BTreeMap<String, Self> {
        let mut modules: BTreeMap<String, Self> = BTreeMap::new();

        // binding types
        for bindings in catalog.values() {
            for entry in bindings.values() {
                collect_binding_type(&entry.return_binding, &mut modules);
                for parameter in &entry.parameters {
                    collect_binding_type(&parameter.binding_type, &mut modules);
                }
            }
        }

        // constant types
        for (module_name, module_constants) in constants {
            for constant in module_constants.values() {
                collect_binding_type(&constant.binding_type, &mut modules);
            }

            modules.entry(module_name.clone()).or_default();
        }

        // exported type declarations
        for binding_type in exported_types {
            collect_binding_type(binding_type, &mut modules);
        }

        modules
    }

    /// Collect one binding type into this module ABI set.
    pub(crate) fn collect_binding_type(&mut self, binding_type: &BindingType) {
        match binding_type {
            BindingType::Newtype { name, inner, .. } => {
                self.newtypes
                    .entry(name.clone())
                    .or_insert(binding_type.clone());
                self.collect_binding_type(inner);
            }
            BindingType::Struct { name, fields, .. } => {
                self.structs
                    .entry(name.clone())
                    .or_insert(binding_type.clone());

                // collect field types
                for field in fields {
                    self.collect_field(field);
                }
            }
            BindingType::Enum { name, .. } => {
                self.enums
                    .entry(name.clone())
                    .or_insert(binding_type.clone());
            }
            BindingType::TaggedUnion { name, variants, .. } => {
                self.tagged_unions
                    .entry(name.clone())
                    .or_insert(binding_type.clone());

                // collect variant types
                for variant in variants {
                    self.collect_variant(variant);
                }
            }
            BindingType::Slice(inner)
            | BindingType::Array(inner)
            | BindingType::Optional(inner) => {
                self.collect_binding_type(inner);
            }
            _ => {}
        }
    }

    /// Collect one struct field into this module ABI set.
    fn collect_field(&mut self, field: &BindingField) {
        self.collect_binding_type(&field.binding_type);
    }

    /// Collect one tagged-union variant into this module ABI set.
    fn collect_variant(&mut self, variant: &BindingTaggedUnionVariant) {
        self.collect_binding_type(&variant.binding_type);
    }
}

/// Walk one binding type and collect named types into the owning module map.
fn collect_binding_type(
    binding_type: &BindingType,
    modules: &mut BTreeMap<String, ModuleAbiTypes>,
) {
    match binding_type {
        BindingType::Newtype { domain, .. }
        | BindingType::Struct { domain, .. }
        | BindingType::Enum { domain, .. }
        | BindingType::TaggedUnion { domain, .. } => {
            let module = modules.entry(domain.clone()).or_default();
            module.collect_binding_type(binding_type);
        }
        BindingType::Slice(inner) | BindingType::Array(inner) | BindingType::Optional(inner) => {
            collect_binding_type(inner, modules);
        }
        _ => {}
    }
}

use std::collections::{BTreeMap, BTreeSet};

use super::codegen::ModuleCodegen;
use super::usage::{RenderTypes, RenderUsage};
use crate::platform::model::BindingEntry;

/// One binding module catalog keyed by extern name.
pub(super) type ModuleBindings = BTreeMap<String, BindingEntry>;

/// Canonical binding specification for one platform module.
#[derive(Debug, Clone)]
pub(crate) struct RenderSpec<'a> {
    /// Platform module name.
    pub(super) module: &'a str,
    /// Binding definitions for the module.
    pub(super) bindings: &'a ModuleBindings,
    /// Canonical binding descriptor constants.
    pub(super) consts: Vec<BindingSymbol<'a>>,
    /// Render usage flags for helper generation.
    pub(super) usage: RenderUsage,
    /// Rendered type usage for import decisions.
    pub(super) types: RenderTypes,
}

impl<'a> RenderSpec<'a> {
    /// Build a module specification from binding metadata.
    pub(crate) fn new(module: &'a str, bindings: &'a ModuleBindings) -> Self {
        let codegen = ModuleCodegen::new(module);
        let usage = RenderUsage::new(bindings);
        let types = RenderTypes::new(module, bindings);
        let consts = BindingSymbol::collect(&codegen, bindings);

        Self {
            module,
            bindings,
            consts,
            usage,
            types,
        }
    }

    /// Return the Rust code generator for this module.
    pub(super) fn codegen(&self) -> ModuleCodegen<'a> {
        ModuleCodegen::new(self.module)
    }
}

/// Canonical binding descriptor information for rendering.
#[derive(Debug, Clone)]
pub(super) struct BindingSymbol<'a> {
    /// Constant name for the binding descriptor.
    pub(super) const_name: String,
    /// Fully qualified extern binding name.
    pub(super) extern_name: &'a str,
    /// Runtime implementation function name.
    pub(super) implementation_fn_name: String,
    /// Binding metadata payload.
    pub(super) entry: &'a BindingEntry,
}

impl BindingSymbol<'_> {
    /// Collect canonical binding constants for one module.
    pub(super) fn collect<'a>(
        codegen: &ModuleCodegen<'a>,
        bindings: &'a ModuleBindings,
    ) -> Vec<BindingSymbol<'a>> {
        build_binding_symbols(codegen, bindings)
    }
}

/// Build a deterministic list of binding constants for one module.
pub(super) fn build_binding_symbols<'a>(
    codegen: &ModuleCodegen<'a>,
    bindings: &'a ModuleBindings,
) -> Vec<BindingSymbol<'a>> {
    let mut used_const_names = BTreeSet::new();
    let mut consts = Vec::new();

    for (extern_name, entry) in bindings {
        let mut const_name = codegen.const_name_for_extern(extern_name);
        if used_const_names.contains(&const_name) {
            let mut index = 2;
            let base = const_name.clone();
            while used_const_names.contains(&const_name) {
                const_name = format!("{base}_{index}");
                index += 1;
            }
        }

        used_const_names.insert(const_name.clone());
        consts.push(BindingSymbol {
            const_name,
            extern_name,
            implementation_fn_name: codegen.implementation_fn_name(&entry.implementation_name),
            entry,
        });
    }

    consts
}

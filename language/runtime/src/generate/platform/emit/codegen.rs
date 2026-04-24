use destack_dir::{EnumBackingType, IntType};

use super::binding_type_requires_abi;
use crate::platform::model::{
    BindingEntry, BindingParameter, BindingTaggedUnionVariant, BindingType, CatalogBindingAffinity,
    CatalogBindingBlocking, CatalogBindingReplayKind, CatalogBindingScope,
    CatalogBindingSimulation, CatalogEntropyKind,
};

/// Module-scoped Rust code generation helpers.
#[derive(Clone, Copy)]
pub(super) struct ModuleCodegen<'a> {
    /// The current platform module name.
    module: &'a str,
}

impl<'a> ModuleCodegen<'a> {
    /// Convert one module name into one Rust-safe identifier.
    pub(super) fn sanitize_module_name(name: &str) -> String {
        let mut out = String::new();

        // lowercase and replace separators
        for ch in name.chars() {
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push('_');
            }
        }

        // preserve one non-empty identifier
        if out.is_empty() {
            out.push_str("bindings");
        }

        // avoid one leading digit
        if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
            out.insert(0, '_');
        }

        out
    }

    /// Create one Rust code generator for a binding module.
    pub(super) fn new(module: &'a str) -> Self {
        Self { module }
    }

    /// Return the current platform module name.
    pub(super) fn module(&self) -> &'a str {
        self.module
    }

    /// Build the native symbol name for a binding.
    pub(super) fn native_fn_name(&self, extern_name: &str) -> String {
        let suffix = self.binding_suffix_for_extern(extern_name, Some(self.module));
        format!("destack_{}_{suffix}", self.module)
    }

    /// Build the native replay helper name for a binding.
    pub(super) fn native_replay_fn_name(&self, extern_name: &str) -> String {
        format!("{}_replay", self.native_fn_name(extern_name))
    }

    /// Build the VM binding registration function name for this module.
    pub(super) fn register_fn_name(&self) -> String {
        format!(
            "register_{}_vm_bindings",
            Self::sanitize_module_name(self.module)
        )
    }

    /// Build the VM binding install function name for this module.
    pub(super) fn install_fn_name(&self) -> String {
        format!(
            "install_{}_vm_bindings",
            Self::sanitize_module_name(self.module)
        )
    }

    /// Build the VM aggregate registration helper name for this module.
    pub(super) fn register_aggregate_types_fn_name(&self) -> String {
        format!(
            "register_{}_vm_aggregate_types",
            Self::sanitize_module_name(self.module)
        )
    }

    /// Build the native binding set constant name for this module.
    pub(super) fn native_set_name(&self) -> String {
        format!("{}_NATIVE_BINDINGS", self.const_name(self.module))
    }

    /// Build the VM binding set constant name for this module.
    pub(super) fn vm_set_name(&self) -> String {
        format!("{}_VM_BINDINGS", self.const_name(self.module))
    }

    /// Build the VM handler name for a binding.
    pub(super) fn vm_fn_name(&self, extern_name: &str) -> String {
        self.native_fn_name(extern_name)
    }

    /// Build the VM replay helper name for a binding.
    pub(super) fn vm_replay_fn_name(&self, extern_name: &str) -> String {
        format!("{}_vm_replay", self.native_fn_name(extern_name))
    }

    /// Build the decode helper name for a binding.
    pub(super) fn decode_helper_name(&self, base: &str) -> String {
        format!("decode_{base}_args")
    }

    /// Build the encode helper name for a binding.
    pub(super) fn encode_helper_name(&self, base: &str) -> String {
        format!("encode_{base}_result")
    }

    /// Render the parameter list for a binding entry.
    pub(super) fn render_params(&self, entry: &BindingEntry) -> Vec<String> {
        entry
            .parameters
            .iter()
            .enumerate()
            .map(|(index, param)| {
                let rust_type = self.vm_type_for_binding(&param.binding_type);
                let name = self.sanitize_param_name(&param.name, index);
                format!("{name}: {rust_type}")
            })
            .collect()
    }

    /// Render the return type for a binding entry.
    pub(super) fn render_return_type(&self, entry: &BindingEntry) -> String {
        self.vm_type_for_binding(&entry.return_binding)
    }

    /// Build a qualified path for a named binding type.
    pub(super) fn named_type_path(&self, type_domain: &str, name: &str) -> String {
        if type_domain == self.module {
            name.to_string()
        } else {
            format!("{type_domain}::{name}")
        }
    }

    /// Build the canonical metadata name for one named binding type.
    pub(super) fn named_type_metadata_name(&self, type_domain: &str, name: &str) -> String {
        format!("{type_domain}::{name}")
    }

    /// Build one value type name for a named binding type.
    pub(super) fn value_type_name(&self, name: &str) -> String {
        format!("{name}Value")
    }

    /// Build a qualified path for one named value type.
    pub(super) fn value_type_path(&self, type_domain: &str, name: &str) -> String {
        let value_name = self.value_type_name(name);

        if type_domain == self.module {
            value_name
        } else {
            let alias = self.platform_domain_alias(type_domain);
            format!("{alias}::abi_generated::{value_name}")
        }
    }

    /// Build a stable module alias for a platform domain.
    pub(super) fn platform_domain_alias(&self, domain: &str) -> String {
        let module_name = Self::sanitize_module_name(domain);
        format!("platform_{module_name}")
    }

    /// Render one escaped Rust string literal payload.
    pub(super) fn escape_rust_string(&self, text: &str) -> String {
        let mut out = String::new();
        for ch in text.chars() {
            for escaped in ch.escape_default() {
                out.push(escaped);
            }
        }

        out
    }

    /// Render one binding scope enum expression.
    pub(super) fn render_binding_scope(&self, scope: CatalogBindingScope) -> String {
        match scope {
            CatalogBindingScope::Host => "BindingScope::Host".to_string(),
            CatalogBindingScope::Runtime => "BindingScope::Runtime".to_string(),
        }
    }

    /// Render one binding blocking enum expression.
    pub(super) fn render_binding_blocking(&self, blocking: CatalogBindingBlocking) -> String {
        match blocking {
            CatalogBindingBlocking::Never => "BindingBlocking::Never".to_string(),
            CatalogBindingBlocking::Sometimes => "BindingBlocking::Sometimes".to_string(),
            CatalogBindingBlocking::Always => "BindingBlocking::Always".to_string(),
        }
    }

    /// Render one binding affinity enum expression.
    pub(super) fn render_binding_affinity(&self, affinity: CatalogBindingAffinity) -> String {
        match affinity {
            CatalogBindingAffinity::Any => "BindingAffinity::Any".to_string(),
            CatalogBindingAffinity::EventLoop => "BindingAffinity::EventLoop".to_string(),
            CatalogBindingAffinity::Owner => "BindingAffinity::Owner".to_string(),
            CatalogBindingAffinity::ProcessMain => "BindingAffinity::ProcessMain".to_string(),
        }
    }

    /// Render one entropy replay kind value.
    pub(super) fn render_entropy_kind(&self, kind: CatalogEntropyKind) -> &'static str {
        match kind {
            CatalogEntropyKind::TimeReadMonotonic => "EntropyKind::TimeReadMonotonic",
            CatalogEntropyKind::TimeReadWall => "EntropyKind::TimeReadWall",
            CatalogEntropyKind::RandomStreamCreate => "EntropyKind::RandomStreamCreate",
            CatalogEntropyKind::RandomReadBytes => "EntropyKind::RandomReadBytes",
            CatalogEntropyKind::RandomReadU64 => "EntropyKind::RandomReadU64",
        }
    }

    /// Render one binding replay kind value.
    pub(super) fn render_binding_replay_kind(&self, kind: CatalogBindingReplayKind) -> String {
        match kind {
            CatalogBindingReplayKind::BindingCall => "BindingReplayKind::BindingCall".to_string(),
            CatalogBindingReplayKind::Entropy(entropy_kind) => format!(
                "BindingReplayKind::Entropy({})",
                self.render_entropy_kind(entropy_kind)
            ),
        }
    }

    /// Render one native implementation call with world dispatch.
    pub(super) fn render_native_world_dispatch(
        &self,
        binding_const: &str,
        scope: CatalogBindingScope,
        simulation: CatalogBindingSimulation,
        implementation_fn_name: &str,
        args: &[String],
    ) -> String {
        let runtime_call = if args.is_empty() {
            format!("unsafe {{ platform_runtime_native::{implementation_fn_name}(context) }}")
        } else {
            format!(
                "unsafe {{ platform_runtime_native::{implementation_fn_name}(context, {}) }}",
                args.join(", ")
            )
        };

        let host_call = if args.is_empty() {
            format!("unsafe {{ platform_native::{implementation_fn_name}(context) }}")
        } else {
            format!(
                "unsafe {{ platform_native::{implementation_fn_name}(context, {}) }}",
                args.join(", ")
            )
        };

        // runtime scope
        if scope == CatalogBindingScope::Runtime {
            return format!(
                "{{\n            let _binding_hook_guard = context.on_before_binding({binding_const})?;\n            {runtime_call}\n        }}"
            );
        }

        // simulation fallback
        let simulation_call = match simulation {
            CatalogBindingSimulation::Unsupported => format!(
                "Err(RuntimeError::from(PlatformError::not_supported({binding_const}.name)).boxed())"
            ),
            CatalogBindingSimulation::Stub | CatalogBindingSimulation::Model => {
                if args.is_empty() {
                    format!(
                        "unsafe {{ platform_simulation_native::{implementation_fn_name}(context) }}"
                    )
                } else {
                    format!(
                        "unsafe {{ platform_simulation_native::{implementation_fn_name}(context, {}) }}",
                        args.join(", ")
                    )
                }
            }
        };

        format!(
            "{{\n            let (world, _binding_hook_guard) = context.on_before_binding_resolve_world({binding_const})?;\n            match world {{\n                RuntimeWorld::Host => {host_call},\n                RuntimeWorld::Simulation => {simulation_call},\n            }}\n        }}"
        )
    }

    /// Write documentation for one generated struct field.
    pub(super) fn write_field_docs(
        &self,
        output: &mut String,
        documentation: Option<&str>,
        field_name: &str,
    ) {
        if let Some(documentation) = documentation {
            let mut wrote_line = false;

            // copy each documentation line
            for line in documentation.lines() {
                if line.trim().is_empty() {
                    output.push_str("    ///\n");
                    continue;
                }

                output.push_str(&format!("    /// {}\n", line.trim_end()));
                wrote_line = true;
            }

            if wrote_line {
                return;
            }
        }

        output.push_str(&format!("    /// The {field_name} field.\n"));
    }

    /// Compute deterministic variant tags for one tagged union.
    pub(super) fn tagged_union_variant_tags(
        &self,
        union_name: &str,
        variants: &[BindingTaggedUnionVariant],
    ) -> Vec<u32> {
        let mut tags = Vec::with_capacity(variants.len());
        let mut seen = std::collections::BTreeMap::<u32, String>::new();

        // assign stable tags
        for variant in variants {
            let tag = Self::tagged_union_variant_tag(union_name, variant.name.as_str());

            if let Some(existing) = seen.insert(tag, variant.name.clone()) {
                panic!(
                    "tagged union {union_name} has colliding stable tags for variants {existing} and {}",
                    variant.name
                );
            }

            tags.push(tag);
        }

        tags
    }

    /// Render one VM implementation call with world dispatch.
    pub(super) fn render_vm_world_dispatch(
        &self,
        binding_const: &str,
        scope: CatalogBindingScope,
        simulation: CatalogBindingSimulation,
        implementation_fn_name: &str,
        invoke_args: &str,
    ) -> String {
        let runtime_call =
            format!("platform_runtime_vm::{implementation_fn_name}(binding, context{invoke_args})");
        let host_call =
            format!("platform_vm::{implementation_fn_name}(binding, context{invoke_args})");

        // runtime scope
        if scope == CatalogBindingScope::Runtime {
            return format!(
                "{{\n                        let _binding_hook_guard = binding.on_before_binding({binding_const})?;\n                        {runtime_call}\n                    }}"
            );
        }

        // simulation fallback
        let simulation_call = match simulation {
            CatalogBindingSimulation::Unsupported => format!(
                "Err(RuntimeError::from(PlatformError::not_supported({binding_const}.name)).boxed())"
            ),
            CatalogBindingSimulation::Stub | CatalogBindingSimulation::Model => {
                format!(
                    "platform_simulation_vm::{implementation_fn_name}(binding, context{invoke_args})"
                )
            }
        };

        format!(
            "{{\n                        let (world, _binding_hook_guard) = binding.on_before_binding_resolve_world({binding_const})?;\n                        match world {{\n                            RuntimeWorld::Host => {host_call},\n                            RuntimeWorld::Simulation => {simulation_call},\n                        }}\n                    }}"
        )
    }

    /// Render one binding requires array expression.
    pub(super) fn render_binding_requires(&self, requires: &[String]) -> String {
        if requires.is_empty() {
            return "&[]".to_string();
        }

        let mut values = String::from("&[");
        for (index, capability) in requires.iter().enumerate() {
            if index > 0 {
                values.push_str(", ");
            }
            values.push('"');
            values.push_str(&self.escape_rust_string(capability));
            values.push('"');
        }
        values.push(']');

        values
    }

    /// Render one optional host platform array expression.
    pub(super) fn render_binding_host_platforms(
        &self,
        host_platforms: &[String],
    ) -> Option<String> {
        if host_platforms.is_empty() {
            return None;
        }

        let mut values = String::from("&[");
        for (index, platform) in host_platforms.iter().enumerate() {
            if index > 0 {
                values.push_str(", ");
            }
            values.push('"');
            values.push_str(&self.escape_rust_string(platform));
            values.push('"');
        }
        values.push(']');

        Some(values)
    }

    /// Build a qualified ABI struct path for a named binding type.
    pub(super) fn struct_abi_path(&self, type_domain: &str, name: &str, abi: &str) -> String {
        let alias = if type_domain == self.module {
            self.platform_domain_alias(self.module)
        } else {
            self.platform_domain_alias(type_domain)
        };

        format!("{alias}::{name}Abi<{abi}>")
    }

    /// Build a qualified ABI newtype path for a named binding type.
    pub(super) fn newtype_abi_path(&self, type_domain: &str, name: &str, abi: &str) -> String {
        let alias = if type_domain == self.module {
            self.platform_domain_alias(self.module)
        } else {
            self.platform_domain_alias(type_domain)
        };

        format!("{alias}::{name}Abi<{abi}>")
    }

    /// Build a qualified ABI tagged union path for a named binding type.
    pub(super) fn tagged_union_abi_path(&self, type_domain: &str, name: &str, abi: &str) -> String {
        let alias = if type_domain == self.module {
            self.platform_domain_alias(self.module)
        } else {
            self.platform_domain_alias(type_domain)
        };

        format!("{alias}::{name}Abi<{abi}>")
    }

    /// Build a qualified ABI newtype constructor path.
    pub(super) fn newtype_abi_constructor_path(
        &self,
        type_domain: &str,
        name: &str,
        abi: &str,
    ) -> String {
        let alias = if type_domain == self.module {
            self.platform_domain_alias(self.module)
        } else {
            self.platform_domain_alias(type_domain)
        };

        format!("{alias}::{name}Abi::<{abi}>")
    }

    /// Build a qualified VM alias path for a struct type.
    pub(super) fn struct_vm_path(&self, type_domain: &str, name: &str) -> String {
        let vm_name = format!("{name}Vm");
        self.named_type_path(type_domain, vm_name.as_str())
    }

    /// Build a qualified VM alias path for a newtype.
    pub(super) fn newtype_vm_path(&self, type_domain: &str, name: &str) -> String {
        let vm_name = format!("{name}Vm");
        self.named_type_path(type_domain, vm_name.as_str())
    }

    /// Build a qualified VM alias path for a tagged union.
    pub(super) fn tagged_union_vm_path(&self, type_domain: &str, name: &str) -> String {
        let vm_name = format!("{name}Vm");
        self.named_type_path(type_domain, vm_name.as_str())
    }

    /// Convert a binding type into a native ABI type.
    pub(super) fn native_type_for_binding(&self, binding_type: &BindingType) -> String {
        match binding_type {
            BindingType::Void => "()".to_string(),
            BindingType::Bool => "bool".to_string(),
            BindingType::Int(8) => "i8".to_string(),
            BindingType::Int(16) => "i16".to_string(),
            BindingType::Int(32) => "i32".to_string(),
            BindingType::Int(64) => "i64".to_string(),
            BindingType::UInt(8) => "u8".to_string(),
            BindingType::UInt(16) => "u16".to_string(),
            BindingType::UInt(32) => "u32".to_string(),
            BindingType::UInt(64) => "u64".to_string(),
            BindingType::Float(32) => "f32".to_string(),
            BindingType::Float(64) => "f64".to_string(),
            BindingType::String => "NativeStringRef".to_string(),
            BindingType::StringSlice => "NativeStringSlice".to_string(),
            BindingType::Slice(inner) => {
                format!("NativeSlice<{}>", self.native_type_for_binding(inner))
            }
            BindingType::Array(inner) => {
                format!("NativeArray<{}>", self.native_type_for_binding(inner))
            }
            BindingType::Optional(inner) => {
                format!("Option<{}>", self.native_type_for_binding(inner))
            }
            BindingType::Newtype {
                name,
                domain: type_domain,
                ..
            }
            | BindingType::Struct {
                name,
                domain: type_domain,
                ..
            }
            | BindingType::Enum {
                name,
                domain: type_domain,
                ..
            }
            | BindingType::TaggedUnion {
                name,
                domain: type_domain,
                ..
            } => {
                if self.module == "error" && type_domain == "error" && name == "PlatformError" {
                    "platform_error::PlatformError".to_string()
                } else {
                    self.named_type_path(type_domain, name)
                }
            }
            BindingType::Int(_) | BindingType::UInt(_) | BindingType::Float(_) => {
                panic!("unsupported numeric width for native bindings")
            }
        }
    }

    /// Convert a binding type into one owned Rust binding type.
    pub(super) fn owned_type_for_binding(&self, binding_type: &BindingType) -> String {
        match binding_type {
            BindingType::Void => "()".to_string(),
            BindingType::Bool => "bool".to_string(),
            BindingType::Int(8) => "i8".to_string(),
            BindingType::Int(16) => "i16".to_string(),
            BindingType::Int(32) => "i32".to_string(),
            BindingType::Int(64) => "i64".to_string(),
            BindingType::UInt(8) => "u8".to_string(),
            BindingType::UInt(16) => "u16".to_string(),
            BindingType::UInt(32) => "u32".to_string(),
            BindingType::UInt(64) => "u64".to_string(),
            BindingType::Float(32) => "f32".to_string(),
            BindingType::Float(64) => "f64".to_string(),
            BindingType::String => "String".to_string(),
            BindingType::StringSlice => "Vec<String>".to_string(),
            BindingType::Slice(inner) | BindingType::Array(inner) => {
                let inner = self.owned_type_for_binding(inner);
                format!("Vec<{inner}>")
            }
            BindingType::Optional(inner) => {
                let inner = self.owned_type_for_binding(inner);
                format!("Option<{inner}>")
            }
            BindingType::Newtype {
                name,
                domain: type_domain,
                ..
            }
            | BindingType::Struct {
                name,
                domain: type_domain,
                ..
            }
            | BindingType::Enum {
                name,
                domain: type_domain,
                ..
            }
            | BindingType::TaggedUnion {
                name,
                domain: type_domain,
                ..
            } => {
                if binding_type_requires_abi(binding_type) {
                    self.value_type_path(type_domain, name)
                } else {
                    self.named_type_path(type_domain, name)
                }
            }
            BindingType::Int(_) | BindingType::UInt(_) | BindingType::Float(_) => {
                panic!("unsupported numeric width for owned bindings")
            }
        }
    }

    /// Render an ABI field type for a binding type.
    pub(super) fn abi_struct_field_type(&self, binding_type: &BindingType) -> String {
        match binding_type {
            BindingType::Void => "()".to_string(),
            BindingType::Bool => "bool".to_string(),
            BindingType::Int(8) => "i8".to_string(),
            BindingType::Int(16) => "i16".to_string(),
            BindingType::Int(32) => "i32".to_string(),
            BindingType::Int(64) => "i64".to_string(),
            BindingType::Int(width) => panic!("unsupported int width for ABI type: {width}"),
            BindingType::UInt(8) => "u8".to_string(),
            BindingType::UInt(16) => "u16".to_string(),
            BindingType::UInt(32) => "u32".to_string(),
            BindingType::UInt(64) => "u64".to_string(),
            BindingType::UInt(width) => panic!("unsupported uint width for ABI type: {width}"),
            BindingType::Float(32) => "f32".to_string(),
            BindingType::Float(64) => "f64".to_string(),
            BindingType::Float(width) => panic!("unsupported float width for ABI type: {width}"),
            BindingType::String => "A::String".to_string(),
            BindingType::StringSlice => "A::StringSlice".to_string(),
            BindingType::Slice(inner) => {
                format!("A::Slice<{}>", self.abi_struct_field_type(inner))
            }
            BindingType::Array(inner) => {
                format!("A::Array<{}>", self.abi_struct_field_type(inner))
            }
            BindingType::Optional(inner) => {
                format!("Option<{}>", self.abi_struct_field_type(inner))
            }
            BindingType::Newtype {
                name,
                domain: type_domain,
                inner,
            } => {
                if binding_type_requires_abi(inner) {
                    self.newtype_abi_path(type_domain, name, "A")
                } else {
                    self.named_type_path(type_domain, name)
                }
            }
            BindingType::Struct {
                name,
                domain: type_domain,
                ..
            } => {
                if binding_type_requires_abi(binding_type) {
                    self.struct_abi_path(type_domain, name, "A")
                } else {
                    self.named_type_path(type_domain, name)
                }
            }
            BindingType::Enum {
                name,
                domain: type_domain,
                ..
            } => self.named_type_path(type_domain, name),
            BindingType::TaggedUnion {
                name,
                domain: type_domain,
                ..
            } => {
                if binding_type_requires_abi(binding_type) {
                    self.tagged_union_abi_path(type_domain, name, "A")
                } else {
                    self.named_type_path(type_domain, name)
                }
            }
        }
    }

    /// Render a newtype inner ABI type.
    pub(super) fn abi_newtype_inner_type(&self, binding_type: &BindingType) -> String {
        match binding_type {
            BindingType::Bool => "bool".to_string(),
            BindingType::Int(8) => "i8".to_string(),
            BindingType::Int(16) => "i16".to_string(),
            BindingType::Int(32) => "i32".to_string(),
            BindingType::Int(64) => "i64".to_string(),
            BindingType::UInt(8) => "u8".to_string(),
            BindingType::UInt(16) => "u16".to_string(),
            BindingType::UInt(32) => "u32".to_string(),
            BindingType::UInt(64) => "u64".to_string(),
            BindingType::Float(32) => "f32".to_string(),
            BindingType::Float(64) => "f64".to_string(),
            BindingType::Optional(inner) => {
                format!("Option<{}>", self.abi_newtype_inner_type(inner))
            }
            BindingType::Newtype {
                name,
                domain: type_domain,
                inner,
            } => {
                if binding_type_requires_abi(inner) {
                    self.newtype_vm_path(type_domain, name)
                } else {
                    self.named_type_path(type_domain, name)
                }
            }
            BindingType::Enum {
                name,
                domain: type_domain,
                ..
            } => self.named_type_path(type_domain, name),
            BindingType::Struct {
                name,
                domain: type_domain,
                ..
            } => {
                if binding_type_requires_abi(binding_type) {
                    self.struct_abi_path(type_domain, name, "A")
                } else {
                    self.named_type_path(type_domain, name)
                }
            }
            BindingType::TaggedUnion {
                name,
                domain: type_domain,
                ..
            } => {
                if binding_type_requires_abi(binding_type) {
                    self.tagged_union_abi_path(type_domain, name, "A")
                } else {
                    self.named_type_path(type_domain, name)
                }
            }
            BindingType::Void
            | BindingType::String
            | BindingType::StringSlice
            | BindingType::Slice(_)
            | BindingType::Array(_)
            | BindingType::Int(_)
            | BindingType::UInt(_)
            | BindingType::Float(_) => {
                panic!("unsupported newtype inner type for ABI generation")
            }
        }
    }

    /// Build the generated replay struct name for a named binding struct.
    pub(super) fn replay_struct_name(&self, name: &str) -> String {
        let name = self.to_pascal_case(name);

        format!("{name}ReplayRecord")
    }

    /// Build the generated replay enum name for a named tagged union.
    pub(super) fn replay_tagged_union_name(&self, name: &str) -> String {
        let name = self.to_pascal_case(name);

        format!("{name}ReplayRecord")
    }

    /// Render the argument list for invoking one binding handler.
    pub(super) fn render_invoke_args(&self, entry: &BindingEntry) -> String {
        entry
            .parameters
            .iter()
            .enumerate()
            .map(|(index, param)| self.sanitize_param_name(&param.name, index))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Render one invoke argument list with a leading comma when needed.
    pub(super) fn render_invoke_args_with_prefix(&self, entry: &BindingEntry) -> String {
        let args = self.render_invoke_args(entry);
        if args.is_empty() {
            String::new()
        } else {
            format!(", {args}")
        }
    }

    /// Build the tuple type for VM binding arguments.
    pub(super) fn vm_args_tuple_type(&self, params: &[BindingParameter]) -> String {
        let mut parts = Vec::new();
        for param in params {
            parts.push(self.vm_type_for_binding(&param.binding_type));
        }
        if parts.len() == 1 {
            format!("({},)", parts[0])
        } else {
            format!("({})", parts.join(", "))
        }
    }

    /// Build the VM return type for one binding result.
    pub(super) fn vm_return_type(&self, binding_type: &BindingType) -> String {
        self.vm_type_for_binding(binding_type)
    }

    /// Convert a binding type into a VM type.
    pub(super) fn vm_type_for_binding(&self, binding_type: &BindingType) -> String {
        match binding_type {
            BindingType::Void => "()".to_string(),
            BindingType::Bool => "bool".to_string(),
            BindingType::Int(8) => "i8".to_string(),
            BindingType::Int(16) => "i16".to_string(),
            BindingType::Int(32) => "i32".to_string(),
            BindingType::Int(64) => "i64".to_string(),
            BindingType::Int(width) => panic!("unsupported int width for VM binding: {width}"),
            BindingType::UInt(8) => "u8".to_string(),
            BindingType::UInt(16) => "u16".to_string(),
            BindingType::UInt(32) => "u32".to_string(),
            BindingType::UInt(64) => "u64".to_string(),
            BindingType::UInt(width) => panic!("unsupported uint width for VM binding: {width}"),
            BindingType::Float(32) => "f32".to_string(),
            BindingType::Float(64) => "f64".to_string(),
            BindingType::Float(width) => panic!("unsupported float width for VM binding: {width}"),
            BindingType::String => "vm::StringHandle".to_string(),
            BindingType::StringSlice => "VmSlice<vm::StringHandle>".to_string(),
            BindingType::Slice(inner) => {
                format!("VmSlice<{}>", self.vm_type_for_binding(inner))
            }
            BindingType::Array(inner) => {
                format!("VmArray<{}>", self.vm_type_for_binding(inner))
            }
            BindingType::Optional(inner) => {
                format!("Option<{}>", self.vm_type_for_binding(inner))
            }
            BindingType::Newtype {
                name,
                domain: type_domain,
                inner,
            } => {
                if binding_type_requires_abi(inner) {
                    self.newtype_vm_path(type_domain, name)
                } else {
                    self.named_type_path(type_domain, name)
                }
            }
            BindingType::Struct {
                name,
                domain: type_domain,
                ..
            } => self.struct_vm_path(type_domain, name),
            BindingType::Enum {
                name,
                domain: type_domain,
                ..
            } => self.named_type_path(type_domain, name),
            BindingType::TaggedUnion {
                name,
                domain: type_domain,
                ..
            } => self.tagged_union_vm_path(type_domain, name),
        }
    }

    /// Convert one binding type into one replay payload Rust type.
    pub(super) fn replay_type_for_binding(&self, binding_type: &BindingType) -> String {
        match binding_type {
            BindingType::Void => "()".to_string(),
            BindingType::Bool => "bool".to_string(),
            BindingType::Int(8) => "i8".to_string(),
            BindingType::Int(16) => "i16".to_string(),
            BindingType::Int(32) => "i32".to_string(),
            BindingType::Int(64) => "i64".to_string(),
            BindingType::Int(width) => panic!("unsupported int width for replay: {width}"),
            BindingType::UInt(8) => "u8".to_string(),
            BindingType::UInt(16) => "u16".to_string(),
            BindingType::UInt(32) => "u32".to_string(),
            BindingType::UInt(64) => "u64".to_string(),
            BindingType::UInt(width) => panic!("unsupported uint width for replay: {width}"),
            BindingType::Float(32) => "f32".to_string(),
            BindingType::Float(64) => "f64".to_string(),
            BindingType::Float(width) => panic!("unsupported float width for replay: {width}"),
            BindingType::String => "String".to_string(),
            BindingType::StringSlice => "Vec<String>".to_string(),
            BindingType::Slice(inner) | BindingType::Array(inner) => {
                format!("Vec<{}>", self.replay_type_for_binding(inner))
            }
            BindingType::Optional(inner) => {
                format!("Option<{}>", self.replay_type_for_binding(inner))
            }
            BindingType::Newtype {
                name,
                domain: type_domain,
                inner,
            } => {
                if binding_type_requires_abi(inner) {
                    self.replay_type_for_binding(inner)
                } else {
                    self.named_type_path(type_domain, name)
                }
            }
            BindingType::Enum {
                name,
                domain: type_domain,
                ..
            } => self.named_type_path(type_domain, name),
            BindingType::Struct {
                name,
                domain: type_domain,
                ..
            } => {
                if binding_type_requires_abi(binding_type) {
                    let replay_name = self.replay_struct_name(name);
                    self.named_type_path(type_domain, replay_name.as_str())
                } else {
                    self.named_type_path(type_domain, name)
                }
            }
            BindingType::TaggedUnion {
                name,
                domain: type_domain,
                ..
            } => {
                if binding_type_requires_abi(binding_type) {
                    let replay_name = self.replay_tagged_union_name(name);
                    self.named_type_path(type_domain, replay_name.as_str())
                } else {
                    self.named_type_path(type_domain, name)
                }
            }
        }
    }

    /// Build one const name from one extern or module name.
    fn const_name(&self, name: &str) -> String {
        self.const_name_for_extern(name)
    }

    /// Build a stable identifier suffix for one extern binding name.
    pub(super) fn binding_suffix_for_extern(
        &self,
        extern_name: &str,
        domain: Option<&str>,
    ) -> String {
        let mut parts = extern_name.split('.').collect::<Vec<_>>();
        if parts.first().is_some_and(|part| *part == "destack") {
            let _ = parts.remove(0);
        }

        if let Some(domain) = domain
            && parts.first().is_some_and(|part| *part == domain)
        {
            let _ = parts.remove(0);
        }

        let mut suffix_parts = Vec::new();
        for part in parts {
            let value = self.snake_case(part);
            if !value.is_empty() {
                suffix_parts.push(value);
            }
        }

        if suffix_parts.is_empty() {
            return "binding".to_string();
        }

        suffix_parts.join("_")
    }

    /// Build one const name from one extern binding name.
    pub(super) fn const_name_for_extern(&self, extern_name: &str) -> String {
        let tail = self.binding_suffix_for_extern(extern_name, None);
        let mut out = String::new();
        for ch in self.snake_case(tail.as_str()).chars() {
            out.push(ch.to_ascii_uppercase());
        }
        if out.is_empty() {
            out.push_str("BINDING");
        }
        if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
            out.insert(0, '_');
        }
        out
    }

    /// Build the runtime implementation function name for a binding declaration.
    pub(super) fn implementation_fn_name(&self, implementation_name: &str) -> String {
        format!(
            "destack_{}_{}",
            self.module,
            self.snake_case(implementation_name)
        )
    }

    /// Return the Rust repr attribute for an enum backing type.
    pub(super) fn enum_backing_repr(backing: EnumBackingType) -> &'static str {
        match backing {
            EnumBackingType::Int(int_type) => match int_type.simplify() {
                IntType::Int8 => "i8",
                IntType::Int16 => "i16",
                IntType::Int32 => "i32",
                IntType::Int64 => "i64",
                IntType::Uint8 => "u8",
                IntType::Uint16 => "u16",
                IntType::Uint32 => "u32",
                IntType::Uint64 => "u64",
                _ => panic!("unsupported enum backing type: {int_type:?}"),
            },
            EnumBackingType::String => panic!("string-backed enums are not supported"),
        }
    }

    /// Convert enum backing types into a Rust primitive name.
    pub(super) fn enum_backing_rust_type(backing: EnumBackingType) -> &'static str {
        match backing {
            EnumBackingType::Int(int_type) => match int_type.simplify() {
                IntType::Int8 => "i8",
                IntType::Int16 => "i16",
                IntType::Int32 => "i32",
                IntType::Int64 => "i64",
                IntType::Uint8 => "u8",
                IntType::Uint16 => "u16",
                IntType::Uint32 => "u32",
                IntType::Uint64 => "u64",
                _ => panic!("unsupported enum backing width: {int_type:?}"),
            },
            EnumBackingType::String => "&str",
        }
    }

    /// Build one deterministic tagged union variant tag.
    pub(super) fn tagged_union_variant_tag(union_name: &str, variant_name: &str) -> u32 {
        const FNV64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
        const FNV64_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV64_OFFSET_BASIS;
        for byte in union_name.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(FNV64_PRIME);
        }
        hash ^= u64::from(b':');
        hash = hash.wrapping_mul(FNV64_PRIME);
        hash ^= u64::from(b':');
        hash = hash.wrapping_mul(FNV64_PRIME);
        for byte in variant_name.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(FNV64_PRIME);
        }

        ((hash % u64::from(u32::MAX)) as u32).saturating_add(1)
    }

    /// Convert a field name to snake case.
    pub(super) fn to_snake_case(&self, name: &str) -> String {
        let mut out = String::new();
        for ch in name.chars() {
            if ch.is_ascii_uppercase() {
                if !out.is_empty() {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push(ch);
            }
        }
        out
    }

    /// Convert one identifier to general snake case.
    pub(super) fn snake_case(&self, name: &str) -> String {
        let mut out = String::new();
        let mut prev_lower = false;
        for ch in name.chars() {
            if ch.is_ascii_alphanumeric() {
                if ch.is_ascii_uppercase() {
                    if prev_lower && !out.ends_with('_') {
                        out.push('_');
                    }
                    out.push(ch.to_ascii_lowercase());
                    prev_lower = true;
                } else {
                    out.push(ch.to_ascii_lowercase());
                    prev_lower = true;
                }
            } else if !out.ends_with('_') {
                out.push('_');
                prev_lower = false;
            }
        }
        if out.is_empty() {
            out.push_str("binding");
        }
        out
    }

    /// Convert one identifier to PascalCase.
    pub(super) fn to_pascal_case(&self, name: &str) -> String {
        let mut out = String::new();
        let mut capitalize_next = true;

        for ch in name.chars() {
            if ch.is_ascii_alphanumeric() {
                if capitalize_next {
                    out.push(ch.to_ascii_uppercase());
                } else {
                    out.push(ch.to_ascii_lowercase());
                }

                capitalize_next = false;
            } else {
                capitalize_next = true;
            }
        }

        if out.is_empty() {
            out.push_str("Generated");
        }

        if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
            out.insert(0, '_');
        }

        out
    }

    /// Normalize a parameter name into a safe identifier.
    pub(super) fn sanitize_param_name(&self, name: &str, index: usize) -> String {
        let stripped = name.trim().trim_start_matches("...");
        let mut out = String::new();
        for ch in stripped.chars() {
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push('_');
            }
        }
        if out.is_empty() {
            out.push_str(&format!("arg{index}"));
        }
        if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
            out.insert(0, '_');
        }

        if is_reserved_param_name(out.as_str()) {
            out = format!("argument_{out}");
        }

        out
    }
}

/// Return true when a generated parameter name is reserved.
fn is_reserved_param_name(name: &str) -> bool {
    matches!(
        name,
        "context"
            | "runtime"
            | "world"
            | "out"
            | "args"
            | "_args"
            | "registry"
            | "isolate"
            | "result"
            | "value"
            | "bytes"
            | "payload"
    ) || is_rust_keyword(name)
}

/// Return true when a generated identifier is a Rust keyword.
fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
    )
}

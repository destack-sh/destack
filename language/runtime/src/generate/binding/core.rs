use std::collections::{BTreeMap, BTreeSet};

use destack_dir::{EnumBackingType, IntType};

use crate::model::{
    BindingBlocking, BindingCatalog, BindingEntry, BindingEnumValue, BindingEnumVariant,
    BindingField, BindingParameter, BindingReplayKind, BindingScope, BindingType, EffectClass,
    RandomEventKind, ReplayPayload, ReplayPolicy, TimeEventKind,
};

/// Catalog entry grouping bindings by extern name.
pub(super) type BindingCatalogEntry = BTreeMap<String, BindingEntry>;

#[path = "abi.rs"]
mod abi;
#[path = "path.rs"]
mod path;
#[path = "replay.rs"]
mod replay;
#[path = "stub.rs"]
mod stub;

use abi::trim_unused_domain_imports;
pub(crate) use abi::{collect_domain_abi_types, render_abi_types};
pub(crate) use path::{
    runtime_domain_abi_types_path, runtime_domain_bindings_path, runtime_domain_host_path,
    runtime_domain_mod_path, runtime_domain_native_path, runtime_domain_runtime_mod_path,
    runtime_domain_runtime_native_path, runtime_domain_runtime_vm_path,
    runtime_domain_simulated_mod_path, runtime_domain_simulated_native_path,
    runtime_domain_simulated_vm_path, runtime_domain_test_harness_generated_path,
    runtime_domain_test_harness_path, runtime_domain_tests_basic_path,
    runtime_domain_tests_dir_path, runtime_domain_tests_mod_path, runtime_domain_tests_path,
    runtime_domain_unix_mod_path, runtime_domain_unsupported_path, runtime_domain_vm_path,
    runtime_domain_windows_mod_path, runtime_platform_generated_path, write_domain_bindings,
};
use replay::*;
pub(crate) use stub::{
    render_domain_mod_stub, render_domain_test_harness_generated, render_domain_test_harness_stub,
    render_host_router_stub, render_host_stub, render_native_stub, render_os_backend_mod_stub,
    render_runtime_mod_stub, render_runtime_native_stub, render_runtime_vm_stub,
    render_simulated_mod_stub, render_simulated_native_stub, render_simulated_vm_stub,
    render_vm_stub,
};

/// Canonical binding specification for a platform domain.
#[derive(Debug, Clone)]
pub(super) struct DomainSpec<'a> {
    /// Platform domain name.
    pub(super) domain: &'a str,
    /// Binding definitions for the domain.
    pub(super) bindings: &'a BindingCatalogEntry,
    /// Canonical binding descriptor constants.
    pub(super) consts: Vec<BindingConst<'a>>,
    /// Render usage flags for helper generation.
    pub(super) usage: RenderUsage,
    /// Rendered type usage for import decisions.
    pub(super) types: RenderTypes,
}

impl<'a> DomainSpec<'a> {
    /// Build a domain specification from binding metadata.
    fn new(domain: &'a str, bindings: &'a BindingCatalogEntry) -> Self {
        let consts = build_binding_consts(domain, bindings);
        let vm_usage = collect_vm_decode_usage(bindings);
        let vm_types = collect_vm_stub_usage(bindings);
        let native_usage = collect_native_signature_usage(bindings);
        let named_types = collect_vm_named_types(domain, bindings);
        let native_named_types = collect_native_named_types(domain, bindings);
        let replay_vm_named_types = collect_replay_vm_named_types(domain, bindings);
        let replay_named_types = collect_replay_named_types(domain, bindings);
        let type_domains = collect_type_domains(domain, bindings);
        let uses_replay_policy = bindings
            .values()
            .any(|entry| matches!(entry.effect_class, EffectClass::External { .. }));
        let uses_binding_replay = bindings.values().any(|entry| {
            matches!(
                entry.effect_class,
                EffectClass::External {
                    replay: ReplayPolicy::Recordable
                }
            ) && entry.replay_kind == BindingReplayKind::Regular
        });
        let uses_time_replay_kind = bindings
            .values()
            .any(|entry| matches!(entry.replay_kind, BindingReplayKind::Time(_)));
        let uses_random_replay_kind = bindings
            .values()
            .any(|entry| matches!(entry.replay_kind, BindingReplayKind::Random(_)));
        let uses_replay_payload_type = bindings.values().any(|entry| {
            matches!(entry.replay_payload, ReplayPayload::ArgumentsAndResults)
                && entry.replay_kind == BindingReplayKind::Regular
        });
        let uses_replay_payload = bindings.values().any(|entry| {
            matches!(
                entry.effect_class,
                EffectClass::External {
                    replay: ReplayPolicy::Recordable
                }
            ) && entry.replay_kind == BindingReplayKind::Regular
        });
        let needs_vm = bindings.values().any(|entry| {
            entry
                .parameters
                .iter()
                .any(|param| binding_type_requires_vm_for_decode(&param.binding_type))
                || binding_type_requires_vm_for_encode(&entry.return_binding)
        });
        let needs_decode = bindings.values().any(|entry| !entry.parameters.is_empty());
        let needs_native_out = bindings
            .values()
            .any(|entry| entry.return_binding != BindingType::Void);
        let uses_world_dispatch = bindings
            .values()
            .any(|entry| entry.scope != BindingScope::Runtime);
        let uses_runtime_dispatch = bindings
            .values()
            .any(|entry| entry.scope == BindingScope::Runtime);

        Self {
            domain,
            bindings,
            consts,
            usage: RenderUsage {
                vm_usage,
                vm_types,
                native_usage,
                uses_replay_policy,
                uses_binding_replay,
                uses_time_replay_kind,
                uses_random_replay_kind,
                uses_replay_payload_type,
                uses_replay_payload,
                needs_vm,
                needs_decode,
                needs_native_out,
                uses_world_dispatch,
                uses_runtime_dispatch,
            },
            types: RenderTypes {
                named_types,
                native_named_types,
                replay_vm_named_types,
                replay_named_types,
                type_domains,
            },
        }
    }
}

/// Stateful writer for a domain binding catalog.
pub(super) struct DomainWriter<'a> {
    /// Render specification derived from bindings.
    pub(super) spec: &'a DomainSpec<'a>,
    /// Output buffer for generated content.
    pub(super) output: &'a mut String,
}

impl<'a> DomainWriter<'a> {
    /// Create a writer for a domain catalog.
    fn new(spec: &'a DomainSpec<'a>, output: &'a mut String) -> Self {
        Self { spec, output }
    }
}

/// Named ABI types grouped by platform domain.
#[derive(Debug, Default, Clone)]
pub(crate) struct DomainAbiTypes {
    /// Newtype definitions keyed by name.
    pub newtypes: BTreeMap<String, BindingType>,
    /// Struct definitions keyed by name.
    pub structs: BTreeMap<String, BindingType>,
    /// Enum definitions keyed by name.
    pub enums: BTreeMap<String, BindingType>,
}

/// Usage flags for native stub bindings.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct NativeUsage {
    /// Platform slices are referenced in the native ABI.
    uses_platform_slice: bool,
    /// Platform arrays are referenced in the native ABI.
    uses_platform_array: bool,
    /// Platform string references are referenced in the native ABI.
    uses_platform_string_ref: bool,
    /// Platform string slices are referenced in the native ABI.
    uses_platform_string_slice: bool,
}

/// Usage flags for VM argument decoding helpers.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct VmDecodeUsage {
    /// Boolean decoding is used.
    uses_bool: bool,
    /// Signed 8-bit decoding is used.
    uses_int8: bool,
    /// Signed 16-bit decoding is used.
    uses_int16: bool,
    /// Signed 32-bit decoding is used.
    uses_int32: bool,
    /// Signed 64-bit decoding is used.
    uses_int64: bool,
    /// Unsigned 8-bit decoding is used.
    uses_uint8: bool,
    /// Unsigned 16-bit decoding is used.
    uses_uint16: bool,
    /// Unsigned 32-bit decoding is used.
    uses_uint32: bool,
    /// Unsigned 64-bit decoding is used.
    uses_uint64: bool,
    /// 32-bit float decoding is used.
    uses_float32: bool,
    /// 64-bit float decoding is used.
    uses_float64: bool,
    /// String decoding is used.
    uses_string: bool,
    /// Slice decoding is used.
    uses_slice: bool,
    /// Array decoding is used.
    uses_array: bool,
}

/// Usage flags derived from a binding catalog.
#[derive(Debug, Clone)]
pub(super) struct RenderUsage {
    /// VM wrapper usage flags.
    pub(super) vm_usage: VmDecodeUsage,
    /// VM type usage flags for imports.
    pub(super) vm_types: VmStubUsage,
    /// Native signature usage flags.
    pub(super) native_usage: NativeUsage,
    /// Whether replay policy is referenced.
    pub(super) uses_replay_policy: bool,
    /// Whether bindings use replay helpers.
    pub(super) uses_binding_replay: bool,
    /// Whether time replay routing is referenced.
    pub(super) uses_time_replay_kind: bool,
    /// Whether random replay routing is referenced.
    pub(super) uses_random_replay_kind: bool,
    /// Whether replay payload enum is referenced.
    pub(super) uses_replay_payload_type: bool,
    /// Whether replay payload helpers are needed.
    pub(super) uses_replay_payload: bool,
    /// Whether VM helper imports are required.
    pub(super) needs_vm: bool,
    /// Whether argument decoding helpers are required.
    pub(super) needs_decode: bool,
    /// Whether native out parameters are used.
    pub(super) needs_native_out: bool,
    /// Whether world dispatch should be emitted.
    pub(super) uses_world_dispatch: bool,
    /// Whether runtime dispatch should be emitted.
    pub(super) uses_runtime_dispatch: bool,
}

/// Named type usage derived from bindings.
#[derive(Debug, Clone)]
pub(super) struct RenderTypes {
    /// VM-level named types for imports.
    pub(super) named_types: BTreeSet<String>,
    /// Native ABI named types for imports.
    pub(super) native_named_types: BTreeSet<String>,
    /// Replay VM named types for imports.
    pub(super) replay_vm_named_types: BTreeSet<String>,
    /// Replay ABI named types for imports.
    pub(super) replay_named_types: BTreeSet<String>,
    /// Referenced foreign type domains.
    pub(super) type_domains: BTreeSet<String>,
}

/// Canonical binding descriptor information for rendering.
#[derive(Debug, Clone)]
pub(super) struct BindingConst<'a> {
    /// Constant name for the binding descriptor.
    pub(super) const_name: String,
    /// Fully qualified extern binding name.
    pub(super) extern_name: &'a str,
    /// Runtime implementation function name.
    pub(super) implementation_fn_name: String,
    /// Binding metadata payload.
    pub(super) entry: &'a BindingEntry,
}

/// Render generated bindings for a runtime domain.
pub(crate) fn render_domain_bindings(domain: &str, bindings: &BindingCatalogEntry) -> String {
    let spec = DomainSpec::new(domain, bindings);
    let mut output = String::new();
    let mut writer = DomainWriter::new(&spec, &mut output);

    writer.write_header();
    if spec.usage.uses_binding_replay {
        writer.write_replay_payloads();
    }
    writer.write_descriptor_consts();
    writer.write_bindings_slice();
    writer.write_native_set();
    if spec.usage.uses_binding_replay {
        writer.write_native_replay_helpers();
    }
    writer.write_native_exports();
    if spec.usage.uses_binding_replay {
        writer.write_vm_replay_helpers();
    }
    writer.write_vm_register_fn();
    writer.write_vm_set();

    trim_unused_domain_imports(domain, &mut output);

    output
}

/// Collect native usage flags for a binding catalog entry.
fn collect_native_usage(bindings: &BindingCatalogEntry) -> NativeUsage {
    let mut usage = NativeUsage::default();
    for entry in bindings.values() {
        collect_native_usage_for_binding(&entry.return_binding, &mut usage);
        for param in &entry.parameters {
            collect_native_usage_for_binding(&param.binding_type, &mut usage);
        }
    }

    usage
}

/// Collect native signature usage flags for a binding catalog entry.
fn collect_native_signature_usage(bindings: &BindingCatalogEntry) -> NativeUsage {
    let mut usage = NativeUsage::default();
    for entry in bindings.values() {
        collect_native_signature_usage_for_binding(&entry.return_binding, &mut usage);
        for param in &entry.parameters {
            collect_native_signature_usage_for_binding(&param.binding_type, &mut usage);
        }
    }

    usage
}

/// Record native ABI signature usage for a binding type.
fn collect_native_signature_usage_for_binding(binding_type: &BindingType, usage: &mut NativeUsage) {
    match binding_type {
        BindingType::String => usage.uses_platform_string_ref = true,
        BindingType::StringSlice => usage.uses_platform_string_slice = true,
        BindingType::Slice(_) => usage.uses_platform_slice = true,
        BindingType::Array(_) => usage.uses_platform_array = true,
        _ => {}
    }
}

/// Collect VM decode helper usage from binding parameters.
fn collect_vm_decode_usage(bindings: &BindingCatalogEntry) -> VmDecodeUsage {
    let mut usage = VmDecodeUsage::default();
    for entry in bindings.values() {
        let uses_binding_replay = matches!(
            entry.effect_class,
            EffectClass::External {
                replay: ReplayPolicy::Recordable
            }
        ) && entry.replay_kind == BindingReplayKind::Regular;
        let supports_args = matches!(entry.replay_payload, ReplayPayload::ArgumentsAndResults);
        for param in &entry.parameters {
            collect_vm_decode_usage_for_binding(&param.binding_type, &mut usage, false, true);
            if uses_binding_replay && supports_args {
                collect_vm_decode_usage_for_binding(&param.binding_type, &mut usage, true, false);
            }
        }
        if uses_binding_replay {
            collect_vm_decode_usage_for_binding(&entry.return_binding, &mut usage, true, false);
        }
    }
    usage
}

/// Record decode helper usage for a binding type.
fn collect_vm_decode_usage_for_binding(
    binding_type: &BindingType,
    usage: &mut VmDecodeUsage,
    deep_collections: bool,
    include_collection_decoders: bool,
) {
    match binding_type {
        BindingType::Void => {}
        BindingType::Bool => usage.uses_bool = true,
        BindingType::Int(8) => usage.uses_int8 = true,
        BindingType::Int(16) => usage.uses_int16 = true,
        BindingType::Int(32) => usage.uses_int32 = true,
        BindingType::Int(64) => usage.uses_int64 = true,
        BindingType::UInt(8) => usage.uses_uint8 = true,
        BindingType::UInt(16) => usage.uses_uint16 = true,
        BindingType::UInt(32) => usage.uses_uint32 = true,
        BindingType::UInt(64) => usage.uses_uint64 = true,
        BindingType::Float(32) => usage.uses_float32 = true,
        BindingType::Float(64) => usage.uses_float64 = true,
        BindingType::Int(_) | BindingType::UInt(_) | BindingType::Float(_) => {}
        BindingType::String => usage.uses_string = true,
        BindingType::StringSlice => {
            if include_collection_decoders {
                usage.uses_slice = true;
            }
            if deep_collections {
                usage.uses_string = true;
            }
        }
        BindingType::Slice(inner) => {
            if include_collection_decoders {
                usage.uses_slice = true;
            }
            if deep_collections {
                collect_vm_decode_usage_for_binding(inner, usage, true, false);
            }
        }
        BindingType::Array(inner) => {
            if include_collection_decoders {
                usage.uses_array = true;
            }
            if deep_collections {
                collect_vm_decode_usage_for_binding(inner, usage, true, false);
            }
        }
        BindingType::Newtype { inner, .. } => {
            collect_vm_decode_usage_for_binding(
                inner,
                usage,
                deep_collections,
                include_collection_decoders,
            );
        }
        BindingType::Struct { fields, .. } => {
            for field in fields {
                collect_vm_decode_usage_for_binding(
                    &field.binding_type,
                    usage,
                    deep_collections,
                    include_collection_decoders,
                );
            }
        }
        BindingType::Enum { backing, .. } => {
            let backing_type = enum_backing_binding_type(*backing);
            collect_vm_decode_usage_for_binding(
                &backing_type,
                usage,
                deep_collections,
                include_collection_decoders,
            );
        }
    }
}

/// Record native ABI usage for a binding type.
fn collect_native_usage_for_binding(binding_type: &BindingType, usage: &mut NativeUsage) {
    match binding_type {
        BindingType::String => usage.uses_platform_string_ref = true,
        BindingType::StringSlice => usage.uses_platform_string_slice = true,
        BindingType::Slice(inner) => {
            usage.uses_platform_slice = true;
            collect_native_usage_for_binding(inner, usage);
        }
        BindingType::Array(inner) => {
            usage.uses_platform_array = true;
            collect_native_usage_for_binding(inner, usage);
        }
        BindingType::Newtype { inner, .. } => {
            collect_native_usage_for_binding(inner, usage);
        }
        BindingType::Struct { fields, .. } => {
            for field in fields {
                collect_native_usage_for_binding(&field.binding_type, usage);
            }
        }
        BindingType::Enum { .. } => {}
        _ => {}
    }
}

/// Render the generated platform binding lists.
pub(crate) fn render_platform_bindings_index(domains: &BTreeSet<String>) -> String {
    let mut output = String::new();
    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("use crate::platform::bindings::{NativeBindingSet, VmBindingSet};\n");

    if !domains.is_empty() {
        let imports = domains
            .iter()
            .map(|domain| sanitize_module_name(domain))
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!("use crate::platform::{{{imports}}};\n\n"));
    } else {
        output.push_str("\n");
    }

    output.push_str("/// Native binding sets for all platform domains.\n");
    output.push_str("pub const PLATFORM_NATIVE_BINDINGS: &[NativeBindingSet] = &[\n");
    for domain in domains {
        let module = sanitize_module_name(domain);
        let set_name = native_set_name_for_domain(domain);
        output.push_str(&format!("    {module}::{set_name},\n"));
    }
    output.push_str("];\n\n");

    output.push_str("/// VM binding sets for all platform domains.\n");
    output.push_str("pub const PLATFORM_VM_BINDINGS: &[VmBindingSet] = &[\n");
    for domain in domains {
        let module = sanitize_module_name(domain);
        let set_name = vm_set_name_for_domain(domain);
        output.push_str(&format!("    {module}::{set_name},\n"));
    }
    output.push_str("];\n");

    output
}

/// Render stub VM bindings for a runtime domain.
/// Build a deterministic list of binding constants for a domain.
fn build_binding_consts<'a>(
    domain: &str,
    bindings: &'a BindingCatalogEntry,
) -> Vec<BindingConst<'a>> {
    // track constant names to avoid collisions
    let mut used_const_names = BTreeSet::new();
    let mut consts = Vec::new();

    // render each binding into a stable constant name
    for (extern_name, entry) in bindings {
        let mut const_name = const_name_for_extern(extern_name);
        if used_const_names.contains(&const_name) {
            let mut index = 2;
            let base = const_name.clone();
            while used_const_names.contains(&const_name) {
                const_name = format!("{base}_{index}");
                index += 1;
            }
        }

        used_const_names.insert(const_name.clone());
        consts.push(BindingConst {
            const_name,
            extern_name,
            implementation_fn_name: implementation_fn_name(domain, &entry.implementation_name),
            entry,
        });
    }

    consts
}

impl<'a> DomainWriter<'a> {
    /// Render the shared header for generated binding files.
    fn write_header(&mut self) {
        let domain = self.spec.domain;
        let usage = &self.spec.usage;
        let types = &self.spec.types;

        self.output
            .push_str("// generated by generate-bindings: do not edit\n\n");
        self.output.push_str("#![allow(unused_imports)]\n\n");
        self.output.push_str("#![allow(clippy::clone_on_copy)]\n\n");
        self.output
            .push_str("#![allow(clippy::type_complexity)]\n\n");
        if usage.needs_vm {
            self.output.push_str("use destack_vm as vm;\n");
        }
        self.output.push_str("use destack_vm::Isolate;\n");
        let mut binding_imports = vec![
            "BindingDescriptor",
            "BindingBlocking",
            "BindingRegistry",
            "NativeBinding",
            "NativeBindingSet",
            "BindingScope",
            "native_call",
        ];
        if usage.uses_replay_policy {
            binding_imports.push("BindingReplayKind");
            binding_imports.push("ReplayPolicy");
        }
        if usage.uses_replay_payload_type {
            binding_imports.push("ReplayPayload");
        }
        if usage.uses_world_dispatch {
            binding_imports.push("RuntimeWorld");
        }
        self.output.push_str(&format!(
            "use crate::platform::bindings::{{{}}};\n",
            binding_imports.join(", ")
        ));
        if usage.uses_time_replay_kind || usage.uses_random_replay_kind {
            let mut replay_imports = Vec::new();
            if usage.uses_time_replay_kind {
                replay_imports.push("TimeEventKind");
            }
            if usage.uses_random_replay_kind {
                replay_imports.push("RandomEventKind");
            }
            self.output.push_str(&format!(
                "use crate::replay::{{{}}};\n",
                replay_imports.join(", ")
            ));
        }
        if usage.uses_random_replay_kind {
            self.output.push_str("use crate::random::RandomStreamId;\n");
        }
        self.output.push_str("use crate::vm_binding_set;\n");
        if usage.needs_decode
            || usage.uses_replay_payload
            || usage.needs_native_out
            || usage.uses_world_dispatch
        {
            self.output
                .push_str("use crate::diagnostic::RuntimeError;\n");
            self.output
                .push_str("use crate::platform::PlatformError;\n");
        }
        self.output
            .push_str("use crate::diagnostic::RuntimeResult;\n");
        let mut vm_imports = Vec::new();
        if usage.vm_types.uses_vm_slice {
            vm_imports.push("VmSlice");
        }
        if usage.vm_types.uses_vm_array {
            vm_imports.push("VmArray");
        }
        if !vm_imports.is_empty() {
            self.output.push_str(&format!(
                "use crate::platform::{{{}}};\n",
                vm_imports.join(", ")
            ));
        }
        let mut native_imports = Vec::new();
        if usage.native_usage.uses_platform_slice {
            native_imports.push("NativeSlice");
        }
        if usage.native_usage.uses_platform_array {
            native_imports.push("NativeArray");
        }
        if usage.native_usage.uses_platform_string_ref {
            native_imports.push("NativeStringRef");
        }
        if usage.native_usage.uses_platform_string_slice {
            native_imports.push("NativeStringSlice");
        }
        native_imports.push("RuntimeStatus");
        if !native_imports.is_empty() {
            self.output.push_str(&format!(
                "use crate::platform::{{{}}};\n",
                native_imports.join(", ")
            ));
        }
        self.output
            .push_str("use crate::platform::abi as platform_abi;\n");
        let mut domain_types = types.named_types.clone();
        domain_types.extend(types.native_named_types.iter().cloned());
        domain_types.extend(types.replay_vm_named_types.iter().cloned());
        domain_types.extend(types.replay_named_types.iter().cloned());
        if domain == "error" {
            domain_types.remove("PlatformError");
        }
        let local_types = domain_types
            .iter()
            .filter(|name| !name.contains("::"))
            .cloned()
            .collect::<Vec<_>>();
        if !local_types.is_empty() {
            let names = local_types.join(", ");
            self.output
                .push_str(&format!("use crate::platform::{domain}::{{{names}}};\n"));
        }
        self.output
            .push_str("use crate::runtime::with_runtime_call_context;\n");
        if usage.uses_binding_replay {
            self.output
                .push_str("use crate::runtime::RuntimeCallContext;\n");
        }
        self.output.push_str("use crate::binding;\n\n");

        if usage.uses_binding_replay {
            self.output
                .push_str("use serde::{Deserialize, Serialize};\n\n");
        }

        let mut alias_domains = BTreeSet::new();
        alias_domains.insert(domain.to_string());
        alias_domains.extend(types.type_domains.iter().cloned());
        for alias_domain in alias_domains {
            let alias = platform_domain_alias(alias_domain.as_str());
            self.output.push_str(&format!(
                "use crate::platform::{alias_domain} as {alias};\n"
            ));
        }

        if !types.type_domains.is_empty() {
            let imports = types
                .type_domains
                .iter()
                .map(|domain| domain.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            self.output
                .push_str(&format!("use crate::platform::{{{imports}}};\n"));
        }
        self.output.push_str(&format!(
            "use crate::platform::{domain}::{{native as platform_native, vm as platform_vm}};\n"
        ));
        if usage.uses_runtime_dispatch {
            self.output.push_str(&format!(
                "use crate::platform::{domain}::runtime::{{native as platform_runtime_native, vm as platform_runtime_vm}};\n"
            ));
        }
        if usage.uses_world_dispatch {
            self.output.push_str(&format!(
                "use crate::platform::{domain}::simulated::{{native as platform_simulated_native, vm as platform_simulated_vm}};\n"
            ));
        }
        self.output.push('\n');

        if usage.needs_decode {
            self.write_vm_decode_helpers(&usage.vm_usage);
        }

        self.write_vm_binding_helpers();
    }
}

impl<'a> DomainWriter<'a> {
    /// Render VM decode helper functions for a binding domain.
    fn write_vm_decode_helpers(&mut self, usage: &VmDecodeUsage) {
        let output = &mut self.output;

        output.push_str("/// Read a positional argument value.\n");
        output.push_str("#[allow(dead_code)]\n");
        output.push_str("fn arg_value(\n");
        output.push_str("    args: &[vm::Value],\n");
        output.push_str("    index: usize,\n");
        output.push_str("    name: &'static str,\n");
        output.push_str("    expected: &'static str,\n");
        output.push_str(") -> RuntimeResult<vm::Value> {\n");
        output.push_str("    let value = args.get(index).copied().ok_or_else(|| {\n");
        output.push_str("        RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed()\n");
        output.push_str("    })?;\n");
        output.push_str("\n");
        output.push_str("    Ok(value)\n");
        output.push_str("}\n\n");

        if usage.uses_bool {
            output.push_str("/// Decode a boolean argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_bool(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<bool> {\n");
            output.push_str("    value\n");
            output.push_str("        .as_bool()\n");
            output.push_str("        .ok_or_else(|| RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed())\n");
            output.push_str("}\n\n");
        }

        if usage.uses_int8 || usage.uses_int16 || usage.uses_int32 || usage.uses_int64 {
            output.push_str("/// Decode a signed integer argument with an explicit width.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_int(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str("    bits: u8,\n");
            output.push_str(") -> RuntimeResult<i64> {\n");
            output.push_str("    let (raw, width) = value\n");
            output.push_str("        .as_int_with_width()\n");
            output.push_str("        .ok_or_else(|| RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed())?;\n");
            output.push_str("    if width != bits {\n");
            output.push_str("        return Err(RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed());\n");
            output.push_str("    }\n");
            output.push_str("\n");
            output.push_str("    Ok(raw)\n");
            output.push_str("}\n\n");
        }

        if usage.uses_uint8 || usage.uses_uint16 || usage.uses_uint32 || usage.uses_uint64 {
            output.push_str("/// Decode an unsigned integer argument with an explicit width.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_uint(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str("    bits: u8,\n");
            output.push_str(") -> RuntimeResult<u64> {\n");
            output.push_str("    let (raw, width) = value\n");
            output.push_str("        .as_uint_with_width()\n");
            output.push_str("        .ok_or_else(|| RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed())?;\n");
            output.push_str("    if width != bits {\n");
            output.push_str("        return Err(RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed());\n");
            output.push_str("    }\n");
            output.push_str("\n");
            output.push_str("    Ok(raw)\n");
            output.push_str("}\n\n");
        }

        if usage.uses_int8 {
            output.push_str("/// Decode an i8 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_int8(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<i8> {\n");
            output.push_str("    Ok(decode_int(value, name, expected, 8)? as i8)\n");
            output.push_str("}\n\n");
        }
        if usage.uses_int16 {
            output.push_str("/// Decode an i16 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_int16(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<i16> {\n");
            output.push_str("    Ok(decode_int(value, name, expected, 16)? as i16)\n");
            output.push_str("}\n\n");
        }
        if usage.uses_int32 {
            output.push_str("/// Decode an i32 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_int32(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<i32> {\n");
            output.push_str("    Ok(decode_int(value, name, expected, 32)? as i32)\n");
            output.push_str("}\n\n");
        }
        if usage.uses_int64 {
            output.push_str("/// Decode an i64 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_int64(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<i64> {\n");
            output.push_str("    decode_int(value, name, expected, 64)\n");
            output.push_str("}\n\n");
        }

        if usage.uses_uint8 {
            output.push_str("/// Decode a u8 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_uint8(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<u8> {\n");
            output.push_str("    Ok(decode_uint(value, name, expected, 8)? as u8)\n");
            output.push_str("}\n\n");
        }
        if usage.uses_uint16 {
            output.push_str("/// Decode a u16 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_uint16(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<u16> {\n");
            output.push_str("    Ok(decode_uint(value, name, expected, 16)? as u16)\n");
            output.push_str("}\n\n");
        }
        if usage.uses_uint32 {
            output.push_str("/// Decode a u32 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_uint32(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<u32> {\n");
            output.push_str("    Ok(decode_uint(value, name, expected, 32)? as u32)\n");
            output.push_str("}\n\n");
        }
        if usage.uses_uint64 {
            output.push_str("/// Decode a u64 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_uint64(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<u64> {\n");
            output.push_str("    decode_uint(value, name, expected, 64)\n");
            output.push_str("}\n\n");
        }

        if usage.uses_float32 {
            output.push_str("/// Decode an f32 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_float32(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<f32> {\n");
            output.push_str("    value\n");
            output.push_str("        .as_float32()\n");
            output.push_str("        .ok_or_else(|| RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed())\n");
            output.push_str("}\n\n");
        }
        if usage.uses_float64 {
            output.push_str("/// Decode an f64 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_float64(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<f64> {\n");
            output.push_str("    value\n");
            output.push_str("        .as_float64()\n");
            output.push_str("        .ok_or_else(|| RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed())\n");
            output.push_str("}\n\n");
        }

        if usage.uses_string {
            output.push_str("/// Decode a string argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_string(\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<vm::StringHandle> {\n");
            output.push_str("    if value.tag() != vm::ValueTag::String {\n");
            output.push_str("        return Err(RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed());\n");
            output.push_str("    }\n");
            output.push_str("\n");
            output.push_str("    Ok(vm::StringHandle::new(value))\n");
            output.push_str("}\n\n");
        }

        if usage.uses_slice {
            output.push_str("/// Decode a slice argument.\n");
            output.push_str("fn decode_slice<T>(\n");
            output.push_str("    context: &mut vm::ExternalCallContext<'_>,\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<VmSlice<T>> {\n");
            output.push_str("    VmSlice::<T>::from_value(context, value, name, expected)\n");
            output.push_str("}\n\n");
        }

        if usage.uses_array {
            output.push_str("/// Decode an array argument.\n");
            output.push_str("fn decode_array<T>(\n");
            output.push_str("    context: &mut vm::ExternalCallContext<'_>,\n");
            output.push_str("    value: vm::Value,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<VmArray<T>> {\n");
            output.push_str("    VmArray::<T>::from_value(context, value, name, expected)\n");
            output.push_str("}\n\n");
        }
    }

    /// Render per-binding VM argument and result helpers.
    fn write_vm_binding_helpers(&mut self) {
        let output = &mut self.output;
        let domain = self.spec.domain;
        let bindings = self.spec.bindings;

        for (extern_name, entry) in bindings {
            let helper_base = vm_fn_name(domain, extern_name);
            if !entry.parameters.is_empty() {
                let decode_helper = decode_helper_name(&helper_base);
                let decode_uses_context = entry
                    .parameters
                    .iter()
                    .any(|param| binding_type_requires_context_for_decode(&param.binding_type));
                let decode_context_name = if decode_uses_context {
                    "context"
                } else {
                    "_context"
                };
                output.push_str(&format!("/// Decode arguments for {}.\n", extern_name));
                output.push_str("#[inline]\n");
                output.push_str(&format!(
                "fn {decode_helper}(\n    {decode_context_name}: &mut vm::ExternalCallContext<'_>,\n    args: &[vm::Value],\n) -> RuntimeResult<{}> {{\n",
                vm_args_tuple_type(domain, &entry.parameters)
            ));
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = sanitize_param_name(&param.name, index);
                    let expected = param
                        .type_text
                        .clone()
                        .unwrap_or_else(|| "value".to_string());
                    output.push_str(&format!(
                    "    let {name}_value = arg_value(args, {index}, \"{name}\", \"{expected}\")?;\n"
                ));
                    for line in render_decode_value_lines(
                        domain,
                        &param.binding_type,
                        &name,
                        &format!("{name}_value"),
                        &expected,
                    ) {
                        output.push_str(&format!("    {line}\n"));
                    }
                }
                let tuple_values = entry
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(index, param)| sanitize_param_name(&param.name, index))
                    .collect::<Vec<_>>();
                if tuple_values.len() == 1 {
                    output.push_str(&format!("    Ok(({},))\n", tuple_values[0]));
                } else {
                    output.push_str(&format!("    Ok(({}))\n", tuple_values.join(", ")));
                }
                output.push_str("}\n\n");
            }

            let encode_helper = encode_helper_name(&helper_base);
            let encode_uses_context =
                binding_type_requires_context_for_encode(&entry.return_binding);
            let encode_context_name = if encode_uses_context {
                "context"
            } else {
                "_context"
            };
            output.push_str(&format!("/// Encode the result for {}.\n", extern_name));
            output.push_str("#[inline]\n");
            output.push_str(&format!(
            "fn {encode_helper}(\n    {encode_context_name}: &mut vm::ExternalCallContext<'_>,\n    result: RuntimeResult<{}>,\n) -> RuntimeResult<vm::Value> {{\n",
            vm_return_type(domain, &entry.return_binding)
        ));
            for line in render_return_encode_lines(domain, &entry.return_binding) {
                output.push_str(&format!("    {line}\n"));
            }
            output.push_str("}\n\n");
        }
    }
}

impl<'a> DomainWriter<'a> {
    /// Render the binding descriptor constants for a domain.
    fn write_descriptor_consts(&mut self) {
        let output = &mut self.output;
        let consts = &self.spec.consts;
        for binding in consts {
            let signature = escape_rust_string(&binding.entry.signature);
            let (ctor, args) = Self::descriptor_ctor_for_binding(
                binding.entry.effect_class,
                binding.entry.replay_payload,
                binding.entry.replay_kind,
                &binding.entry.requires,
                binding.entry.scope,
                binding.entry.blocking,
            );
            let host_platforms = render_binding_host_platforms(&binding.entry.host_platforms);
            output.push_str(&format!(
                "/// Binding descriptor for {}.\n",
                binding.extern_name
            ));
            output.push_str(&format!(
                "pub const {}: BindingDescriptor = BindingDescriptor::{}(\n",
                binding.const_name, ctor,
            ));
            output.push_str(&format!("    \"{}\",\n", binding.extern_name));
            output.push_str(&format!("    \"{signature}\",\n"));
            for arg in args {
                output.push_str(&format!("    {arg},\n"));
            }
            output.push_str(")");
            if let Some(host_platforms) = host_platforms {
                output.push_str(&format!("\n    .with_host_platforms({host_platforms})"));
            }
            output.push_str(";\n\n");
        }
    }

    /// Resolve the descriptor constructor for an effect class.
    fn descriptor_ctor_for_binding(
        effect_class: EffectClass,
        replay_payload: ReplayPayload,
        replay_kind: BindingReplayKind,
        requires: &[String],
        scope: BindingScope,
        blocking: BindingBlocking,
    ) -> (&'static str, Vec<String>) {
        let requires_arg = render_binding_requires(requires);
        let scope_arg = render_binding_scope(scope);
        let blocking_arg = render_binding_blocking(blocking);
        match effect_class {
            EffectClass::Pure => (
                "pure_with_requires_and_behavior",
                vec![requires_arg, scope_arg, blocking_arg],
            ),
            EffectClass::Deterministic => (
                "deterministic_with_requires_and_behavior",
                vec![requires_arg, scope_arg, blocking_arg],
            ),
            EffectClass::External { replay } => {
                let replay = match replay {
                    ReplayPolicy::Recordable => "ReplayPolicy::Recordable",
                    ReplayPolicy::NonRecordable => "ReplayPolicy::NonRecordable",
                };
                let replay_kind = render_binding_replay_kind(replay_kind);
                let payload_arg = match replay_payload {
                    ReplayPayload::ResultsOnly => None,
                    ReplayPayload::ArgumentsAndResults => {
                        Some("ReplayPayload::ArgumentsAndResults".to_string())
                    }
                };
                if let Some(payload_arg) = payload_arg {
                    (
                        "external_with_payload_with_requires",
                        vec![
                            replay.to_string(),
                            replay_kind,
                            payload_arg,
                            requires_arg,
                            scope_arg,
                            blocking_arg,
                        ],
                    )
                } else {
                    (
                        "external_with_requires_and_behavior",
                        vec![
                            replay.to_string(),
                            replay_kind,
                            requires_arg,
                            scope_arg,
                            blocking_arg,
                        ],
                    )
                }
            }
        }
    }

    /// Render the descriptor slice for a domain.
    fn write_bindings_slice(&mut self) {
        let output = &mut self.output;
        let domain = self.spec.domain;
        let consts = &self.spec.consts;
        output.push_str(&format!("/// Binding descriptors for {domain}.\n"));
        output.push_str("pub const BINDINGS: &[BindingDescriptor] = &[\n");
        for binding in consts {
            output.push_str(&format!("    {},\n", binding.const_name));
        }
        output.push_str("];\n\n");
    }

    /// Render the native binding set for a domain.
    fn write_native_set(&mut self) {
        let output = &mut self.output;
        let domain = self.spec.domain;
        let consts = &self.spec.consts;
        // compute the native binding set name
        let native_set_name = native_set_name_for_domain(domain);

        // render the binding set descriptor
        output.push_str(&format!("/// Native binding set for {domain}.\n"));
        output.push_str(&format!(
            "pub const {native_set_name}: NativeBindingSet = NativeBindingSet {{\n"
        ));
        output.push_str(&format!("    name: \"{domain}\",\n"));
        output.push_str("    bindings: &[\n");
        for binding in consts {
            let native_fn = native_fn_name(domain, binding.extern_name);
            output.push_str(&format!(
                "        NativeBinding::new({}, \"{}\", {native_fn} as *const ()),\n",
                binding.const_name, binding.extern_name
            ));
        }
        output.push_str("    ],\n};\n\n");
    }

    /// Render native export wrappers for a domain.
    fn write_native_exports(&mut self) {
        let output = &mut self.output;
        let domain = self.spec.domain;
        let consts = &self.spec.consts;
        output.push_str(&format!(
            "/// Native export wrappers for {domain} bindings.\n"
        ));
        for binding in consts {
            let entry = binding.entry;
            let export_fn_name = native_fn_name(domain, binding.extern_name);
            let implementation_fn_name = &binding.implementation_fn_name;
            let replay_fn_name = native_replay_fn_name(domain, binding.extern_name);
            let is_recordable = matches!(
                entry.effect_class,
                EffectClass::External {
                    replay: ReplayPolicy::Recordable
                }
            );
            let replay_kind = entry.replay_kind;
            let uses_binding_replay = is_recordable && replay_kind == BindingReplayKind::Regular;
            let mut params = Vec::new();
            let mut args = Vec::new();
            if entry.return_binding != BindingType::Void {
                let out_type = native_type_for_binding(domain, &entry.return_binding);
                params.push(format!("out: *mut {out_type}"));
                args.push("out".to_string());
            }
            for (index, param) in entry.parameters.iter().enumerate() {
                let name = sanitize_param_name(&param.name, index);
                let ty = native_type_for_binding(domain, &param.binding_type);
                params.push(format!("{name}: {ty}"));
                args.push(name);
            }

            output.push_str(&format!(
                "#[unsafe(export_name = \"{}\")]\n",
                binding.extern_name
            ));
            output.push_str(&format!("pub unsafe extern \"C\" fn {export_fn_name}(\n"));
            for param in &params {
                output.push_str(&format!("    {param},\n"));
            }
            output.push_str(") -> RuntimeStatus {\n");
            output.push_str("    native_call(|context| {\n");
            if entry.return_binding != BindingType::Void {
                output.push_str("        if out.is_null() {\n");
                output.push_str(
                    "            return Err(RuntimeError::from(PlatformError::null_pointer(\"out\")).boxed());\n",
                );
                output.push_str("        }\n");
            }
            if !args.is_empty() {
                let args_refs = args
                    .iter()
                    .map(|name| format!("&{name}"))
                    .collect::<Vec<_>>();
                if args_refs.len() == 1 {
                    output.push_str(&format!("        let _ = {};\n", args_refs[0]));
                } else {
                    output.push_str(&format!("        let _ = ({});\n", args_refs.join(", ")));
                }
            }
            output.push_str("\n");
            match replay_kind {
                BindingReplayKind::Regular => {
                    if uses_binding_replay {
                        output.push_str(&format!(
                            "        context.check_policy({})?;\n",
                            binding.const_name
                        ));
                        if entry.scope != BindingScope::Runtime {
                            output.push_str(&format!(
                                "        let world = context.check_and_resolve_world({})?;\n",
                                binding.const_name
                            ));
                        }
                        if args.is_empty() {
                            if entry.scope == BindingScope::Runtime {
                                output.push_str(&format!("        {replay_fn_name}(context)\n"));
                            } else {
                                output.push_str(&format!(
                                    "        {replay_fn_name}(context, world)\n"
                                ));
                            }
                        } else {
                            if entry.scope == BindingScope::Runtime {
                                output.push_str(&format!(
                                    "        {replay_fn_name}(context, {})\n",
                                    args.join(", ")
                                ));
                            } else {
                                output.push_str(&format!(
                                    "        {replay_fn_name}(context, world, {})\n",
                                    args.join(", ")
                                ));
                            }
                        }
                    } else {
                        let call = render_native_checked_world_dispatch_expr(
                            &binding.const_name,
                            entry.scope,
                            implementation_fn_name,
                            &args,
                        );
                        output.push_str(&format!("        {call}\n"));
                    }
                }
                BindingReplayKind::Time(kind) => {
                    let kind_value = time_event_kind_value(kind);
                    output.push_str(&format!(
                        "        let value = context.replay().run_time_read({kind_value}, || {{\n"
                    ));
                    let call = render_native_checked_world_dispatch_expr(
                        &binding.const_name,
                        entry.scope,
                        implementation_fn_name,
                        &args,
                    );
                    output.push_str(&format!("            {call}?;\n"));
                    output.push_str("            unsafe { Ok(*out) }\n");
                    output.push_str("        })?;\n");
                    output.push_str("        unsafe { *out = value; }\n");
                    output.push_str("        Ok(())\n");
                }
                BindingReplayKind::Random(kind) => match kind {
                    RandomEventKind::Stream => {
                        output.push_str(
                            "        let value = context.replay().run_random_stream(|| {\n",
                        );
                        let call = render_native_checked_world_dispatch_expr(
                            &binding.const_name,
                            entry.scope,
                            implementation_fn_name,
                            &args,
                        );
                        output.push_str(&format!("            {call}?;\n"));
                        output.push_str("            unsafe { Ok((*out).0) }\n");
                        output.push_str("        })?;\n");
                        output.push_str("        unsafe { *out = RandomStream(value); }\n");
                        output.push_str("        Ok(())\n");
                    }
                    RandomEventKind::NextU64 => {
                        let stream_expr =
                            binding_random_stream_id_expr(entry, "context.random_stream_id()");
                        output.push_str(&format!(
                            "        let value = context.replay().run_random_u64({stream_expr}, || {{\n"
                        ));
                        let call = render_native_checked_world_dispatch_expr(
                            &binding.const_name,
                            entry.scope,
                            implementation_fn_name,
                            &args,
                        );
                        output.push_str(&format!("            {call}?;\n"));
                        output.push_str("            unsafe { Ok(*out) }\n");
                        output.push_str("        })?;\n");
                        output.push_str("        unsafe { *out = value; }\n");
                        output.push_str("        Ok(())\n");
                    }
                    RandomEventKind::Bytes => {
                        let stream_expr =
                            binding_random_stream_id_expr(entry, "context.random_stream_id()");
                        let buffer_name = binding_random_bytes_buffer_arg(entry);
                        output.push_str("        context.replay().run_random_bytes(\n");
                        output.push_str(&format!("            {stream_expr},\n"));
                        output.push_str("            || {\n");
                        let call = render_native_checked_world_dispatch_expr(
                            &binding.const_name,
                            entry.scope,
                            implementation_fn_name,
                            &args,
                        );
                        output.push_str(&format!("                {call}\n"));
                        output.push_str("            },\n");
                        output.push_str(&format!(
                            "            || {{\n                let slice = unsafe {{ {buffer_name}.as_slice()? }};\n                Ok(slice.to_vec())\n            }},\n"
                        ));
                        output.push_str(&format!(
                            "            |bytes| {{\n                let slice = unsafe {{ {buffer_name}.as_mut_slice()? }};\n                if slice.len() != bytes.len() {{\n                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(\n                        \"buffer\",\n                        \"buffer length mismatch\",\n                    ))\n                    .boxed());\n                }}\n                slice.copy_from_slice(&bytes);\n                Ok(())\n            }},\n"
                        ));
                        output.push_str("        )\n");
                    }
                    RandomEventKind::Seed => {
                        panic!("random seed replay is not supported for bindings yet");
                    }
                },
            }
            output.push_str("    })\n");
            output.push_str("}\n\n");
        }
    }

    /// Render the binding registration function for a domain.
    fn write_vm_register_fn(&mut self) {
        let output = &mut self.output;
        let domain = self.spec.domain;
        let consts = &self.spec.consts;
        let register_fn = register_fn_name(domain);

        // render the registration function
        output.push_str(&format!("/// Register VM bindings for {domain}.\n"));
        output.push_str(&format!("pub fn {register_fn}(\n"));
        output.push_str("    registry: &mut BindingRegistry,\n");
        output.push_str("    isolate: &mut Isolate,\n");
        output.push_str(") {\n");
        for binding in consts {
            let decode_base_name = vm_fn_name(domain, binding.extern_name);
            let implementation_fn_name = &binding.implementation_fn_name;
            let args_ident = if binding.entry.parameters.is_empty() {
                "_args"
            } else {
                "args"
            };
            let is_recordable = matches!(
                binding.entry.effect_class,
                EffectClass::External {
                    replay: ReplayPolicy::Recordable
                }
            );
            let replay_kind = binding.entry.replay_kind;
            let uses_binding_replay = is_recordable && replay_kind == BindingReplayKind::Regular;
            let invoke_args = render_invoke_args_with_prefix(binding.entry);
            let decode_helper = decode_helper_name(&decode_base_name);
            let encode_helper = encode_helper_name(&decode_base_name);

            // emit the binding wrapper closure
            output.push_str("    {\n");
            output.push_str(&format!(
                "        binding!(registry, isolate, {}, move |context, {args_ident}| {{\n",
                binding.const_name
            ));
            output.push_str("            with_runtime_call_context(|runtime| {\n");
            if !binding.entry.parameters.is_empty() {
                output.push_str("                // decode args\n");
                let arg_names = binding
                    .entry
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(index, param)| sanitize_param_name(&param.name, index))
                    .collect::<Vec<_>>();
                if arg_names.len() == 1 {
                    output.push_str(&format!(
                        "                let ({},) = {decode_helper}(context, {args_ident})?;\n",
                        arg_names[0]
                    ));
                } else {
                    output.push_str(&format!(
                        "                let ({}) = {decode_helper}(context, {args_ident})?;\n",
                        arg_names.join(", ")
                    ));
                }
                output.push_str("\n");
            }
            output.push_str("                // execute binding\n");
            match replay_kind {
                BindingReplayKind::Regular => {
                    if uses_binding_replay {
                        let replay_fn = vm_replay_fn_name(domain, binding.extern_name);
                        output.push_str(&format!(
                            "                runtime.check_policy({})?;\n",
                            binding.const_name
                        ));
                        if binding.entry.scope != BindingScope::Runtime {
                            output.push_str(&format!(
                                "                let world = runtime.check_and_resolve_world({})?;\n",
                                binding.const_name
                            ));
                            output.push_str(&format!(
                                "                {replay_fn}(runtime, context, world{invoke_args})\n"
                            ));
                        } else {
                            output.push_str(&format!(
                                "                {replay_fn}(runtime, context{invoke_args})\n"
                            ));
                        }
                    } else {
                        let call = render_vm_checked_world_dispatch_expr(
                            &binding.const_name,
                            binding.entry.scope,
                            implementation_fn_name,
                            &invoke_args,
                        );
                        output.push_str(&format!("                let result = {call};\n"));
                        output.push_str(&format!(
                            "                {encode_helper}(context, result)\n"
                        ));
                    }
                }
                BindingReplayKind::Time(kind) => {
                    let kind_value = time_event_kind_value(kind);
                    let call = render_vm_checked_world_dispatch_expr(
                        &binding.const_name,
                        binding.entry.scope,
                        implementation_fn_name,
                        &invoke_args,
                    );
                    output.push_str(&format!(
                        "                let result = runtime.replay().run_time_read({kind_value}, || {{\n"
                    ));
                    output.push_str(&format!("                    {call}\n"));
                    output.push_str("                });\n");
                    output.push_str(&format!(
                        "                {encode_helper}(context, result)\n"
                    ));
                }
                BindingReplayKind::Random(kind) => match kind {
                    RandomEventKind::Stream => {
                        let call = render_vm_checked_world_dispatch_expr(
                            &binding.const_name,
                            binding.entry.scope,
                            implementation_fn_name,
                            &invoke_args,
                        );
                        output.push_str(
                            "                let result = runtime.replay().run_random_stream(|| {\n",
                        );
                        output.push_str(&format!(
                            "                    {call}.map(|stream| stream.0)\n"
                        ));
                        output.push_str("                });\n");
                        output.push_str("                let result = result.map(RandomStream);\n");
                        output.push_str(&format!(
                            "                {encode_helper}(context, result)\n"
                        ));
                    }
                    RandomEventKind::NextU64 => {
                        let stream_expr = binding_random_stream_id_expr(
                            &binding.entry,
                            "runtime.random_stream_id()",
                        );
                        let call = render_vm_checked_world_dispatch_expr(
                            &binding.const_name,
                            binding.entry.scope,
                            implementation_fn_name,
                            &invoke_args,
                        );
                        output.push_str(&format!(
                            "                let result = runtime.replay().run_random_u64({stream_expr}, || {{\n"
                        ));
                        output.push_str(&format!("                    {call}\n"));
                        output.push_str("                });\n");
                        output.push_str(&format!(
                            "                {encode_helper}(context, result)\n"
                        ));
                    }
                    RandomEventKind::Bytes => {
                        let stream_expr = binding_random_stream_id_expr(
                            &binding.entry,
                            "runtime.random_stream_id()",
                        );
                        let buffer_name = binding_random_bytes_buffer_arg(&binding.entry);
                        output.push_str(
                            "                let context_ptr = context as *mut vm::ExternalCallContext<'_>;\n",
                        );
                        output.push_str(
                            "                let result = runtime.replay().run_random_bytes(\n",
                        );
                        output.push_str(&format!("                    {stream_expr},\n"));
                        output.push_str("                    || {\n");
                        let call = render_vm_checked_world_dispatch_expr(
                            &binding.const_name,
                            binding.entry.scope,
                            implementation_fn_name,
                            &invoke_args,
                        )
                        .replace("context", "&mut *context_ptr");
                        output.push_str(&format!("                        unsafe {{ {call} }}\n"));
                        output.push_str("                    },\n");
                        output.push_str(&format!(
                            "                    || unsafe {{ {buffer_name}.read_bytes(&*context_ptr) }},\n"
                        ));
                        output.push_str(&format!(
                            "                    |bytes| unsafe {{ {buffer_name}.write_bytes(&mut *context_ptr, &bytes) }},\n"
                        ));
                        output.push_str("                );\n");
                        output.push_str(&format!(
                            "                {encode_helper}(context, result)\n"
                        ));
                    }
                    RandomEventKind::Seed => {
                        panic!("random seed replay is not supported for bindings yet");
                    }
                },
            }
            output.push_str("            })\n");
            output.push_str("            .map_err(Into::into)\n");
            output.push_str("        });\n");
            output.push_str("    }\n");
        }
        output.push_str("}\n\n");
    }

    /// Render the VM binding set for a domain.
    fn write_vm_set(&mut self) {
        let output = &mut self.output;
        let domain = self.spec.domain;
        let register_fn = register_fn_name(domain);
        let set_name = vm_set_name_for_domain(domain);
        let install_fn = install_fn_name(domain);

        output.push_str(&format!("/// Install VM bindings for {domain}.\n"));
        output.push_str(&format!("pub fn {install_fn}(\n"));
        output.push_str("    registry: &mut BindingRegistry,\n");
        output.push_str("    isolate: &mut Isolate,\n");
        output.push_str(") {\n");
        output.push_str(&format!("    {register_fn}(registry, isolate);\n"));
        output.push_str("}\n\n");

        output.push_str(&format!(
            "vm_binding_set!(pub {set_name}, \"{domain}\", {install_fn});\n\n"
        ));
    }
}

fn time_event_kind_value(kind: TimeEventKind) -> &'static str {
    match kind {
        TimeEventKind::Seed => "TimeEventKind::Seed",
        TimeEventKind::MonotonicSample => "TimeEventKind::MonotonicSample",
        TimeEventKind::WallClockRead => "TimeEventKind::WallClockRead",
        TimeEventKind::TimerScheduled => "TimeEventKind::TimerScheduled",
        TimeEventKind::TimerFired => "TimeEventKind::TimerFired",
        TimeEventKind::TimerCanceled => "TimeEventKind::TimerCanceled",
        TimeEventKind::SleepScheduled => "TimeEventKind::SleepScheduled",
        TimeEventKind::SleepWake => "TimeEventKind::SleepWake",
    }
}

/// Render one native implementation call expression with policy/world dispatch.
fn render_native_checked_world_dispatch_expr(
    binding_const: &str,
    scope: BindingScope,
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

    let call = if args.is_empty() {
        format!("unsafe {{ platform_native::{implementation_fn_name}(context) }}")
    } else {
        format!(
            "unsafe {{ platform_native::{implementation_fn_name}(context, {}) }}",
            args.join(", ")
        )
    };

    if scope == BindingScope::Runtime {
        return format!("{{ context.check_policy({binding_const})?; {runtime_call} }}");
    }

    let simulated = if args.is_empty() {
        format!("unsafe {{ platform_simulated_native::{implementation_fn_name}(context) }}")
    } else {
        format!(
            "unsafe {{ platform_simulated_native::{implementation_fn_name}(context, {}) }}",
            args.join(", ")
        )
    };

    format!(
        "{{\n            context.check_policy({binding_const})?;\n            let world = context.check_and_resolve_world({binding_const})?;\n            match world {{\n                RuntimeWorld::Host => {call},\n                RuntimeWorld::Simulated => {simulated},\n            }}\n        }}"
    )
}

/// Render one VM implementation call expression with policy/world dispatch.
fn render_vm_checked_world_dispatch_expr(
    binding_const: &str,
    scope: BindingScope,
    implementation_fn_name: &str,
    invoke_args: &str,
) -> String {
    let runtime_call =
        format!("platform_runtime_vm::{implementation_fn_name}(runtime, context{invoke_args})");
    let call = format!("platform_vm::{implementation_fn_name}(runtime, context{invoke_args})");

    if scope == BindingScope::Runtime {
        return format!("{{ runtime.check_policy({binding_const})?; {runtime_call} }}");
    }

    let simulated =
        format!("platform_simulated_vm::{implementation_fn_name}(runtime, context{invoke_args})");

    format!(
        "{{\n                        runtime.check_policy({binding_const})?;\n                        let world = runtime.check_and_resolve_world({binding_const})?;\n                        match world {{\n                            RuntimeWorld::Host => {call},\n                            RuntimeWorld::Simulated => {simulated},\n                        }}\n                    }}"
    )
}

fn random_event_kind_value(kind: RandomEventKind) -> &'static str {
    match kind {
        RandomEventKind::Seed => "RandomEventKind::Seed",
        RandomEventKind::Stream => "RandomEventKind::Stream",
        RandomEventKind::Bytes => "RandomEventKind::Bytes",
        RandomEventKind::NextU64 => "RandomEventKind::NextU64",
    }
}

fn render_binding_replay_kind(kind: BindingReplayKind) -> String {
    match kind {
        BindingReplayKind::Regular => "BindingReplayKind::Regular".to_string(),
        BindingReplayKind::Time(time_kind) => format!(
            "BindingReplayKind::Time({})",
            time_event_kind_value(time_kind)
        ),
        BindingReplayKind::Random(random_kind) => format!(
            "BindingReplayKind::Random({})",
            random_event_kind_value(random_kind)
        ),
    }
}

/// Render a binding scope constant.
fn render_binding_scope(scope: BindingScope) -> String {
    match scope {
        BindingScope::Os => "BindingScope::Os".to_string(),
        BindingScope::Runtime => "BindingScope::Runtime".to_string(),
        BindingScope::Hybrid => "BindingScope::Hybrid".to_string(),
    }
}

/// Render a binding blocking constant.
fn render_binding_blocking(blocking: BindingBlocking) -> String {
    match blocking {
        BindingBlocking::Always => "BindingBlocking::Always".to_string(),
        BindingBlocking::Never => "BindingBlocking::Never".to_string(),
        BindingBlocking::Sometimes => "BindingBlocking::Sometimes".to_string(),
    }
}

/// Render required capability literals for a descriptor constant.
fn render_binding_requires(requires: &[String]) -> String {
    if requires.is_empty() {
        return "&[]".to_string();
    }

    let mut values = String::new();
    values.push_str("&[");
    for (index, capability) in requires.iter().enumerate() {
        if index > 0 {
            values.push_str(", ");
        }
        values.push('"');
        values.push_str(&escape_rust_string(capability));
        values.push('"');
    }
    values.push(']');
    values
}

/// Render host platform literals for a descriptor constant.
fn render_binding_host_platforms(host_platforms: &[String]) -> Option<String> {
    if host_platforms.is_empty() {
        return None;
    }

    let mut values = String::new();
    values.push_str("&[");
    for (index, platform) in host_platforms.iter().enumerate() {
        if index > 0 {
            values.push_str(", ");
        }
        values.push('"');
        values.push_str(&escape_rust_string(platform));
        values.push('"');
    }
    values.push(']');
    Some(values)
}

/// Convert a domain into a module safe identifier.
fn sanitize_module_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("bindings");
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// Convert a binding name into a constant identifier.
fn const_name_for_extern(extern_name: &str) -> String {
    let tail = binding_suffix_for_extern(extern_name, None);
    let mut out = String::new();
    for ch in snake_case(tail.as_str()).chars() {
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

/// Build a stable identifier suffix for one extern binding name.
fn binding_suffix_for_extern(extern_name: &str, domain: Option<&str>) -> String {
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
        let value = snake_case(part);
        if !value.is_empty() {
            suffix_parts.push(value);
        }
    }

    if suffix_parts.is_empty() {
        return "binding".to_string();
    }

    suffix_parts.join("_")
}

/// Escape a string for embedding in Rust source.
fn escape_rust_string(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        for escaped in ch.escape_default() {
            out.push(escaped);
        }
    }
    out
}

/// Build the register function name for a domain.
fn register_fn_name(domain: &str) -> String {
    format!("register_{}_vm_bindings", sanitize_module_name(domain))
}

/// Build the VM binding set constant name for a domain.
fn vm_set_name_for_domain(domain: &str) -> String {
    format!("{}_VM_BINDINGS", const_name_for_extern(domain))
}

/// Build the install function name for a domain.
fn install_fn_name(domain: &str) -> String {
    format!("install_{}_vm_bindings", sanitize_module_name(domain))
}

/// Build the native binding set constant name for a domain.
fn native_set_name_for_domain(domain: &str) -> String {
    format!("{}_NATIVE_BINDINGS", const_name_for_extern(domain))
}

/// Build the handler method name for an extern binding.
/// Build the native symbol name for a binding.
pub(crate) fn native_fn_name(domain: &str, extern_name: &str) -> String {
    let suffix = binding_suffix_for_extern(extern_name, Some(domain));
    format!("destack_{domain}_{suffix}")
}

/// Build the runtime implementation function name for a binding declaration.
pub(crate) fn implementation_fn_name(domain: &str, implementation_name: &str) -> String {
    format!("destack_{domain}_{}", snake_case(implementation_name))
}

/// Build the native replay helper name for a binding.
pub(super) fn native_replay_fn_name(domain: &str, extern_name: &str) -> String {
    format!("{}_replay", native_fn_name(domain, extern_name))
}

/// Build the VM handler name for a binding.
pub(super) fn vm_fn_name(domain: &str, extern_name: &str) -> String {
    native_fn_name(domain, extern_name)
}

/// Build the VM replay helper name for a binding.
pub(super) fn vm_replay_fn_name(domain: &str, extern_name: &str) -> String {
    format!("{}_vm_replay", native_fn_name(domain, extern_name))
}

/// Build the decode helper name for a binding.
fn decode_helper_name(base: &str) -> String {
    format!("decode_{base}_args")
}

/// Build the encode helper name for a binding.
pub(super) fn encode_helper_name(base: &str) -> String {
    format!("encode_{base}_result")
}

/// Convert a string to snake case.
fn snake_case(name: &str) -> String {
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

/// Render the parameter list for a binding entry.
fn render_params(domain: &str, entry: &BindingEntry) -> Vec<String> {
    entry
        .parameters
        .iter()
        .enumerate()
        .map(|(index, param)| {
            let rust_type = vm_type_for_binding(domain, &param.binding_type);
            let name = sanitize_param_name(&param.name, index);
            format!("{name}: {rust_type}")
        })
        .collect()
}

/// Render the return type for a binding entry.
fn render_return_type(domain: &str, entry: &BindingEntry) -> String {
    vm_type_for_binding(domain, &entry.return_binding)
}

/// Build a qualified path for a named binding type.
pub(super) fn named_type_path(domain: &str, type_domain: &str, name: &str) -> String {
    if type_domain == domain {
        name.to_string()
    } else {
        format!("{type_domain}::{name}")
    }
}

/// Build a stable module alias for a platform domain.
fn platform_domain_alias(domain: &str) -> String {
    let module_name = sanitize_module_name(domain);
    format!("platform_{module_name}")
}

/// Build a qualified ABI struct path for a named binding type.
fn struct_abi_path(domain: &str, type_domain: &str, name: &str, abi: &str) -> String {
    let alias = if type_domain == domain {
        platform_domain_alias(domain)
    } else {
        platform_domain_alias(type_domain)
    };

    format!("{alias}::{name}Abi<{abi}>")
}

/// Build a qualified ABI newtype path for a named binding type.
fn newtype_abi_path(domain: &str, type_domain: &str, name: &str, abi: &str) -> String {
    let alias = if type_domain == domain {
        platform_domain_alias(domain)
    } else {
        platform_domain_alias(type_domain)
    };

    format!("{alias}::{name}Abi<{abi}>")
}

/// Build a qualified ABI newtype constructor path.
fn newtype_abi_constructor_path(domain: &str, type_domain: &str, name: &str, abi: &str) -> String {
    let alias = if type_domain == domain {
        platform_domain_alias(domain)
    } else {
        platform_domain_alias(type_domain)
    };

    format!("{alias}::{name}Abi::<{abi}>")
}

/// Build a qualified VM alias path for a struct type.
fn struct_vm_path(domain: &str, type_domain: &str, name: &str) -> String {
    let vm_name = format!("{name}Vm");
    named_type_path(domain, type_domain, vm_name.as_str())
}

/// Build a qualified VM alias path for a newtype.
fn newtype_vm_path(domain: &str, type_domain: &str, name: &str) -> String {
    let vm_name = format!("{name}Vm");
    named_type_path(domain, type_domain, vm_name.as_str())
}

/// Convert a binding type into a native ABI type.
pub(super) fn native_type_for_binding(domain: &str, binding_type: &BindingType) -> String {
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
            format!("NativeSlice<{}>", native_type_for_binding(domain, inner))
        }
        BindingType::Array(inner) => {
            format!("NativeArray<{}>", native_type_for_binding(domain, inner))
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
        } => {
            if domain == "error" && type_domain == "error" && name == "PlatformError" {
                "platform_error::PlatformError".to_string()
            } else {
                named_type_path(domain, type_domain, name)
            }
        }
        BindingType::Int(_) | BindingType::UInt(_) | BindingType::Float(_) => {
            panic!("unsupported numeric width for native bindings")
        }
    }
}

/// Collect named binding types referenced by bindings.
fn collect_native_named_types(domain: &str, bindings: &BindingCatalogEntry) -> BTreeSet<String> {
    // track unique names for imports
    let mut names = BTreeSet::new();

    // walk each binding signature
    for entry in bindings.values() {
        collect_signature_type_names(domain, &entry.return_binding, &mut names, "");
        for param in &entry.parameters {
            collect_signature_type_names(domain, &param.binding_type, &mut names, "");
        }
    }

    names
}

/// Collect named types required by simulated native stub signatures.
fn collect_native_stub_named_types(
    domain: &str,
    bindings: &BindingCatalogEntry,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for entry in bindings.values() {
        collect_signature_stub_type_names(domain, &entry.return_binding, &mut names, "");
        for param in &entry.parameters {
            collect_signature_stub_type_names(domain, &param.binding_type, &mut names, "");
        }
    }

    names
}

/// Collect VM-named types required for decoding collection elements.
pub(super) fn collect_collection_vm_names(
    domain: &str,
    binding_type: &BindingType,
    names: &mut BTreeSet<String>,
) {
    match binding_type {
        BindingType::StringSlice => {}
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            collect_collection_vm_type_names(domain, inner, names);
        }
        _ => {}
    }
}

fn collect_collection_vm_type_names(
    domain: &str,
    binding_type: &BindingType,
    names: &mut BTreeSet<String>,
) {
    match binding_type {
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            collect_collection_vm_type_names(domain, inner, names);
        }
        BindingType::Newtype {
            name,
            domain: type_domain,
            inner,
        } => {
            if binding_type_requires_abi(inner) {
                collect_collection_vm_type_names(domain, inner, names);
            } else if type_domain == domain {
                names.insert(format!("{name}Vm"));
            }
        }
        BindingType::Struct {
            name,
            domain: type_domain,
            fields,
        } => {
            if type_domain == domain {
                names.insert(format!("{name}Vm"));
            }
            for field in fields {
                collect_collection_vm_type_names(domain, &field.binding_type, names);
            }
        }
        BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => {
            if type_domain == domain {
                names.insert(name.clone());
            }
        }
        _ => {}
    }
}

/// Collect platform module names referenced by binding types.
fn collect_type_domains(domain: &str, bindings: &BindingCatalogEntry) -> BTreeSet<String> {
    let mut domains = BTreeSet::new();

    for entry in bindings.values() {
        collect_binding_type_domains(domain, &entry.return_binding, &mut domains);
        for param in &entry.parameters {
            collect_binding_type_domains(domain, &param.binding_type, &mut domains);
        }
    }

    domains
}

fn collect_binding_type_domains(
    domain: &str,
    binding_type: &BindingType,
    domains: &mut BTreeSet<String>,
) {
    match binding_type {
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            collect_binding_type_domains(domain, inner, domains);
        }
        BindingType::Newtype {
            domain: type_domain,
            inner,
            ..
        } => {
            if type_domain != domain {
                domains.insert(type_domain.clone());
            }
            collect_binding_type_domains(domain, inner, domains);
        }
        BindingType::Struct {
            domain: type_domain,
            fields,
            ..
        } => {
            if type_domain != domain {
                domains.insert(type_domain.clone());
            }
            for field in fields {
                collect_binding_type_domains(domain, &field.binding_type, domains);
            }
        }
        BindingType::Enum {
            domain: type_domain,
            ..
        } => {
            if type_domain != domain {
                domains.insert(type_domain.clone());
            }
        }
        _ => {}
    }
}

/// Usage flags for VM stub imports.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct VmStubUsage {
    /// VM slice types are referenced.
    uses_vm_slice: bool,
    /// VM array types are referenced.
    uses_vm_array: bool,
}

/// Collect VM stub usage flags from bindings.
fn collect_vm_stub_usage(bindings: &BindingCatalogEntry) -> VmStubUsage {
    let mut usage = VmStubUsage::default();
    for entry in bindings.values() {
        collect_vm_stub_usage_for_binding(&entry.return_binding, &mut usage);
        for param in &entry.parameters {
            collect_vm_stub_usage_for_binding(&param.binding_type, &mut usage);
        }
    }
    usage
}

/// Collect VM stub usage for a binding type.
fn collect_vm_stub_usage_for_binding(binding_type: &BindingType, usage: &mut VmStubUsage) {
    match binding_type {
        BindingType::StringSlice => usage.uses_vm_slice = true,
        BindingType::Slice(inner) => {
            usage.uses_vm_slice = true;
            collect_vm_stub_usage_for_binding(inner, usage);
        }
        BindingType::Array(inner) => {
            usage.uses_vm_array = true;
            collect_vm_stub_usage_for_binding(inner, usage);
        }
        BindingType::Newtype { inner, .. } => {
            collect_vm_stub_usage_for_binding(inner, usage);
        }
        BindingType::Struct { fields, .. } => {
            for field in fields {
                collect_vm_stub_usage_for_binding(&field.binding_type, usage);
            }
        }
        _ => {}
    }
}

/// Collect VM-visible named types.
fn collect_vm_named_types(domain: &str, bindings: &BindingCatalogEntry) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for entry in bindings.values() {
        collect_signature_type_names(domain, &entry.return_binding, &mut names, "Vm");
        for param in &entry.parameters {
            collect_signature_type_names(domain, &param.binding_type, &mut names, "Vm");
        }
    }
    names
}

/// Collect named types required by simulated VM stub signatures.
fn collect_vm_stub_named_types(domain: &str, bindings: &BindingCatalogEntry) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for entry in bindings.values() {
        collect_signature_stub_type_names(domain, &entry.return_binding, &mut names, "Vm");
        for param in &entry.parameters {
            collect_signature_stub_type_names(domain, &param.binding_type, &mut names, "Vm");
        }
    }

    names
}

/// Walk a binding type and record named types referenced in signatures.
fn collect_signature_type_names(
    domain: &str,
    binding_type: &BindingType,
    names: &mut BTreeSet<String>,
    struct_suffix: &str,
) {
    match binding_type {
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            collect_signature_type_names(domain, inner, names, struct_suffix);
        }
        BindingType::Newtype {
            name,
            domain: type_domain,
            inner,
            ..
        } => {
            if type_domain == domain {
                let requires_abi = binding_type_requires_abi(inner);
                if !struct_suffix.is_empty() && requires_abi {
                    names.insert(format!("{name}{struct_suffix}"));
                } else {
                    names.insert(name.clone());
                }
            }
            collect_signature_type_names(domain, inner, names, struct_suffix);
        }
        BindingType::Struct {
            name,
            domain: type_domain,
            fields,
            ..
        } => {
            if type_domain == domain {
                names.insert(format!("{name}{struct_suffix}"));
            }
            for field in fields {
                collect_signature_type_names(domain, &field.binding_type, names, struct_suffix);
            }
        }
        BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => {
            if type_domain == domain {
                names.insert(name.clone());
            }
        }
        _ => {}
    }
}

/// Walk a binding type and record only named types that appear in function signatures.
///
/// This skips nested field recursion so generated simulated stubs do not import
/// transitive types that are never referenced by the signature itself.
fn collect_signature_stub_type_names(
    domain: &str,
    binding_type: &BindingType,
    names: &mut BTreeSet<String>,
    struct_suffix: &str,
) {
    match binding_type {
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            collect_signature_stub_type_names(domain, inner, names, struct_suffix);
        }
        BindingType::Newtype {
            name,
            domain: type_domain,
            inner,
            ..
        } => {
            if type_domain == domain {
                let requires_abi = binding_type_requires_abi(inner);
                if !struct_suffix.is_empty() && requires_abi {
                    names.insert(format!("{name}{struct_suffix}"));
                } else {
                    names.insert(name.clone());
                }
            }
        }
        BindingType::Struct {
            name,
            domain: type_domain,
            ..
        } => {
            if type_domain == domain {
                names.insert(format!("{name}{struct_suffix}"));
            }
        }
        BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => {
            if type_domain == domain {
                names.insert(name.clone());
            }
        }
        _ => {}
    }
}

/// Return true if decoding a binding type requires the VM module.
fn binding_type_requires_vm_for_decode(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::String
        | BindingType::StringSlice
        | BindingType::Slice(_)
        | BindingType::Array(_)
        | BindingType::Struct { .. } => true,
        BindingType::Newtype { inner, .. } => binding_type_requires_vm_for_decode(inner),
        BindingType::Enum { backing, .. } => matches!(backing, EnumBackingType::String),
        _ => false,
    }
}

/// Return true if encoding a binding type requires the VM module.
fn binding_type_requires_vm_for_encode(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::Void
        | BindingType::Bool
        | BindingType::Int(_)
        | BindingType::UInt(_)
        | BindingType::Float(_) => true,
        BindingType::Newtype { inner, .. } => binding_type_requires_vm_for_encode(inner),
        BindingType::Enum { backing, .. } => matches!(backing, EnumBackingType::Int(_)),
        BindingType::Struct { fields, .. } => fields
            .iter()
            .any(|field| binding_type_requires_vm_for_encode(&field.binding_type)),
        _ => false,
    }
}

/// Return true if decoding a binding type needs a runtime context.
fn binding_type_requires_context_for_decode(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::StringSlice | BindingType::Slice(_) | BindingType::Array(_) => true,
        BindingType::Struct { .. } => true,
        BindingType::Enum { backing, .. } => matches!(backing, EnumBackingType::String),
        BindingType::Newtype { inner, .. } => binding_type_requires_context_for_decode(inner),
        _ => false,
    }
}

/// Return true if encoding a binding type needs a runtime context.
fn binding_type_requires_context_for_encode(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::StringSlice | BindingType::Slice(_) | BindingType::Array(_) => true,
        BindingType::Struct { .. } => true,
        BindingType::Enum { backing, .. } => matches!(backing, EnumBackingType::String),
        BindingType::Newtype { inner, .. } => binding_type_requires_context_for_encode(inner),
        _ => false,
    }
}

/// Return true if a binding type depends on the ABI parameter.
pub(super) fn binding_type_requires_abi(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::String
        | BindingType::StringSlice
        | BindingType::Slice(_)
        | BindingType::Array(_) => true,
        BindingType::Struct { fields, .. } => fields
            .iter()
            .any(|field| binding_type_requires_abi(&field.binding_type)),
        BindingType::Newtype { inner, .. } => binding_type_requires_abi(inner),
        _ => false,
    }
}

/// Return the Rust repr attribute for an enum backing type.
fn enum_backing_repr(backing: EnumBackingType) -> &'static str {
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

/// Render an ABI field type for a binding type.
fn abi_struct_field_type(domain: &str, binding_type: &BindingType) -> String {
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
        BindingType::Slice(inner) => format!("A::Slice<{}>", abi_struct_field_type(domain, inner)),
        BindingType::Array(inner) => format!("A::Array<{}>", abi_struct_field_type(domain, inner)),
        BindingType::Newtype {
            name,
            domain: type_domain,
            inner,
        } => {
            if binding_type_requires_abi(inner) {
                newtype_abi_path(domain, type_domain, name, "A")
            } else {
                named_type_path(domain, type_domain, name)
            }
        }
        BindingType::Struct {
            name,
            domain: type_domain,
            ..
        } => {
            if binding_type_requires_abi(binding_type) {
                struct_abi_path(domain, type_domain, name, "A")
            } else {
                named_type_path(domain, type_domain, name)
            }
        }
        BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => named_type_path(domain, type_domain, name),
    }
}

/// Render a newtype inner ABI type.
fn abi_newtype_inner_type(domain: &str, binding_type: &BindingType) -> String {
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
        BindingType::Newtype {
            name,
            domain: type_domain,
            inner,
        } => {
            if binding_type_requires_abi(inner) {
                newtype_vm_path(domain, type_domain, name)
            } else {
                named_type_path(domain, type_domain, name)
            }
        }
        BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => named_type_path(domain, type_domain, name),
        BindingType::Struct {
            name,
            domain: type_domain,
            ..
        } => {
            if binding_type_requires_abi(binding_type) {
                struct_abi_path(domain, type_domain, name, "A")
            } else {
                named_type_path(domain, type_domain, name)
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

/// Resolve the VM-facing type for a struct field.
/// Render the argument decoding lines for a binding entry.
/// Render the argument list for invoking a handler.
fn render_invoke_args(entry: &BindingEntry) -> String {
    entry
        .parameters
        .iter()
        .enumerate()
        .map(|(index, param)| sanitize_param_name(&param.name, index))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render the invoke argument list with a leading comma.
pub(super) fn render_invoke_args_with_prefix(entry: &BindingEntry) -> String {
    let args = render_invoke_args(entry);
    if args.is_empty() {
        String::new()
    } else {
        format!(", {args}")
    }
}

/// Render the return encoding lines for a binding type.
fn render_return_encode_lines(domain: &str, binding_type: &BindingType) -> Vec<String> {
    if matches!(binding_type, BindingType::Void) {
        return vec!["result.map(|_| vm::Value::VOID)".to_string()];
    }

    if matches!(binding_type, BindingType::Bool) {
        return vec!["result.map(vm::Value::bool)".to_string()];
    }

    let expr = render_encode_expr(domain, binding_type, "value");
    vec![format!("result.map(|value| {expr})")]
}

/// Build the tuple type for VM binding arguments.
fn vm_args_tuple_type(domain: &str, params: &[BindingParameter]) -> String {
    let mut parts = Vec::new();
    for param in params {
        parts.push(vm_type_for_binding(domain, &param.binding_type));
    }
    if parts.len() == 1 {
        format!("({},)", parts[0])
    } else {
        format!("({})", parts.join(", "))
    }
}

/// Build the VM return type for a binding result.
fn vm_return_type(domain: &str, binding_type: &BindingType) -> String {
    vm_type_for_binding(domain, binding_type)
}

/// Render a VM value expression for an encoded binding value.
fn render_encode_expr(domain: &str, binding_type: &BindingType, value_expr: &str) -> String {
    match binding_type {
        BindingType::Void => "vm::Value::VOID".to_string(),
        BindingType::Bool => format!("vm::Value::bool({value_expr})"),
        BindingType::Int(64) => format!("vm::Value::int({value_expr}, 64)"),
        BindingType::Int(bits) => format!("vm::Value::int({value_expr} as i64, {bits})"),
        BindingType::UInt(64) => format!("vm::Value::uint({value_expr}, 64)"),
        BindingType::UInt(bits) => format!("vm::Value::uint({value_expr} as u64, {bits})"),
        BindingType::Float(32) => format!("vm::Value::float32({value_expr})"),
        BindingType::Float(64) => format!("vm::Value::float64({value_expr})"),
        BindingType::Float(width) => panic!("unsupported float width for VM binding: {width}"),
        BindingType::String => format!("{value_expr}.value()"),
        BindingType::StringSlice | BindingType::Slice(_) | BindingType::Array(_) => {
            format!("{value_expr}.to_value(context)")
        }
        BindingType::Newtype { inner, .. } => {
            let inner_expr = format!("{value_expr}.0");
            render_encode_expr(domain, inner, inner_expr.as_str())
        }
        BindingType::Enum {
            name,
            domain: enum_domain,
            backing,
            variants,
        } => match backing {
            EnumBackingType::Int(_) => render_encode_expr(
                domain,
                &enum_backing_binding_type(*backing),
                &format!("{value_expr} as {}", enum_backing_rust_type(*backing)),
            ),
            EnumBackingType::String => {
                let enum_path = named_type_path(domain, enum_domain, name);
                let mut arms = Vec::new();
                for variant in variants {
                    if let BindingEnumValue::String(value) = &variant.value {
                        arms.push(format!(
                            "{enum_path}::{} => context.intern_string(\"{value}\")",
                            variant.name
                        ));
                    }
                }
                format!(
                    "match {value_expr} {{ {} , _ => context.intern_string(\"\"), }}",
                    arms.join(", ")
                )
            }
        },
        BindingType::Struct { fields, .. } => {
            let mut lines = Vec::new();
            let mut encoded_fields = Vec::new();
            for (index, field) in fields.iter().enumerate() {
                let field_name = to_snake_case(&field.name);
                let field_expr = format!("{value_expr}.{field_name}");
                let encoded_expr = render_encode_expr(domain, &field.binding_type, &field_expr);
                let local_name = format!("field_{index}");
                lines.push(format!("let {local_name} = {encoded_expr};"));
                encoded_fields.push(local_name);
            }
            lines.push(format!(
                "context.allocate_aggregate(vec![{}])",
                encoded_fields.join(", ")
            ));
            format!("{{ {} }}", lines.join(" "))
        }
    }
}

/// Render replay encoding lines for a binding value.
pub(super) fn render_replay_encode_lines(
    domain: &str,
    binding_type: &BindingType,
    name: &str,
    value_expr: &str,
) -> Vec<String> {
    match binding_type {
        BindingType::Void => vec![format!("let {name} = ();")],
        BindingType::Bool
        | BindingType::Int(_)
        | BindingType::UInt(_)
        | BindingType::Float(_)
        | BindingType::Enum { .. } => vec![format!("let {name} = {value_expr};")],
        BindingType::Newtype { inner, .. } => {
            if binding_type_requires_abi(inner) {
                let inner_name = format!("{name}_inner");
                let inner_expr = format!("{value_expr}.0");
                let mut lines =
                    render_replay_encode_lines(domain, inner, &inner_name, inner_expr.as_str());
                lines.push(format!("let {name} = {inner_name};"));
                lines
            } else {
                vec![format!("let {name} = {value_expr};")]
            }
        }
        BindingType::String => vec![
            format!("let {name} = {{"),
            format!(
                "    let {name}_ref = context.string_ref({value_expr}).map_err(|error| RuntimeError::from(error).boxed())?;"
            ),
            format!("    {name}_ref.as_str().to_string()"),
            "};".to_string(),
        ],
        BindingType::StringSlice => {
            render_replay_encode_collection_lines(domain, &BindingType::String, name, value_expr)
        }
        BindingType::Slice(inner) => {
            if matches!(**inner, BindingType::UInt(8)) {
                vec![format!("let {name} = {value_expr}.read_bytes(context)?;")]
            } else {
                render_replay_encode_collection_lines(domain, inner, name, value_expr)
            }
        }
        BindingType::Array(inner) => {
            if matches!(**inner, BindingType::UInt(8)) {
                vec![format!("let {name} = {value_expr}.read_bytes(context)?;")]
            } else {
                render_replay_encode_collection_lines(domain, inner, name, value_expr)
            }
        }
        BindingType::Struct {
            name: struct_name,
            domain: struct_domain,
            fields,
        } => {
            let mut lines = Vec::new();
            let struct_type = if binding_type_requires_abi(binding_type) {
                replay_struct_name(struct_name)
            } else {
                struct_name.clone()
            };
            let mut field_names = Vec::new();
            for field in fields {
                let field_name = to_snake_case(&field.name);
                let field_value = format!("{value_expr}.{field_name}");
                let field_var = format!("{name}_{field_name}");
                lines.extend(render_replay_encode_lines(
                    domain,
                    &field.binding_type,
                    &field_var,
                    &field_value,
                ));
                field_names.push((field_name, field_var));
            }
            let struct_path = named_type_path(domain, struct_domain, struct_type.as_str());
            lines.push(format!("let {name} = {struct_path} {{"));
            for (field_name, field_var) in field_names {
                lines.push(format!("    {field_name}: {field_var},"));
            }
            lines.push("};".to_string());
            lines
        }
    }
}

/// Render replay encoding for a slice or array.
fn render_replay_encode_collection_lines(
    domain: &str,
    inner: &BindingType,
    name: &str,
    value_expr: &str,
) -> Vec<String> {
    let mut lines = Vec::new();
    let raw_var = format!("{name}_raw");
    let item_var = format!("{name}_item");
    let item_recorded_var = format!("{name}_item_recorded");
    lines.push(format!(
        "let {raw_var} = {value_expr}.raw_values(context)?;"
    ));
    lines.push(format!(
        "let mut {name} = Vec::with_capacity({raw_var}.len());"
    ));
    lines.push(format!("for {item_var}_value in {raw_var} {{"));
    lines.extend(
        render_decode_value_lines(
            domain,
            inner,
            &item_var,
            &format!("{item_var}_value"),
            "item",
        )
        .into_iter()
        .map(|line| format!("    {line}")),
    );
    lines.extend(
        render_replay_encode_lines(domain, inner, &item_recorded_var, &item_var)
            .into_iter()
            .map(|line| format!("    {line}")),
    );
    lines.push(format!("    {name}.push({item_recorded_var});"));
    lines.push("}".to_string());
    lines
}

/// Report whether a binding type can be copied without cloning.
fn binding_type_is_copy(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::Void
        | BindingType::Bool
        | BindingType::Int(_)
        | BindingType::UInt(_)
        | BindingType::Float(_)
        | BindingType::Enum { .. }
        | BindingType::StringSlice
        | BindingType::Slice(_)
        | BindingType::Array(_) => true,
        BindingType::Newtype { inner, .. } => binding_type_is_copy(inner),
        BindingType::Struct { fields, .. } => fields
            .iter()
            .all(|field| binding_type_is_copy(&field.binding_type)),
        BindingType::String => false,
    }
}

/// Report whether a replay payload binding type can be copied without cloning.
fn binding_type_is_copy_for_replay(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::Void
        | BindingType::Bool
        | BindingType::Int(_)
        | BindingType::UInt(_)
        | BindingType::Float(_)
        | BindingType::Enum { .. } => true,
        BindingType::Newtype { inner, .. } => binding_type_is_copy_for_replay(inner),
        BindingType::Struct { fields, .. } => fields
            .iter()
            .all(|field| binding_type_is_copy_for_replay(&field.binding_type)),
        BindingType::String
        | BindingType::StringSlice
        | BindingType::Slice(_)
        | BindingType::Array(_) => false,
    }
}

/// Report whether a handle-backed binding type can be copied without cloning.
pub(super) fn binding_type_is_copy_for_handle(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::String => true,
        BindingType::Newtype { inner, .. } => binding_type_is_copy_for_handle(inner),
        BindingType::Struct { fields, .. } => fields
            .iter()
            .all(|field| binding_type_is_copy_for_handle(&field.binding_type)),
        _ => binding_type_is_copy(binding_type),
    }
}

/// Report whether a binding type is the random stream newtype.
fn binding_type_is_random_stream(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::Newtype { name, domain, .. } => {
            domain == "random" && (name == "RandomStream" || name == "RandomStreamId")
        }
        _ => false,
    }
}

/// Locate the random stream argument name, if present.
fn binding_random_stream_arg(binding: &BindingEntry) -> Option<String> {
    binding
        .parameters
        .iter()
        .enumerate()
        .find(|(_, param)| binding_type_is_random_stream(&param.binding_type))
        .map(|(index, param)| sanitize_param_name(&param.name, index))
}

/// Build the random stream id expression for replay helpers.
fn binding_random_stream_id_expr(binding: &BindingEntry, default_expr: &str) -> String {
    binding_random_stream_arg(binding).map_or_else(
        || default_expr.to_string(),
        |name| format!("RandomStreamId::new({name}.0)"),
    )
}

/// Report whether a binding type is a byte buffer slice.
fn binding_type_is_byte_buffer(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            matches!(**inner, BindingType::UInt(8))
        }
        _ => false,
    }
}

/// Locate the byte buffer argument name for random byte bindings.
fn binding_random_bytes_buffer_arg(binding: &BindingEntry) -> String {
    binding
        .parameters
        .iter()
        .enumerate()
        .find(|(_, param)| binding_type_is_byte_buffer(&param.binding_type))
        .map(|(index, param)| sanitize_param_name(&param.name, index))
        .unwrap_or_else(|| {
            panic!("random byte bindings must take a Slice<uint8> or Array<uint8> parameter")
        })
}

/// Report whether a binding type implements VmValueCodec.
fn binding_type_is_vm_value_codec(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::Void
        | BindingType::StringSlice
        | BindingType::Slice(_)
        | BindingType::Array(_)
        | BindingType::Struct { .. } => false,
        BindingType::Bool
        | BindingType::Int(_)
        | BindingType::UInt(_)
        | BindingType::Float(_)
        | BindingType::String
        | BindingType::Enum { .. } => true,
        BindingType::Newtype { inner, .. } => !binding_type_requires_abi(inner),
    }
}

/// Render native replay encoding lines for a binding value.
pub(super) fn render_native_replay_encode_lines(
    domain: &str,
    binding_type: &BindingType,
    name: &str,
    value_expr: &str,
) -> Vec<String> {
    match binding_type {
        BindingType::Void => vec![format!("let {name} = ();")],
        BindingType::Bool
        | BindingType::Int(_)
        | BindingType::UInt(_)
        | BindingType::Float(_)
        | BindingType::Enum { .. } => vec![format!("let {name} = {value_expr};")],
        BindingType::String => vec![format!(
            "let {name} = unsafe {{ {value_expr}.as_str()? }}.to_string();"
        )],
        BindingType::StringSlice => {
            render_native_replay_encode_string_slice_lines(name, value_expr)
        }
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            render_native_replay_encode_collection_lines(domain, inner, name, value_expr)
        }
        BindingType::Newtype { inner, .. } => {
            if binding_type_requires_abi(inner) {
                render_native_replay_encode_lines(domain, inner, name, &format!("{value_expr}.0"))
            } else if binding_type_is_copy(binding_type) {
                vec![format!("let {name} = {value_expr};")]
            } else {
                vec![format!("let {name} = {value_expr}.clone();")]
            }
        }
        BindingType::Struct {
            name: struct_name,
            domain: struct_domain,
            fields,
        } => {
            let mut lines = Vec::new();
            let struct_type = if binding_type_requires_abi(binding_type) {
                replay_struct_name(struct_name)
            } else {
                struct_name.clone()
            };
            let mut field_names = Vec::new();
            for field in fields {
                let field_name = to_snake_case(&field.name);
                let field_value = format!("{value_expr}.{field_name}");
                let field_var = format!("{name}_{field_name}");
                lines.extend(render_native_replay_encode_lines(
                    domain,
                    &field.binding_type,
                    &field_var,
                    &field_value,
                ));
                field_names.push((field_name, field_var));
            }
            let struct_path = named_type_path(domain, struct_domain, struct_type.as_str());
            lines.push(format!("let {name} = {struct_path} {{"));
            for (field_name, field_var) in field_names {
                lines.push(format!("    {field_name}: {field_var},"));
            }
            lines.push("};".to_string());
            lines
        }
    }
}

/// Render native replay decode lines for a binding value.
fn render_native_replay_decode_lines(
    domain: &str,
    binding_type: &BindingType,
    name: &str,
    value_expr: &str,
) -> Vec<String> {
    match binding_type {
        BindingType::Void => vec![format!("let {name} = ();")],
        BindingType::Bool
        | BindingType::Int(_)
        | BindingType::UInt(_)
        | BindingType::Float(_)
        | BindingType::Enum { .. } => {
            if binding_type_is_copy(binding_type) {
                vec![format!("let {name} = {value_expr};")]
            } else {
                vec![format!("let {name} = {value_expr}.clone();")]
            }
        }
        BindingType::Newtype {
            name: type_name,
            domain: type_domain,
            inner,
        } => {
            if binding_type_requires_abi(inner) {
                let inner_name = format!("{name}_inner");
                let mut lines =
                    render_native_replay_decode_lines(domain, inner, &inner_name, value_expr);
                let type_path = newtype_abi_constructor_path(
                    domain,
                    type_domain,
                    type_name,
                    "platform_abi::NativeAbi",
                );
                lines.push(format!("let {name} = {type_path}({inner_name});"));
                lines
            } else if binding_type_is_copy(binding_type) {
                vec![format!("let {name} = {value_expr};")]
            } else {
                vec![format!("let {name} = {value_expr}.clone();")]
            }
        }
        BindingType::String => vec![format!("let {name} = context.store_string(&{value_expr});")],
        BindingType::StringSlice => {
            let mut lines = Vec::new();
            let values_var = format!("{name}_values");
            lines.push(format!(
                "let mut {values_var} = Vec::with_capacity({value_expr}.len());"
            ));
            lines.push(format!("for item in {value_expr}.iter() {{"));
            lines.push("    let stored = context.store_string(item);".to_string());
            lines.push(format!("    {values_var}.push(stored);"));
            lines.push("}".to_string());
            lines.push(format!(
                "let {name} = context.store_string_slice({values_var});"
            ));
            lines
        }
        BindingType::Slice(inner) => {
            render_native_replay_decode_collection_lines(domain, inner, name, value_expr, false)
        }
        BindingType::Array(inner) => {
            render_native_replay_decode_collection_lines(domain, inner, name, value_expr, true)
        }
        BindingType::Struct {
            name: struct_name,
            domain: struct_domain,
            fields,
        } => {
            let mut lines = Vec::new();
            let mut field_names = Vec::new();
            for field in fields {
                let field_name = to_snake_case(&field.name);
                let field_value = format!("{value_expr}.{field_name}");
                let field_var = format!("{name}_{field_name}");
                lines.extend(render_native_replay_decode_lines(
                    domain,
                    &field.binding_type,
                    &field_var,
                    &field_value,
                ));
                field_names.push((field_name, field_var));
            }
            let struct_path = named_type_path(domain, struct_domain, struct_name);
            lines.push(format!("let {name} = {struct_path} {{"));
            for (field_name, field_var) in field_names {
                lines.push(format!("    {field_name}: {field_var},"));
            }
            lines.push("};".to_string());
            lines
        }
    }
}

/// Render native replay decode for a slice or array.
fn render_native_replay_decode_collection_lines(
    domain: &str,
    inner: &BindingType,
    name: &str,
    value_expr: &str,
    is_array: bool,
) -> Vec<String> {
    let mut lines = Vec::new();
    let values_var = format!("{name}_values");
    let item_var = format!("{name}_item");
    let item_native_var = format!("{name}_item_native");
    lines.push(format!(
        "let mut {values_var} = Vec::with_capacity({value_expr}.len());"
    ));
    lines.push(format!("for {item_var} in {value_expr} {{"));
    lines.extend(
        render_native_replay_decode_lines(domain, inner, &item_native_var, &item_var)
            .into_iter()
            .map(|line| format!("    {line}")),
    );
    lines.push(format!("    {values_var}.push({item_native_var});"));
    lines.push("}".to_string());
    if is_array {
        lines.push(format!("let {name} = context.store_array({values_var});"));
    } else {
        lines.push(format!("let {name} = context.store_slice({values_var});"));
    }
    lines
}

/// Render native replay encoding for a string slice.
fn render_native_replay_encode_string_slice_lines(name: &str, value_expr: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let raw_var = format!("{name}_raw");
    let item_var = format!("{name}_item");
    let item_recorded_var = format!("{name}_item_recorded");
    lines.push(format!(
        "let {raw_var} = unsafe {{ {value_expr}.as_slice()? }};"
    ));
    lines.push(format!(
        "let mut {name} = Vec::with_capacity({raw_var}.len());"
    ));
    lines.push(format!("for {item_var} in {raw_var} {{"));
    lines.push(format!(
        "    let {item_recorded_var} = unsafe {{ {item_var}.as_str()? }}.to_string();"
    ));
    lines.push(format!("    {name}.push({item_recorded_var});"));
    lines.push("}".to_string());
    lines
}

/// Render native replay encoding for a slice or array.
fn render_native_replay_encode_collection_lines(
    domain: &str,
    inner: &BindingType,
    name: &str,
    value_expr: &str,
) -> Vec<String> {
    let mut lines = Vec::new();
    let raw_var = format!("{name}_raw");
    let item_var = format!("{name}_item");
    let item_recorded_var = format!("{name}_item_recorded");
    lines.push(format!(
        "let {raw_var} = unsafe {{ {value_expr}.as_slice()? }};"
    ));
    lines.push(format!(
        "let mut {name} = Vec::with_capacity({raw_var}.len());"
    ));
    lines.push(format!("for {item_var}_value in {raw_var} {{"));
    if binding_type_is_copy(inner) || binding_type_requires_abi(inner) {
        lines.push(format!("    let {item_var} = *{item_var}_value;"));
    } else {
        lines.push(format!("    let {item_var} = {item_var}_value.clone();"));
    }
    lines.extend(
        render_native_replay_encode_lines(domain, inner, &item_recorded_var, &item_var)
            .into_iter()
            .map(|line| format!("    {line}")),
    );
    lines.push(format!("    {name}.push({item_recorded_var});"));
    lines.push("}".to_string());
    lines
}

/// Render native replay read from an out pointer.
pub(super) fn render_native_replay_read_out_lines(
    _domain: &str,
    binding_type: &BindingType,
    out_name: &str,
    result_var: &str,
) -> Vec<String> {
    let read_expr = if binding_type_is_copy_for_handle(binding_type) {
        format!("*{out_name}")
    } else {
        format!("(*{out_name}).clone()")
    };
    vec![
        format!("let {result_var} = unsafe {{"),
        format!(
            "    if {out_name}.is_null() {{ return Err(RuntimeError::from(PlatformError::null_pointer(\"out\")).boxed()); }}"
        ),
        format!("    {read_expr}"),
        "};".to_string(),
    ]
}

/// Render native replay write to an out pointer.
pub(super) fn render_native_replay_store_lines(
    domain: &str,
    binding_type: &BindingType,
    out_name: &str,
    value_expr: &str,
    native_var: &str,
) -> Vec<String> {
    let mut lines = render_native_replay_decode_lines(domain, binding_type, native_var, value_expr);
    lines.push(format!(
        "unsafe {{ std::ptr::write({out_name}, {native_var}); }}"
    ));
    lines
}

/// Render replay comparisons for two values.
pub(super) fn render_replay_compare_lines(
    domain: &str,
    binding_type: &BindingType,
    left_expr: &str,
    right_expr: &str,
    mismatch_stmt: &str,
    counter: &mut usize,
) -> Vec<String> {
    match binding_type {
        BindingType::Void => Vec::new(),
        BindingType::Bool
        | BindingType::Int(_)
        | BindingType::UInt(_)
        | BindingType::Enum { .. }
        | BindingType::String => vec![
            format!("if {left_expr} != {right_expr} {{"),
            format!("    {mismatch_stmt}"),
            "}".to_string(),
        ],
        BindingType::Float(_) => vec![
            format!("if {left_expr}.to_bits() != {right_expr}.to_bits() {{"),
            format!("    {mismatch_stmt}"),
            "}".to_string(),
        ],
        BindingType::StringSlice => render_replay_compare_collection_lines(
            domain,
            &BindingType::String,
            left_expr,
            right_expr,
            mismatch_stmt,
            counter,
        ),
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            render_replay_compare_collection_lines(
                domain,
                inner,
                left_expr,
                right_expr,
                mismatch_stmt,
                counter,
            )
        }
        BindingType::Newtype { inner, .. } => {
            if binding_type_requires_abi(inner) {
                render_replay_compare_lines(
                    domain,
                    inner,
                    left_expr,
                    right_expr,
                    mismatch_stmt,
                    counter,
                )
            } else {
                render_replay_compare_lines(
                    domain,
                    inner,
                    &format!("&{left_expr}.0"),
                    &format!("&{right_expr}.0"),
                    mismatch_stmt,
                    counter,
                )
            }
        }
        BindingType::Struct { fields, .. } => {
            let mut lines = Vec::new();
            for field in fields {
                let field_name = to_snake_case(&field.name);
                let left_field = format!("&{left_expr}.{field_name}");
                let right_field = format!("&{right_expr}.{field_name}");
                lines.extend(render_replay_compare_lines(
                    domain,
                    &field.binding_type,
                    &left_field,
                    &right_field,
                    mismatch_stmt,
                    counter,
                ));
            }
            lines
        }
    }
}

/// Render replay comparisons for slice or array payloads.
fn render_replay_compare_collection_lines(
    domain: &str,
    inner: &BindingType,
    left_expr: &str,
    right_expr: &str,
    mismatch_stmt: &str,
    counter: &mut usize,
) -> Vec<String> {
    let index_var = next_compare_ident("index", counter);
    let left_item = next_compare_ident("left_item", counter);
    let right_item = next_compare_ident("right_item", counter);
    let mut lines = Vec::new();
    lines.push(format!("if {left_expr}.len() != {right_expr}.len() {{"));
    lines.push(format!("    {mismatch_stmt}"));
    lines.push("}".to_string());
    lines.push(format!(
        "for ({index_var}, {left_item}) in {left_expr}.iter().enumerate() {{"
    ));
    lines.push(format!(
        "    let {right_item} = &{right_expr}[{index_var}];"
    ));
    lines.extend(
        render_replay_compare_lines(
            domain,
            inner,
            &left_item,
            &right_item,
            mismatch_stmt,
            counter,
        )
        .into_iter()
        .map(|line| format!("    {line}")),
    );
    lines.push("}".to_string());
    lines
}

/// Generate a unique comparison identifier.
fn next_compare_ident(prefix: &str, counter: &mut usize) -> String {
    let name = format!("{prefix}_{counter}");
    *counter += 1;
    name
}

/// Render replay decoding lines into a VM binding type.
pub(super) fn render_replay_to_vm_binding_lines(
    domain: &str,
    binding_type: &BindingType,
    name: &str,
    value_expr: &str,
) -> Vec<String> {
    match binding_type {
        BindingType::Void => vec![format!("let {name} = ();")],
        BindingType::Bool
        | BindingType::Int(_)
        | BindingType::UInt(_)
        | BindingType::Float(_)
        | BindingType::Enum { .. } => vec![format!("let {name} = {value_expr};")],
        BindingType::Newtype {
            name: type_name,
            domain: type_domain,
            inner,
        } => {
            if binding_type_requires_abi(inner) {
                let inner_name = format!("{name}_inner");
                let mut lines =
                    render_replay_to_vm_binding_lines(domain, inner, &inner_name, value_expr);
                let type_path = newtype_abi_constructor_path(
                    domain,
                    type_domain,
                    type_name,
                    "platform_abi::VmAbi",
                );
                lines.push(format!("let {name} = {type_path}({inner_name});"));
                lines
            } else {
                vec![format!("let {name} = {value_expr};")]
            }
        }
        BindingType::String => vec![
            format!("let {name}_value = context.intern_string({value_expr}.as_str());"),
            format!("let {name} = vm::StringHandle::new({name}_value);"),
        ],
        BindingType::StringSlice => render_replay_to_vm_binding_collection_lines(
            domain,
            &BindingType::String,
            name,
            value_expr,
            false,
            false,
        ),
        BindingType::Slice(inner) => {
            let is_bytes = matches!(**inner, BindingType::UInt(8));
            render_replay_to_vm_binding_collection_lines(
                domain, inner, name, value_expr, is_bytes, false,
            )
        }
        BindingType::Array(inner) => {
            let is_bytes = matches!(**inner, BindingType::UInt(8));
            render_replay_to_vm_binding_collection_lines(
                domain, inner, name, value_expr, is_bytes, true,
            )
        }
        BindingType::Struct {
            name: struct_name,
            domain: struct_domain,
            fields,
        } => {
            let mut lines = Vec::new();
            let mut field_values = Vec::new();
            for field in fields {
                let field_name = to_snake_case(&field.name);
                let field_var = format!("{name}_{field_name}");
                let field_expr = format!("{value_expr}.{field_name}");
                lines.extend(
                    render_replay_to_vm_binding_lines(
                        domain,
                        &field.binding_type,
                        &field_var,
                        &field_expr,
                    )
                    .into_iter(),
                );
                field_values.push((field_name, field_var));
            }
            let struct_path = struct_vm_path(domain, struct_domain, struct_name);
            lines.push(format!("let {name} = {struct_path} {{"));
            for (field_name, field_var) in field_values {
                lines.push(format!("    {field_name}: {field_var},"));
            }
            lines.push("};".to_string());
            lines
        }
    }
}

/// Render replay decoding for a slice or array into VM binding types.
fn render_replay_to_vm_binding_collection_lines(
    domain: &str,
    inner: &BindingType,
    name: &str,
    value_expr: &str,
    is_bytes: bool,
    is_array: bool,
) -> Vec<String> {
    let mut lines = Vec::new();
    if is_bytes {
        if is_array {
            lines.push(format!(
                "let {name} = VmArray::from_bytes(context, {value_expr}.as_slice());"
            ));
        } else {
            lines.push(format!(
                "let {name} = VmSlice::from_bytes(context, {value_expr}.as_slice());"
            ));
        }
        return lines;
    }

    let values_var = format!("{name}_values");
    let item_var = format!("{name}_item");
    let item_value_var = format!("{name}_item_value");
    lines.push(format!(
        "let mut {values_var} = Vec::with_capacity({value_expr}.len());"
    ));
    lines.push(format!("for {item_var} in {value_expr}.iter() {{"));
    if binding_type_is_copy_for_replay(inner) {
        lines.push(format!("    let {item_var} = *{item_var};"));
    } else if matches!(inner, BindingType::String) {
        // use string references directly
    } else {
        lines.push(format!("    let {item_var} = {item_var}.clone();"));
    }
    lines.extend(
        render_replay_to_vm_binding_lines(domain, inner, &item_value_var, &item_var)
            .into_iter()
            .map(|line| format!("    {line}")),
    );
    if binding_type_is_vm_value_codec(inner) {
        lines.push(format!("    {values_var}.push({item_value_var});"));
    } else {
        let encoded_expr = render_encode_expr(domain, inner, &item_value_var);
        lines.push(format!(
            "    let {item_value_var}_encoded = {encoded_expr};"
        ));
        lines.push(format!("    {values_var}.push({item_value_var}_encoded);"));
    }
    lines.push("}".to_string());
    if binding_type_is_vm_value_codec(inner) {
        if is_array {
            lines.push(format!(
                "let {name} = VmArray::from_values(context, &{values_var})?;"
            ));
        } else {
            lines.push(format!(
                "let {name} = VmSlice::from_values(context, &{values_var})?;"
            ));
        }
    } else {
        let inner_type = vm_type_for_binding(domain, inner);
        lines.push(format!(
            "let {name}_data = context.allocate_raw_values({values_var});"
        ));
        if is_array {
            lines.push(format!(
                "let {name}: VmArray<{inner_type}> = VmArray {{ data: {name}_data, len: {value_expr}.len() as u32, capacity: {value_expr}.len() as u32, _marker: std::marker::PhantomData }};"
            ));
        } else {
            lines.push(format!(
                "let {name}: VmSlice<{inner_type}> = VmSlice {{ data: {name}_data, len: {value_expr}.len() as u32, _marker: std::marker::PhantomData }};"
            ));
        }
    }
    lines
}

/// Convert a binding type into a VM-facing Rust type.
pub(super) fn vm_type_for_binding(domain: &str, binding_type: &BindingType) -> String {
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
        BindingType::Slice(inner) => format!("VmSlice<{}>", vm_type_for_binding(domain, inner)),
        BindingType::Array(inner) => format!("VmArray<{}>", vm_type_for_binding(domain, inner)),
        BindingType::Newtype {
            name,
            domain: type_domain,
            inner,
        } => {
            if binding_type_requires_abi(inner) {
                newtype_vm_path(domain, type_domain, name)
            } else {
                named_type_path(domain, type_domain, name)
            }
        }
        BindingType::Struct {
            name,
            domain: type_domain,
            ..
        } => struct_vm_path(domain, type_domain, name),
        BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => named_type_path(domain, type_domain, name),
    }
}

/// Convert a binding type into a replay payload type.
pub(super) fn replay_type_for_binding(domain: &str, binding_type: &BindingType) -> String {
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
            format!("Vec<{}>", replay_type_for_binding(domain, inner))
        }
        BindingType::Newtype {
            name,
            domain: type_domain,
            inner,
        } => {
            if binding_type_requires_abi(inner) {
                replay_type_for_binding(domain, inner)
            } else {
                named_type_path(domain, type_domain, name)
            }
        }
        BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => named_type_path(domain, type_domain, name),
        BindingType::Struct {
            name,
            domain: type_domain,
            ..
        } => {
            if binding_type_requires_abi(binding_type) {
                let replay_name = replay_struct_name(name);
                named_type_path(domain, type_domain, replay_name.as_str())
            } else {
                named_type_path(domain, type_domain, name)
            }
        }
    }
}

/// Build the generated replay struct name for a named binding struct.
fn replay_struct_name(name: &str) -> String {
    format!("{name}ReplayRecord")
}

/// Decode binding arguments into typed locals.
/// Decode a binding value into a local variable.
fn render_decode_value_lines(
    domain: &str,
    binding_type: &BindingType,
    name: &str,
    value_expr: &str,
    expected: &str,
) -> Vec<String> {
    match binding_type {
        BindingType::Bool => vec![format!(
            "let {name} = decode_bool({value_expr}, \"{name}\", \"{expected}\")?;"
        )],
        BindingType::Int(bits) if matches!(bits, 8 | 16 | 32 | 64) => {
            let decoder = match bits {
                8 => "decode_int8",
                16 => "decode_int16",
                32 => "decode_int32",
                64 => "decode_int64",
                _ => unreachable!(),
            };
            vec![format!(
                "let {name} = {decoder}({value_expr}, \"{name}\", \"{expected}\")?;"
            )]
        }
        BindingType::UInt(bits) if matches!(bits, 8 | 16 | 32 | 64) => {
            let decoder = match bits {
                8 => "decode_uint8",
                16 => "decode_uint16",
                32 => "decode_uint32",
                64 => "decode_uint64",
                _ => unreachable!(),
            };
            vec![format!(
                "let {name} = {decoder}({value_expr}, \"{name}\", \"{expected}\")?;"
            )]
        }
        BindingType::Float(32) => vec![format!(
            "let {name} = decode_float32({value_expr}, \"{name}\", \"{expected}\")?;"
        )],
        BindingType::Float(64) => vec![format!(
            "let {name} = decode_float64({value_expr}, \"{name}\", \"{expected}\")?;"
        )],
        BindingType::String => vec![format!(
            "let {name} = decode_string({value_expr}, \"{name}\", \"{expected}\")?;"
        )],
        BindingType::StringSlice => vec![format!(
            "let {name} = decode_slice::<vm::StringHandle>(context, {value_expr}, \"{name}\", \"{expected}\")?;"
        )],
        BindingType::Slice(inner) => {
            let inner_type = vm_type_for_binding(domain, inner);
            vec![format!(
                "let {name} = decode_slice::<{inner_type}>(context, {value_expr}, \"{name}\", \"{expected}\")?;"
            )]
        }
        BindingType::Array(inner) => {
            let inner_type = vm_type_for_binding(domain, inner);
            vec![format!(
                "let {name} = decode_array::<{inner_type}>(context, {value_expr}, \"{name}\", \"{expected}\")?;"
            )]
        }
        BindingType::Newtype {
            name: type_name,
            domain: type_domain,
            inner,
        } => {
            let inner_name = format!("{name}_inner");
            let mut lines =
                render_decode_value_lines(domain, inner, &inner_name, value_expr, expected);
            let type_path = if binding_type_requires_abi(inner) {
                newtype_abi_constructor_path(domain, type_domain, type_name, "platform_abi::VmAbi")
            } else {
                named_type_path(domain, type_domain, type_name)
            };
            lines.push(format!("let {name} = {type_path}({inner_name});"));
            lines
        }
        BindingType::Enum {
            name: enum_name,
            domain: enum_domain,
            backing,
            variants,
        } => {
            let enum_path = named_type_path(domain, enum_domain, enum_name);
            render_decode_enum_lines(
                domain,
                name,
                value_expr,
                expected,
                enum_path.as_str(),
                *backing,
                variants,
            )
        }
        BindingType::Struct {
            name: struct_name,
            domain: struct_domain,
            fields,
        } => render_decode_struct_lines(
            domain,
            name,
            value_expr,
            expected,
            struct_name,
            struct_domain,
            fields,
        ),
        BindingType::Void => Vec::new(),
        _ => Vec::new(),
    }
}

/// Render enum decoding lines.
fn render_decode_enum_lines(
    domain: &str,
    name: &str,
    value_expr: &str,
    expected: &str,
    enum_name: &str,
    backing: EnumBackingType,
    variants: &[BindingEnumVariant],
) -> Vec<String> {
    let mut lines = Vec::new();
    let raw_name = format!("{name}_raw");
    let backing_type = enum_backing_binding_type(backing);
    lines.extend(render_decode_value_lines(
        domain,
        &backing_type,
        &raw_name,
        value_expr,
        expected,
    ));

    let match_expr = match backing {
        EnumBackingType::Int(int_type) => {
            let suffix = enum_backing_rust_type(EnumBackingType::Int(int_type));
            let mut arms = Vec::new();
            for variant in variants {
                if let BindingEnumValue::Int(value) = variant.value {
                    let literal = format!("{value}{suffix}");
                    arms.push(format!("{literal} => {enum_name}::{}", variant.name));
                }
            }
            format!(
                "match {raw_name} {{ {} , _ => return Err(RuntimeError::from(PlatformError::invalid_argument_value(\"{name}\", \"unknown {enum_name} value\")).boxed()), }}",
                arms.join(", ")
            )
        }
        EnumBackingType::String => {
            lines.push(format!(
                "let {raw_name}_ref = context.string_ref({raw_name}).map_err(|error| RuntimeError::from(error).boxed())?;"
            ));
            lines.push(format!("let {raw_name} = {raw_name}_ref.as_str();"));
            let mut arms = Vec::new();
            for variant in variants {
                if let BindingEnumValue::String(value) = &variant.value {
                    arms.push(format!("\"{value}\" => {enum_name}::{}", variant.name));
                }
            }
            format!(
                "match {raw_name} {{ {} , _ => return Err(RuntimeError::from(PlatformError::invalid_argument_value(\"{name}\", \"unknown {enum_name} value\")).boxed()), }}",
                arms.join(", ")
            )
        }
    };
    lines.push(format!("let {name} = {match_expr};"));
    lines
}

/// Render struct decoding lines.
fn render_decode_struct_lines(
    domain: &str,
    name: &str,
    value_expr: &str,
    expected: &str,
    struct_name: &str,
    struct_domain: &str,
    fields: &[BindingField],
) -> Vec<String> {
    let mut lines = Vec::new();
    let struct_type = struct_vm_path(domain, struct_domain, struct_name);
    lines.push(format!("let {name} = {{"));
    lines.push(format!(
        "    if {value_expr}.tag() != vm::ValueTag::Aggregate {{ return Err(RuntimeError::from(PlatformError::invalid_argument_type(\"{name}\", \"{expected}\")).boxed()); }}"
    ));
    lines.push(format!(
        "    let slots = context.aggregate_slots({value_expr}).map_err(|error| RuntimeError::from(error).boxed())?;"
    ));
    lines.push(format!(
        "    if slots.len() != {} {{ return Err(RuntimeError::from(PlatformError::invalid_argument_value(\"{name}\", \"expected {} fields\")).boxed()); }}",
        fields.len(),
        fields.len()
    ));

    let mut field_names = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        let field_name = to_snake_case(&field.name);
        let local_name = format!("{name}_{field_name}");
        field_names.push((field_name, local_name.clone()));
        let field_lines = render_decode_value_lines(
            domain,
            &field.binding_type,
            &local_name,
            &format!("slots[{index}]"),
            field.name.as_str(),
        );
        for line in field_lines {
            lines.push(format!("    {line}"));
        }
    }

    lines.push(format!("    {struct_type} {{"));
    for (field_name, local_name) in field_names {
        lines.push(format!("        {field_name}: {local_name},"));
    }
    lines.push("    }".to_string());
    lines.push("};".to_string());
    lines
}

/// Convert enum backing types into binding types.
fn enum_backing_binding_type(backing: EnumBackingType) -> BindingType {
    match backing {
        EnumBackingType::Int(int_type) => match int_type.simplify() {
            IntType::Int8 => BindingType::Int(8),
            IntType::Int16 => BindingType::Int(16),
            IntType::Int32 => BindingType::Int(32),
            IntType::Int64 => BindingType::Int(64),
            IntType::Uint8 => BindingType::UInt(8),
            IntType::Uint16 => BindingType::UInt(16),
            IntType::Uint32 => BindingType::UInt(32),
            IntType::Uint64 => BindingType::UInt(64),
            _ => panic!("unsupported enum backing width: {int_type:?}"),
        },
        EnumBackingType::String => BindingType::String,
    }
}

/// Convert enum backing types into a Rust primitive name.
fn enum_backing_rust_type(backing: EnumBackingType) -> &'static str {
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

/// Convert a field name to snake case.
fn to_snake_case(name: &str) -> String {
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

/// Normalize a parameter name into a safe identifier.
pub(super) fn sanitize_param_name(name: &str, index: usize) -> String {
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

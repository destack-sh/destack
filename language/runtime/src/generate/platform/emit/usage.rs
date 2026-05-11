use std::collections::BTreeSet;

use destack_dir::{EnumBackingType, IntegerType};

use super::ModuleBindings;
use super::replay::{collect_replay_named_types, collect_replay_vm_named_types};
use crate::platform::model::{
    BindingType, CatalogBindingProvider, CatalogBindingReplayKind, CatalogBindingSimulation,
    CatalogEffect, CatalogReplayPayload, CatalogReplayPolicy,
};

/// Usage flags for native stub bindings.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct NativeUsage {
    /// Platform slices are referenced in the native ABI.
    pub(crate) uses_platform_slice: bool,
    /// Platform arrays are referenced in the native ABI.
    pub(crate) uses_platform_array: bool,
    /// Platform string references are referenced in the native ABI.
    pub(crate) uses_platform_string_ref: bool,
    /// Platform string slices are referenced in the native ABI.
    pub(crate) uses_platform_string_slice: bool,
}

/// Usage flags for VM argument decoding helpers.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct VmDecodeUsage {
    /// Boolean decoding is used.
    pub(crate) uses_bool: bool,
    /// Signed 8-bit decoding is used.
    pub(crate) uses_int8: bool,
    /// Signed 16-bit decoding is used.
    pub(crate) uses_int16: bool,
    /// Signed 32-bit decoding is used.
    pub(crate) uses_int32: bool,
    /// Signed 64-bit decoding is used.
    pub(crate) uses_int64: bool,
    /// Unsigned 8-bit decoding is used.
    pub(crate) uses_uint8: bool,
    /// Unsigned 16-bit decoding is used.
    pub(crate) uses_uint16: bool,
    /// Unsigned 32-bit decoding is used.
    pub(crate) uses_uint32: bool,
    /// Unsigned 64-bit decoding is used.
    pub(crate) uses_uint64: bool,
    /// 32-bit float decoding is used.
    pub(crate) uses_float32: bool,
    /// 64-bit float decoding is used.
    pub(crate) uses_float64: bool,
    /// String decoding is used.
    pub(crate) uses_string: bool,
    /// Slice decoding is used.
    pub(crate) uses_slice: bool,
    /// Array decoding is used.
    pub(crate) uses_array: bool,
}

/// Usage flags derived from a binding catalog.
#[derive(Debug, Clone)]
pub(crate) struct RenderUsage {
    /// VM wrapper usage flags.
    pub(crate) vm_usage: VmDecodeUsage,
    /// VM type usage flags for imports.
    pub(crate) vm_types: VmStubUsage,
    /// Native signature usage flags.
    pub(crate) native_usage: NativeUsage,
    /// Whether replay policy is referenced.
    pub(crate) uses_replay_policy: bool,
    /// Whether bindings use replay helpers.
    pub(crate) uses_binding_replay: bool,
    /// Whether entropy replay routing is referenced.
    pub(crate) uses_entropy_replay_kind: bool,
    /// Whether replay payload enum is referenced.
    pub(crate) uses_replay_payload_type: bool,
    /// Whether replay payload helpers are needed.
    pub(crate) uses_replay_payload: bool,
    /// Whether VM helper imports are required.
    pub(crate) needs_vm: bool,
    /// Whether argument decoding helpers are required.
    pub(crate) needs_decode: bool,
    /// Whether native out parameters are used.
    pub(crate) needs_native_out: bool,
    /// Whether world dispatch should be emitted.
    pub(crate) uses_world_dispatch: bool,
    /// Whether simulation dispatch should be emitted.
    pub(crate) uses_simulation_dispatch: bool,
    /// Whether runtime dispatch should be emitted.
    pub(crate) uses_runtime_dispatch: bool,
}

impl RenderUsage {
    /// Build render usage flags from one binding set.
    pub(crate) fn new(bindings: &ModuleBindings) -> Self {
        let uses_replay_policy = bindings
            .values()
            .any(|entry| matches!(entry.effect, CatalogEffect::External { .. }));
        let uses_binding_replay = bindings.values().any(|entry| {
            matches!(
                entry.effect,
                CatalogEffect::External {
                    replay: CatalogReplayPolicy::Recordable
                }
            ) && entry.replay_kind == CatalogBindingReplayKind::BindingCall
        });
        let uses_entropy_replay_kind = bindings
            .values()
            .any(|entry| matches!(entry.replay_kind, CatalogBindingReplayKind::Entropy(_)));
        let uses_replay_payload_type = bindings.values().any(|entry| {
            matches!(
                entry.replay_payload,
                CatalogReplayPayload::ArgumentsAndResults
            ) && entry.replay_kind == CatalogBindingReplayKind::BindingCall
        });
        let uses_replay_payload = bindings.values().any(|entry| {
            matches!(
                entry.effect,
                CatalogEffect::External {
                    replay: CatalogReplayPolicy::Recordable
                }
            ) && entry.replay_kind == CatalogBindingReplayKind::BindingCall
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
            .any(|entry| entry.provider != CatalogBindingProvider::Runtime);
        let uses_simulation_dispatch = bindings.values().any(|entry| {
            entry.provider != CatalogBindingProvider::Runtime
                && entry.simulation != CatalogBindingSimulation::Unsupported
        });
        let uses_runtime_dispatch = bindings
            .values()
            .any(|entry| entry.provider == CatalogBindingProvider::Runtime);

        Self {
            vm_usage: VmDecodeUsage::collect(bindings),
            vm_types: VmStubUsage::collect(bindings),
            native_usage: NativeUsage::collect_signature_usage(bindings),
            uses_replay_policy,
            uses_binding_replay,
            uses_entropy_replay_kind,
            uses_replay_payload_type,
            uses_replay_payload,
            needs_vm,
            needs_decode,
            needs_native_out,
            uses_world_dispatch,
            uses_simulation_dispatch,
            uses_runtime_dispatch,
        }
    }
}

/// Named type usage derived from bindings.
#[derive(Debug, Clone)]
pub(crate) struct RenderTypes {
    /// VM-level named types for imports.
    pub(crate) named_types: BTreeSet<String>,
    /// Native ABI named types for imports.
    pub(crate) native_named_types: BTreeSet<String>,
    /// Replay VM named types for imports.
    pub(crate) replay_vm_named_types: BTreeSet<String>,
    /// Replay ABI named types for imports.
    pub(crate) replay_named_types: BTreeSet<String>,
    /// Referenced foreign type domains.
    pub(crate) type_domains: BTreeSet<String>,
}

impl RenderTypes {
    /// Build rendered type usage from one binding set.
    pub(crate) fn new(module: &str, bindings: &ModuleBindings) -> Self {
        Self {
            named_types: collect_vm_named_types(module, bindings),
            native_named_types: collect_native_named_types(module, bindings),
            replay_vm_named_types: collect_replay_vm_named_types(module, bindings),
            replay_named_types: collect_replay_named_types(module, bindings),
            type_domains: collect_type_domains(module, bindings),
        }
    }
}

/// Usage flags for VM stub imports.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct VmStubUsage {
    /// VM slice types are referenced.
    pub(crate) uses_vm_slice: bool,
    /// VM array types are referenced.
    pub(crate) uses_vm_array: bool,
}

impl NativeUsage {
    /// Collect native signature usage for one binding set.
    pub(crate) fn collect_signature_usage(bindings: &ModuleBindings) -> Self {
        collect_native_signature_usage(bindings)
    }
}

impl VmDecodeUsage {
    /// Collect VM decode helper usage for one binding set.
    pub(crate) fn collect(bindings: &ModuleBindings) -> Self {
        collect_vm_decode_usage(bindings)
    }
}

impl VmStubUsage {
    /// Collect VM stub usage for one binding set.
    pub(crate) fn collect(bindings: &ModuleBindings) -> Self {
        collect_vm_stub_usage(bindings)
    }
}

/// Collect native signature usage flags for one binding catalog.
fn collect_native_signature_usage(bindings: &ModuleBindings) -> NativeUsage {
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
        BindingType::Optional(inner) => collect_native_signature_usage_for_binding(inner, usage),
        _ => {}
    }
}

/// Collect VM decode helper usage from binding parameters.
fn collect_vm_decode_usage(bindings: &ModuleBindings) -> VmDecodeUsage {
    let mut usage = VmDecodeUsage::default();
    for entry in bindings.values() {
        let uses_binding_replay = matches!(
            entry.effect,
            CatalogEffect::External {
                replay: CatalogReplayPolicy::Recordable
            }
        ) && entry.replay_kind == CatalogBindingReplayKind::BindingCall;
        let supports_args = matches!(
            entry.replay_payload,
            CatalogReplayPayload::ArgumentsAndResults
        );
        for param in &entry.parameters {
            collect_vm_decode_usage_for_binding(&param.binding_type, &mut usage, false, true);
            if uses_binding_replay && supports_args {
                collect_vm_decode_usage_for_binding(&param.binding_type, &mut usage, true, true);
            }
        }
        if uses_binding_replay {
            collect_vm_decode_usage_for_binding(&entry.return_binding, &mut usage, true, true);
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
        BindingType::Optional(inner) => {
            collect_vm_decode_usage_for_binding(
                inner,
                usage,
                deep_collections,
                include_collection_decoders,
            );
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
        BindingType::TaggedUnion { variants, .. } => {
            usage.uses_array = true;
            usage.uses_uint8 = true;
            for variant in variants {
                collect_vm_decode_usage_for_binding(
                    &variant.binding_type,
                    usage,
                    deep_collections,
                    include_collection_decoders,
                );
            }
        }
    }
}

/// Collect named binding types referenced by bindings.
pub(crate) fn collect_native_named_types(
    domain: &str,
    bindings: &ModuleBindings,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    for entry in bindings.values() {
        collect_signature_type_names(domain, &entry.return_binding, &mut names, "");
        for param in &entry.parameters {
            collect_signature_type_names(domain, &param.binding_type, &mut names, "");
        }
    }

    names
}

/// Collect named types required by simulation native stub signatures.
pub(crate) fn collect_native_stub_named_types(
    domain: &str,
    bindings: &ModuleBindings,
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

/// Collect platform module names referenced by binding types.
pub(crate) fn collect_type_domains(domain: &str, bindings: &ModuleBindings) -> BTreeSet<String> {
    let mut domains = BTreeSet::new();

    for entry in bindings.values() {
        collect_binding_type_domains(domain, &entry.return_binding, &mut domains);
        for param in &entry.parameters {
            collect_binding_type_domains(domain, &param.binding_type, &mut domains);
        }
    }

    domains
}

/// Collect platform module names required by stub signatures only.
pub(crate) fn collect_stub_type_domains(
    domain: &str,
    bindings: &ModuleBindings,
) -> BTreeSet<String> {
    let mut domains = BTreeSet::new();

    for entry in bindings.values() {
        collect_stub_binding_type_domains(domain, &entry.return_binding, &mut domains);
        for param in &entry.parameters {
            collect_stub_binding_type_domains(domain, &param.binding_type, &mut domains);
        }
    }

    domains
}

/// Collect foreign domains referenced by one binding type.
pub(crate) fn collect_binding_type_domains(
    domain: &str,
    binding_type: &BindingType,
    domains: &mut BTreeSet<String>,
) {
    match binding_type {
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            collect_binding_type_domains(domain, inner, domains);
        }
        BindingType::Optional(inner) => {
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
        BindingType::TaggedUnion {
            domain: type_domain,
            variants,
            ..
        } => {
            if type_domain != domain {
                domains.insert(type_domain.clone());
            }
            for variant in variants {
                collect_binding_type_domains(domain, &variant.binding_type, domains);
            }
        }
        _ => {}
    }
}

/// Collect foreign domains referenced directly in stub signatures.
fn collect_stub_binding_type_domains(
    domain: &str,
    binding_type: &BindingType,
    domains: &mut BTreeSet<String>,
) {
    match binding_type {
        BindingType::Slice(inner) | BindingType::Array(inner) | BindingType::Optional(inner) => {
            collect_stub_binding_type_domains(domain, inner, domains);
        }
        BindingType::Newtype {
            domain: type_domain,
            ..
        }
        | BindingType::Struct {
            domain: type_domain,
            ..
        }
        | BindingType::Enum {
            domain: type_domain,
            ..
        }
        | BindingType::TaggedUnion {
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

/// Collect VM stub usage flags from bindings.
pub(crate) fn collect_vm_stub_usage(bindings: &ModuleBindings) -> VmStubUsage {
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
        BindingType::Optional(inner) => {
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
        BindingType::TaggedUnion { variants, .. } => {
            for variant in variants {
                collect_vm_stub_usage_for_binding(&variant.binding_type, usage);
            }
        }
        _ => {}
    }
}

/// Collect VM-visible named types.
pub(crate) fn collect_vm_named_types(domain: &str, bindings: &ModuleBindings) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for entry in bindings.values() {
        collect_signature_type_names(domain, &entry.return_binding, &mut names, "Vm");
        for param in &entry.parameters {
            collect_signature_type_names(domain, &param.binding_type, &mut names, "Vm");
        }
    }
    names
}

/// Collect named types required by simulation VM stub signatures.
pub(crate) fn collect_vm_stub_named_types(
    domain: &str,
    bindings: &ModuleBindings,
) -> BTreeSet<String> {
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
        BindingType::Optional(inner) => {
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
        BindingType::TaggedUnion {
            name,
            domain: type_domain,
            variants,
            ..
        } => {
            if type_domain == domain {
                if !struct_suffix.is_empty() && binding_type_requires_abi(binding_type) {
                    names.insert(format!("{name}{struct_suffix}"));
                } else {
                    names.insert(name.clone());
                }
            }
            for variant in variants {
                collect_signature_type_names(domain, &variant.binding_type, names, struct_suffix);
            }
        }
        _ => {}
    }
}

/// Walk a binding type and record only named types that appear in function signatures.
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
        BindingType::Optional(inner) => {
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
        BindingType::TaggedUnion {
            name,
            domain: type_domain,
            ..
        } => {
            if type_domain == domain {
                if !struct_suffix.is_empty() && binding_type_requires_abi(binding_type) {
                    names.insert(format!("{name}{struct_suffix}"));
                } else {
                    names.insert(name.clone());
                }
            }
        }
        _ => {}
    }
}

/// Return true if decoding a binding type requires the VM module.
pub(crate) fn binding_type_requires_vm_for_decode(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::String
        | BindingType::StringSlice
        | BindingType::Slice(_)
        | BindingType::Array(_)
        | BindingType::Struct { .. }
        | BindingType::TaggedUnion { .. } => true,
        BindingType::Optional(inner) => binding_type_requires_vm_for_decode(inner),
        BindingType::Newtype { inner, .. } => binding_type_requires_vm_for_decode(inner),
        BindingType::Enum { backing, .. } => matches!(backing, EnumBackingType::String),
        _ => false,
    }
}

/// Return true if encoding a binding type requires the VM module.
pub(crate) fn binding_type_requires_vm_for_encode(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::Void
        | BindingType::Bool
        | BindingType::Int(_)
        | BindingType::UInt(_)
        | BindingType::Float(_) => true,
        BindingType::Newtype { inner, .. } => binding_type_requires_vm_for_encode(inner),
        BindingType::Optional(inner) => binding_type_requires_vm_for_encode(inner),
        BindingType::Enum { backing, .. } => matches!(backing, EnumBackingType::Integer(_)),
        BindingType::Struct { fields, .. } => fields
            .iter()
            .any(|field| binding_type_requires_vm_for_encode(&field.binding_type)),
        BindingType::TaggedUnion { variants, .. } => variants
            .iter()
            .any(|variant| binding_type_requires_vm_for_encode(&variant.binding_type)),
        _ => false,
    }
}

/// Return true if decoding a binding type needs a runtime context.
pub(crate) fn binding_type_requires_context_for_decode(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::String
        | BindingType::StringSlice
        | BindingType::Slice(_)
        | BindingType::Array(_) => true,
        BindingType::Struct { .. } | BindingType::TaggedUnion { .. } => true,
        BindingType::Enum { backing, .. } => matches!(backing, EnumBackingType::String),
        BindingType::Optional(inner) => binding_type_requires_context_for_decode(inner),
        BindingType::Newtype { inner, .. } => binding_type_requires_context_for_decode(inner),
        _ => false,
    }
}

/// Return true if encoding a binding type needs a runtime context.
pub(crate) fn binding_type_requires_context_for_encode(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::StringSlice | BindingType::Slice(_) | BindingType::Array(_) => true,
        BindingType::Struct { .. } | BindingType::TaggedUnion { .. } => true,
        BindingType::Enum { backing, .. } => matches!(backing, EnumBackingType::String),
        BindingType::Optional(inner) => binding_type_requires_context_for_encode(inner),
        BindingType::Newtype { inner, .. } => binding_type_requires_context_for_encode(inner),
        _ => false,
    }
}

/// Return true if a binding type depends on the ABI parameter.
pub(crate) fn binding_type_requires_abi(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::String
        | BindingType::StringSlice
        | BindingType::Slice(_)
        | BindingType::Array(_) => true,
        BindingType::Optional(inner) => binding_type_requires_abi(inner),
        BindingType::Struct { fields, .. } => fields
            .iter()
            .any(|field| binding_type_requires_abi(&field.binding_type)),
        BindingType::TaggedUnion { variants, .. } => variants
            .iter()
            .any(|variant| binding_type_requires_abi(&variant.binding_type)),
        BindingType::Newtype { inner, .. } => binding_type_requires_abi(inner),
        _ => false,
    }
}

/// Convert enum backing types into binding types.
pub(crate) fn enum_backing_binding_type(backing: EnumBackingType) -> BindingType {
    match backing {
        EnumBackingType::Integer(int_type) => match int_type {
            IntegerType::Fixed {
                width,
                is_signed: true,
            } if matches!(width, 8 | 16 | 32 | 64) => BindingType::Int(width),
            IntegerType::Fixed {
                width,
                is_signed: false,
            } if matches!(width, 8 | 16 | 32 | 64) => BindingType::UInt(width),
            _ => panic!("unsupported enum backing width: {int_type:?}"),
        },
        EnumBackingType::String => BindingType::String,
    }
}

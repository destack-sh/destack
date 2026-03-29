#![allow(dead_code)]
#![allow(unreachable_pub)]
#![allow(unused_imports)]
#![allow(unused_macros)]

#[cfg(feature = "generator")]
/// One generator-only native-array placeholder.
#[derive(Clone, Copy, Debug)]
pub struct HostAbiGeneratedNativeArray<T> {
    /// The element pointer.
    pub data: *mut T,
    /// The number of elements.
    pub len: u32,
    /// The allocated capacity.
    pub capacity: u32,
}

#[cfg(not(feature = "generator"))]
/// One authored native-array projection.
pub type HostAbiNativeArray<T> = crate::platform::abi::NativeArray<T>;
#[cfg(feature = "generator")]
/// One generator-only native-array projection.
pub type HostAbiNativeArray<T> = HostAbiGeneratedNativeArray<T>;

#[cfg(not(feature = "generator"))]
/// One authored runtime-status projection.
pub type HostAbiRuntimeStatus = crate::platform::RuntimeStatus;
#[cfg(feature = "generator")]
/// One generator-only runtime-status placeholder.
pub type HostAbiRuntimeStatus = crate::host::abi::describe::HostAbiGeneratedRuntimeStatus;

#[cfg(feature = "generator")]
/// One generator-only runtime-status placeholder.
#[derive(Clone, Copy, Debug)]
pub struct HostAbiGeneratedRuntimeStatus {
    /// The status code.
    pub code: u32,
    /// The optional error identifier.
    pub error_id: u64,
}

/// One generated host ABI module.
#[derive(Clone, Debug)]
pub struct HostAbiModule {
    /// The canonical module name.
    pub name: &'static str,
    /// The supported host platforms for this module.
    pub platforms: Vec<HostAbiModulePlatform>,
    /// The named ABI payload types authored for this module.
    pub types: Vec<HostAbiNamedType>,
    /// The module request surface.
    pub requests: Vec<HostAbiFunction>,
    /// The module ingress surface.
    pub ingress: Vec<HostAbiFunction>,
    /// Selector-backed control exports for this module.
    pub selector_controls: Vec<HostAbiSelectorControl>,
    /// Additional generated Rust capability probes for this module.
    pub rust_capability_probes: Vec<HostAbiRustCapabilityProbe>,
}

/// One authored host platform for one ABI module.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostAbiModulePlatform {
    /// The Apple iOS host surface.
    Ios,
    /// The Android host surface.
    Android,
}

/// One named host ABI payload type.
#[derive(Clone, Debug)]
pub struct HostAbiNamedType {
    /// The canonical type name.
    pub name: &'static str,
    /// The type documentation.
    pub documentation: &'static str,
    /// The concrete type layout.
    pub definition: HostAbiNamedTypeDefinition,
}

/// One named host ABI type definition.
#[derive(Clone, Debug)]
pub enum HostAbiNamedTypeDefinition {
    /// One `repr(C)` struct payload.
    Struct {
        /// The struct fields in declaration order.
        fields: Vec<HostAbiField>,
    },
    /// One integer-backed enum payload.
    Enum {
        /// The concrete integer representation.
        repr: HostAbiEnumRepresentation,
        /// The enum variants in declaration order.
        variants: Vec<HostAbiVariant>,
    },
}

/// One integer representation for one ABI enum.
#[derive(Clone, Copy, Debug)]
pub enum HostAbiEnumRepresentation {
    /// One `i32` enum representation.
    I32,
    /// One `u32` enum representation.
    U32,
}

/// One named host ABI struct field.
#[derive(Clone, Debug)]
pub struct HostAbiField {
    /// The field name.
    pub name: &'static str,
    /// The field documentation.
    pub documentation: &'static str,
    /// The field ABI type.
    pub ty: HostAbiType,
}

/// One named host ABI enum variant.
#[derive(Clone, Debug)]
pub struct HostAbiVariant {
    /// The variant name.
    pub name: &'static str,
    /// The variant documentation.
    pub documentation: &'static str,
    /// The explicit discriminant value.
    pub discriminant: u32,
}

/// One generated host ABI function.
#[derive(Clone, Debug)]
pub struct HostAbiFunction {
    /// The canonical function name.
    pub name: &'static str,
    /// The function documentation.
    pub documentation: &'static str,
    /// The function parameters in ABI order.
    pub parameters: Vec<HostAbiParameter>,
    /// The result ABI type.
    pub result: HostAbiType,
}

/// One generated host ABI function parameter.
#[derive(Clone, Debug)]
pub struct HostAbiParameter {
    /// The parameter name.
    pub name: &'static str,
    /// The ABI parameter type.
    pub ty: HostAbiType,
}

/// One selector-backed control surface.
#[derive(Clone, Debug)]
pub struct HostAbiSelectorControl {
    /// The getter export stem.
    pub getter_export_stem: &'static str,
    /// The optional setter export stem.
    pub setter_export_stem: Option<&'static str>,
    /// The optional range export stem.
    pub range_export_stem: Option<&'static str>,
    /// The selector value.
    pub selector: u32,
    /// The scalar value type.
    pub value_ty: HostAbiType,
}

/// One generated Rust capability probe.
#[derive(Clone, Debug)]
pub struct HostAbiRustCapabilityProbe {
    /// The exported probe name.
    pub export_name: &'static str,
    /// The probe documentation.
    pub documentation: &'static str,
    /// The probe input parameters after `session_handle`.
    pub parameters: Vec<HostAbiParameter>,
    /// The required callback availability expression.
    pub availability: HostAbiRustAvailability,
    /// The supported-path action.
    pub on_supported: HostAbiRustCapabilityAction,
    /// The unsupported-path status.
    pub unsupported_status: HostAbiRustStatus,
}

/// One generated Rust callback availability expression.
#[derive(Clone, Debug)]
pub enum HostAbiRustAvailability {
    /// One required callback field.
    CallbackPresent(&'static str),
    /// One equality check on one integer parameter.
    ParameterEquals {
        /// The parameter name.
        parameter: &'static str,
        /// The expected value.
        value: u32,
    },
    /// All nested predicates must hold.
    All(Vec<HostAbiRustAvailability>),
    /// Any nested predicate may hold.
    Any(Vec<HostAbiRustAvailability>),
}

/// One generated Rust supported-path action.
#[derive(Clone, Debug)]
pub enum HostAbiRustCapabilityAction {
    /// Return one fixed status code.
    ReturnStatus(HostAbiRustStatus),
    /// Invoke one resolved callback with named arguments.
    InvokeCallback {
        /// The callback field name.
        callback: &'static str,
        /// The ordered forwarded argument names.
        arguments: Vec<&'static str>,
    },
}

/// One generated Rust status expression.
#[derive(Clone, Copy, Debug)]
pub enum HostAbiRustStatus {
    /// The host operation succeeded.
    Ok,
    /// The host operation is not supported.
    NotSupported,
}

/// One host ABI type descriptor.
#[derive(Clone, Debug, PartialEq)]
pub enum HostAbiType {
    /// One 8-bit unsigned integer.
    U8,
    /// One 16-bit unsigned integer.
    U16,
    /// One 8-bit signed integer.
    I8,
    /// One 16-bit signed integer.
    I16,
    /// One 32-bit unsigned integer.
    U32,
    /// One 32-bit signed integer.
    I32,
    /// One 64-bit unsigned integer.
    U64,
    /// One host request identifier.
    HostRequestId,
    /// One boolean flag.
    Bool,
    /// One 64-bit floating-point number.
    F64,
    /// One native string reference.
    StringRef,
    /// One borrowed native string-slice payload.
    StringSlice,
    /// One host session handle.
    HostSessionHandle,
    /// One host status code.
    HostStatus,
    /// One runtime status payload.
    RuntimeStatus,
    /// One native ABI array payload.
    NativeArray(Box<HostAbiType>),
    /// One borrowed native slice payload.
    NativeSlice(Box<HostAbiType>),
    /// One optional mutable output pointer.
    OutputPointer(Box<HostAbiType>),
    /// One named Rust ABI type.
    Named(&'static str),
}

macro_rules! host_abi_documentation {
    ($(#[doc = $doc:literal])*) => {
        concat!($($doc),*)
    };
}

macro_rules! host_abi_type {
    (u8) => {
        $crate::host::abi::describe::HostAbiType::U8
    };
    (u16) => {
        $crate::host::abi::describe::HostAbiType::U16
    };
    (i8) => {
        $crate::host::abi::describe::HostAbiType::I8
    };
    (i16) => {
        $crate::host::abi::describe::HostAbiType::I16
    };
    (u32) => {
        $crate::host::abi::describe::HostAbiType::U32
    };
    (i32) => {
        $crate::host::abi::describe::HostAbiType::I32
    };
    (u64) => {
        $crate::host::abi::describe::HostAbiType::U64
    };
    (host_request_id) => {
        $crate::host::abi::describe::HostAbiType::HostRequestId
    };
    (bool) => {
        $crate::host::abi::describe::HostAbiType::Bool
    };
    (f64) => {
        $crate::host::abi::describe::HostAbiType::F64
    };
    (string_ref) => {
        $crate::host::abi::describe::HostAbiType::StringRef
    };
    (string_slice) => {
        $crate::host::abi::describe::HostAbiType::StringSlice
    };
    (session_handle) => {
        $crate::host::abi::describe::HostAbiType::HostSessionHandle
    };
    (host_status) => {
        $crate::host::abi::describe::HostAbiType::HostStatus
    };
    (runtime_status) => {
        $crate::host::abi::describe::HostAbiType::RuntimeStatus
    };
    (array($inner:ident $(($($args:tt)*))?)) => {
        $crate::host::abi::describe::HostAbiType::NativeArray(Box::new(
            $crate::host::abi::describe::host_abi_type!($inner $(($($args)*))?)
        ))
    };
    (slice($inner:ident $(($($args:tt)*))?)) => {
        $crate::host::abi::describe::HostAbiType::NativeSlice(Box::new(
            $crate::host::abi::describe::host_abi_type!($inner $(($($args)*))?)
        ))
    };
    (output($inner:ident $(($($args:tt)*))?)) => {
        $crate::host::abi::describe::HostAbiType::OutputPointer(Box::new(
            $crate::host::abi::describe::host_abi_type!($inner $(($($args)*))?)
        ))
    };
    ($name:ident) => {
        $crate::host::abi::describe::HostAbiType::Named(stringify!($name))
    };
}

macro_rules! host_abi_rust_type {
    (u8) => {
        u8
    };
    (u16) => {
        u16
    };
    (i8) => {
        i8
    };
    (i16) => {
        i16
    };
    (u32) => {
        u32
    };
    (i32) => {
        i32
    };
    (u64) => {
        u64
    };
    (host_request_id) => {
        u64
    };
    (bool) => {
        bool
    };
    (f64) => {
        f64
    };
    (string_ref) => {
        $crate::platform::abi::NativeStringRef
    };
    (string_slice) => {
        $crate::platform::abi::NativeStringSlice
    };
    (session_handle) => {
        u64
    };
    (host_status) => {
        u32
    };
    (runtime_status) => {
        $crate::host::abi::describe::HostAbiRuntimeStatus
    };
    (array($inner:ident $(($($args:tt)*))?)) => {
        $crate::host::abi::describe::HostAbiNativeArray<
            $crate::host::abi::describe::host_abi_rust_type!($inner $(($($args)*))?)
        >
    };
    (slice($inner:ident $(($($args:tt)*))?)) => {
        $crate::platform::abi::NativeSlice<
            $crate::host::abi::describe::host_abi_rust_type!($inner $(($($args)*))?)
        >
    };
    (output($inner:ident $(($($args:tt)*))?)) => {
        *mut $crate::host::abi::describe::host_abi_rust_type!($inner $(($($args)*))?)
    };
    ($name:ident) => {
        $name
    };
}

macro_rules! host_abi_parameter {
    ($name:ident : $ty:ident $(($($args:tt)*))? ) => {
        $crate::host::abi::describe::HostAbiParameter {
            name: stringify!($name),
            ty: $crate::host::abi::describe::host_abi_type!($ty $(($($args)*))?),
        }
    };
}

macro_rules! host_abi_function {
    (
        $(#[doc = $doc:literal])*
        fn $name:ident (
            $($parameter_name:ident : $parameter_ty:ident $(($($parameter_args:tt)*))? ),* $(,)?
        ) -> $result_ty:ident $(($($result_args:tt)*))?;
    ) => {
        $crate::host::abi::describe::HostAbiFunction {
            name: stringify!($name),
            documentation: $crate::host::abi::describe::host_abi_documentation!($(#[doc = $doc]) *),
            parameters: vec![
                $($crate::host::abi::describe::host_abi_parameter!($parameter_name : $parameter_ty $(($($parameter_args)*))?)),*
            ],
            result: $crate::host::abi::describe::host_abi_type!($result_ty $(($($result_args)*))?),
        }
    };
}

macro_rules! host_abi_field {
    (
        $(#[doc = $doc:literal])*
        $name:ident : $ty:ident $(($($args:tt)*))?,
    ) => {
        $crate::host::abi::describe::HostAbiField {
            name: stringify!($name),
            documentation: $crate::host::abi::describe::host_abi_documentation!($(#[doc = $doc]) *),
            ty: $crate::host::abi::describe::host_abi_type!($ty $(($($args)*))?),
        }
    };
}

macro_rules! host_abi_variant {
    (
        $(#[doc = $doc:literal])*
        $name:ident = $discriminant:literal,
    ) => {
        $crate::host::abi::describe::HostAbiVariant {
            name: stringify!($name),
            documentation: $crate::host::abi::describe::host_abi_documentation!($(#[doc = $doc]) *),
            discriminant: $discriminant,
        }
    };
}

macro_rules! host_abi_types {
    (
        fn $function_name:ident() {
            $($items:tt)*
        }
    ) => {
        $crate::host::abi::describe::host_abi_types!(@emit_items $($items)*);

        /// Return the authored host ABI type declarations for this module.
        #[cfg(feature = "generator")]
        pub(crate) fn $function_name() -> Vec<$crate::host::abi::describe::HostAbiNamedType> {
            let mut types = Vec::new();
            $crate::host::abi::describe::host_abi_types!(@push types; $($items)*);
            types
        }
    };
    (@emit_items) => {};
    (@emit_items
        $(#[doc = $doc:literal])*
        $(#[derive($($derive:tt)*)])*
        struct $name:ident {
            $(
                $(#[doc = $field_doc:literal])*
                $field_name:ident : $field_ty:ident $(($($field_args:tt)*))?,
            )*
        }
        $($rest:tt)*
    ) => {
        $(#[doc = $doc])*
        #[derive(Clone, Copy, Debug)]
        $(#[derive($($derive)*)])*
        #[repr(C)]
        pub(crate) struct $name {
            $(
                $(#[doc = $field_doc])*
                pub $field_name: $crate::host::abi::describe::host_abi_rust_type!(
                    $field_ty $(($($field_args)*))?
                ),
            )*
        }

        $crate::host::abi::describe::host_abi_types!(@emit_items $($rest)*);
    };
    (@emit_items
        $(#[doc = $doc:literal])*
        enum $name:ident : $repr:ident {
            $(
                $(#[doc = $variant_doc:literal])*
                $variant_name:ident = $discriminant:literal,
            )*
        }
        $($rest:tt)*
    ) => {
        $(#[doc = $doc])*
        #[derive(Clone, Copy, Debug)]
        #[repr($repr)]
        pub(crate) enum $name {
            $(
                $(#[doc = $variant_doc])*
                $variant_name = $discriminant,
            )*
        }

        $crate::host::abi::describe::host_abi_types!(@emit_items $($rest)*);
    };
    (@push $types:ident;) => {};
    (@push $types:ident;
        $(#[doc = $doc:literal])*
        $(#[derive($($derive:tt)*)])*
        struct $name:ident {
            $(
                $(#[doc = $field_doc:literal])*
                $field_name:ident : $field_ty:ident $(($($field_args:tt)*))?,
            )*
        }
        $($rest:tt)*
    ) => {
        $types.push($crate::host::abi::describe::HostAbiNamedType {
            name: stringify!($name),
            documentation: $crate::host::abi::describe::host_abi_documentation!($(#[doc = $doc]) *),
            definition: $crate::host::abi::describe::HostAbiNamedTypeDefinition::Struct {
                fields: vec![
                    $(
                        $crate::host::abi::describe::host_abi_field!(
                            $(#[doc = $field_doc])*
                            $field_name : $field_ty $(($($field_args)*))?,
                        )
                    ),*
                ],
            },
        });
        $crate::host::abi::describe::host_abi_types!(@push $types; $($rest)*);
    };
    (@push $types:ident;
        $(#[doc = $doc:literal])*
        enum $name:ident : $repr:ident {
            $(
                $(#[doc = $variant_doc:literal])*
                $variant_name:ident = $discriminant:literal,
            )*
        }
        $($rest:tt)*
    ) => {
        $types.push($crate::host::abi::describe::HostAbiNamedType {
            name: stringify!($name),
            documentation: $crate::host::abi::describe::host_abi_documentation!($(#[doc = $doc]) *),
            definition: $crate::host::abi::describe::HostAbiNamedTypeDefinition::Enum {
                repr: $crate::host::abi::describe::host_abi_types!(@enum_repr $repr),
                variants: vec![
                    $(
                        $crate::host::abi::describe::host_abi_variant!(
                            $(#[doc = $variant_doc])*
                            $variant_name = $discriminant,
                        )
                    ),*
                ],
            },
        });
        $crate::host::abi::describe::host_abi_types!(@push $types; $($rest)*);
    };
    (@enum_repr i32) => {
        $crate::host::abi::describe::HostAbiEnumRepresentation::I32
    };
    (@enum_repr u32) => {
        $crate::host::abi::describe::HostAbiEnumRepresentation::U32
    };
}

macro_rules! host_abi_module {
    (
        fn $function_name:ident() -> $module_name:literal {
            platforms: [$($platform:ident),* $(,)?];
            types: $types:expr;
            requests {
                $($requests:tt)*
            }
            ingress {
                $($ingress:tt)*
            }
        }
    ) => {
        /// Return the authored host ABI declaration for this module.
        pub fn $function_name() -> $crate::host::abi::describe::HostAbiModule {
            let requests = host_abi_module!(@functions $($requests)*);
            let ingress = host_abi_module!(@functions $($ingress)*);

            $crate::host::abi::describe::HostAbiModule {
                name: $module_name,
                platforms: vec![
                    $(
                        $crate::host::abi::describe::host_abi_module!(@platform $platform)
                    ),*
                ],
                types: $types,
                requests,
                ingress,
                selector_controls: Vec::new(),
                rust_capability_probes: Vec::new(),
            }
        }
    };
    (@platform ios) => {
        $crate::host::abi::describe::HostAbiModulePlatform::Ios
    };
    (@platform android) => {
        $crate::host::abi::describe::HostAbiModulePlatform::Android
    };
    (@functions $($items:tt)*) => {{
        #[allow(unused_mut)]
        let mut functions = Vec::new();
        $crate::host::abi::describe::host_abi_module!(@push functions; $($items)*);
        functions
    }};
    (@push $functions:ident;) => {};
    (@push $functions:ident;
        $(#[doc = $doc:literal])*
        fn $name:ident (
            $($parameter_name:ident : $parameter_ty:ident $(($($parameter_args:tt)*))? ),* $(,)?
        ) -> $result_ty:ident $(($($result_args:tt)*))?;
        $($rest:tt)*
    ) => {
        $functions.push($crate::host::abi::describe::host_abi_function!(
            $(#[doc = $doc])*
            fn $name(
                $($parameter_name : $parameter_ty $(($($parameter_args)*))? ),*
            ) -> $result_ty $(($($result_args)*))?;
        ));
        $crate::host::abi::describe::host_abi_module!(@push $functions; $($rest)*);
    };
}

pub(crate) use {
    host_abi_documentation, host_abi_field, host_abi_function, host_abi_module, host_abi_parameter,
    host_abi_rust_type, host_abi_type, host_abi_types, host_abi_variant,
};

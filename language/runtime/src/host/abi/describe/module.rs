#![allow(dead_code)]
#![allow(unreachable_pub)]
#![allow(unused_imports)]
#![allow(unused_macros)]

/// One authored native-array projection.
pub type HostAbiNativeArray<T> = crate::platform::abi::NativeArray<T>;

/// One authored runtime-status projection.
pub type HostAbiRuntimeStatus = crate::platform::RuntimeStatus;

/// One authored platform-path projection.
pub type HostAbiOsPath = crate::platform::fs::OsPath;

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
    /// The platforms that expose this module on `RuntimeHost`.
    pub runtime_host_platforms: Vec<HostAbiModulePlatform>,
    /// The wrapper ownership for the `RuntimeHost` surface.
    pub runtime_host_wrapper_kind: HostAbiRuntimeHostWrapperKind,
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

/// One `RuntimeHost` wrapper ownership mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostAbiRuntimeHostWrapperKind {
    /// The wrapper is emitted by the generator.
    Generated,
    /// The wrapper remains handwritten.
    Manual,
}

/// One named host ABI payload type.
#[derive(Clone, Debug)]
pub struct HostAbiNamedType {
    /// The canonical type name.
    pub name: &'static str,
    /// The type documentation.
    pub documentation: &'static str,
    /// The optional runtime value projection path.
    pub value_path: Option<&'static str>,
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
    /// One tagged payload enum.
    TaggedEnum {
        /// The enum variants in declaration order.
        variants: Vec<HostAbiTaggedVariant>,
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

/// One named host ABI tagged-enum variant.
#[derive(Clone, Debug)]
pub struct HostAbiTaggedVariant {
    /// The variant name.
    pub name: &'static str,
    /// The variant documentation.
    pub documentation: &'static str,
    /// The payload type name.
    pub payload_type: &'static str,
}

/// One generated host ABI function.
#[derive(Clone, Debug)]
pub struct HostAbiFunction {
    /// The canonical function name.
    pub name: &'static str,
    /// The function documentation.
    pub documentation: &'static str,
    /// Whether the Android bridge callback must run on the main thread.
    pub android_main_thread: bool,
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
    /// One platform path payload.
    OsPath,
    /// One semantic optional payload.
    Optional(Box<HostAbiType>),
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
    (os_path) => {
        $crate::host::abi::describe::HostAbiType::OsPath
    };
    (option($inner:ident $(($($args:tt)*))?)) => {
        $crate::host::abi::describe::HostAbiType::Optional(Box::new(
            $crate::host::abi::describe::host_abi_type!($inner $(($($args)*))?)
        ))
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
    (os_path) => {
        $crate::host::abi::describe::HostAbiOsPath
    };
    (option($inner:ident $(($($args:tt)*))?)) => {
        $crate::host::abi::core::HostAbiOptional<
            $crate::host::abi::describe::host_abi_rust_type!($inner $(($($args)*))?)
        >
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
    (@android_main_thread) => {
        false
    };
    (@android_main_thread $value:literal) => {
        $value
    };
    (@android_main_thread_from_metadata) => {
        false
    };
    (@android_main_thread_from_metadata {
        host: {
            android_main_thread: $value:literal $(,)?
        }
    }) => {
        $value
    };
    (
        $(#[doc = $doc:literal])*
        $(android_main_thread: $android_main_thread:literal;)?
        fn $name:ident (
            $($parameter_name:ident : $parameter_ty:ident $(($($parameter_args:tt)*))? ),* $(,)?
        ) -> $result_ty:ident $(($($result_args:tt)*))?;
    ) => {
        $crate::host::abi::describe::HostAbiFunction {
            name: stringify!($name),
            documentation: $crate::host::abi::describe::host_abi_documentation!($(#[doc = $doc]) *),
            android_main_thread: $crate::host::abi::describe::host_abi_function!(
                @android_main_thread
                $($android_main_thread)?
            ),
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

    };
    (@emit_items) => {};
    (@value_path_string) => {
        None
    };
    (@value_path_string $value_path:path) => {
        Some(stringify!($value_path))
    };
    (@emit_items
        $(#[doc = $doc:literal])*
        $(#[value($value_path:path)])?
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

        $crate::host::abi::describe::host_abi_types!(
            @emit_native_struct_codec
            $name
            $(=> $value_path)?
            {
                $(
                    $field_name : $field_ty $(($($field_args)*))?,
                )*
            }
        );

        $crate::host::abi::describe::host_abi_types!(@emit_items $($rest)*);
    };
    (@emit_items
        $(#[doc = $doc:literal])*
        $(#[value($value_path:path)])?
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

        $crate::host::abi::describe::host_abi_types!(
            @emit_native_enum_codec
            $name
            $(=> $value_path)?
            {
                $(
                    $variant_name,
                )*
            }
        );

        $crate::host::abi::describe::host_abi_types!(@emit_items $($rest)*);
    };
    (@emit_items
        $(#[doc = $doc:literal])*
        $(#[value($value_path:path)])?
        enum $name:ident {
            $(
                $(#[doc = $variant_doc:literal])*
                $variant_name:ident($payload:ident),
            )*
        }
        $($rest:tt)*
    ) => {
        $(#[doc = $doc])*
        #[derive(Clone, Copy, Debug)]
        pub(crate) enum $name {
            $(
                $(#[doc = $variant_doc])*
                $variant_name($payload),
            )*
        }

        $crate::host::abi::describe::host_abi_types!(
            @emit_native_tagged_enum_codec
            $name
            $(=> $value_path)?
            {
                $(
                    $variant_name($payload),
                )*
            }
        );

        $crate::host::abi::describe::host_abi_types!(@emit_items $($rest)*);
    };
    (@push $types:ident;) => {};
    (@push $types:ident;
        $(#[doc = $doc:literal])*
        $(#[value($value_path:path)])?
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
            value_path: $crate::host::abi::describe::host_abi_types!(
                @value_path_string
                $($value_path)?
            ),
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
        $(#[value($value_path:path)])?
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
            value_path: $crate::host::abi::describe::host_abi_types!(
                @value_path_string
                $($value_path)?
            ),
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
    (@push $types:ident;
        $(#[doc = $doc:literal])*
        $(#[value($value_path:path)])?
        enum $name:ident {
            $(
                $(#[doc = $variant_doc:literal])*
                $variant_name:ident($payload:ident),
            )*
        }
        $($rest:tt)*
    ) => {
        $types.push($crate::host::abi::describe::HostAbiNamedType {
            name: stringify!($name),
            documentation: $crate::host::abi::describe::host_abi_documentation!($(#[doc = $doc]) *),
            value_path: $crate::host::abi::describe::host_abi_types!(
                @value_path_string
                $($value_path)?
            ),
            definition: $crate::host::abi::describe::HostAbiNamedTypeDefinition::TaggedEnum {
                variants: vec![
                    $(
                        $crate::host::abi::describe::HostAbiTaggedVariant {
                            name: stringify!($variant_name),
                            documentation: $crate::host::abi::describe::host_abi_documentation!($(#[doc = $variant_doc]) *),
                            payload_type: stringify!($payload),
                        }
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
    (@emit_native_struct_codec $name:ident { $($fields:tt)* }) => {};
    (@emit_native_struct_codec
        $name:ident => $value_path:path {
            $(
                $field_name:ident : $field_ty:ident $(($($field_args:tt)*))?,
            )*
        }
    ) => {
        impl $crate::platform::NativeAbiCodec for $name {
            type Value = $value_path;

            unsafe fn into_value(
                self,
            ) -> $crate::diagnostic::RuntimeResult<<Self as $crate::platform::NativeAbiCodec>::Value>
            {
                Ok(Self::Value {
                    $(
                        $field_name: unsafe {
                            <$crate::host::abi::describe::host_abi_rust_type!(
                                $field_ty $(($($field_args)*))?
                            ) as $crate::platform::NativeAbiCodec>::into_value(self.$field_name)?
                        },
                    )*
                })
            }

            fn from_value(
                binding: &$crate::runtime::BindingCallContext,
                value: <Self as $crate::platform::NativeAbiCodec>::Value,
            ) -> Self {
                Self {
                    $(
                        $field_name: <$crate::host::abi::describe::host_abi_rust_type!(
                            $field_ty $(($($field_args)*))?
                        ) as $crate::platform::NativeAbiCodec>::from_value(
                            binding,
                            value.$field_name,
                        ),
                    )*
                }
            }
        }
    };
    (@emit_native_enum_codec $name:ident { $($variants:tt)* }) => {};
    (@emit_native_enum_codec
        $name:ident => $value_path:path {
            $(
                $variant_name:ident,
            )*
        }
    ) => {
        impl $crate::platform::NativeAbiCodec for $name {
            type Value = $value_path;

            unsafe fn into_value(
                self,
            ) -> $crate::diagnostic::RuntimeResult<<Self as $crate::platform::NativeAbiCodec>::Value>
            {
                use $value_path as ValuePath;

                Ok(match self {
                    $(
                        Self::$variant_name => ValuePath::$variant_name,
                    )*
                })
            }

            fn from_value(
                _binding: &$crate::runtime::BindingCallContext,
                value: <Self as $crate::platform::NativeAbiCodec>::Value,
            ) -> Self {
                use $value_path as ValuePath;

                match value {
                    $(
                        ValuePath::$variant_name => Self::$variant_name,
                    )*
                }
            }
        }
    };
    (@emit_native_tagged_enum_codec $name:ident { $($variants:tt)* }) => {};
    (@emit_native_tagged_enum_codec
        $name:ident => $value_path:path {
            $(
                $variant_name:ident($payload:ident),
            )*
        }
    ) => {
        impl $crate::platform::NativeAbiCodec for $name {
            type Value = $value_path;

            unsafe fn into_value(
                self,
            ) -> $crate::diagnostic::RuntimeResult<<Self as $crate::platform::NativeAbiCodec>::Value>
            {
                use $value_path as ValuePath;

                Ok(match self {
                    $(
                        Self::$variant_name(value) => ValuePath::$variant_name(
                            unsafe { <$payload as $crate::platform::NativeAbiCodec>::into_value(value)? }
                        ),
                    )*
                })
            }

            fn from_value(
                binding: &$crate::runtime::BindingCallContext,
                value: <Self as $crate::platform::NativeAbiCodec>::Value,
            ) -> Self {
                use $value_path as ValuePath;

                match value {
                    $(
                        ValuePath::$variant_name(value) => Self::$variant_name(
                            <$payload as $crate::platform::NativeAbiCodec>::from_value(binding, value)
                        ),
                    )*
                }
            }
        }
    };
}

macro_rules! host_abi_module {
    (
        module $module_name:ident as $function_name:ident {
            platforms: [$($platform:ident),* $(,)?];
            $(host: $runtime_host_wrapper:ident [$($runtime_host_platform:ident),* $(,)?];)?
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
            let requests = host_abi_module!(@request_functions $($requests)*);
            let ingress = host_abi_module!(@functions $($ingress)*);

            $crate::host::abi::describe::HostAbiModule {
                name: stringify!($module_name),
                platforms: vec![
                    $(
                        $crate::host::abi::describe::host_abi_module!(@platform $platform)
                    ),*
                ],
                types: $types,
                requests,
                ingress,
                runtime_host_platforms: $crate::host::abi::describe::host_abi_module!(
                    @runtime_host_platforms $($($runtime_host_platform),*)?
                ),
                runtime_host_wrapper_kind: $crate::host::abi::describe::host_abi_module!(
                    @runtime_host_wrapper_kind $($runtime_host_wrapper)?
                ),
                selector_controls: Vec::new(),
                rust_capability_probes: Vec::new(),
            }
        }
    };
    (
        module $module_name:ident as $function_name:ident {
            platforms: [$($platform:ident),* $(,)?];
            $(host: $runtime_host_wrapper:ident [$($runtime_host_platform:ident),* $(,)?];)?
            types {
                $($types:tt)*
            }
            requests {
                $($requests:tt)*
            }
            ingress {
                $($ingress:tt)*
            }
        }
    ) => {
        $crate::host::abi::describe::host_abi_types!(@emit_items $($types)*);

        /// Return the authored host ABI declaration for this module.
        pub fn $function_name() -> $crate::host::abi::describe::HostAbiModule {
            let requests = host_abi_module!(@request_functions $($requests)*);
            let ingress = host_abi_module!(@functions $($ingress)*);

            #[allow(unused_mut)]
            let mut types = Vec::new();
            $crate::host::abi::describe::host_abi_types!(@push types; $($types)*);

            $crate::host::abi::describe::HostAbiModule {
                name: stringify!($module_name),
                platforms: vec![
                    $(
                        $crate::host::abi::describe::host_abi_module!(@platform $platform)
                    ),*
                ],
                types,
                requests,
                ingress,
                runtime_host_platforms: $crate::host::abi::describe::host_abi_module!(
                    @runtime_host_platforms $($($runtime_host_platform),*)?
                ),
                runtime_host_wrapper_kind: $crate::host::abi::describe::host_abi_module!(
                    @runtime_host_wrapper_kind $($runtime_host_wrapper)?
                ),
                selector_controls: Vec::new(),
                rust_capability_probes: Vec::new(),
            }
        }
    };
    (
        module $module_name:ident {
            platforms: [$($platform:ident),* $(,)?];
            $(host: $runtime_host_wrapper:ident [$($runtime_host_platform:ident),* $(,)?];)?
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
        pub fn host_abi_module() -> $crate::host::abi::describe::HostAbiModule {
            let requests = host_abi_module!(@request_functions $($requests)*);
            let ingress = host_abi_module!(@functions $($ingress)*);

            $crate::host::abi::describe::HostAbiModule {
                name: stringify!($module_name),
                platforms: vec![
                    $(
                        $crate::host::abi::describe::host_abi_module!(@platform $platform)
                    ),*
                ],
                types: $types,
                requests,
                ingress,
                runtime_host_platforms: $crate::host::abi::describe::host_abi_module!(
                    @runtime_host_platforms $($($runtime_host_platform),*)?
                ),
                runtime_host_wrapper_kind: $crate::host::abi::describe::host_abi_module!(
                    @runtime_host_wrapper_kind $($runtime_host_wrapper)?
                ),
                selector_controls: Vec::new(),
                rust_capability_probes: Vec::new(),
            }
        }
    };
    (
        module $module_name:ident {
            platforms: [$($platform:ident),* $(,)?];
            $(host: $runtime_host_wrapper:ident [$($runtime_host_platform:ident),* $(,)?];)?
            types {
                $($types:tt)*
            }
            requests {
                $($requests:tt)*
            }
            ingress {
                $($ingress:tt)*
            }
        }
    ) => {
        $crate::host::abi::describe::host_abi_types!(@emit_items $($types)*);

        /// Return the authored host ABI declaration for this module.
        pub fn host_abi_module() -> $crate::host::abi::describe::HostAbiModule {
            let requests = host_abi_module!(@request_functions $($requests)*);
            let ingress = host_abi_module!(@functions $($ingress)*);

            #[allow(unused_mut)]
            let mut types = Vec::new();
            $crate::host::abi::describe::host_abi_types!(@push types; $($types)*);

            $crate::host::abi::describe::HostAbiModule {
                name: stringify!($module_name),
                platforms: vec![
                    $(
                        $crate::host::abi::describe::host_abi_module!(@platform $platform)
                    ),*
                ],
                types,
                requests,
                ingress,
                runtime_host_platforms: $crate::host::abi::describe::host_abi_module!(
                    @runtime_host_platforms $($($runtime_host_platform),*)?
                ),
                runtime_host_wrapper_kind: $crate::host::abi::describe::host_abi_module!(
                    @runtime_host_wrapper_kind $($runtime_host_wrapper)?
                ),
                selector_controls: Vec::new(),
                rust_capability_probes: Vec::new(),
            }
        }
    };
    (
        fn $function_name:ident() -> $module_name:literal {
            platforms: [$($platform:ident),* $(,)?];
            $(host: $runtime_host_wrapper:ident [$($runtime_host_platform:ident),* $(,)?];)?
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
                runtime_host_platforms: $crate::host::abi::describe::host_abi_module!(
                    @runtime_host_platforms $($($runtime_host_platform),*)?
                ),
                runtime_host_wrapper_kind: $crate::host::abi::describe::host_abi_module!(
                    @runtime_host_wrapper_kind $($runtime_host_wrapper)?
                ),
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
    (@runtime_host_platforms) => {
        Vec::new()
    };
    (@runtime_host_platforms $($platform:ident),+ $(,)?) => {
        vec![
            $(
                $crate::host::abi::describe::host_abi_module!(@platform $platform)
            ),*
        ]
    };
    (@runtime_host_wrapper_kind) => {
        $crate::host::abi::describe::HostAbiRuntimeHostWrapperKind::Generated
    };
    (@runtime_host_wrapper_kind generated) => {
        $crate::host::abi::describe::HostAbiRuntimeHostWrapperKind::Generated
    };
    (@runtime_host_wrapper_kind manual) => {
        $crate::host::abi::describe::HostAbiRuntimeHostWrapperKind::Manual
    };
    (@functions $($items:tt)*) => {{
        #[allow(unused_mut)]
        let mut functions = Vec::new();
        $crate::host::abi::describe::host_abi_module!(@push functions; $($items)*);
        functions
    }};
    (@request_functions $($items:tt)*) => {{
        #[allow(unused_mut)]
        let mut functions = Vec::new();
        $crate::host::abi::describe::host_abi_module!(@push_requests functions; $($items)*);
        functions
    }};
    (@push $functions:ident;) => {};
    (@push $functions:ident;
        $(#[doc = $doc:literal])*
        $(android_main_thread: $android_main_thread:literal;)?
        fn $name:ident (
            $($parameter_name:ident : $parameter_ty:ident $(($($parameter_args:tt)*))? ),* $(,)?
        ) -> $result_ty:ident $(($($result_args:tt)*))?;
        $($rest:tt)*
    ) => {
        $functions.push($crate::host::abi::describe::host_abi_function!(
            $(#[doc = $doc])*
            $(android_main_thread: $android_main_thread;)?
            fn $name(
                $($parameter_name : $parameter_ty $(($($parameter_args)*))? ),*
            ) -> $result_ty $(($($result_args)*))?;
        ));
        $crate::host::abi::describe::host_abi_module!(@push $functions; $($rest)*);
    };
    (@push $functions:ident;
        $(#[doc = $doc:literal])*
        fn $name:ident (
            $($parameter_name:ident : $parameter_ty:ident $(($($parameter_args:tt)*))? ),* $(,)?
        ) -> $result_ty:ident $(($($result_args:tt)*))?
        {
            $($metadata:tt)*
        }
        $(;)?
        $($rest:tt)*
    ) => {
        $functions.push($crate::host::abi::describe::HostAbiFunction {
            name: stringify!($name),
            documentation: $crate::host::abi::describe::host_abi_documentation!($(#[doc = $doc]) *),
            android_main_thread: $crate::host::abi::describe::host_abi_function!(
                @android_main_thread_from_metadata
                {
                    $($metadata)*
                }
            ),
            parameters: vec![
                $($crate::host::abi::describe::host_abi_parameter!(
                    $parameter_name : $parameter_ty $(($($parameter_args)*))?
                )),*
            ],
            result: $crate::host::abi::describe::host_abi_type!($result_ty $(($($result_args)*))?),
        });
        $crate::host::abi::describe::host_abi_module!(@push $functions; $($rest)*);
    };
    (@push_requests $functions:ident;) => {};
    (@push_requests $functions:ident;
        $(#[doc = $doc:literal])*
        fn $name:ident (
            $($parameter_name:ident : $parameter_ty:ident $(($($parameter_args:tt)*))? ),* $(,)?
        ) -> $result_ty:ident $(($($result_args:tt)*))?;
        $($rest:tt)*
    ) => {
        $functions.push($crate::host::abi::describe::HostAbiFunction {
            name: stringify!($name),
            documentation: $crate::host::abi::describe::host_abi_documentation!($(#[doc = $doc]) *),
            android_main_thread: false,
            parameters: vec![
                $crate::host::abi::describe::host_abi_parameter!(session_handle : session_handle)
                $(, $crate::host::abi::describe::host_abi_parameter!(
                    $parameter_name : $parameter_ty $(($($parameter_args)*))?
                ))*
            ],
            result: $crate::host::abi::describe::host_abi_type!($result_ty $(($($result_args)*))?),
        });
        $crate::host::abi::describe::host_abi_module!(@push_requests $functions; $($rest)*);
    };
    (@push_requests $functions:ident;
        $(#[doc = $doc:literal])*
        fn $name:ident (
            $($parameter_name:ident : $parameter_ty:ident $(($($parameter_args:tt)*))? ),* $(,)?
        ) -> $result_ty:ident $(($($result_args:tt)*))?
        {
            $($metadata:tt)*
        }
        $(;)?
        $($rest:tt)*
    ) => {
        $functions.push($crate::host::abi::describe::HostAbiFunction {
            name: stringify!($name),
            documentation: $crate::host::abi::describe::host_abi_documentation!($(#[doc = $doc]) *),
            android_main_thread: $crate::host::abi::describe::host_abi_function!(
                @android_main_thread_from_metadata
                {
                    $($metadata)*
                }
            ),
            parameters: vec![
                $crate::host::abi::describe::host_abi_parameter!(session_handle : session_handle)
                $(, $crate::host::abi::describe::host_abi_parameter!(
                    $parameter_name : $parameter_ty $(($($parameter_args)*))?
                ))*
            ],
            result: $crate::host::abi::describe::host_abi_type!($result_ty $(($($result_args)*))?),
        });
        $crate::host::abi::describe::host_abi_module!(@push_requests $functions; $($rest)*);
    };
}

pub(crate) use {
    host_abi_documentation, host_abi_field, host_abi_function, host_abi_module, host_abi_parameter,
    host_abi_rust_type, host_abi_type, host_abi_types, host_abi_variant,
};

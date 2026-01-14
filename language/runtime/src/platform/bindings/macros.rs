/// Define and register a binding handler.
#[macro_export]
macro_rules! binding {
    ($registry:expr, $isolate:expr, $spec:expr, fn $name:ident($($args:tt)*) -> $ret:ty $body:block) => {{
        #[doc = "Binding handler."]
        fn $name($($args)*) -> $ret $body
        $registry.register_external($isolate, $spec, $name);
    }};
    ($registry:expr, $isolate:expr, $spec:expr, $handler:expr) => {{
        $registry.register_external($isolate, $spec, $handler);
    }};
}

/// Define a binding set for a host domain.
#[macro_export]
macro_rules! binding_set {
    ($vis:vis $name:ident, $label:expr, |$registry:ident, $isolate:ident, $host:ident| $body:block) => {
        #[doc = "Install the binding set into a registry."]
        fn __install(
            $registry: &mut $crate::platform::bindings::BindingRegistry,
            $isolate: &mut destack_vm::Isolate,
            $host: &$crate::platform::host::HostContext,
        ) {
            $body
        }

        #[doc = "Binding set for a host domain."]
        $vis const $name: $crate::platform::bindings::BindingSet =
            $crate::platform::bindings::BindingSet {
                name: $label,
                install: __install,
            };
    };
}

/// Define a native binding set for a host domain.
#[macro_export]
macro_rules! native_binding_set {
    ($vis:vis $name:ident, $label:expr, [$( $binding:expr ),* $(,)?]) => {
        #[doc = "Native binding set for a host domain."]
        $vis const $name: $crate::platform::bindings::NativeBindingSet =
            $crate::platform::bindings::NativeBindingSet {
                name: $label,
                bindings: &[$($binding),*],
            };
    };
}

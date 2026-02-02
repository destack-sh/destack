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

/// Define a native binding set for a platform domain.
#[macro_export]
macro_rules! native_binding_set {
    ($vis:vis $name:ident, $label:expr, [$( $binding:expr ),* $(,)?]) => {
        #[doc = "Native binding set for a platform domain."]
        $vis const $name: $crate::platform::bindings::NativeBindingSet =
            $crate::platform::bindings::NativeBindingSet {
                name: $label,
                bindings: &[$($binding),*],
            };
    };
}

/// Define a VM binding set for a platform domain.
#[macro_export]
macro_rules! vm_binding_set {
    ($vis:vis $name:ident, $label:expr, $install:expr $(,)?) => {
        #[doc = "VM binding set for a platform domain."]
        $vis const $name: $crate::platform::bindings::VmBindingSet =
            $crate::platform::bindings::VmBindingSet {
                name: $label,
                install: $install,
            };
    };
}

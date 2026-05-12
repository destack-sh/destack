/// Define a native binding set for a platform domain.
#[macro_export]
macro_rules! native_binding_set {
    ($vis:vis $name:ident, $label:expr, [$( $binding:expr ),* $(,)?]) => {
        #[doc = "Native binding set for a platform domain."]
        $vis const $name: $crate::runtime::binding::NativeBindingSet =
            $crate::runtime::binding::NativeBindingSet {
                name: $label,
                bindings: &[$($binding),*],
            };
    };
}

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::generate::core::to_snake;
use crate::generate::schema::Variant;

use super::convert::{into_bridge_value, rust_c_type};
use super::projection::{Projection, artifact_key_fields};

/// Render one artifact key constructor.
pub(super) fn render_artifact_key_constructor(
    projection: &Projection<'_>,
    variant: &Variant,
) -> TokenStream {
    let function = format_ident!("destack_artifact_key_{}", to_snake(&variant.name));
    let variant_name = variant.ident();
    let fields = artifact_key_fields(variant);
    let parameters = fields.iter().map(|field| {
        let name = format_ident!("{}", field.name);
        let ty = rust_c_type(field.ty, projection);

        quote!(#name: #ty,)
    });
    let conversions = fields.iter().map(|field| {
        let name = format_ident!("{}", field.name);
        let value = into_bridge_value(field.ty, quote!(#name), projection);

        quote!(let #name = #value;)
    });
    let value = if fields.is_empty() {
        quote!(destack::language::ArtifactKey::#variant_name)
    } else {
        let values = fields.iter().map(|field| {
            let name = format_ident!("{}", field.name);

            quote!(#name,)
        });

        quote!(destack::language::ArtifactKey::#variant_name {
            #(#values)*
        })
    };

    quote! {
        /// Create one artifact key handle.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #function(
            #(#parameters)*
            out: *mut *mut DestackArtifactKey,
            error: *mut *mut DestackError,
        ) -> DestackStatus {
            return_status(error, || {
                #(#conversions)*
                let value = #value;
                let key = Box::into_raw(Box::new(DestackArtifactKey { value }));

                write_out(out, key, "artifact key output is null")
            })
        }
    }
}

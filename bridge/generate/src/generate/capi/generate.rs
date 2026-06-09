use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Result, bail};
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use crate::generate::core::{
    Field, Item, ModulePath, Payload, PayloadNames, Schema, SchemaModule, Shape, Type, Variant,
    to_snake, write_rust, write_text,
};

const ARTIFACT_KEY: &str = "ArtifactKey";
const ROOTS: &[&str] = &[
    "ArtifactRecord",
    "ArtifactSidecar",
    "ArtifactVersion",
    "Diagnostic",
    "DirChecked",
    "DirParsed",
    "DirResolved",
    "Commit",
    "Module",
    "Revision",
    "SessionFile",
    "TextEdit",
];

/// Generate C ABI bridge bindings.
pub(in crate::generate) fn generate(root: &Path, schema: &Schema) -> Result<()> {
    let projection = Projection::new(schema)?;

    let header = Header::new(schema, &projection);
    for output in header.render() {
        write_text(root, &output.path, output.content)?;
    }

    let rust = Rust::new(schema, &projection);
    write_rust(root, "bridge/capi/src/generated.rs", rust.render())?;

    Ok(())
}

/// C ABI bridge projection.
struct Projection {
    /// Projected value DTOs in dependency order.
    values: Vec<String>,
    /// Projected opaque handles.
    handles: BTreeSet<String>,
    /// Artifact key variants.
    artifact_key_variants: Vec<String>,
}

/// C ABI header generator.
struct Header<'schema> {
    /// Bridge schema.
    schema: &'schema Schema,
    /// C ABI projection.
    projection: &'schema Projection,
}

/// One generated C header.
struct HeaderOutput {
    /// Header path under the workspace root.
    path: String,
    /// Header content.
    content: String,
}

/// C ABI Rust generator.
struct Rust<'schema> {
    /// Bridge schema.
    schema: &'schema Schema,
    /// C ABI projection.
    projection: &'schema Projection,
}

/// Text writer for generated C targets.
struct Text {
    /// Generated source.
    source: String,
}

/// One artifact key constructor field.
struct KeyField<'schema> {
    /// Field name.
    name: &'schema str,
    /// Field type.
    ty: &'schema Type,
}

impl Projection {
    /// Create one C ABI projection.
    fn new(schema: &Schema) -> Result<Self> {
        let handles = schema
            .items
            .values()
            .filter(|item| item.is_capi_handle)
            .map(|item| item.name.clone())
            .collect::<BTreeSet<_>>();
        let mut values = BTreeSet::new();

        for root in ROOTS {
            collect_value(schema, root, &mut values)?;
        }

        let variants = artifact_key_variants(schema)?
            .iter()
            .map(|variant| variant.name.clone())
            .collect::<Vec<_>>();
        for variant in artifact_key_variants(schema)? {
            for field in artifact_key_fields(variant) {
                collect_type(schema, field.ty, &mut values)?;
            }
        }

        for handle in &handles {
            values.remove(handle);
        }
        let values = order_values(schema, values)?;

        Ok(Self {
            values,
            handles,
            artifact_key_variants: variants,
        })
    }

    /// Return whether one bridge type is projected as a C ABI handle.
    fn is_handle(&self, name: &str) -> bool {
        self.handles.contains(name)
    }
}

impl<'schema> Header<'schema> {
    /// Create one C ABI header generator.
    fn new(schema: &'schema Schema, projection: &'schema Projection) -> Self {
        Self { schema, projection }
    }

    /// Render C ABI headers.
    fn render(&self) -> Vec<HeaderOutput> {
        let mut outputs = Vec::new();

        outputs.push(HeaderOutput {
            path: "bridge/capi/include/destack/generated.h".to_string(),
            content: self.render_umbrella(),
        });
        outputs.push(HeaderOutput {
            path: "bridge/capi/include/destack/core.generated.h".to_string(),
            content: self.render_core(),
        });

        for module in &self.schema.modules {
            if !self.module_has_header(module) {
                continue;
            }

            outputs.push(HeaderOutput {
                path: format!(
                    "bridge/capi/include/destack/{}.generated.h",
                    module.path.slash_path()
                ),
                content: self.render_module(module),
            });
        }

        outputs
    }

    /// Render the generated umbrella header.
    fn render_umbrella(&self) -> String {
        let mut text = Text::new();
        text.line("/* generated bridge target, do not edit */");
        text.blank();

        text.line("#ifndef DESTACK_GENERATED_H");
        text.line("#define DESTACK_GENERATED_H");
        text.blank();

        text.line("#include \"destack/core.generated.h\"");

        for module in &self.schema.modules {
            if !self.module_has_header(module) {
                continue;
            }

            text.line(format!(
                "#include \"destack/{}.generated.h\"",
                module.path.slash_path()
            ));
        }

        text.blank();
        text.line("#endif");

        text.finish()
    }

    /// Render C ABI core declarations.
    fn render_core(&self) -> String {
        let mut text = Text::new();
        text.line("/* generated bridge target, do not edit */");
        text.blank();

        text.line("#ifndef DESTACK_CORE_GENERATED_H");
        text.line("#define DESTACK_CORE_GENERATED_H");
        text.blank();
        text.line("#include \"destack/core.h\"");
        text.blank();
        text.line("#ifdef __cplusplus");
        text.line("extern \"C\" {");
        text.line("#endif");
        text.blank();

        for handle in &self.projection.handles {
            text.line(format!("typedef struct Destack{handle} Destack{handle};"));
        }
        text.blank();

        text.line("typedef struct DestackByteArray {");
        text.line("    uint8_t *ptr;");
        text.line("    size_t len;");
        text.line("} DestackByteArray;");
        text.blank();
        text.line("typedef struct DestackOptionalString {");
        text.line("    bool is_some;");
        text.line("    char *value;");
        text.line("} DestackOptionalString;");
        text.blank();

        text.line("void destack_byte_array_destroy(DestackByteArray array);");
        text.line("void destack_optional_string_destroy(DestackOptionalString value);");

        text.blank();
        text.line("#ifdef __cplusplus");
        text.line("}");
        text.line("#endif");
        text.blank();
        text.line("#endif");

        text.finish()
    }

    /// Render one module C ABI header.
    fn render_module(&self, module: &SchemaModule) -> String {
        let mut text = Text::new();
        let guard = header_guard(&module.path);
        text.line("/* generated bridge target, do not edit */");
        text.blank();
        text.line(format!("#ifndef {guard}"));
        text.line(format!("#define {guard}"));
        text.blank();

        text.line("#include \"destack/core.generated.h\"");
        for dependency in self.module_dependencies(module) {
            text.line(format!(
                "#include \"destack/{}.generated.h\"",
                dependency.slash_path()
            ));
        }
        text.blank();

        text.line("#ifdef __cplusplus");
        text.line("extern \"C\" {");
        text.line("#endif");
        text.blank();

        for name in self.module_values(module) {
            if !module.names.iter().any(|module_name| module_name == name) {
                continue;
            }

            self.render_item(&mut text, name.as_str());
            text.blank();
        }

        for name in &module.names {
            if self.projection.is_handle(name) {
                let snake = to_snake(name);
                text.line(format!(
                    "void destack_{snake}_destroy(Destack{name} *value);"
                ));
            }
        }
        for name in &self.projection.artifact_key_variants {
            let variant = artifact_key_variant(self.schema, name);
            if module
                .names
                .iter()
                .any(|module_name| module_name == ARTIFACT_KEY)
            {
                self.render_artifact_key_function(&mut text, variant);
            }
        }

        for name in self.module_values(module) {
            if !module.names.iter().any(|module_name| module_name == name) {
                continue;
            }

            let snake = to_snake(name);
            text.line(format!(
                "void destack_{snake}_destroy(Destack{name} *value);"
            ));
            text.line(format!(
                "void destack_{snake}_array_destroy(Destack{name}Array array);"
            ));
        }

        text.blank();
        text.line("#ifdef __cplusplus");
        text.line("}");
        text.line("#endif");
        text.blank();
        text.line("#endif");

        text.finish()
    }

    /// Return whether one module emits any C ABI header declarations.
    fn module_has_header(&self, module: &SchemaModule) -> bool {
        module.names.iter().any(|name| {
            self.projection.values.iter().any(|value| value == name)
                || self.projection.is_handle(name)
        })
    }

    /// Return generated header dependencies for one module.
    fn module_dependencies(&self, module: &SchemaModule) -> Vec<&ModulePath> {
        let mut dependencies = BTreeSet::<&ModulePath>::new();
        let current = &module.path;
        let values = module
            .names
            .iter()
            .filter(|name| self.projection.values.iter().any(|value| value == *name))
            .cloned()
            .collect::<Vec<_>>();

        for item in self.schema.referenced_types(&values) {
            if !self
                .projection
                .values
                .iter()
                .any(|value| value == &item.name)
                && !self.projection.is_handle(&item.name)
            {
                continue;
            }

            let dependency = self.schema.module_path(&item.name);
            if dependency != current {
                dependencies.insert(dependency);
            }
        }

        dependencies.into_iter().collect()
    }

    /// Return projected module value names in dependency order.
    fn module_values(&self, module: &SchemaModule) -> Vec<&String> {
        self.projection
            .values
            .iter()
            .filter(|value| module.names.iter().any(|name| name == *value))
            .collect()
    }

    /// Render one projected item.
    fn render_item(&self, text: &mut Text, name: &str) {
        let item = self.schema.item(name);

        match &item.shape {
            Shape::Struct(fields) => self.render_struct(text, item, fields),
            Shape::Enum(variants) if self.schema.is_unit_enum(name) => {
                self.render_unit_enum(text, item, variants);
            }
            Shape::Enum(variants) => self.render_payload_enum(text, item, variants),
        }

        text.blank();
        text.line(format!("typedef struct Destack{name}Array {{"));
        text.line(format!("    Destack{name} *ptr;"));
        text.line("    size_t len;");
        text.line(format!("}} Destack{name}Array;"));
        text.blank();
        text.line(format!("typedef struct DestackOptional{name} {{"));
        text.line("    bool is_some;");
        text.line(format!("    Destack{name} value;"));
        text.line(format!("}} DestackOptional{name};"));
    }

    /// Render one C struct.
    fn render_struct(&self, text: &mut Text, item: &Item, fields: &[Field]) {
        let name = c_type_name(&item.name);

        text.line(format!("typedef struct {name} {{"));
        for field in fields {
            let declaration = c_declaration(&field.name, &field.ty, self.projection);
            text.line(format!("    {declaration};"));
        }
        text.line(format!("}} {name};"));
    }

    /// Render one C unit enum.
    fn render_unit_enum(&self, text: &mut Text, item: &Item, variants: &[Variant]) {
        let name = c_type_name(&item.name);

        text.line(format!("typedef enum {name} {{"));
        for (index, variant) in variants.iter().enumerate() {
            let variant = c_enum_variant(&item.name, &variant.name);
            text.line(format!("    {variant} = {index},"));
        }
        text.line(format!("}} {name};"));
    }

    /// Render one C payload enum.
    fn render_payload_enum(&self, text: &mut Text, item: &Item, variants: &[Variant]) {
        let kind = format!("{}Kind", c_type_name(&item.name));

        text.line(format!("typedef enum {kind} {{"));
        for (index, variant) in variants.iter().enumerate() {
            let variant = c_enum_variant(&format!("{}Kind", item.name), &variant.name);
            text.line(format!("    {variant} = {index},"));
        }
        text.line(format!("}} {kind};"));
        text.blank();

        let names = PayloadNames::new(variants);
        let name = c_type_name(&item.name);
        text.line(format!("typedef struct {name} {{"));
        text.line(format!("    {kind} kind;"));
        for variant in variants {
            for field in payload_fields(&names, variant) {
                let declaration = c_declaration(&field.name, &field.ty, self.projection);
                text.line(format!("    {declaration};"));
            }
        }
        text.line(format!("}} {name};"));
    }

    /// Render one artifact key constructor declaration.
    fn render_artifact_key_function(&self, text: &mut Text, variant: &Variant) {
        let name = to_snake(&variant.name);
        text.line(format!("DestackStatus destack_artifact_key_{name}("));

        let fields = artifact_key_fields(variant);
        for field in &fields {
            let field = c_declaration(field.name, field.ty, self.projection);
            text.line(format!("    {field},"));
        }

        text.line("    DestackArtifactKey **out,");
        text.line("    DestackError **error");
        text.line(");");
    }
}

impl<'schema> Rust<'schema> {
    /// Create one C ABI Rust generator.
    fn new(schema: &'schema Schema, projection: &'schema Projection) -> Self {
        Self { schema, projection }
    }

    /// Render the generated Rust C ABI.
    fn render(&self) -> TokenStream {
        let items = self
            .projection
            .values
            .iter()
            .map(|name| self.render_item(name));
        let destructors = self
            .projection
            .values
            .iter()
            .map(|name| self.render_destructors(name));
        let handles = self
            .projection
            .handles
            .iter()
            .map(|name| self.render_handle(name));
        let handle_destructors = self
            .projection
            .handles
            .iter()
            .map(|name| self.render_handle_destructor(name));
        let constructors = self.projection.artifact_key_variants.iter().map(|name| {
            self.render_artifact_key_constructor(artifact_key_variant(self.schema, name))
        });

        quote! {
            use std::{ffi::c_char, ptr};

            use destack as rust;

            use crate::core::{
                DestackError, DestackStatus, c_string, destroy_array, destroy_string, owned_array,
                read_bytes, read_string, return_status, write_out,
            };

            /// C ABI owned byte array.
            #[repr(C)]
            #[derive(Debug)]
            pub struct DestackByteArray {
                /// Owned byte pointer.
                pub(crate) ptr: *mut u8,
                /// Byte count.
                pub(crate) len: usize,
            }

            /// C ABI optional owned string.
            #[repr(C)]
            #[derive(Debug)]
            pub struct DestackOptionalString {
                /// Whether the value is present.
                pub(crate) is_some: bool,
                /// Owned string value when present.
                pub(crate) value: *mut c_char,
            }

            impl DestackByteArray {
                /// Convert Rust bytes into one C ABI byte array.
                pub(crate) fn from_vec(values: Vec<u8>) -> Self {
                    let (ptr, len) = owned_array(values);

                    Self { ptr, len }
                }

                /// Convert this C ABI byte array into Rust bytes.
                pub(crate) fn into_vec(self) -> Result<Vec<u8>, String> {
                    read_bytes(self.ptr.cast_const(), self.len)
                }

                /// Destroy this C ABI byte array.
                pub(crate) fn destroy(&mut self) {
                    if self.ptr.is_null() {
                        return;
                    }

                    unsafe {
                        destroy_array(self.ptr, self.len, |_| {});
                    }
                    self.ptr = ptr::null_mut();
                    self.len = 0;
                }
            }

            impl DestackOptionalString {
                /// Convert one optional string into one C ABI optional string.
                pub(crate) fn from_bridge(value: Option<String>) -> Result<Self, String> {
                    let Some(value) = value else {
                        return Ok(Self::empty());
                    };

                    Ok(Self {
                        is_some: true,
                        value: c_string(value)?,
                    })
                }

                /// Convert this C ABI optional string into one optional string.
                pub(crate) fn to_bridge(&self) -> Result<Option<String>, String> {
                    if self.is_some {
                        Ok(Some(read_string(self.value)?))
                    } else {
                        Ok(None)
                    }
                }

                /// Destroy this C ABI optional string.
                pub(crate) fn destroy(&mut self) {
                    if self.is_some {
                        destroy_string(self.value);
                    }
                    *self = Self::empty();
                }

                /// Return one empty C ABI optional string.
                pub(crate) fn empty() -> Self {
                    Self {
                        is_some: false,
                        value: ptr::null_mut(),
                    }
                }
            }

            #(#handles)*

            #(#items)*

            #(#constructors)*

            /// Destroy one owned byte array.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn destack_byte_array_destroy(mut array: DestackByteArray) {
                array.destroy();
            }

            /// Destroy one optional owned string.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn destack_optional_string_destroy(mut value: DestackOptionalString) {
                value.destroy();
            }

            #(#handle_destructors)*

            #(#destructors)*
        }
    }

    /// Render one opaque C ABI handle.
    fn render_handle(&self, name: &str) -> TokenStream {
        let handle = format_ident!("Destack{name}");
        let bridge = format_ident!("{name}");
        let docs = format!(" C ABI {} handle.", to_snake(name).replace('_', " "));

        quote! {
            #[doc = #docs]
            #[repr(C)]
            #[derive(Debug)]
            pub struct #handle {
                /// Rust bridge value.
                pub(crate) value: rust::#bridge,
            }
        }
    }

    /// Render one opaque C ABI handle destructor.
    fn render_handle_destructor(&self, name: &str) -> TokenStream {
        let handle = format_ident!("Destack{name}");
        let destroy = format_ident!("destack_{}_destroy", to_snake(name));
        let docs = format!(" Destroy one {} handle.", to_snake(name).replace('_', " "));

        quote! {
            #[doc = #docs]
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn #destroy(value: *mut #handle) {
                if value.is_null() {
                    return;
                }

                drop(unsafe { Box::from_raw(value) });
            }
        }
    }

    /// Render one projected item.
    fn render_item(&self, name: &str) -> TokenStream {
        let item = self.schema.item(name);

        match &item.shape {
            Shape::Struct(fields) => self.render_struct(item, fields),
            Shape::Enum(variants) if self.schema.is_unit_enum(name) => {
                self.render_unit_enum(item, variants)
            }
            Shape::Enum(variants) => self.render_payload_enum(item, variants),
        }
    }

    /// Render one Rust C struct.
    fn render_struct(&self, item: &Item, fields: &[Field]) -> TokenStream {
        let name = format_ident!("Destack{}", item.name);
        let bridge_name = item.ident();
        let array_name = format_ident!("Destack{}Array", item.name);
        let optional_name = format_ident!("DestackOptional{}", item.name);
        let array_impl = array_impl(&name, &bridge_name, &array_name);
        let optional_impl = optional_impl(&name, &bridge_name, &optional_name);
        let field_defs = fields.iter().map(|field| {
            let docs = field.docs();
            let name = format_ident!("{}", field.name);
            let ty = rust_c_type(&field.ty, self.projection);

            quote! {
                #docs
                pub(crate) #name: #ty,
            }
        });
        let from_fields = fields.iter().map(|field| {
            let name = format_ident!("{}", field.name);
            let value = from_bridge_value(&field.ty, quote!(value.#name), self.projection);

            quote!(#name: #value,)
        });
        let to_fields = fields.iter().map(|field| {
            let name = format_ident!("{}", field.name);
            let value = to_bridge_value(&field.ty, quote!(self.#name), self.projection);

            quote!(#name: #value,)
        });
        let destroy_fields = fields.iter().map(|field| {
            let name = format_ident!("{}", field.name);
            destroy_value(&field.ty, quote!(self.#name), self.projection)
        });
        let empty_fields = fields.iter().map(|field| {
            let name = format_ident!("{}", field.name);
            let value = empty_value(&field.ty, self.projection);

            quote!(#name: #value,)
        });

        quote! {
            /// C ABI bridge value.
            #[repr(C)]
            #[derive(Debug)]
            pub struct #name {
                #(#field_defs)*
            }

            /// C ABI bridge value array.
            #[repr(C)]
            #[derive(Debug)]
            pub struct #array_name {
                /// Owned value pointer.
                pub(crate) ptr: *mut #name,
                /// Value count.
                pub(crate) len: usize,
            }

            /// C ABI optional bridge value.
            #[repr(C)]
            #[derive(Debug)]
            pub struct #optional_name {
                /// Whether the value is present.
                pub(crate) is_some: bool,
                /// Value when present.
                pub(crate) value: #name,
            }

            impl #name {
                /// Convert one bridge value into one C ABI value.
                pub(crate) fn from_bridge(value: rust::#bridge_name) -> Result<Self, String> {
                    Ok(Self {
                        #(#from_fields)*
                    })
                }

                /// Convert this C ABI value into one bridge value.
                pub(crate) fn to_bridge(&self) -> Result<rust::#bridge_name, String> {
                    Ok(rust::#bridge_name {
                        #(#to_fields)*
                    })
                }

                /// Destroy this C ABI value.
                pub(crate) fn destroy(&mut self) {
                    #(#destroy_fields)*
                }

                /// Return one empty C ABI value.
                pub(crate) fn empty() -> Self {
                    Self {
                        #(#empty_fields)*
                    }
                }
            }

            #array_impl

            #optional_impl
        }
    }

    /// Render one Rust C unit enum.
    fn render_unit_enum(&self, item: &Item, variants: &[Variant]) -> TokenStream {
        let name = format_ident!("Destack{}", item.name);
        let bridge_name = item.ident();
        let array_name = format_ident!("Destack{}Array", item.name);
        let optional_name = format_ident!("DestackOptional{}", item.name);
        let array_impl = array_impl(&name, &bridge_name, &array_name);
        let optional_impl = optional_impl(&name, &bridge_name, &optional_name);
        let empty = variants[0].ident();
        let variant_defs = variants.iter().enumerate().map(|(index, variant)| {
            let docs = variant.docs();
            let name = variant.ident();
            let index = Literal::usize_unsuffixed(index);

            quote! {
                #docs
                #name = #index,
            }
        });
        let from_arms = variants.iter().map(|variant| {
            let name = variant.ident();

            quote!(rust::#bridge_name::#name => Self::#name,)
        });
        let into_arms = variants.iter().map(|variant| {
            let name = variant.ident();

            quote!(Self::#name => rust::#bridge_name::#name,)
        });

        quote! {
            /// C ABI bridge enum.
            #[repr(C)]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum #name {
                #(#variant_defs)*
            }

            /// C ABI bridge enum array.
            #[repr(C)]
            #[derive(Debug)]
            pub struct #array_name {
                /// Owned value pointer.
                pub(crate) ptr: *mut #name,
                /// Value count.
                pub(crate) len: usize,
            }

            /// C ABI optional bridge enum.
            #[repr(C)]
            #[derive(Debug)]
            pub struct #optional_name {
                /// Whether the value is present.
                pub(crate) is_some: bool,
                /// Value when present.
                pub(crate) value: #name,
            }

            impl #name {
                /// Convert one bridge enum into one C ABI enum.
                pub(crate) fn from_bridge(value: rust::#bridge_name) -> Result<Self, String> {
                    Ok(match value {
                        #(#from_arms)*
                    })
                }

                /// Convert this C ABI enum into one bridge enum.
                pub(crate) fn to_bridge(&self) -> Result<rust::#bridge_name, String> {
                    Ok(match *self {
                        #(#into_arms)*
                    })
                }

                /// Destroy this C ABI enum.
                pub(crate) fn destroy(&mut self) {}

                /// Return one empty C ABI enum.
                pub(crate) fn empty() -> Self {
                    Self::#empty
                }
            }

            #array_impl

            #optional_impl
        }
    }

    /// Render one Rust C payload enum.
    fn render_payload_enum(&self, item: &Item, variants: &[Variant]) -> TokenStream {
        let name = format_ident!("Destack{}", item.name);
        let kind = format_ident!("Destack{}Kind", item.name);
        let bridge_name = item.ident();
        let array_name = format_ident!("Destack{}Array", item.name);
        let optional_name = format_ident!("DestackOptional{}", item.name);
        let array_impl = array_impl(&name, &bridge_name, &array_name);
        let optional_impl = optional_impl(&name, &bridge_name, &optional_name);
        let empty_kind = variants[0].ident();
        let kind_defs = variants.iter().enumerate().map(|(index, variant)| {
            let docs = variant.docs();
            let name = variant.ident();
            let index = Literal::usize_unsuffixed(index);

            quote! {
                #docs
                #name = #index,
            }
        });
        let names = PayloadNames::new(variants);
        let field_defs = variants
            .iter()
            .flat_map(|variant| payload_fields(&names, variant))
            .map(|field| {
                let name = format_ident!("{}", field.name);
                let ty = rust_c_type(&field.ty, self.projection);

                quote!(pub(crate) #name: #ty,)
            })
            .collect::<Vec<_>>();
        let all_fields = variants
            .iter()
            .flat_map(|variant| payload_fields(&names, variant))
            .collect::<Vec<_>>();
        let empty_fields = all_fields.iter().map(|field| {
            let name = format_ident!("{}", field.name);
            let value = empty_value(&field.ty, self.projection);

            quote!(#name: #value,)
        });
        let from_arms = variants.iter().map(|variant| {
            let variant_name = variant.ident();
            let active = payload_fields(&names, variant);
            let values = all_fields.iter().map(|field| {
                let target = format_ident!("{}", field.name);
                if let Some(active) = active.iter().find(|active| active.name == field.name) {
                    let source = format_ident!("{}", active.source_name);
                    let value = from_bridge_value(active.ty, quote!(#source), self.projection);

                    quote!(#target: #value,)
                } else {
                    let value = empty_value(field.ty, self.projection);

                    quote!(#target: #value,)
                }
            });
            let pattern = variant_pattern(variant);

            quote! {
                rust::#bridge_name::#variant_name #pattern => Self {
                    kind: #kind::#variant_name,
                    #(#values)*
                },
            }
        });
        let into_arms = variants.iter().map(|variant| {
            let variant_name = variant.ident();
            let fields = payload_fields(&names, variant);
            let conversions = fields.iter().map(|field| {
                let name = format_ident!("{}", field.name);
                let value = to_bridge_value(&field.ty, quote!(self.#name), self.projection);

                quote!(let #name = #value;)
            });
            let value = variant_value(variant, &fields);

            quote! {
                #kind::#variant_name => {
                    #(#conversions)*
                    Ok(rust::#bridge_name::#variant_name #value)
                }
            }
        });
        let destroy_fields = variants
            .iter()
            .flat_map(|variant| payload_fields(&names, variant))
            .map(|field| {
                let name = format_ident!("{}", field.name);
                destroy_value(&field.ty, quote!(self.#name), self.projection)
            });

        quote! {
            /// C ABI bridge enum kind.
            #[repr(C)]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum #kind {
                #(#kind_defs)*
            }

            /// C ABI bridge enum.
            #[repr(C)]
            #[derive(Debug)]
            pub struct #name {
                /// Active enum variant.
                pub(crate) kind: #kind,
                #(#field_defs)*
            }

            /// C ABI bridge enum array.
            #[repr(C)]
            #[derive(Debug)]
            pub struct #array_name {
                /// Owned value pointer.
                pub(crate) ptr: *mut #name,
                /// Value count.
                pub(crate) len: usize,
            }

            /// C ABI optional bridge enum.
            #[repr(C)]
            #[derive(Debug)]
            pub struct #optional_name {
                /// Whether the value is present.
                pub(crate) is_some: bool,
                /// Value when present.
                pub(crate) value: #name,
            }

            impl #name {
                /// Convert one bridge enum into one C ABI enum.
                pub(crate) fn from_bridge(value: rust::#bridge_name) -> Result<Self, String> {
                    Ok(match value {
                        #(#from_arms)*
                    })
                }

                /// Convert this C ABI enum into one bridge enum.
                pub(crate) fn to_bridge(&self) -> Result<rust::#bridge_name, String> {
                    match self.kind {
                        #(#into_arms)*
                    }
                }

                /// Destroy this C ABI enum.
                pub(crate) fn destroy(&mut self) {
                    #(#destroy_fields)*
                }

                /// Return one empty C ABI enum.
                pub(crate) fn empty() -> Self {
                    Self {
                        kind: #kind::#empty_kind,
                        #(#empty_fields)*
                    }
                }
            }

            #array_impl

            #optional_impl
        }
    }

    /// Render destructors for one projected value.
    fn render_destructors(&self, name: &str) -> TokenStream {
        let value = format_ident!("Destack{name}");
        let array = format_ident!("Destack{name}Array");
        let destroy_value = format_ident!("destack_{}_destroy", to_snake(name));
        let destroy_array = format_ident!("destack_{}_array_destroy", to_snake(name));

        quote! {
            /// Destroy one C ABI bridge value.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn #destroy_value(value: *mut #value) {
                if let Some(value) = unsafe { value.as_mut() } {
                    value.destroy();
                }
            }

            /// Destroy one C ABI bridge value array.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn #destroy_array(mut array: #array) {
                array.destroy();
            }
        }
    }

    /// Render one artifact key constructor.
    fn render_artifact_key_constructor(&self, variant: &Variant) -> TokenStream {
        let function = format_ident!("destack_artifact_key_{}", to_snake(&variant.name));
        let variant_name = variant.ident();
        let fields = artifact_key_fields(variant);
        let parameters = fields.iter().map(|field| {
            let name = format_ident!("{}", field.name);
            let ty = rust_c_type(field.ty, self.projection);

            quote!(#name: #ty,)
        });
        let conversions = fields.iter().map(|field| {
            let name = format_ident!("{}", field.name);
            let value = into_bridge_value(field.ty, quote!(#name), self.projection);

            quote!(let #name = #value;)
        });
        let value = if fields.is_empty() {
            quote!(rust::ArtifactKey::#variant_name)
        } else {
            let values = fields.iter().map(|field| {
                let name = format_ident!("{}", field.name);

                quote!(#name,)
            });

            quote!(rust::ArtifactKey::#variant_name {
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
}

impl Text {
    /// Create one empty text writer.
    fn new() -> Self {
        Self {
            source: String::new(),
        }
    }

    /// Write one source line.
    fn line(&mut self, line: impl AsRef<str>) {
        self.source.push_str(line.as_ref());
        self.source.push('\n');
    }

    /// Write one blank line.
    fn blank(&mut self) {
        self.source.push('\n');
    }

    /// Return the generated text.
    fn finish(self) -> String {
        self.source
    }
}

/// One C ABI payload field.
struct PayloadField<'schema> {
    /// Rust field name in the source variant.
    source_name: String,
    /// C ABI field name.
    name: String,
    /// Field type.
    ty: &'schema Type,
}

/// Return fields for one payload variant.
fn payload_fields<'schema>(
    names: &PayloadNames,
    variant: &'schema Variant,
) -> Vec<PayloadField<'schema>> {
    match &variant.payload {
        Payload::Unit => Vec::new(),
        Payload::Tuple(ty) => vec![PayloadField {
            source_name: variant.payload_field_name(),
            name: names.tuple_label(variant),
            ty,
        }],
        Payload::Struct(fields) => fields
            .iter()
            .map(|field| PayloadField {
                source_name: field.name.clone(),
                name: names.field_name(variant, field),
                ty: &field.ty,
            })
            .collect(),
    }
}

/// Return one Rust pattern for a bridge enum variant.
fn variant_pattern(variant: &Variant) -> TokenStream {
    match &variant.payload {
        Payload::Unit => quote!(),
        Payload::Tuple(_) => {
            let name = variant.payload_field_ident();

            quote!((#name))
        }
        Payload::Struct(fields) => {
            let fields = fields.iter().map(|field| field.ident());

            quote!({ #(#fields,)* })
        }
    }
}

/// Return one Rust value for a bridge enum variant.
fn variant_value(variant: &Variant, fields: &[PayloadField<'_>]) -> TokenStream {
    match &variant.payload {
        Payload::Unit => quote!(),
        Payload::Tuple(_) => {
            let name = format_ident!("{}", fields[0].name);

            quote!((#name))
        }
        Payload::Struct(_) => {
            let fields = fields.iter().map(|field| {
                let source = format_ident!("{}", field.source_name);
                let name = format_ident!("{}", field.name);

                quote!(#source: #name,)
            });

            quote!({ #(#fields)* })
        }
    }
}

/// Render one array implementation.
fn array_impl(
    value: &proc_macro2::Ident,
    bridge: &proc_macro2::Ident,
    array: &proc_macro2::Ident,
) -> TokenStream {
    quote! {
        impl #array {
            /// Convert bridge values into one C ABI array.
            pub(crate) fn from_bridge(values: Vec<rust::#bridge>) -> Result<Self, String> {
                let mut converted = Vec::with_capacity(values.len());
                for value in values {
                    converted.push(#value::from_bridge(value)?);
                }
                let (ptr, len) = owned_array(converted);

                Ok(Self { ptr, len })
            }

            /// Convert this C ABI array into bridge values.
            pub(crate) fn to_bridge(&self) -> Result<Vec<rust::#bridge>, String> {
                if self.len == 0 {
                    return Ok(Vec::new());
                }
                if self.ptr.is_null() {
                    return Err("array pointer is null".to_string());
                }

                let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
                let mut converted = Vec::with_capacity(values.len());
                for value in values {
                    converted.push(value.to_bridge()?);
                }

                Ok(converted)
            }

            /// Destroy this C ABI array.
            pub(crate) fn destroy(&mut self) {
                if self.ptr.is_null() {
                    return;
                }

                unsafe {
                    destroy_array(self.ptr, self.len, |value| value.destroy());
                }
                self.ptr = ptr::null_mut();
                self.len = 0;
            }
        }
    }
}

/// Render one optional implementation.
fn optional_impl(
    value: &proc_macro2::Ident,
    bridge: &proc_macro2::Ident,
    optional: &proc_macro2::Ident,
) -> TokenStream {
    quote! {
        impl #optional {
            /// Convert one optional bridge value into one C ABI optional value.
            pub(crate) fn from_bridge(value: Option<rust::#bridge>) -> Result<Self, String> {
                let Some(value) = value else {
                    return Ok(Self {
                        is_some: false,
                        value: #value::empty(),
                    });
                };

                Ok(Self {
                    is_some: true,
                    value: #value::from_bridge(value)?,
                })
            }

            /// Convert this C ABI optional value into one bridge optional value.
            pub(crate) fn to_bridge(&self) -> Result<Option<rust::#bridge>, String> {
                if self.is_some {
                    Ok(Some(self.value.to_bridge()?))
                } else {
                    Ok(None)
                }
            }

            /// Destroy this C ABI optional value.
            pub(crate) fn destroy(&mut self) {
                if self.is_some {
                    self.value.destroy();
                }
                self.is_some = false;
                self.value = #value::empty();
            }
        }
    }
}

/// Return `ArtifactKey` variants.
fn artifact_key_variants(schema: &Schema) -> Result<&[Variant]> {
    let Shape::Enum(variants) = &schema.item(ARTIFACT_KEY).shape else {
        bail!("ArtifactKey must be a bridge enum");
    };

    Ok(variants)
}

/// Return one artifact key variant.
fn artifact_key_variant<'schema>(schema: &'schema Schema, name: &str) -> &'schema Variant {
    artifact_key_variants(schema)
        .expect("ArtifactKey must be a bridge enum")
        .iter()
        .find(|variant| variant.name == name)
        .expect("artifact key variant must exist")
}

/// Return artifact key constructor fields.
fn artifact_key_fields(variant: &Variant) -> Vec<KeyField<'_>> {
    match &variant.payload {
        Payload::Unit => Vec::new(),
        Payload::Struct(fields) => fields
            .iter()
            .map(|field| KeyField {
                name: field.name.as_str(),
                ty: &field.ty,
            })
            .collect(),
        Payload::Tuple(_) => Vec::new(),
    }
}

/// Collect one named value.
fn collect_value(schema: &Schema, name: &str, values: &mut BTreeSet<String>) -> Result<()> {
    if name == ARTIFACT_KEY || !values.insert(name.to_string()) {
        return Ok(());
    }

    let item = schema.item(name);
    match &item.shape {
        Shape::Struct(fields) => {
            for field in fields {
                collect_type(schema, &field.ty, values)?;
            }
        }
        Shape::Enum(variants) => {
            for variant in variants {
                for (_name, ty) in variant.payload_fields() {
                    collect_type(schema, &ty, values)?;
                }
            }
        }
    }

    Ok(())
}

/// Collect one field type.
fn collect_type(schema: &Schema, ty: &Type, values: &mut BTreeSet<String>) -> Result<()> {
    match ty {
        Type::Vec(inner) | Type::Option(inner) => collect_type(schema, inner, values),
        Type::Named(name) => collect_value(schema, name, values),
        Type::String | Type::Bool | Type::U8 | Type::U32 | Type::Usize => Ok(()),
    }
}

/// Return values in dependency order.
fn order_values(schema: &Schema, values: BTreeSet<String>) -> Result<Vec<String>> {
    let mut dependencies = BTreeMap::<String, BTreeSet<String>>::new();

    for name in &values {
        let mut item_dependencies = BTreeSet::new();
        collect_dependencies(schema, name, &values, &mut item_dependencies)?;
        dependencies.insert(name.clone(), item_dependencies);
    }

    let mut ordered = Vec::new();
    let mut emitted = BTreeSet::new();
    for name in values {
        emit_value(&name, &dependencies, &mut emitted, &mut ordered);
    }

    Ok(ordered)
}

/// Collect dependencies for one projected value.
fn collect_dependencies(
    schema: &Schema,
    name: &str,
    values: &BTreeSet<String>,
    dependencies: &mut BTreeSet<String>,
) -> Result<()> {
    let item = schema.item(name);
    match &item.shape {
        Shape::Struct(fields) => {
            for field in fields {
                collect_type_dependencies(&field.ty, values, dependencies);
            }
        }
        Shape::Enum(variants) => {
            for variant in variants {
                for (_name, ty) in variant.payload_fields() {
                    collect_type_dependencies(&ty, values, dependencies);
                }
            }
        }
    }

    Ok(())
}

/// Collect type dependencies.
fn collect_type_dependencies(
    ty: &Type,
    values: &BTreeSet<String>,
    dependencies: &mut BTreeSet<String>,
) {
    match ty {
        Type::Vec(inner) | Type::Option(inner) => {
            collect_type_dependencies(inner, values, dependencies)
        }
        Type::Named(name) if values.contains(name) => {
            dependencies.insert(name.clone());
        }
        Type::Named(_) | Type::String | Type::Bool | Type::U8 | Type::U32 | Type::Usize => {}
    }
}

/// Emit one value after its dependencies.
fn emit_value(
    name: &str,
    dependencies: &BTreeMap<String, BTreeSet<String>>,
    emitted: &mut BTreeSet<String>,
    ordered: &mut Vec<String>,
) {
    if emitted.contains(name) {
        return;
    }

    if let Some(type_dependencies) = dependencies.get(name) {
        for dependency in type_dependencies {
            emit_value(dependency, dependencies, emitted, ordered);
        }
    }

    emitted.insert(name.to_string());
    ordered.push(name.to_string());
}

/// Return one generated C type name.
fn c_type_name(name: &str) -> String {
    format!("Destack{name}")
}

/// Render one C declaration fragment.
fn c_declaration(name: &str, ty: &Type, projection: &Projection) -> String {
    let ty = c_type(ty, projection);
    if ty.ends_with('*') {
        format!("{ty}{name}")
    } else {
        format!("{ty} {name}")
    }
}

/// Render one C type.
fn c_type(ty: &Type, projection: &Projection) -> String {
    match ty {
        Type::String => "char *".to_string(),
        Type::Bool => "bool".to_string(),
        Type::U8 => "uint8_t".to_string(),
        Type::U32 => "uint32_t".to_string(),
        Type::Usize => "size_t".to_string(),
        Type::Vec(inner) if **inner == Type::U8 => "DestackByteArray".to_string(),
        Type::Vec(inner) => format!("Destack{}Array", named_type(inner)),
        Type::Option(inner) if **inner == Type::String => "DestackOptionalString".to_string(),
        Type::Option(inner) => format!("DestackOptional{}", named_type(inner)),
        Type::Named(name) if projection.is_handle(name) => format!("Destack{name} *"),
        Type::Named(name) => c_type_name(name),
    }
}

/// Return one named type inside a collection.
fn named_type(ty: &Type) -> &str {
    let Type::Named(name) = ty else {
        panic!("C ABI collection element must be named");
    };

    name
}

/// Return one Rust C type.
fn rust_c_type(ty: &Type, projection: &Projection) -> TokenStream {
    match ty {
        Type::String => quote!(*mut c_char),
        Type::Bool => quote!(bool),
        Type::U8 => quote!(u8),
        Type::U32 => quote!(u32),
        Type::Usize => quote!(usize),
        Type::Vec(inner) if **inner == Type::U8 => quote!(DestackByteArray),
        Type::Vec(inner) => {
            let ty = format_ident!("Destack{}Array", named_type(inner));

            quote!(#ty)
        }
        Type::Option(inner) if **inner == Type::String => quote!(DestackOptionalString),
        Type::Option(inner) => {
            let ty = format_ident!("DestackOptional{}", named_type(inner));

            quote!(#ty)
        }
        Type::Named(name) if projection.is_handle(name) => {
            let ty = format_ident!("Destack{name}");

            quote!(*mut #ty)
        }
        Type::Named(name) => {
            let ty = format_ident!("Destack{name}");

            quote!(#ty)
        }
    }
}

/// Return one value converted from bridge DTO space.
fn from_bridge_value(ty: &Type, value: TokenStream, projection: &Projection) -> TokenStream {
    match ty {
        Type::String => quote!(c_string(#value)?),
        Type::Bool | Type::U8 | Type::U32 | Type::Usize => value,
        Type::Vec(inner) if **inner == Type::U8 => quote!(DestackByteArray::from_vec(#value)),
        Type::Vec(inner) => {
            let ty = format_ident!("Destack{}Array", named_type(inner));

            quote!(#ty::from_bridge(#value)?)
        }
        Type::Option(inner) if **inner == Type::String => {
            quote!(DestackOptionalString::from_bridge(#value)?)
        }
        Type::Option(inner) => {
            let ty = format_ident!("DestackOptional{}", named_type(inner));

            quote!(#ty::from_bridge(#value)?)
        }
        Type::Named(name) if projection.is_handle(name) => {
            let ty = format_ident!("Destack{name}");

            quote!(Box::into_raw(Box::new(#ty { value: #value })))
        }
        Type::Named(name) => {
            let ty = format_ident!("Destack{name}");

            quote!(#ty::from_bridge(#value)?)
        }
    }
}

/// Return one value converted into bridge DTO space.
fn into_bridge_value(ty: &Type, value: TokenStream, projection: &Projection) -> TokenStream {
    match ty {
        Type::String => quote!(read_string(#value)?),
        Type::Bool | Type::U8 | Type::U32 | Type::Usize => value,
        Type::Vec(inner) if **inner == Type::U8 => quote!(#value.into_vec()?),
        Type::Vec(_) => quote!(#value.to_bridge()?),
        Type::Option(inner) if **inner == Type::String => quote!(#value.to_bridge()?),
        Type::Option(_) => quote!(#value.to_bridge()?),
        Type::Named(name) if projection.is_handle(name) => {
            quote!({
                let value = unsafe { #value.as_ref() }.ok_or("artifact key is null")?;
                value.value.clone()
            })
        }
        Type::Named(_) => quote!(#value.to_bridge()?),
    }
}

/// Return one borrowed value converted into bridge DTO space.
fn to_bridge_value(ty: &Type, value: TokenStream, projection: &Projection) -> TokenStream {
    match ty {
        Type::String => quote!(read_string(#value)?),
        Type::Bool | Type::U8 | Type::U32 | Type::Usize => value,
        Type::Vec(inner) if **inner == Type::U8 => {
            quote!(read_bytes(#value.ptr.cast_const(), #value.len)?)
        }
        Type::Vec(_) | Type::Option(_) => quote!(#value.to_bridge()?),
        Type::Named(name) if projection.is_handle(name) => {
            quote!({
                let value = unsafe { #value.as_ref() }.ok_or("artifact key is null")?;
                value.value.clone()
            })
        }
        Type::Named(_) => quote!(#value.to_bridge()?),
    }
}

/// Return code that destroys one value.
fn destroy_value(ty: &Type, value: TokenStream, projection: &Projection) -> TokenStream {
    match ty {
        Type::String => quote! {
            destroy_string(#value);
            #value = ptr::null_mut();
        },
        Type::Vec(inner) if **inner == Type::U8 => quote!(#value.destroy();),
        Type::Named(name) if projection.is_handle(name) => {
            let destroy = format_ident!("destack_{}_destroy", to_snake(name));

            quote! {
                if !#value.is_null() {
                    unsafe {
                        #destroy(#value);
                    }
                    #value = ptr::null_mut();
                }
            }
        }
        Type::Vec(_) | Type::Option(_) | Type::Named(_) => quote!(#value.destroy();),
        Type::Bool | Type::U8 | Type::U32 | Type::Usize => quote!(),
    }
}

/// Return one empty C ABI value.
fn empty_value(ty: &Type, projection: &Projection) -> TokenStream {
    match ty {
        Type::String => quote!(ptr::null_mut()),
        Type::Bool => quote!(false),
        Type::U8 | Type::U32 | Type::Usize => quote!(0),
        Type::Vec(inner) if **inner == Type::U8 => quote!(DestackByteArray {
            ptr: ptr::null_mut(),
            len: 0,
        }),
        Type::Vec(inner) => {
            let ty = format_ident!("Destack{}Array", named_type(inner));

            quote!(#ty {
                ptr: ptr::null_mut(),
                len: 0,
            })
        }
        Type::Option(inner) if **inner == Type::String => quote!(DestackOptionalString::empty()),
        Type::Option(inner) => {
            let ty = format_ident!("DestackOptional{}", named_type(inner));
            let value = empty_named_value(named_type(inner));

            quote!(#ty {
                is_some: false,
                value: #value,
            })
        }
        Type::Named(name) if projection.is_handle(name) => quote!(ptr::null_mut()),
        Type::Named(name) => empty_named_value(name),
    }
}

/// Return one empty named C ABI value.
fn empty_named_value(name: &str) -> TokenStream {
    let ty = format_ident!("Destack{name}");

    quote!(#ty::empty())
}

/// Return one C enum variant name.
fn c_enum_variant(ty: &str, variant: &str) -> String {
    let ty = to_snake(ty).to_ascii_uppercase();
    let variant = to_snake(variant).to_ascii_uppercase();

    format!("DESTACK_{ty}_{variant}")
}

/// Return one generated C header guard.
fn header_guard(path: &ModulePath) -> String {
    let path = path
        .segments()
        .iter()
        .map(|segment| to_snake(segment).to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join("_");

    format!("DESTACK_{path}_GENERATED_H")
}

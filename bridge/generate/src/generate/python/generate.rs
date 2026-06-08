use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::generate::core::{
    Field, Item, ModulePath, Payload, Schema, SchemaModule, Shape, Type, Variant, write_rust,
    write_text,
};

/// Generate Python bridge bindings.
pub(in crate::generate) fn generate(root: &Path, schema: &Schema) -> Result<()> {
    for module in &schema.modules {
        let tokens = render_module(schema, &module.names);
        let path = generated_path(module);

        write_rust(root, &path, tokens)?;

        let facade = render_module_facade(module);
        write_text(root, &module_facade_path(module), facade)?;

        let stub = render_module_stub(schema, module);
        write_text(root, &module_stub_path(module), stub)?;
    }

    for package in python_packages(schema).values() {
        let facade = render_package_facade(package);
        write_text(root, &package_facade_path(package), facade)?;

        let stub = render_package_stub(package);
        write_text(root, &package_stub_path(package), stub)?;
    }

    let facade = render_root_facade(schema);
    write_text(root, "bridge/python/src/destack/__init__.py", facade)?;

    let stub = render_root_stub(schema);
    write_text(root, "bridge/python/src/destack/__init__.pyi", stub)?;

    Ok(())
}

/// Return one generated Python module path.
fn generated_path(module: &SchemaModule) -> String {
    format!(
        "bridge/python/rust/src/{}/generated.rs",
        module.path.slash_path()
    )
}

/// Return one generated Python facade path.
fn module_facade_path(module: &SchemaModule) -> String {
    format!("bridge/python/src/destack/{}.py", module.path.slash_path())
}

/// Return one generated Python facade stub path.
fn module_stub_path(module: &SchemaModule) -> String {
    format!("bridge/python/src/destack/{}.pyi", module.path.slash_path())
}

/// Return one generated Python package facade path.
fn package_facade_path(package: &PythonPackage) -> String {
    let path = package.segments.join("/");

    format!("bridge/python/src/destack/{path}/__init__.py")
}

/// Return one generated Python package stub path.
fn package_stub_path(package: &PythonPackage) -> String {
    let path = package.segments.join("/");

    format!("bridge/python/src/destack/{path}/__init__.pyi")
}

/// Return a relative import path to the native module.
fn native_import_path(segments: &[String]) -> String {
    let package_depth = segments.len().saturating_sub(1);
    let dots = ".".repeat(package_depth + 1);

    format!("{dots}_native")
}

/// Return one absolute Python module path.
fn absolute_module_path(path: &ModulePath) -> String {
    format!("destack.{}", path.segments().join("."))
}

/// One generated Python package.
struct PythonPackage {
    /// Package path segments below `destack`.
    segments: Vec<String>,
    /// Re-exported names in stable order.
    names: Vec<String>,
    /// Re-export source module keyed by exported name.
    modules: BTreeMap<String, String>,
}

/// One Python stub method declaration.
struct StubMethod {
    /// Method decorators.
    decorators: Vec<String>,
    /// Method name.
    name: String,
    /// Method arguments.
    arguments: Vec<String>,
    /// Return type.
    output: String,
}

/// One generated text document.
struct Text {
    /// Generated source.
    source: String,
}

/// Return generated Python packages.
fn python_packages(schema: &Schema) -> BTreeMap<Vec<String>, PythonPackage> {
    let mut packages = BTreeMap::<Vec<String>, PythonPackage>::new();

    for module in &schema.modules {
        let segments = module.path.segments();
        let module_name = segments.last().cloned().unwrap_or_default();

        for depth in 1..segments.len() {
            let package_segments = segments[..depth].to_vec();
            let package =
                packages
                    .entry(package_segments.clone())
                    .or_insert_with(|| PythonPackage {
                        segments: package_segments,
                        names: Vec::new(),
                        modules: BTreeMap::new(),
                    });

            let suffix = segments[depth..].join(".");
            for name in &module.names {
                if package.modules.contains_key(name) {
                    continue;
                }

                package.names.push(name.clone());
                package.modules.insert(name.clone(), suffix.clone());
            }
        }

        let package_segments = segments[..segments.len() - 1].to_vec();
        if package_segments.is_empty() {
            continue;
        }

        let package = packages
            .entry(package_segments.clone())
            .or_insert_with(|| PythonPackage {
                segments: package_segments,
                names: Vec::new(),
                modules: BTreeMap::new(),
            });

        for name in &module.names {
            package
                .modules
                .entry(name.clone())
                .or_insert(module_name.clone());
        }
    }

    packages
}

impl StubMethod {
    /// Create one method declaration.
    fn new(name: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            decorators: Vec::new(),
            name: name.into(),
            arguments: Vec::new(),
            output: output.into(),
        }
    }

    /// Add one decorator.
    fn with_decorator(mut self, decorator: impl Into<String>) -> Self {
        self.decorators.push(decorator.into());

        self
    }

    /// Add one argument.
    fn with_argument(mut self, argument: impl Into<String>) -> Self {
        self.arguments.push(argument.into());

        self
    }

    /// Render this method declaration.
    fn render(&self, text: &mut Text) {
        for decorator in &self.decorators {
            text.line(format!("    {decorator}"));
        }

        let mut arguments = Vec::new();
        if self
            .decorators
            .iter()
            .any(|decorator| decorator == "@staticmethod")
        {
            arguments.extend(self.arguments.iter().map(String::as_str));
        } else {
            arguments.push("self");
            arguments.extend(self.arguments.iter().map(String::as_str));
        }
        let arguments = arguments.join(", ");

        text.line(format!(
            "    def {}({arguments}) -> {}: ...",
            self.name, self.output
        ));
        text.blank();
    }
}

impl Text {
    /// Create one empty text document.
    fn new() -> Self {
        Self {
            source: String::new(),
        }
    }

    /// Create one generated text document.
    fn generated() -> Self {
        let mut text = Self::new();
        text.line("# generated bridge target, do not edit");
        text.blank();

        text
    }

    /// Write raw generated source.
    fn raw(&mut self, source: impl AsRef<str>) {
        self.source.push_str(source.as_ref());
    }

    /// Write one line.
    fn line(&mut self, line: impl AsRef<str>) {
        self.source.push_str(line.as_ref());
        self.source.push('\n');
    }

    /// Write one blank line.
    fn blank(&mut self) {
        self.source.push('\n');
    }

    /// Write one Python stub documentation line.
    fn doc(&mut self, doc: &str, indent: &str) {
        if !doc.is_empty() {
            self.line(format!("{indent}\"\"\"{doc}\"\"\""));
        }
    }

    /// Return the generated source.
    fn finish(self) -> String {
        self.source
    }
}

/// Render one generated Python module.
fn render_module(schema: &Schema, names: &[String]) -> TokenStream {
    let mut items = Vec::new();
    let mut classes = Vec::new();
    let imports = render_imports(schema, names);

    for name in names {
        let item = schema.item(name);
        let class = item.ident();
        classes.push(quote!(module.add_class::<#class>()?;));

        match &item.shape {
            Shape::Struct(fields) => {
                items.push(render_struct(schema, item, fields));
            }
            Shape::Enum(variants) if schema.is_unit_enum(&item.name) => {
                items.push(render_unit_enum(schema, item, variants));
            }
            Shape::Enum(variants) => {
                items.push(render_payload_enum(schema, item, variants));
            }
        }
    }

    quote! {
        use destack_bridge_language as bridge;
        use pyo3::prelude::*;
        #imports

        #(#items)*

        /// Register generated Python bridge classes.
        pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
            #(#classes)*

            Ok(())
        }
    }
}

/// Render one generated Python facade module.
fn render_module_facade(module: &SchemaModule) -> String {
    let mut text = Text::generated();
    let native = native_import_path(module.path.segments());
    text.line(format!("from {native} import ("));

    for name in &module.names {
        text.line(format!("    {name},"));
    }

    text.line(")");
    text.blank();
    render_all(&mut text, &module.names);

    text.finish()
}

/// Render one generated Python facade stub module.
fn render_module_stub(schema: &Schema, module: &SchemaModule) -> String {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    text.line("from collections.abc import Sequence");
    text.blank();
    text.raw(render_module_stub_imports(schema, module));

    for name in &module.names {
        let item = schema.item(name);
        text.raw(render_stub_item(schema, item));
    }

    text.finish()
}

/// Render Python stub imports referenced by one generated module.
fn render_module_stub_imports(schema: &Schema, module: &SchemaModule) -> String {
    let mut imports = BTreeMap::<String, Vec<String>>::new();

    for item in schema.referenced_types(&module.names) {
        let path = schema.module_path(&item.name);
        let path = absolute_module_path(path);
        imports.entry(path).or_default().push(item.name.clone());
    }

    if imports.is_empty() {
        return String::new();
    }

    let mut text = Text::new();
    for (path, names) in imports {
        text.line(format!("from {path} import ("));

        for name in names {
            text.line(format!("    {name},"));
        }

        text.line(")");
        text.blank();
    }

    text.finish()
}

/// Render the root Python facade.
fn render_root_facade(schema: &Schema) -> String {
    let mut text = Text::generated();
    text.line("from ._native import VERSION, Session, version");
    render_reexports(&mut text, schema, "");
    text.blank();
    render_all(&mut text, &root_names(schema));

    text.finish()
}

/// Render the root Python type stub.
fn render_root_stub(schema: &Schema) -> String {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    text.line("from collections.abc import Sequence");
    text.blank();
    render_reexports(&mut text, schema, "");
    text.line("VERSION: str");
    text.blank();
    text.line("def version() -> str: ...");
    text.blank();
    text.raw(render_session_stub());

    text.finish()
}

/// Render one generated Python package facade.
fn render_package_facade(package: &PythonPackage) -> String {
    let mut text = Text::generated();
    render_package_reexports(&mut text, package);
    text.blank();
    render_all(&mut text, &package.names);

    text.finish()
}

/// Render one generated Python package stub.
fn render_package_stub(package: &PythonPackage) -> String {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    render_package_reexports(&mut text, package);

    text.finish()
}

/// Render root re-exports for all generated DTO modules.
fn render_reexports(text: &mut Text, schema: &Schema, indent: &str) {
    for module in &schema.modules {
        let path = module.path.slash_path().replace('/', ".");
        text.line(format!("{indent}from .{path} import ("));

        for name in &module.names {
            text.line(format!("{indent}    {name},"));
        }

        text.line(format!("{indent})"));
    }
}

/// Render package re-exports for generated DTO modules below one package.
fn render_package_reexports(text: &mut Text, package: &PythonPackage) {
    let mut modules = BTreeMap::<&str, Vec<&str>>::new();

    for name in &package.names {
        let module = package
            .modules
            .get(name)
            .unwrap_or_else(|| panic!("python package export {name} has no module"));
        modules.entry(module).or_default().push(name);
    }

    for (module, names) in modules {
        text.line(format!("from .{module} import ("));

        for name in names {
            text.line(format!("    {name},"));
        }

        text.line(")");
    }
}

/// Render one Python `__all__` block.
fn render_all(text: &mut Text, names: &[String]) {
    text.line("__all__ = [");
    for name in names {
        text.line(format!("    {name:?},"));
    }
    text.line("]");
}

/// Return names exported by the root package.
fn root_names(schema: &Schema) -> Vec<String> {
    let mut names = Vec::new();
    names.push("Session".to_string());

    for module in &schema.modules {
        names.extend(module.names.iter().cloned());
    }

    names.push("VERSION".to_string());
    names.push("version".to_string());

    names
}

/// Render the handwritten native session API.
fn render_session_stub() -> String {
    let mut text = Text::new();
    text.line("class Session:");
    text.line("    \"\"\"Python language session.\"\"\"");
    text.blank();

    for method in session_stub_methods() {
        method.render(&mut text);
    }

    text.finish()
}

/// Return native session stub methods.
fn session_stub_methods() -> Vec<StubMethod> {
    vec![
        StubMethod::new("open_path", "Session")
            .with_decorator("@staticmethod")
            .with_argument("path: str"),
        StubMethod::new("open_source", "Session")
            .with_decorator("@staticmethod")
            .with_argument("root: str")
            .with_argument("source: SourceSnapshot"),
        StubMethod::new("revision", "Revision"),
        StubMethod::new("files", "list[SessionFile]"),
        StubMethod::new("update", "SourceUpdateResult").with_argument("update: SourceUpdate"),
        StubMethod::new("reload", "list[FileUpdate]"),
        StubMethod::new("load_module", "Module").with_argument("path: str"),
        StubMethod::new("provide", "None")
            .with_argument("revision: Revision")
            .with_argument("keys: Sequence[ArtifactKey]"),
        StubMethod::new("require", "ArtifactVersion")
            .with_argument("revision: Revision")
            .with_argument("key: ArtifactKey"),
        StubMethod::new("diagnostics", "list[Diagnostic]")
            .with_argument("revision: Revision")
            .with_argument("key: ArtifactKey | None = None"),
        StubMethod::new("sidecars", "list[ArtifactSidecar]")
            .with_argument("revision: Revision")
            .with_argument("key: ArtifactKey"),
    ]
}

/// Render one generated Python stub item.
fn render_stub_item(schema: &Schema, item: &Item) -> String {
    match &item.shape {
        Shape::Struct(fields) => render_struct_stub(schema, item, fields),
        Shape::Enum(variants) if schema.is_unit_enum(&item.name) => {
            render_unit_enum_stub(item, variants)
        }
        Shape::Enum(variants) => render_payload_enum_stub(schema, item, variants),
    }
}

/// Render one Python struct stub.
fn render_struct_stub(schema: &Schema, item: &Item, fields: &[Field]) -> String {
    let mut text = Text::new();
    render_stub_class_header(&mut text, item);

    let arguments = fields
        .iter()
        .map(|field| {
            format!(
                "{}: {}",
                python_parameter_name(&field.name),
                render_input_stub_type(schema, &field.ty)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let separator = (!arguments.is_empty()).then_some(", ").unwrap_or("");
    text.line(format!(
        "    def __init__(self{separator}{arguments}) -> None: ..."
    ));
    text.blank();

    for field in fields {
        text.doc(field.doc(), "    ");
        text.line("    @property");
        text.line(format!(
            "    def {}(self) -> {}: ...",
            field.name,
            render_output_stub_type(schema, &field.ty)
        ));
        text.blank();
    }

    if item.name == "SourceFile" {
        StubMethod::new("text", "SourceFile")
            .with_decorator("@staticmethod")
            .with_argument("path: str")
            .with_argument("text: str")
            .render(&mut text);
        StubMethod::new("bytes", "SourceFile")
            .with_decorator("@staticmethod")
            .with_argument("path: str")
            .with_argument("bytes: bytes | bytearray | Sequence[int]")
            .render(&mut text);
    }

    text.finish()
}

/// Render one Python unit enum stub.
fn render_unit_enum_stub(item: &Item, variants: &[Variant]) -> String {
    let mut text = Text::new();
    render_stub_class_header(&mut text, item);

    for variant in variants {
        text.doc(variant.doc(), "    ");
        StubMethod::new(variant.payload_field_name(), item.name.clone())
            .with_decorator("@staticmethod")
            .render(&mut text);
    }

    StubMethod::new("label", "str")
        .with_decorator("@property")
        .render(&mut text);

    text.finish()
}

/// Render one Python payload enum stub.
fn render_payload_enum_stub(schema: &Schema, item: &Item, variants: &[Variant]) -> String {
    let mut text = Text::new();
    render_stub_class_header(&mut text, item);

    for variant in variants {
        text.doc(variant.doc(), "    ");

        match &variant.payload {
            Payload::Unit => {
                StubMethod::new(variant.payload_field_name(), item.name.clone())
                    .with_decorator("@staticmethod")
                    .render(&mut text);
            }
            Payload::Tuple(ty) => {
                let name = variant.payload_field_name();
                let ty = render_input_stub_type(schema, ty);
                StubMethod::new(name.clone(), item.name.clone())
                    .with_decorator("@staticmethod")
                    .with_argument(format!("{name}: {ty}"))
                    .render(&mut text);
            }
            Payload::Struct(fields) => {
                let mut method = StubMethod::new(variant.payload_field_name(), item.name.clone())
                    .with_decorator("@staticmethod");

                for field in fields {
                    let name = python_parameter_name(&field.name);
                    let ty = render_input_stub_type(schema, &field.ty);
                    method = method.with_argument(format!("{name}: {ty}"));
                }

                method.render(&mut text);
            }
        }
    }

    StubMethod::new("kind", "str")
        .with_decorator("@property")
        .render(&mut text);

    text.finish()
}

/// Append one Python stub class header.
fn render_stub_class_header(text: &mut Text, item: &Item) {
    text.line(format!("class {}:", item.name));
    text.doc(item.doc(), "    ");
    text.blank();
}

/// Render one Python stub input type.
fn render_input_stub_type(schema: &Schema, ty: &Type) -> String {
    match ty {
        Type::String => "str".to_string(),
        Type::Bool => "bool".to_string(),
        Type::U8 | Type::U32 | Type::Usize => "int".to_string(),
        Type::Vec(ty) if matches!(ty.as_ref(), Type::U8) => {
            "bytes | bytearray | Sequence[int]".to_string()
        }
        Type::Vec(ty) => format!("Sequence[{}]", render_input_stub_type(schema, ty)),
        Type::Option(ty) => format!("{} | None", render_input_stub_type(schema, ty)),
        Type::Named(name) if schema.is_unit_enum(name) => name.clone(),
        Type::Named(name) => name.clone(),
    }
}

/// Render one Python stub output type.
fn render_output_stub_type(schema: &Schema, ty: &Type) -> String {
    match ty {
        Type::String => "str".to_string(),
        Type::Bool => "bool".to_string(),
        Type::U8 | Type::U32 | Type::Usize => "int".to_string(),
        Type::Vec(ty) => format!("list[{}]", render_output_stub_type(schema, ty)),
        Type::Option(ty) => format!("{} | None", render_output_stub_type(schema, ty)),
        Type::Named(name) if schema.is_unit_enum(name) => name.clone(),
        Type::Named(name) => name.clone(),
    }
}

/// Return one valid Python parameter name.
fn python_parameter_name(name: &str) -> String {
    match name {
        "False" | "None" | "True" | "and" | "as" | "assert" | "async" | "await" | "break"
        | "class" | "continue" | "def" | "del" | "elif" | "else" | "except" | "finally" | "for"
        | "from" | "global" | "if" | "import" | "in" | "is" | "lambda" | "nonlocal" | "not"
        | "or" | "pass" | "raise" | "return" | "try" | "while" | "with" | "yield" => {
            format!("{name}_")
        }
        _ => name.to_string(),
    }
}

/// Render imports referenced by this generated module.
fn render_imports(schema: &Schema, names: &[String]) -> TokenStream {
    let imports = schema.referenced_items(names);

    if imports.is_empty() {
        return quote!();
    }

    let imports = imports.iter().map(|item| item.ident());

    quote!(use crate::{#(#imports,)*};)
}

/// Render one Python struct class.
fn render_struct(schema: &Schema, item: &Item, fields: &[Field]) -> TokenStream {
    let name = item.ident();
    let py_name = &item.name;
    let docs = item.docs();
    let constructor = render_struct_constructor(schema, item, fields);
    let getters = fields.iter().map(|field| render_getter(schema, field));
    let helpers = render_struct_helpers(schema, item);
    let convenience = render_struct_convenience(schema, item);

    quote! {
        #docs
        #[pyclass(name = #py_name, module = "destack._native", from_py_object)]
        #[derive(Debug, Clone)]
        pub struct #name {
            pub(crate) value: bridge::#name,
        }

        #[pymethods]
        impl #name {
            #constructor
            #(#getters)*
            #convenience
        }

        #helpers
    }
}

/// Render one Python struct constructor.
fn render_struct_constructor(schema: &Schema, item: &Item, fields: &[Field]) -> TokenStream {
    let field_arguments = fields.iter().map(|field| {
        let name = field.ident();
        let ty = render_type(schema, &field.ty);

        quote!(#name: #ty,)
    });
    let field_values = fields.iter().map(|field| {
        let name = field.ident();
        if !type_needs_conversion(schema, &field.ty) {
            return quote!(#name,);
        }

        let value = render_into_bridge_value(schema, quote!(#name), &field.ty);

        quote!(#name: #value,)
    });
    let name = item.ident();

    quote! {
        /// Create one value.
        #[new]
        pub fn new(#(#field_arguments)*) -> Self {
            Self {
                value: bridge::#name {
                    #(#field_values)*
                },
            }
        }
    }
}

/// Render custom convenience constructors for one Python struct.
fn render_struct_convenience(schema: &Schema, item: &Item) -> TokenStream {
    if item.name != "SourceFile" {
        return quote!();
    }

    let content = schema.item("SourceFileContent").ident();

    quote! {
        /// Create one text source file.
        #[staticmethod]
        pub fn text(path: String, text: String) -> Self {
            Self::new(path, #content::text(text))
        }

        /// Create one binary source file.
        #[staticmethod]
        pub fn bytes(path: String, bytes: Vec<u8>) -> Self {
            Self::new(path, #content::bytes(bytes))
        }
    }
}

/// Render one Python getter.
fn render_getter(schema: &Schema, field: &Field) -> TokenStream {
    let docs = field.docs();
    let name = field.ident();
    let ty = render_type(schema, &field.ty);
    let value = render_from_bridge_value(schema, quote!(self.value.#name.clone()), &field.ty);

    quote! {
        #docs
        #[getter]
        pub fn #name(&self) -> #ty {
            #value
        }
    }
}

/// Render one struct conversion helper block.
fn render_struct_helpers(schema: &Schema, item: &Item) -> TokenStream {
    let name = item.ident();
    let into_bridge = generates_to_bridge(schema, item).then(|| {
        quote! {
            /// Convert this Python value into one bridge value.
            pub(crate) fn into_bridge(self) -> bridge::#name {
                self.value
            }
        }
    });
    let from_bridge = generates_from_bridge(schema, item).then(|| {
        quote! {
            /// Convert one bridge value into one Python value.
            pub(crate) fn from_bridge(value: bridge::#name) -> Self {
                Self { value }
            }
        }
    });

    quote! {
        #[allow(dead_code)]
        impl #name {
            #into_bridge
            #from_bridge
        }
    }
}

/// Render one unit enum as a Python class.
fn render_unit_enum(schema: &Schema, item: &Item, variants: &[Variant]) -> TokenStream {
    let name = item.ident();
    let py_name = &item.name;
    let docs = item.docs();
    let constructors = variants.iter().map(|variant| {
        let method = variant.payload_method_ident();
        let variant_name = variant.ident();
        let docs = variant.docs();

        quote! {
            #docs
            #[staticmethod]
            pub fn #method() -> Self {
                Self {
                    value: bridge::#name::#variant_name,
                }
            }
        }
    });
    let labels = variants.iter().map(|variant| {
        let variant_name = variant.ident();
        let label = variant.label();

        quote!(bridge::#name::#variant_name => #label,)
    });
    let into_bridge = generates_to_bridge(schema, item).then(|| {
        quote! {
            /// Convert this Python value into one bridge value.
            pub(crate) fn into_bridge(self) -> bridge::#name {
                self.value
            }
        }
    });
    let from_bridge = generates_from_bridge(schema, item).then(|| {
        quote! {
            /// Convert one bridge value into one Python value.
            pub(crate) fn from_bridge(value: bridge::#name) -> Self {
                Self { value }
            }
        }
    });

    quote! {
        #docs
        #[pyclass(name = #py_name, module = "destack._native", from_py_object)]
        #[derive(Debug, Clone)]
        pub struct #name {
            pub(crate) value: bridge::#name,
        }

        #[pymethods]
        impl #name {
            #(#constructors)*

            /// Return this enum label.
            #[getter]
            pub fn label(&self) -> &'static str {
                match self.value {
                    #(#labels)*
                }
            }
        }

        impl #name {
            #into_bridge
            #from_bridge
        }
    }
}

/// Render one payload enum as a Python class.
fn render_payload_enum(schema: &Schema, item: &Item, variants: &[Variant]) -> TokenStream {
    let name = item.ident();
    let py_name = &item.name;
    let docs = item.docs();
    let constructors = variants
        .iter()
        .map(|variant| render_payload_constructor(schema, item, variant));
    let labels = variants.iter().map(|variant| {
        let variant_name = variant.ident();
        let label = variant.label();

        match &variant.payload {
            Payload::Unit => quote!(bridge::#name::#variant_name => #label,),
            Payload::Tuple(_) => quote!(bridge::#name::#variant_name(_) => #label,),
            Payload::Struct(_) => quote!(bridge::#name::#variant_name { .. } => #label,),
        }
    });
    let into_bridge = generates_to_bridge(schema, item).then(|| {
        quote! {
            /// Convert this Python value into one bridge value.
            pub(crate) fn into_bridge(self) -> bridge::#name {
                self.value
            }
        }
    });
    let from_bridge = generates_from_bridge(schema, item).then(|| {
        quote! {
            /// Convert one bridge value into one Python value.
            pub(crate) fn from_bridge(value: bridge::#name) -> Self {
                Self { value }
            }
        }
    });

    quote! {
        #docs
        #[pyclass(name = #py_name, module = "destack._native", from_py_object)]
        #[derive(Debug, Clone)]
        pub struct #name {
            pub(crate) value: bridge::#name,
        }

        #[pymethods]
        impl #name {
            #(#constructors)*

            /// Return this enum variant label.
            #[getter]
            pub fn kind(&self) -> &'static str {
                match &self.value {
                    #(#labels)*
                }
            }
        }

        impl #name {
            #into_bridge
            #from_bridge
        }
    }
}

/// Render one payload enum constructor.
fn render_payload_constructor(schema: &Schema, item: &Item, variant: &Variant) -> TokenStream {
    let name = item.ident();
    let method = variant.payload_method_ident();
    let variant_name = variant.ident();
    let docs = variant.docs();

    match &variant.payload {
        Payload::Unit => quote! {
            #docs
            #[staticmethod]
            pub fn #method() -> Self {
                Self {
                    value: bridge::#name::#variant_name,
                }
            }
        },
        Payload::Tuple(ty) => {
            let argument = variant.payload_field_ident();
            let argument_ty = render_type(schema, ty);
            let value = render_into_bridge_value(schema, quote!(#argument), ty);

            quote! {
                #docs
                #[staticmethod]
                pub fn #method(#argument: #argument_ty) -> Self {
                    Self {
                        value: bridge::#name::#variant_name(#value),
                    }
                }
            }
        }
        Payload::Struct(fields) => {
            let arguments = fields.iter().map(|field| {
                let name = field.ident();
                let ty = render_type(schema, &field.ty);

                quote!(#name: #ty,)
            });
            let values = fields.iter().map(|field| {
                let name = field.ident();
                if !type_needs_conversion(schema, &field.ty) {
                    return quote!(#name,);
                }

                let value = render_into_bridge_value(schema, quote!(#name), &field.ty);

                quote!(#name: #value,)
            });

            quote! {
                #docs
                #[staticmethod]
                pub fn #method(#(#arguments)*) -> Self {
                    Self {
                        value: bridge::#name::#variant_name {
                            #(#values)*
                        },
                    }
                }
            }
        }
    }
}

/// Render one Python type.
fn render_type(schema: &Schema, ty: &Type) -> TokenStream {
    match ty {
        Type::String => quote!(String),
        Type::Bool => quote!(bool),
        Type::U8 => quote!(u8),
        Type::U32 => quote!(u32),
        Type::Usize => quote!(usize),
        Type::Vec(ty) => {
            let ty = render_type(schema, ty);

            quote!(Vec<#ty>)
        }
        Type::Option(ty) => {
            let ty = render_type(schema, ty);

            quote!(Option<#ty>)
        }
        Type::Named(name) => {
            let name = format_ident!("{name}");

            quote!(#name)
        }
    }
}

/// Render one bridge input conversion.
fn render_into_bridge_value(schema: &Schema, value: TokenStream, ty: &Type) -> TokenStream {
    match ty {
        Type::Vec(ty) => {
            let source = value;
            let item = render_into_bridge_value(schema, quote!(item), ty);

            if type_needs_conversion(schema, ty) {
                quote!(#source.into_iter().map(|item| #item).collect())
            } else {
                quote!(#source)
            }
        }
        Type::Option(ty) => {
            let source = value;
            let item = render_into_bridge_value(schema, quote!(item), ty);

            if type_needs_conversion(schema, ty) {
                quote!(#source.map(|item| #item))
            } else {
                quote!(#source)
            }
        }
        Type::Named(name) if schema.items.contains_key(name) => {
            quote!(#value.into_bridge())
        }
        _ => value,
    }
}

/// Render one bridge output conversion.
fn render_from_bridge_value(schema: &Schema, value: TokenStream, ty: &Type) -> TokenStream {
    match ty {
        Type::Vec(ty) => {
            let source = value;
            let item = render_from_bridge_value(schema, quote!(item), ty);

            if type_needs_conversion(schema, ty) {
                quote!(#source.into_iter().map(|item| #item).collect())
            } else {
                quote!(#source)
            }
        }
        Type::Option(ty) => {
            let source = value;
            let item = render_from_bridge_value(schema, quote!(item), ty);

            if type_needs_conversion(schema, ty) {
                quote!(#source.map(|item| #item))
            } else {
                quote!(#source)
            }
        }
        Type::Named(name) if schema.items.contains_key(name) => {
            let name = format_ident!("{name}");

            quote!(#name::from_bridge(#value))
        }
        _ => value,
    }
}

/// Return whether this item needs an input bridge conversion.
fn generates_to_bridge(schema: &Schema, item: &Item) -> bool {
    item.generates_into_bridge()
        || schema
            .items
            .values()
            .any(|other| other.references(&item.name))
}

/// Return whether this item needs an output bridge conversion.
fn generates_from_bridge(schema: &Schema, item: &Item) -> bool {
    item.generates_from_bridge()
        || schema
            .items
            .values()
            .any(|other| struct_fields_reference(other, &item.name))
}

/// Return whether this Python type needs bridge conversion.
fn type_needs_conversion(schema: &Schema, ty: &Type) -> bool {
    match ty {
        Type::Vec(ty) | Type::Option(ty) => type_needs_conversion(schema, ty),
        Type::Named(name) => schema.items.contains_key(name),
        _ => false,
    }
}

/// Return whether one item has a struct field referencing a bridge DTO.
fn struct_fields_reference(item: &Item, name: &str) -> bool {
    let Shape::Struct(fields) = &item.shape else {
        return false;
    };

    fields.iter().any(|field| type_references(&field.ty, name))
}

/// Return whether one type references a bridge DTO.
fn type_references(ty: &Type, name: &str) -> bool {
    match ty {
        Type::Vec(ty) | Type::Option(ty) => type_references(ty, name),
        Type::Named(reference) => reference == name,
        _ => false,
    }
}

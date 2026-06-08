use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use proc_macro2::TokenStream;

use super::spelling::{ident, lower_camel, render_docs, to_snake};

/// Bridge schema parsed from `bridge/language`.
pub(crate) struct Schema {
    /// Items keyed by Rust item name.
    pub(crate) items: BTreeMap<String, Item>,
    /// Type names in bridge module order.
    pub(crate) modules: BTreeMap<Module, Vec<String>>,
}

/// One bridge schema module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Module {
    /// `repository/revision`.
    RepositoryRevision,
    /// `session/file`.
    SessionFile,
    /// `session/module`.
    SessionModule,
    /// `session/source/file`.
    SourceFile,
    /// `session/source/snapshot`.
    SourceSnapshot,
    /// `session/source/update`.
    SourceUpdate,
}

impl Module {
    /// Return this module generated NAPI path.
    pub(crate) fn napi_file(self) -> &'static str {
        match self {
            Self::RepositoryRevision => "bridge/napi/src/repository/revision/generated.rs",
            Self::SessionFile => "bridge/napi/src/session/file/generated.rs",
            Self::SessionModule => "bridge/napi/src/session/module/generated.rs",
            Self::SourceFile => "bridge/napi/src/session/source/file/generated.rs",
            Self::SourceSnapshot => "bridge/napi/src/session/source/snapshot/generated.rs",
            Self::SourceUpdate => "bridge/napi/src/session/source/update/generated.rs",
        }
    }

    /// Return this module generated WASM path.
    pub(crate) fn wasm_file(self) -> &'static str {
        match self {
            Self::RepositoryRevision => "bridge/wasm/src/repository/revision/generated.rs",
            Self::SessionFile => "bridge/wasm/src/session/file/generated.rs",
            Self::SessionModule => "bridge/wasm/src/session/module/generated.rs",
            Self::SourceFile => "bridge/wasm/src/session/source/file/generated.rs",
            Self::SourceSnapshot => "bridge/wasm/src/session/source/snapshot/generated.rs",
            Self::SourceUpdate => "bridge/wasm/src/session/source/update/generated.rs",
        }
    }

    /// Return this module generated TypeScript path.
    pub(crate) fn typescript_file(self) -> &'static str {
        match self {
            Self::RepositoryRevision => "bridge/typescript/src/repository/revision.generated.ts",
            Self::SessionFile => "bridge/typescript/src/session/file.generated.ts",
            Self::SessionModule => "bridge/typescript/src/session/module.generated.ts",
            Self::SourceFile => "bridge/typescript/src/session/source/file.generated.ts",
            Self::SourceSnapshot => "bridge/typescript/src/session/source/snapshot.generated.ts",
            Self::SourceUpdate => "bridge/typescript/src/session/source/update.generated.ts",
        }
    }

    /// Return the TypeScript import path from this module to another module.
    pub(crate) fn typescript_import_path(self, target: Self) -> String {
        let source = self.typescript_segments();
        let target = target.typescript_segments();
        let source_directory = &source[..source.len() - 1];
        let mut shared = 0;

        while shared < source_directory.len()
            && shared < target.len()
            && source_directory[shared] == target[shared]
        {
            shared += 1;
        }

        let mut segments = Vec::new();
        for _ in shared..source_directory.len() {
            segments.push("..");
        }
        segments.extend(target[shared..].iter().copied());

        let path = segments.join("/");
        if path.starts_with('.') {
            format!("{path}.js")
        } else {
            format!("./{path}.js")
        }
    }

    /// Return generated TypeScript path segments.
    pub(crate) fn typescript_segments(self) -> &'static [&'static str] {
        match self {
            Self::RepositoryRevision => &["repository", "revision.generated"],
            Self::SessionFile => &["session", "file.generated"],
            Self::SessionModule => &["session", "module.generated"],
            Self::SourceFile => &["session", "source", "file.generated"],
            Self::SourceSnapshot => &["session", "source", "snapshot.generated"],
            Self::SourceUpdate => &["session", "source", "update.generated"],
        }
    }
}

/// One bridge DTO type.
pub(crate) struct Item {
    /// Rust type name.
    pub(crate) name: String,
    /// Documentation lines.
    pub(crate) docs: Vec<String>,
    /// Type shape.
    pub(crate) shape: Shape,
}

/// One bridge DTO shape.
pub(crate) enum Shape {
    /// A public struct.
    Struct(Vec<Field>),
    /// A public enum.
    Enum(Vec<Variant>),
}

/// One bridge struct or variant field.
#[derive(Clone)]
pub(crate) struct Field {
    /// Rust field name.
    pub(crate) name: String,
    /// Documentation lines.
    pub(crate) docs: Vec<String>,
    /// Field type.
    pub(crate) ty: Type,
}

/// One bridge enum variant.
pub(crate) struct Variant {
    /// Rust variant name.
    pub(crate) name: String,
    /// Documentation lines.
    pub(crate) docs: Vec<String>,
    /// Variant payload.
    pub(crate) payload: Payload,
}

/// One bridge enum variant payload.
pub(crate) enum Payload {
    /// No payload.
    Unit,
    /// One unnamed payload.
    Tuple(Type),
    /// Named payload fields.
    Struct(Vec<Field>),
}

/// One supported bridge type reference.
#[derive(Clone)]
pub(crate) enum Type {
    /// `String`.
    String,
    /// `bool`.
    Bool,
    /// `u8`.
    U8,
    /// `u32`.
    U32,
    /// `usize`.
    Usize,
    /// `Vec<T>`.
    Vec(Box<Type>),
    /// `Option<T>`.
    Option(Box<Type>),
    /// Another bridge DTO.
    Named(String),
}

impl Schema {
    /// Load the bridge schema from Rust source files.
    pub(crate) fn load(root: &Path) -> Result<Self> {
        let mut items = BTreeMap::new();
        let mut modules = BTreeMap::new();
        let files = [
            (
                Module::RepositoryRevision,
                "bridge/language/src/repository/revision.rs",
            ),
            (Module::SessionFile, "bridge/language/src/session/file.rs"),
            (
                Module::SessionModule,
                "bridge/language/src/session/module.rs",
            ),
            (
                Module::SourceFile,
                "bridge/language/src/session/source/file.rs",
            ),
            (
                Module::SourceSnapshot,
                "bridge/language/src/session/source/snapshot.rs",
            ),
            (
                Module::SourceUpdate,
                "bridge/language/src/session/source/update.rs",
            ),
        ];

        for (module, file) in files {
            let names = Self::load_module(root, file, &mut items)?;
            modules.insert(module, names);
        }

        Ok(Self { items, modules })
    }

    /// Load one bridge schema module.
    fn load_module(
        root: &Path,
        file: &str,
        items: &mut BTreeMap<String, Item>,
    ) -> Result<Vec<String>> {
        let path = root.join(file);
        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let parsed = syn::parse_file(&source)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        let mut names = Vec::new();

        for item in parsed.items {
            let Some(item) = Item::parse(item)? else {
                continue;
            };
            names.push(item.name.clone());
            items.insert(item.name.clone(), item);
        }

        Ok(names)
    }

    /// Validate every referenced bridge DTO.
    pub(crate) fn validate(&self) -> Result<()> {
        for item in self.items.values() {
            item.visit_refs(&mut |name| {
                if !self.items.contains_key(name) {
                    bail!(
                        "bridge type {} references missing bridge type {name}",
                        item.name
                    );
                }

                Ok(())
            })?;
        }

        Ok(())
    }

    /// Return one bridge type by name.
    pub(crate) fn item(&self, name: &str) -> &Item {
        self.items
            .get(name)
            .unwrap_or_else(|| panic!("bridge type {name} was not parsed"))
    }

    /// Return bridge items referenced by one generated module.
    pub(crate) fn referenced_items(&self, names: &[String]) -> Vec<&Item> {
        let mut items = Vec::new();

        for item in self.items.values() {
            if names.iter().any(|name| name == &item.name) || self.is_unit_enum(&item.name) {
                continue;
            }
            if names
                .iter()
                .any(|name| self.item(name).references(&item.name))
            {
                items.push(item);
            }
        }

        items
    }

    /// Return bridge types referenced by one generated module.
    pub(crate) fn referenced_types(&self, names: &[String]) -> Vec<&Item> {
        let mut items = Vec::new();

        for item in self.items.values() {
            if names.iter().any(|name| name == &item.name) {
                continue;
            }
            if names
                .iter()
                .any(|name| self.item(name).references(&item.name))
            {
                items.push(item);
            }
        }

        items
    }

    /// Return whether one bridge type is a unit enum.
    pub(crate) fn is_unit_enum(&self, name: &str) -> bool {
        let Shape::Enum(variants) = &self.item(name).shape else {
            return false;
        };

        variants
            .iter()
            .all(|variant| matches!(variant.payload, Payload::Unit))
    }

    /// Return whether one unit enum parse helper is needed.
    pub(crate) fn unit_enum_needs_parse(&self, name: &str) -> bool {
        self.items
            .values()
            .any(|item| item.generates_into_bridge() && item.references(name))
    }

    /// Return whether one unit enum label helper is needed.
    pub(crate) fn unit_enum_needs_label(&self, name: &str) -> bool {
        self.items
            .values()
            .any(|item| item.generates_from_bridge() && item.references(name))
    }
}

impl Item {
    /// Return this item as a Rust identifier.
    pub(crate) fn ident(&self) -> proc_macro2::Ident {
        ident(&self.name)
    }

    /// Return this item documentation as Rust doc attributes.
    pub(crate) fn docs(&self) -> TokenStream {
        render_docs(&self.docs)
    }

    /// Return this unit enum parse helper identifier.
    pub(crate) fn parse_ident(&self) -> proc_macro2::Ident {
        ident(&format!("parse_{}", to_snake(&self.name)))
    }

    /// Return this unit enum label helper identifier.
    pub(crate) fn label_ident(&self) -> proc_macro2::Ident {
        ident(&format!("{}_label", to_snake(&self.name)))
    }

    /// Return this item's payload content enum identifier.
    pub(crate) fn payload_content_ident(&self) -> proc_macro2::Ident {
        ident(&format!("{}Content", self.name))
    }

    /// Parse one public bridge item.
    fn parse(item: syn::Item) -> Result<Option<Self>> {
        match item {
            syn::Item::Struct(item) if is_public(&item.vis) && has_bridge_attr(&item.attrs) => {
                let name = item.ident.to_string();
                let docs = parse_docs(item.attrs);
                let fields = Field::parse_struct(item.fields)?;
                let shape = Shape::Struct(fields);

                Ok(Some(Self { name, docs, shape }))
            }
            syn::Item::Enum(item) if is_public(&item.vis) && has_bridge_attr(&item.attrs) => {
                let name = item.ident.to_string();
                let docs = parse_docs(item.attrs);
                let variants = item
                    .variants
                    .into_iter()
                    .map(|variant| {
                        let name = variant.ident.to_string();
                        let docs = parse_docs(variant.attrs);
                        let payload = Payload::parse(variant.fields)?;

                        Ok(Variant {
                            name,
                            docs,
                            payload,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let shape = Shape::Enum(variants);

                Ok(Some(Self { name, docs, shape }))
            }
            _ => Ok(None),
        }
    }

    /// Return whether this type references one bridge type.
    pub(crate) fn references(&self, name: &str) -> bool {
        let mut is_referenced = false;
        let _ = self.visit_refs(&mut |reference| {
            if reference == name {
                is_referenced = true;
            }

            Ok(())
        });

        is_referenced
    }

    /// Return whether generated targets accept this type as bridge input.
    pub(crate) fn generates_into_bridge(&self) -> bool {
        matches!(
            self.name.as_str(),
            "Revision"
                | "SourceFile"
                | "SourceFileContent"
                | "SourceSnapshot"
                | "TextRange"
                | "TextEdit"
                | "SourceEdit"
                | "SourceUpdate"
        )
    }

    /// Return whether generated targets project this type from bridge output.
    pub(crate) fn generates_from_bridge(&self) -> bool {
        matches!(
            self.name.as_str(),
            "Revision" | "SessionFile" | "Module" | "FileUpdate" | "SourceUpdateResult"
        )
    }

    /// Return the first documentation line.
    pub(crate) fn doc(&self) -> &str {
        self.docs.first().map(String::as_str).unwrap_or("")
    }

    /// Visit referenced bridge DTO names.
    fn visit_refs(&self, visit: &mut impl FnMut(&str) -> Result<()>) -> Result<()> {
        match &self.shape {
            Shape::Struct(fields) => {
                for field in fields {
                    field.ty.visit_refs(visit)?;
                }
            }
            Shape::Enum(variants) => {
                for variant in variants {
                    variant.payload.visit_refs(visit)?;
                }
            }
        }

        Ok(())
    }
}

impl Payload {
    /// Parse one enum variant payload.
    fn parse(fields: syn::Fields) -> Result<Self> {
        match fields {
            syn::Fields::Unit => Ok(Self::Unit),
            syn::Fields::Unnamed(fields) => Self::parse_tuple(fields),
            syn::Fields::Named(fields) => Field::parse_list(fields, false).map(Self::Struct),
        }
    }

    /// Parse one tuple payload.
    fn parse_tuple(fields: syn::FieldsUnnamed) -> Result<Self> {
        if fields.unnamed.len() != 1 {
            bail!("bridge tuple enum variants must have exactly one field");
        }

        let field = fields
            .unnamed
            .into_iter()
            .next()
            .context("bridge tuple enum variant is empty")?;
        let ty = Type::parse(field.ty)?;

        Ok(Self::Tuple(ty))
    }

    /// Visit referenced bridge DTO names.
    fn visit_refs(&self, visit: &mut impl FnMut(&str) -> Result<()>) -> Result<()> {
        match self {
            Self::Unit => {}
            Self::Tuple(ty) => ty.visit_refs(visit)?,
            Self::Struct(fields) => {
                for field in fields {
                    field.ty.visit_refs(visit)?;
                }
            }
        }

        Ok(())
    }
}

impl Type {
    /// Return whether this type needs NAPI bridge input conversion.
    pub(crate) fn needs_napi_into_bridge_conversion(&self, schema: &Schema) -> bool {
        match self {
            Self::Vec(ty) | Self::Option(ty) => ty.needs_napi_into_bridge_conversion(schema),
            Self::Named(name) => schema.items.contains_key(name),
            _ => false,
        }
    }

    /// Return whether this type needs WASM bridge input conversion.
    pub(crate) fn needs_wasm_into_bridge_conversion(&self, schema: &Schema) -> bool {
        match self {
            Self::Vec(ty) | Self::Option(ty) => ty.needs_wasm_into_bridge_conversion(schema),
            Self::Named(name) => schema.items.contains_key(name) && !schema.is_unit_enum(name),
            _ => false,
        }
    }

    /// Return whether this type needs bridge output conversion.
    pub(crate) fn needs_from_bridge_conversion(&self, schema: &Schema) -> bool {
        match self {
            Self::Vec(ty) | Self::Option(ty) => ty.needs_from_bridge_conversion(schema),
            Self::Named(name) => schema.items.contains_key(name),
            _ => false,
        }
    }

    /// Parse one supported type.
    fn parse(ty: syn::Type) -> Result<Self> {
        let syn::Type::Path(ty) = ty else {
            bail!("bridge type references must be paths");
        };
        if ty.qself.is_some() {
            bail!("bridge type references cannot use qualified paths");
        }
        if ty.path.segments.len() != 1 {
            bail!("bridge type references cannot use qualified paths");
        }

        let segment = ty
            .path
            .segments
            .into_iter()
            .next()
            .context("bridge type path is empty")?;
        let name = segment.ident.to_string();

        match (name.as_str(), segment.arguments) {
            ("String", syn::PathArguments::None) => Ok(Self::String),
            ("bool", syn::PathArguments::None) => Ok(Self::Bool),
            ("u8", syn::PathArguments::None) => Ok(Self::U8),
            ("u32", syn::PathArguments::None) => Ok(Self::U32),
            ("usize", syn::PathArguments::None) => Ok(Self::Usize),
            ("Vec", syn::PathArguments::AngleBracketed(arguments)) => {
                let ty = Self::parse_single_argument(arguments.args.into_iter())?;

                Ok(Self::Vec(Box::new(ty)))
            }
            ("Option", syn::PathArguments::AngleBracketed(arguments)) => {
                let ty = Self::parse_single_argument(arguments.args.into_iter())?;

                Ok(Self::Option(Box::new(ty)))
            }
            (_, syn::PathArguments::None) => Ok(Self::Named(name)),
            _ => bail!("unsupported bridge type reference"),
        }
    }

    /// Parse one generic type argument.
    fn parse_single_argument(
        mut arguments: impl Iterator<Item = syn::GenericArgument>,
    ) -> Result<Self> {
        let Some(syn::GenericArgument::Type(ty)) = arguments.next() else {
            bail!("bridge generic type argument is missing");
        };
        if arguments.next().is_some() {
            bail!("bridge generic type references must have one argument");
        }

        Self::parse(ty)
    }

    /// Visit referenced bridge DTO names.
    fn visit_refs(&self, visit: &mut impl FnMut(&str) -> Result<()>) -> Result<()> {
        match self {
            Self::Vec(ty) | Self::Option(ty) => ty.visit_refs(visit),
            Self::Named(name) => visit(name),
            _ => Ok(()),
        }
    }
}

impl Field {
    /// Return this field as a Rust identifier.
    pub(crate) fn ident(&self) -> proc_macro2::Ident {
        ident(&self.name)
    }

    /// Return this field documentation as Rust doc attributes.
    pub(crate) fn docs(&self) -> TokenStream {
        render_docs(&self.docs)
    }

    /// Return the first documentation line.
    pub(crate) fn doc(&self) -> &str {
        self.docs.first().map(String::as_str).unwrap_or("")
    }

    /// Return this field label.
    pub(crate) fn label(&self) -> String {
        lower_camel(&self.name)
    }

    /// Parse one Rust struct field list.
    fn parse_struct(fields: syn::Fields) -> Result<Vec<Self>> {
        match fields {
            syn::Fields::Named(fields) => Self::parse_list(fields, true),
            syn::Fields::Unit => Ok(Vec::new()),
            syn::Fields::Unnamed(_) => bail!("bridge structs must use named fields"),
        }
    }

    /// Parse one named field list.
    fn parse_list(fields: syn::FieldsNamed, require_public: bool) -> Result<Vec<Self>> {
        fields
            .named
            .into_iter()
            .map(|field| {
                if require_public && !is_public(&field.vis) {
                    bail!("bridge fields must be public");
                }

                let name = field
                    .ident
                    .context("bridge named field is missing an identifier")?
                    .to_string();
                let docs = parse_docs(field.attrs);
                let ty = Type::parse(field.ty)?;

                Ok(Self { name, docs, ty })
            })
            .collect()
    }
}

impl Variant {
    /// Return this variant as a Rust identifier.
    pub(crate) fn ident(&self) -> proc_macro2::Ident {
        ident(&self.name)
    }

    /// Return this variant documentation as Rust doc attributes.
    pub(crate) fn docs(&self) -> TokenStream {
        render_docs(&self.docs)
    }

    /// Return the first documentation line.
    pub(crate) fn doc(&self) -> &str {
        self.docs.first().map(String::as_str).unwrap_or("")
    }

    /// Return this variant label.
    pub(crate) fn label(&self) -> String {
        lower_camel(&self.name)
    }

    /// Return this variant payload field identifier.
    pub(crate) fn payload_field_ident(&self) -> proc_macro2::Ident {
        ident(&self.payload_field_name())
    }

    /// Return this variant payload field name.
    pub(crate) fn payload_field_name(&self) -> String {
        if self.name == "Move" {
            "move_file".to_string()
        } else {
            to_snake(&self.name)
        }
    }

    /// Return this variant payload constructor method identifier.
    pub(crate) fn payload_method_ident(&self) -> proc_macro2::Ident {
        self.payload_field_ident()
    }
}

/// Return public visibility.
fn is_public(visibility: &syn::Visibility) -> bool {
    matches!(visibility, syn::Visibility::Public(_))
}

/// Return whether attributes contain the bridge marker.
fn has_bridge_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("bridge"))
}

/// Keep documentation text.
fn parse_docs(attrs: Vec<syn::Attribute>) -> Vec<String> {
    attrs
        .into_iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }

            let syn::Meta::NameValue(meta) = attr.meta else {
                return None;
            };
            let syn::Expr::Lit(expr) = meta.value else {
                return None;
            };
            let syn::Lit::Str(value) = expr.lit else {
                return None;
            };

            Some(value.value().trim().to_string())
        })
        .collect()
}

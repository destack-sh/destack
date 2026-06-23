use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use proc_macro2::TokenStream;

use super::ty::Type;
use crate::generate::core::{ident, lower_camel, render_docs, to_snake, upper_camel};

const CAPI_HANDLE_ATTRIBUTE: &str = "capi_handle";

/// One bridge DTO type.
pub(crate) struct Item {
    /// Rust type name.
    pub(crate) name: String,
    /// Documentation lines.
    pub(crate) docs: Vec<String>,
    /// Whether C ABI targets project this item as an opaque handle.
    pub(crate) is_capi_handle: bool,
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

/// Transport names for one payload enum.
pub(crate) struct PayloadNames {
    /// Field names that need variant-qualified transport names.
    ambiguous: BTreeSet<String>,
}

impl PayloadNames {
    /// Return transport names for one payload enum.
    pub(crate) fn new(variants: &[Variant]) -> Self {
        let mut fields = BTreeMap::<String, BTreeSet<Type>>::new();

        for variant in variants {
            for (name, ty) in variant.payload_fields() {
                fields.entry(name).or_default().insert(ty);
            }
        }

        let ambiguous = fields
            .into_iter()
            .filter_map(|(name, types)| (types.len() > 1).then_some(name))
            .collect();

        Self { ambiguous }
    }

    /// Return the transport field label for one struct payload field.
    pub(crate) fn field_label(&self, variant: &Variant, field: &Field) -> String {
        if self.ambiguous.contains(&field.name) {
            let variant = lower_camel(&variant.name);
            let field = upper_camel(&field.name);

            format!("{variant}{field}")
        } else {
            field.label()
        }
    }

    /// Return the Rust transport field name for one struct payload field.
    pub(crate) fn field_name(&self, variant: &Variant, field: &Field) -> String {
        to_snake(&self.field_label(variant, field))
    }

    /// Return the transport field label for one tuple payload.
    pub(crate) fn tuple_label(&self, variant: &Variant) -> String {
        variant.payload_field_name()
    }
}

impl Item {
    /// Convert one serde schema item to one generator item.
    pub(super) fn from_schema(
        name: String,
        item: destack_serde::SchemaItem,
        names: &BTreeMap<destack_serde::SchemaName, String>,
    ) -> Result<Self> {
        let is_capi_handle = item
            .attributes
            .iter()
            .any(|attribute| attribute == CAPI_HANDLE_ATTRIBUTE);
        let shape = Shape::from_schema(item.shape, names)?;

        Ok(Self {
            name,
            docs: item.docs,
            is_capi_handle,
            shape,
        })
    }

    /// Return this item as a Rust identifier.
    pub(crate) fn ident(&self) -> proc_macro2::Ident {
        ident(&self.name)
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

    /// Return the first documentation line.
    pub(crate) fn doc(&self) -> &str {
        self.docs.first().map(String::as_str).unwrap_or("")
    }

    /// Visit referenced bridge DTO names.
    pub(super) fn visit_refs(&self, visit: &mut impl FnMut(&str) -> Result<()>) -> Result<()> {
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

impl Shape {
    /// Convert one serde schema shape to one generator shape.
    fn from_schema(
        shape: destack_serde::SchemaShape,
        names: &BTreeMap<destack_serde::SchemaName, String>,
    ) -> Result<Self> {
        match shape {
            destack_serde::SchemaShape::Struct(fields) => {
                let fields = fields
                    .into_iter()
                    .map(|field| Field::from_schema(field, names))
                    .collect::<Result<Vec<_>>>()?;

                Ok(Self::Struct(fields))
            }
            destack_serde::SchemaShape::Enum(variants) => {
                let variants = variants
                    .into_iter()
                    .map(|variant| Variant::from_schema(variant, names))
                    .collect::<Result<Vec<_>>>()?;

                Ok(Self::Enum(variants))
            }
        }
    }
}

impl Field {
    /// Convert one serde schema field to one generator field.
    fn from_schema(
        field: destack_serde::SchemaField,
        names: &BTreeMap<destack_serde::SchemaName, String>,
    ) -> Result<Self> {
        Ok(Self {
            name: field.name,
            docs: field.docs,
            ty: Type::from_schema(field.ty, names)?,
        })
    }

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
}

impl Variant {
    /// Convert one serde schema variant to one generator variant.
    fn from_schema(
        variant: destack_serde::SchemaVariant,
        names: &BTreeMap<destack_serde::SchemaName, String>,
    ) -> Result<Self> {
        Ok(Self {
            name: variant.name,
            docs: variant.docs,
            payload: Payload::from_schema(variant.payload, names)?,
        })
    }

    /// Return payload fields as `(name, type)` pairs.
    pub(crate) fn payload_fields(&self) -> Vec<(String, Type)> {
        match &self.payload {
            Payload::Unit => Vec::new(),
            Payload::Tuple(ty) => vec![(self.payload_field_name(), ty.clone())],
            Payload::Struct(fields) => fields
                .iter()
                .map(|field| (field.name.clone(), field.ty.clone()))
                .collect(),
        }
    }

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
}

impl Payload {
    /// Convert one serde schema payload to one generator payload.
    fn from_schema(
        payload: destack_serde::SchemaPayload,
        names: &BTreeMap<destack_serde::SchemaName, String>,
    ) -> Result<Self> {
        match payload {
            destack_serde::SchemaPayload::Unit => Ok(Self::Unit),
            destack_serde::SchemaPayload::Tuple(ty) => {
                Ok(Self::Tuple(Type::from_schema(ty, names)?))
            }
            destack_serde::SchemaPayload::Struct(fields) => {
                let fields = fields
                    .into_iter()
                    .map(|field| Field::from_schema(field, names))
                    .collect::<Result<Vec<_>>>()?;

                Ok(Self::Struct(fields))
            }
        }
    }

    /// Visit referenced bridge DTO names.
    pub(super) fn visit_refs(&self, visit: &mut impl FnMut(&str) -> Result<()>) -> Result<()> {
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

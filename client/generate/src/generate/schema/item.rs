use std::collections::BTreeMap;

use anyhow::Result;

use super::ty::Type;
use crate::generate::core::{lower_camel, to_snake};

const TUPLE_NEWTYPE_ATTRIBUTE: &str = "tuple_newtype";

/// One client model type.
pub(crate) struct Item {
    /// Unique generator key.
    pub(crate) key: String,
    /// Exported Rust source type name.
    pub(crate) name: String,
    /// Documentation lines.
    pub(crate) docs: Vec<String>,
    /// Whether this item is a single-field tuple struct.
    pub(crate) is_tuple_newtype: bool,
    /// Type shape.
    pub(crate) shape: Shape,
}

/// One client model shape.
pub(crate) enum Shape {
    /// A public struct.
    Struct(Vec<Field>),
    /// A public enum.
    Enum(Vec<Variant>),
}

/// One client struct or variant field.
#[derive(Clone)]
pub(crate) struct Field {
    /// Rust field name.
    pub(crate) name: String,
    /// Documentation lines.
    pub(crate) docs: Vec<String>,
    /// Field type.
    pub(crate) ty: Type,
}

/// One client enum variant.
pub(crate) struct Variant {
    /// Rust variant name.
    pub(crate) name: String,
    /// Documentation lines.
    pub(crate) docs: Vec<String>,
    /// Variant payload.
    pub(crate) payload: Payload,
}

/// One client enum variant payload.
pub(crate) enum Payload {
    /// No payload.
    Unit,
    /// One unnamed payload.
    Tuple(Type),
    /// Named payload fields.
    Struct(Vec<Field>),
}

impl Item {
    /// Convert one serde schema item to one generator item.
    pub(super) fn from_schema(
        key: String,
        item: destack_serde::SchemaItem,
        names: &BTreeMap<destack_serde::SchemaName, String>,
    ) -> Result<Self> {
        let is_tuple_newtype = item
            .attributes
            .iter()
            .any(|attribute| attribute == TUPLE_NEWTYPE_ATTRIBUTE);
        let shape = Shape::from_schema(item.shape, names)?;
        let name = item.name.name;

        Ok(Self {
            key,
            name,
            docs: item.docs,
            is_tuple_newtype,
            shape,
        })
    }

    /// Return whether this type references one client type.
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

    /// Visit referenced client model names.
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

    /// Return whether this item is an enum.
    pub(crate) fn is_enum(&self) -> bool {
        matches!(self.shape, Shape::Enum(_))
    }

    /// Return whether this item contains a map field.
    pub(crate) fn has_map(&self) -> bool {
        match &self.shape {
            Shape::Struct(fields) => fields.iter().any(Field::has_map),
            Shape::Enum(variants) => variants.iter().any(Variant::has_map),
        }
    }

    /// Return this item's scalar tuple newtype field type.
    pub(crate) fn scalar_newtype(&self) -> Option<&Type> {
        let Shape::Struct(fields) = &self.shape else {
            return None;
        };
        let [field] = fields.as_slice() else {
            return None;
        };
        if self.is_tuple_newtype && field.name == "value" && field.ty.is_scalar() {
            Some(&field.ty)
        } else {
            None
        }
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

    /// Return the first documentation line.
    pub(crate) fn doc(&self) -> &str {
        self.docs.first().map(String::as_str).unwrap_or("")
    }

    /// Return this field label.
    pub(crate) fn label(&self) -> String {
        lower_camel(&self.name)
    }

    /// Return whether this field contains a map.
    fn has_map(&self) -> bool {
        self.ty.has_map()
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

    /// Return the first documentation line.
    pub(crate) fn doc(&self) -> &str {
        self.docs.first().map(String::as_str).unwrap_or("")
    }

    /// Return this variant label.
    pub(crate) fn label(&self) -> String {
        lower_camel(&self.name)
    }

    /// Return this variant payload field name.
    pub(crate) fn payload_field_name(&self) -> String {
        if self.name == "Move" {
            "move_file".to_string()
        } else {
            to_snake(&self.name)
        }
    }

    /// Return whether this variant contains a map payload.
    fn has_map(&self) -> bool {
        self.payload.has_map()
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

    /// Visit referenced client model names.
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

    /// Return whether this payload contains a map.
    fn has_map(&self) -> bool {
        match self {
            Self::Unit => false,
            Self::Tuple(ty) => ty.has_map(),
            Self::Struct(fields) => fields.iter().any(Field::has_map),
        }
    }
}

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use destack_serde as serde;

use crate::generate::core::{lower_camel, to_snake};

use super::ty::Type;

/// One client model type.
pub(crate) struct Item {
    /// Unique generator key.
    pub(crate) key: String,
    /// Exported Rust source type name.
    pub(crate) name: String,
    /// Documentation lines.
    pub(crate) docs: Vec<String>,
    /// Reflected client type.
    pub(crate) ty: Type,
}

/// One client struct or variant field.
#[derive(Debug)]
pub(crate) struct Field {
    /// Rust field name.
    pub(crate) name: String,
    /// Documentation lines.
    pub(crate) docs: Vec<String>,
    /// Field type.
    pub(crate) ty: Type,
}

/// One client enum variant.
#[derive(Debug)]
pub(crate) struct Variant {
    /// Rust variant name.
    pub(crate) name: String,
    /// Documentation lines.
    pub(crate) docs: Vec<String>,
    /// Variant payload.
    pub(crate) payload: Payload,
}

/// One client enum variant payload.
#[derive(Debug)]
pub(crate) enum Payload {
    /// No payload.
    Unit,
    /// One unnamed payload.
    Value(Type),
    /// Named payload fields.
    Struct(Vec<Field>),
}

impl Item {
    /// Convert one serde schema item to one generator item.
    pub(super) fn from_schema(
        key: String,
        item: serde::Item,
        keys: &BTreeMap<serde::Name, String>,
    ) -> Result<Self> {
        let ty = Type::from_schema(item.ty, keys)
            .with_context(|| format!("failed to convert schema item {}", item.name.name))?;
        let name = item.name.name;

        Ok(Self {
            key,
            name,
            docs: item.docs,
            ty,
        })
    }

    /// Return whether this type references one client type.
    pub(crate) fn references(&self, name: &str) -> bool {
        self.ty.references(name)
    }

    /// Return the first documentation line.
    pub(crate) fn doc(&self) -> &str {
        self.docs.first().map(String::as_str).unwrap_or("")
    }
}

impl Field {
    /// Convert one serde schema field to one generator field.
    pub(super) fn from_schema(
        field: serde::Field,
        keys: &BTreeMap<serde::Name, String>,
    ) -> Result<Self> {
        let name = field.name;
        let ty = Type::from_schema(field.ty, keys)
            .with_context(|| format!("failed to convert field {name}"))?;

        Ok(Self {
            name,
            docs: field.docs,
            ty,
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
}

impl Variant {
    /// Convert one serde schema variant to one generator variant.
    pub(super) fn from_schema(
        variant: serde::Variant,
        keys: &BTreeMap<serde::Name, String>,
    ) -> Result<Self> {
        let name = variant.name;
        let payload = Payload::from_schema(variant.payload, keys)
            .with_context(|| format!("failed to convert variant {name}"))?;

        Ok(Self {
            name,
            docs: variant.docs,
            payload,
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
        to_snake(&self.name)
    }
}

impl Payload {
    /// Convert one serde schema payload to one generator payload.
    fn from_schema(payload: serde::Payload, keys: &BTreeMap<serde::Name, String>) -> Result<Self> {
        match payload {
            serde::Payload::Unit => Ok(Self::Unit),
            serde::Payload::Value(ty) => Ok(Self::Value(Type::from_schema(ty, keys)?)),
            serde::Payload::Struct(fields) => {
                let fields = fields
                    .into_iter()
                    .map(|field| Field::from_schema(field, keys))
                    .collect::<Result<Vec<_>>>()?;

                Ok(Self::Struct(fields))
            }
        }
    }

    /// Visit referenced client model names.
    pub(super) fn visit_refs(&self, visit: &mut impl FnMut(&str)) {
        match self {
            Self::Unit => {}
            Self::Value(ty) => ty.visit_refs(visit),
            Self::Struct(fields) => {
                for field in fields {
                    field.ty.visit_refs(visit);
                }
            }
        }
    }
}

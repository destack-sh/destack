use std::collections::BTreeMap;

use anyhow::{Result, bail};
use destack_serde as serde;

use super::item::{Field, Variant};

/// One supported client type.
#[derive(Debug)]
pub(crate) enum Type {
    /// Unit value.
    Unit,
    /// `String`.
    String,
    /// `bool`.
    Bool,
    /// `char`.
    Char,
    /// `u8`.
    U8,
    /// `u32`.
    U32,
    /// `u64`.
    U64,
    /// `u128`.
    U128,
    /// Signed integer.
    Signed(u16),
    /// Floating point number.
    Float(u16),
    /// `usize`.
    Usize,
    /// Sequence values.
    Sequence(Box<Type>),
    /// `Option<T>`.
    Option(Box<Type>),
    /// `[T; N]`.
    Array(Box<Type>, usize),
    /// `(A, B, ...)`.
    Tuple(Vec<Type>),
    /// Map-like value.
    Map(Box<Type>, Box<Type>),
    /// Another client model.
    Named { key: String, name: String },
    /// Struct with named fields.
    Struct(Vec<Field>),
    /// Enum with variants.
    Enum(Vec<Variant>),
}

impl Type {
    /// Convert one serde type to one generator type.
    pub(crate) fn from_schema(
        ty: serde::Type,
        keys: &BTreeMap<serde::Name, String>,
    ) -> Result<Self> {
        match ty {
            serde::Type::Unit => Ok(Self::Unit),
            serde::Type::String => Ok(Self::String),
            serde::Type::Bool => Ok(Self::Bool),
            serde::Type::Char => Ok(Self::Char),
            serde::Type::Unsigned { bits: 8 } => Ok(Self::U8),
            serde::Type::Unsigned { bits: 16 | 32 } => Ok(Self::U32),
            serde::Type::Unsigned { bits: 64 } => Ok(Self::U64),
            serde::Type::Unsigned { bits: 128 } => Ok(Self::U128),
            serde::Type::Unsigned { bits } => {
                bail!("unsupported client unsigned width {bits}")
            }
            serde::Type::Signed {
                bits: bits @ (8 | 16 | 32 | 64 | 128),
            } => Ok(Self::Signed(bits)),
            serde::Type::Signed { bits } => {
                bail!("unsupported client signed width {bits}")
            }
            serde::Type::Float { bits: 32 } => Ok(Self::Float(32)),
            serde::Type::Float { bits: 64 } => Ok(Self::Float(64)),
            serde::Type::Float { bits } => {
                bail!("unsupported client float width {bits}")
            }
            serde::Type::Usize => Ok(Self::Usize),
            serde::Type::Sequence(ty) => {
                Ok(Self::Sequence(Box::new(Self::from_schema(*ty, keys)?)))
            }
            serde::Type::Option(ty) => Ok(Self::Option(Box::new(Self::from_schema(*ty, keys)?))),
            serde::Type::Array { item, len } => {
                Ok(Self::Array(Box::new(Self::from_schema(*item, keys)?), len))
            }
            serde::Type::Tuple(types) => {
                let types = types
                    .into_iter()
                    .map(|ty| Self::from_schema(ty, keys))
                    .collect::<Result<Vec<_>>>()?;

                Ok(Self::Tuple(types))
            }
            serde::Type::Map { key, value } => Ok(Self::Map(
                Box::new(Self::from_schema(*key, keys)?),
                Box::new(Self::from_schema(*value, keys)?),
            )),
            serde::Type::Named(name) => {
                let Some(generated) = keys.get(&name).cloned() else {
                    bail!("schema references unknown type {}", name.name);
                };

                Ok(Self::Named {
                    key: generated,
                    name: name.name,
                })
            }
            serde::Type::Struct(fields) => {
                let fields = fields
                    .into_iter()
                    .map(|field| Field::from_schema(field, keys))
                    .collect::<Result<Vec<_>>>()?;

                Ok(Self::Struct(fields))
            }
            serde::Type::Enum(variants) => {
                let variants = variants
                    .into_iter()
                    .map(|variant| Variant::from_schema(variant, keys))
                    .collect::<Result<Vec<_>>>()?;

                Ok(Self::Enum(variants))
            }
        }
    }

    /// Return whether this type references one client model.
    pub(crate) fn references(&self, name: &str) -> bool {
        let mut is_referenced = false;
        self.visit_refs(&mut |reference| is_referenced |= reference == name);

        is_referenced
    }

    /// Abort when this type is unsupported in the current generator position.
    #[track_caller]
    pub(crate) fn unsupported(&self) -> ! {
        unreachable!("unsupported TypeScript generator type: {self:?}")
    }

    /// Visit referenced client model names.
    pub(crate) fn visit_refs(&self, visit: &mut impl FnMut(&str)) {
        match self {
            Self::Sequence(ty) | Self::Option(ty) | Self::Array(ty, _) => ty.visit_refs(visit),
            Self::Tuple(types) => {
                for ty in types {
                    ty.visit_refs(visit);
                }
            }
            Self::Map(key, value) => {
                key.visit_refs(visit);
                value.visit_refs(visit);
            }
            Self::Named { key, .. } => visit(key),
            Self::Struct(fields) => {
                for field in fields {
                    field.ty.visit_refs(visit);
                }
            }
            Self::Enum(variants) => {
                for variant in variants {
                    variant.payload.visit_refs(visit);
                }
            }
            _ => {}
        }
    }
}

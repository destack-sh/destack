use std::collections::BTreeMap;

use anyhow::{Result, bail};

/// One supported client type reference.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
    /// `Vec<T>`.
    Vec(Box<Type>),
    /// `Option<T>`.
    Option(Box<Type>),
    /// `[T; N]`.
    Array(Box<Type>, usize),
    /// `(A, B, ...)`.
    Tuple(Vec<Type>),
    /// Map-like value.
    Map(Box<Type>, Box<Type>),
    /// Dynamic JSON value.
    Json,
    /// Another client model.
    Named { key: String, name: String },
}

impl Type {
    /// Convert one serde schema type reference to one generator type.
    pub(super) fn from_schema(
        ty: destack_serde::SchemaRef,
        names: &BTreeMap<destack_serde::SchemaName, String>,
    ) -> Result<Self> {
        match ty {
            destack_serde::SchemaRef::Unit => Ok(Self::Unit),
            destack_serde::SchemaRef::String => Ok(Self::String),
            destack_serde::SchemaRef::Bool => Ok(Self::Bool),
            destack_serde::SchemaRef::Char => Ok(Self::Char),
            destack_serde::SchemaRef::Unsigned { bits: 8 } => Ok(Self::U8),
            destack_serde::SchemaRef::Unsigned { bits: 16 | 32 } => Ok(Self::U32),
            destack_serde::SchemaRef::Unsigned { bits: 64 } => Ok(Self::U64),
            destack_serde::SchemaRef::Unsigned { bits: 128 } => Ok(Self::U128),
            destack_serde::SchemaRef::Signed { bits } => Ok(Self::Signed(bits)),
            destack_serde::SchemaRef::Float { bits: 32 } => Ok(Self::Float(32)),
            destack_serde::SchemaRef::Float { bits: 64 } => Ok(Self::Float(64)),
            destack_serde::SchemaRef::Float { bits } => {
                bail!("unsupported client float width {bits}")
            }
            destack_serde::SchemaRef::Usize => Ok(Self::Usize),
            destack_serde::SchemaRef::Sequence(ty) => {
                Ok(Self::Vec(Box::new(Self::from_schema(*ty, names)?)))
            }
            destack_serde::SchemaRef::Option(ty) => {
                Ok(Self::Option(Box::new(Self::from_schema(*ty, names)?)))
            }
            destack_serde::SchemaRef::Array { item, len } => {
                Ok(Self::Array(Box::new(Self::from_schema(*item, names)?), len))
            }
            destack_serde::SchemaRef::Tuple(types) => {
                let types = types
                    .into_iter()
                    .map(|ty| Self::from_schema(ty, names))
                    .collect::<Result<Vec<_>>>()?;

                Ok(Self::Tuple(types))
            }
            destack_serde::SchemaRef::Map { key, value } => Ok(Self::Map(
                Box::new(Self::from_schema(*key, names)?),
                Box::new(Self::from_schema(*value, names)?),
            )),
            destack_serde::SchemaRef::Json => Ok(Self::Json),
            destack_serde::SchemaRef::Named(name) => {
                let Some(generated) = names.get(&name).cloned() else {
                    bail!("schema references unknown type {}", name.name);
                };

                Ok(Self::Named {
                    key: generated,
                    name: name.name,
                })
            }
            other => bail!("unsupported client schema type {other:?}"),
        }
    }

    /// Abort when a protocol-only type reaches a public client backend generator.
    #[track_caller]
    pub(crate) fn unsupported_client_type(&self) -> ! {
        unreachable!("protocol-only type reached public client generator: {self:?}")
    }

    /// Visit referenced client model names.
    pub(super) fn visit_refs(&self, visit: &mut impl FnMut(&str) -> Result<()>) -> Result<()> {
        match self {
            Self::Vec(ty) | Self::Option(ty) | Self::Array(ty, _) => ty.visit_refs(visit),
            Self::Tuple(types) => {
                for ty in types {
                    ty.visit_refs(visit)?;
                }

                Ok(())
            }
            Self::Map(key, value) => {
                key.visit_refs(visit)?;
                value.visit_refs(visit)
            }
            Self::Named { key, .. } => visit(key),
            _ => Ok(()),
        }
    }

    /// Return whether this type is a scalar client value.
    pub(crate) fn is_scalar(&self) -> bool {
        matches!(
            self,
            Self::String
                | Self::Bool
                | Self::Char
                | Self::U8
                | Self::U32
                | Self::U64
                | Self::U128
                | Self::Signed(_)
                | Self::Float(_)
                | Self::Usize
        )
    }
}

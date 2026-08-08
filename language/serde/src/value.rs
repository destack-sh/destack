use serde::{Deserialize, Serialize};

use crate::Reflect;

/// One dynamically typed canonical value.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Value {
    /// Null value.
    #[default]
    Null,
    /// Boolean value.
    Bool(bool),
    /// Signed integer value.
    Signed(i128),
    /// Unsigned integer value.
    Unsigned(u128),
    /// Floating point value.
    Float(f64),
    /// String value.
    String(String),
    /// Array value.
    Array(Vec<Value>),
    /// Object entries in source order.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// Convert this value into a JSON value.
    pub fn into_json(self) -> Result<serde_json::Value, ValueError> {
        match self {
            Self::Null => Ok(serde_json::Value::Null),
            Self::Bool(value) => Ok(serde_json::Value::Bool(value)),
            Self::Signed(value) => {
                let value =
                    serde_json::Number::from_i128(value).ok_or(ValueError::UnsupportedInteger)?;

                Ok(serde_json::Value::Number(value))
            }
            Self::Unsigned(value) => {
                let value =
                    serde_json::Number::from_u128(value).ok_or(ValueError::UnsupportedInteger)?;

                Ok(serde_json::Value::Number(value))
            }
            Self::Float(value) => serde_json::Number::from_f64(value)
                .map(serde_json::Value::Number)
                .ok_or(ValueError::NonFiniteFloat),
            Self::String(value) => Ok(serde_json::Value::String(value)),
            Self::Array(values) => {
                let values = values
                    .into_iter()
                    .map(Self::into_json)
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(serde_json::Value::Array(values))
            }
            Self::Object(values) => {
                let values = values
                    .into_iter()
                    .map(|(key, value)| value.into_json().map(|value| (key, value)))
                    .collect::<Result<serde_json::Map<_, _>, _>>()?;

                Ok(serde_json::Value::Object(values))
            }
        }
    }

    /// Convert one JSON number into a canonical value.
    fn from_number(value: serde_json::Number) -> Self {
        if let Some(value) = value.as_i64() {
            Self::Signed(value as i128)
        } else if let Some(value) = value.as_u64() {
            Self::Unsigned(value as u128)
        } else if let Some(value) = value.as_i128() {
            Self::Signed(value)
        } else if let Some(value) = value.as_u128() {
            Self::Unsigned(value)
        } else if let Some(value) = value.as_f64() {
            Self::Float(value)
        } else {
            unreachable!("serde_json numbers are integer or floating point values")
        }
    }
}

impl From<serde_json::Value> for Value {
    /// Convert one JSON value into a canonical value.
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(value) => Self::Bool(value),
            serde_json::Value::Number(value) => Self::from_number(value),
            serde_json::Value::String(value) => Self::String(value),
            serde_json::Value::Array(values) => {
                Self::Array(values.into_iter().map(Self::from).collect())
            }
            serde_json::Value::Object(values) => Self::Object(
                values
                    .into_iter()
                    .map(|(key, value)| (key, Self::from(value)))
                    .collect(),
            ),
        }
    }
}

/// Failure to convert one canonical value into JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueError {
    /// JSON cannot represent a wide integer with the current number backend.
    UnsupportedInteger,
    /// JSON cannot represent a non-finite floating point value.
    NonFiniteFloat,
}

impl std::fmt::Display for ValueError {
    /// Format this value conversion failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedInteger => formatter.write_str("value contains a wide integer"),
            Self::NonFiniteFloat => formatter.write_str("value contains a non-finite float"),
        }
    }
}

impl std::error::Error for ValueError {}

use destack_dir as dir;

use crate::{CompilerError, CompilerResult};

/// One evaluated structural object used as decorator configuration.
pub(in crate::check) struct DecoratorObject<'a> {
    /// The evaluated name fields in application order.
    pub(in crate::check) fields: Vec<(dir::StringId, &'a dir::StaticTerm)>,
}

impl<'a> TryFrom<&'a dir::StaticTerm> for DecoratorObject<'a> {
    type Error = CompilerError;

    /// Flatten one decorator object.
    fn try_from(value: &'a dir::StaticTerm) -> CompilerResult<Self> {
        let Some(properties) = value.as_object() else {
            return Err(CompilerError::Internal {
                message: "decorator value is not a structural object".to_string(),
            });
        };
        let mut object = Self {
            fields: Vec::with_capacity(properties.len()),
        };
        object.extend(properties)?;

        Ok(object)
    }
}

impl<'a> DecoratorObject<'a> {
    /// Extend this object with evaluated fields and spreads.
    fn extend(&mut self, properties: &'a [dir::StaticProperty]) -> CompilerResult<()> {
        for property in properties {
            match property {
                dir::StaticProperty::Field {
                    key: dir::StaticKey::Name(key),
                    value,
                } => self.fields.push((*key, value)),
                dir::StaticProperty::Spread { value } => {
                    let Some(properties) = value.as_object() else {
                        return Err(CompilerError::Internal {
                            message: "decorator object spread is not an object".to_string(),
                        });
                    };

                    self.extend(properties)?;
                }
                dir::StaticProperty::Field { .. } | dir::StaticProperty::Method { .. } => {
                    return Err(CompilerError::Internal {
                        message: "decorator object contains a non-name field".to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

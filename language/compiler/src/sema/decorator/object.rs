use tspp_dir as dir;

use crate::{CompilerError, CompilerResult};

/// One evaluated structural object used as decorator configuration.
pub(in crate::sema) struct DecoratorObject<'a> {
    /// The evaluated name fields in application order.
    pub(in crate::sema) fields: Vec<(dir::StringId, &'a dir::StaticTerm)>,
}

impl<'a> TryFrom<&'a dir::StaticTerm> for DecoratorObject<'a> {
    type Error = CompilerError;

    /// Build one decorator object from an evaluated structural object.
    fn try_from(value: &'a dir::StaticTerm) -> CompilerResult<Self> {
        let Some(properties) = value.as_object() else {
            return Err(CompilerError::Internal {
                message: "decorator value is not a structural object".to_string(),
            });
        };
        let mut fields = Vec::with_capacity(properties.len());

        // retain evaluated name fields in source order
        for property in properties {
            let Some(field) = property.as_name_field() else {
                return Err(CompilerError::Internal {
                    message: "decorator object contains a non-name field".to_string(),
                });
            };
            fields.push(field);
        }
        Ok(Self { fields })
    }
}

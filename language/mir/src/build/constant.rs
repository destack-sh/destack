use destack_core::StringId;

use crate::tree::{Constant, Global, GlobalInitializer, LocalNodeId, Type};

use super::error::{BuildError, BuildResult};
use super::module::ModuleBuilder;

/// One named field value of a pre-built constant object.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    /// A scalar constant at the field's carrier.
    Scalar(Constant),
    /// A slice header addressing one constant element global.
    Slice {
        /// The global holding the constant elements.
        elements: LocalNodeId<Global>,
        /// The element count.
        length: u64,
    },
}

impl ModuleBuilder {
    /// Build one constant object initializer over a struct row, field by name.
    pub fn constant_object(
        &mut self,
        row: LocalNodeId<Type>,
        values: &[(&str, ConstantValue)],
    ) -> BuildResult<GlobalInitializer> {
        // read the declared row fields in order
        let Type::Struct { fields, .. } = self.tree.get(row) else {
            return Err(BuildError::InvalidConstantRow { row });
        };
        let fields = fields.clone();

        // intern the provided names once for field matching
        let names: Vec<(StringId, &ConstantValue)> = values
            .iter()
            .map(|(name, value)| (self.strings.intern(name), value))
            .collect();

        // fill each declared field from its named value
        let mut ordered = Vec::with_capacity(fields.len());
        for field_id in fields {
            let field = self.tree.get(field_id).clone();
            let Some(name) = field.name else {
                return Err(BuildError::InvalidConstantField { name: None });
            };
            let Some((_, value)) = names.iter().find(|(provided, _)| *provided == name) else {
                return Err(BuildError::InvalidConstantField {
                    name: Some(self.strings.get(name).to_string()),
                });
            };
            ordered.push(self.constant_field(name, field.ty, value)?);
        }

        // reject values naming no declared field
        if names.len() != ordered.len() {
            return Err(BuildError::InvalidConstantField { name: None });
        }

        Ok(GlobalInitializer::Aggregate(ordered))
    }

    /// Render one field value at its declared carrier.
    fn constant_field(
        &self,
        name: StringId,
        carrier: LocalNodeId<Type>,
        value: &ConstantValue,
    ) -> BuildResult<GlobalInitializer> {
        match value {
            // scalar constants require a scalar carrier
            ConstantValue::Scalar(constant) => {
                let is_scalar = matches!(
                    self.tree.get(carrier),
                    Type::Boolean
                        | Type::Character
                        | Type::Int { .. }
                        | Type::Isize
                        | Type::Usize
                        | Type::Float(_)
                        | Type::Newtype { .. }
                );
                if !is_scalar {
                    return Err(BuildError::InvalidConstantField {
                        name: Some(self.strings.get(name).to_string()),
                    });
                }

                Ok(GlobalInitializer::Scalar(constant.clone()))
            }
            // slice headers require a slice carrier: address word, then length
            ConstantValue::Slice { elements, length } => {
                if !matches!(self.tree.get(carrier), Type::Slice { .. }) {
                    return Err(BuildError::InvalidConstantField {
                        name: Some(self.strings.get(name).to_string()),
                    });
                }
                let pointer_width = (self.pointer_bytes() * 8) as u16;

                Ok(GlobalInitializer::Aggregate(vec![
                    GlobalInitializer::GlobalAddress(*elements),
                    GlobalInitializer::Scalar(Constant::UInt {
                        value: *length as u128,
                        width: pointer_width,
                    }),
                ]))
            }
        }
    }
}

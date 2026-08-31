use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{NominalField, TypeLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

impl TypeLowerer<'_, '_> {
    /// Lower one value enum declaration to its variant type.
    pub(in crate::lower) fn lower_enum(
        &mut self,
        _symbol: dir::GlobalSymbolId,
        definition: dir::EnumDefinition,
        ty: mir::LocalNodeId<mir::Type>,
        copy: mir::Copy,
    ) -> CompilerResult<Vec<NominalField>> {
        // require an integer representation
        let dir::EnumBackingType::Integer(_) = definition.backing else {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a string-backed enum".to_string(),
            }
            .into());
        };

        // collect the case values
        let mut values = Vec::new();
        for variant in definition.variants() {
            let dir::EnumVariantValue::Integer(value) = variant.value else {
                return Err(CompilerError::Internal {
                    message: "a non-integer variant in an integer-backed enum".to_string(),
                });
            };
            values.push(value);
        }

        // fit the discriminant to the narrowest width holding every case value
        let is_signed = values.iter().any(|value| *value < 0);
        let width = [8u16, 16, 32]
            .into_iter()
            .find(|width| {
                let bound = 1i64 << (width - u16::from(is_signed));
                let floor = if is_signed { -bound } else { 0 };
                values.iter().all(|value| (floor..bound).contains(value))
            })
            .unwrap_or(64);
        let integer = dir::IntegerType::Fixed { width, is_signed };

        // lower the discriminant scalar
        let discriminant = self
            .lower
            .scalar_type(&dir::Type::Primitive(dir::PrimitiveType::Integer(integer)))?;
        let discriminant = self.tree.intern_type(discriminant);

        // build the variant cases in declaration order
        let storage = self.tree.intern_type(mir::Type::Void);
        let mut fields = Vec::new();
        let mut variants = Vec::new();
        for (variant, value) in definition.variants().zip(values) {
            // record the variant member
            fields.push(NominalField {
                key: variant.key,
                symbol: variant.symbol,
                is_optional: false,
                initializer: None,
            });

            // build the case beside the discriminant it carries
            let discriminant = match integer.is_signed() {
                true => mir::Constant::Int {
                    value: i128::from(value),
                    width,
                    is_signed: true,
                },
                false => mir::Constant::UInt {
                    value: value as u128,
                    width,
                },
            };
            variants.push(mir::VariantCase {
                discriminant,
                ty: storage,
            });
        }

        // define the variant representation from its cases
        self.tree.define_type(
            ty,
            mir::Type::Variant {
                discriminant,
                cases: variants,
                copy,
            },
        );

        Ok(fields)
    }
}

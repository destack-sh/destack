use tspp_dir as dir;
use tspp_mir as mir;

use tspp_core::StringId;

use crate::lower::{ModuleLowerer, NominalField, TypeLowerer};
use crate::{CompilerError, CompilerResult};

/// The runtime form of one enum case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::lower) enum EnumCase {
    /// A tagged variant case.
    Tag(u32),
    /// A constant String object.
    String(StringId),
}

impl TypeLowerer<'_, '_> {
    /// Lower one value enum declaration to its representation.
    pub(in crate::lower) fn lower_enum(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: dir::EnumDefinition,
        declaration: mir::LocalNodeId<mir::TypeDeclaration>,
    ) -> CompilerResult<Vec<NominalField>> {
        // carry string cases as String objects
        if definition.backing == dir::EnumBackingType::String {
            let string = self
                .lower
                .lower_literal_nominal(self.tree, dir::LanguageItem::String)?;
            let definition = self.tree.intern_type(mir::Type::Newtype {
                value: string.value,
            });
            self.tree.get_mut(declaration).definition = Some(definition);

            return self.lower.nominal_fields(symbol);
        }

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
            .unwrap_or(i64::BITS as u16);
        let integer = dir::IntegerType::Fixed { width, is_signed };

        // lower the discriminant scalar
        let discriminant = self
            .lower
            .scalar_type(&dir::Type::Primitive(dir::PrimitiveType::Integer(integer)))?;
        let discriminant = self.tree.intern_type(discriminant);

        // build the variant cases in declaration order
        let storage = self.tree.intern_type(mir::Type::Void);
        let fields = self.lower.nominal_fields(symbol)?;
        let mut variants = Vec::new();
        for (_, value) in definition.variants().zip(values) {
            // build the case beside its discriminant
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
        let definition = self.tree.intern_type(mir::Type::Variant {
            discriminant,
            cases: variants,
        });
        self.tree.get_mut(declaration).definition = Some(definition);

        Ok(fields)
    }
}

impl ModuleLowerer<'_> {
    /// Return one enum variant's declaration position.
    pub(in crate::lower) fn variant_position(
        &mut self,
        owner: dir::GlobalSymbolId,
        variant: dir::GlobalSymbolId,
    ) -> CompilerResult<u32> {
        let definition = self.enum_definition(owner)?;
        let Some(index) = definition.variant_position(variant) else {
            return Err(CompilerError::Internal {
                message: "a variant missing from its owner definition".to_string(),
            });
        };

        Ok(index as u32)
    }

    /// Return the runtime form of one enum case.
    pub(in crate::lower) fn enum_case(
        &mut self,
        owner: dir::GlobalSymbolId,
        variant: dir::GlobalSymbolId,
    ) -> CompilerResult<EnumCase> {
        let definition = self.enum_definition(owner)?;
        let Some((position, case)) = definition
            .variants()
            .enumerate()
            .find(|(_, case)| case.symbol == variant)
        else {
            return Err(CompilerError::Internal {
                message: "a variant missing from its owner definition".to_string(),
            });
        };

        Ok(match case.value {
            dir::EnumVariantValue::Integer(_) => EnumCase::Tag(position as u32),
            dir::EnumVariantValue::String(string) => EnumCase::String(string),
        })
    }

    /// Return the enum one type names.
    pub(in crate::lower) fn enum_owner(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let owner = match self.ty(ty)? {
            dir::Type::Variant(case) => case.owner,
            _ => ty,
        };
        let dir::Type::Application(application) = self.ty(owner)? else {
            return Ok(None);
        };
        let is_enum = matches!(
            self.definition(application.symbol)?,
            Some(dir::Definition::Enum(_))
        );

        Ok(is_enum.then_some(application.symbol))
    }

    /// Return the runtime form of the case declaring one value.
    pub(in crate::lower) fn enum_case_of_value(
        &mut self,
        owner: dir::GlobalSymbolId,
        value: dir::Literal,
    ) -> CompilerResult<EnumCase> {
        let definition = self.enum_definition(owner)?;
        let variant = definition
            .variants()
            .find(|case| match (case.value, value) {
                (dir::EnumVariantValue::Integer(declared), dir::Literal::Integer(value)) => {
                    declared == value
                }
                (dir::EnumVariantValue::String(declared), dir::Literal::String(value)) => {
                    declared == value
                }
                _ => false,
            });
        let Some(variant) = variant else {
            return Err(CompilerError::Internal {
                message: "an enum constant naming no declared value".to_string(),
            });
        };
        let variant = variant.symbol;

        self.enum_case(owner, variant)
    }

    /// Return one enum definition.
    fn enum_definition(
        &mut self,
        owner: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::EnumDefinition> {
        match self.definition(owner)?.cloned() {
            Some(dir::Definition::Enum(definition)) => Ok(definition),
            _ => Err(CompilerError::Internal {
                message: "a variant owner without an enum definition".to_string(),
            }),
        }
    }
}

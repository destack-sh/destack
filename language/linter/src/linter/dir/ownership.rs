use destack_dir as dir;
use destack_repository::ProviderError;

use super::Dir;

impl Dir<'_> {
    /// Read the Copy decision recorded by the type's module.
    pub(crate) fn copies(&self, ty: dir::GlobalTypeId) -> Result<Option<bool>, ProviderError> {
        self.read_declaration_tables(ty.module_id, |tables| Ok(tables.representations.copies(ty)))
    }

    /// Return whether one type is a mutable managed reference.
    pub(crate) fn is_mutable_reference(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Result<bool, ProviderError> {
        let ty = self.get_type(type_id)?;

        // readonly views cannot expose mutation through repeated aliases
        if matches!(ty, dir::Type::Form(form) if form.form == dir::Form::Readonly) {
            return Ok(false);
        }

        // unknown and dynamic values expose no mutable members
        if matches!(ty, dir::Type::Unknown | dir::Type::Dynamic(_)) {
            return Ok(false);
        }

        // inspect every union element
        if matches!(ty, dir::Type::Union(_)) {
            for element in self.union_elements(type_id)? {
                if self.is_mutable_reference(element)? {
                    return Ok(true);
                }
            }

            return Ok(false);
        }

        // inspect every intersection element
        if matches!(ty, dir::Type::Intersection(_)) {
            for element in self.intersection_elements(type_id)? {
                if self.is_mutable_reference(element)? {
                    return Ok(true);
                }
            }

            return Ok(false);
        }

        // require managed storage with a mutable runtime representation
        if self.default_ownership(type_id)? != Some(dir::Ownership::Managed) {
            return Ok(false);
        }
        let representation = self.representation_item(type_id)?;
        let is_immutable = matches!(
            representation,
            Some(
                dir::LanguageItem::BigInt | dir::LanguageItem::Function | dir::LanguageItem::String
            )
        );

        Ok(!is_immutable)
    }

    /// Return the runtime ownership one persisted type defaults to.
    pub fn default_ownership(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Result<Option<dir::Ownership>, ProviderError> {
        let ownership = match self.get_type(type_id)? {
            dir::Type::Unknown
            | dir::Type::Object(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Slice(_)
            | dir::Type::Function(_) => Some(dir::Ownership::Managed),
            dir::Type::Never
            | dir::Type::Void
            | dir::Type::Undefined
            | dir::Type::Null
            | dir::Type::Static(_)
            | dir::Type::Intrinsic
            | dir::Type::Key(_)
            | dir::Type::Region(_)
            | dir::Type::Range(_)
            | dir::Type::Tuple(_)
            | dir::Type::FixedArray(_)
            | dir::Type::FunctionPointer(_) => Some(dir::Ownership::Owned),
            dir::Type::Primitive(primitive) => Some(primitive.ownership()),
            // take a literal's ownership from its widened runtime scalar
            dir::Type::Literal(literal) => match literal.widen() {
                dir::Type::Primitive(primitive) => Some(primitive.ownership()),
                _ => Some(dir::Ownership::Owned),
            },
            dir::Type::Application(_)
                if self.representation_item(type_id)? == Some(dir::LanguageItem::WithAccess) =>
            {
                let value = self.application_argument(type_id, 0)?.ok_or_else(|| {
                    ProviderError::internal(format!(
                        "WithAccess application {type_id:?} has no value argument"
                    ))
                })?;

                return self.default_ownership(value);
            }
            dir::Type::Application(instance) => {
                return self.nominal_ownership(instance.symbol);
            }
            dir::Type::Variant(variant) => {
                return self.default_ownership(variant.owner);
            }
            dir::Type::Form(form) => match form.form {
                dir::Form::Readonly => {
                    return self.default_ownership(form.value);
                }
                // take an explicit form as the value's own ownership
                dir::Form::Managed { .. }
                | dir::Form::Owned
                | dir::Form::Borrowed(_)
                | dir::Form::Raw => form.form.ownership(),
            },
            dir::Type::Reference(_)
            | dir::Type::Parameter(_)
            | dir::Type::Erased(_)
            | dir::Type::Variable(_)
            | dir::Type::This
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Intersection(_)
            | dir::Type::Union(_)
            // refined bases live in the owning module's refinement table
            | dir::Type::Refined(_)
            | dir::Type::Error => None,
        };

        Ok(ownership)
    }

    /// Return the ownership one nominal declaration's family defaults to.
    fn nominal_ownership(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<dir::Ownership>, ProviderError> {
        let definition = self.read_declaration_tables(symbol.module_id, |tables| {
            Ok(tables.definitions.definition(symbol).cloned())
        })?;

        match definition {
            Some(dir::Definition::Class(_) | dir::Definition::Interface(_)) => {
                Ok(Some(dir::Ownership::Managed))
            }
            Some(dir::Definition::Struct(_) | dir::Definition::Enum(_)) => {
                Ok(Some(dir::Ownership::Owned))
            }
            // take a newtype's ownership from its backing representation
            Some(dir::Definition::Newtype(definition)) => {
                self.default_ownership(definition.backing)
            }
            _ => Ok(None),
        }
    }
}

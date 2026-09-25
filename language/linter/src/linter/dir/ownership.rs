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

    /// Return the ownership one type's head defaults to, as its module recorded it.
    pub fn default_ownership(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Result<Option<dir::Ownership>, ProviderError> {
        self.read_declaration_tables(type_id.module_id, |tables| {
            match tables.representations.ownership(type_id) {
                Some(dir::DefaultOwnership::Decided(ownership)) => Ok(Some(ownership)),
                Some(dir::DefaultOwnership::Open) => Ok(None),
                None => Err(ProviderError::internal(format!(
                    "no ownership recorded for type {type_id:?}"
                ))),
            }
        })
    }
}

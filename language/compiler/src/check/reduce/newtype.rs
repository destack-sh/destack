use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{CheckState, Origin};

/// One newtype instance with its declared backing substituted.
pub(in crate::check) struct NewtypeInstance {
    /// The applied newtype symbol.
    symbol: dir::GlobalSymbolId,
    /// The applied generic arguments.
    generic_arguments: Vec<dir::GenericArgumentBinding>,
    /// The substituted backing type.
    pub(in crate::check) backing: dir::GlobalTypeId,
}

impl NewtypeInstance {
    /// Convert this application into a runtime payload projection.
    pub(in crate::check) fn into_projection(self) -> dir::Projection {
        dir::Projection::NewtypePayload {
            symbol: self.symbol,
            generic_arguments: self.generic_arguments,
            ty: self.backing,
        }
    }
}

impl CheckState<'_> {
    /// Decompose one nominal newtype instance.
    pub(in crate::check) fn decompose_newtype(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<NewtypeInstance>> {
        let dir::Type::Instance(instance) = self.ty(value)? else {
            return Ok(None);
        };
        let symbol = self.resolve_symbol_alias(instance.symbol)?;
        let Some(dir::Definition::Newtype(definition)) = self.definition(symbol)? else {
            return Ok(None);
        };
        let instance = dir::GenericInstance { symbol, ..instance };
        let declared_backing = definition.backing;

        // apply the written arguments to the declared backing
        let arguments = self.type_ids(value.module_id, instance.arguments)?.to_vec();
        let substitution = self
            .instance_substitution(value.module_id, &instance)?
            .with_receiver(value);
        let backing = self.substitute_type(origin.module(), declared_backing, &substitution)?;
        let generic_arguments = self.symbol_generic_argument_bindings(symbol, &arguments)?;

        Ok(Some(NewtypeInstance {
            symbol,
            generic_arguments,
            backing,
        }))
    }
}

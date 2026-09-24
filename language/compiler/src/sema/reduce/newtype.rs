use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin};

/// One newtype instance with its declared backing substituted.
pub(in crate::sema) struct NewtypeInstance {
    /// The applied newtype symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The applied generic arguments.
    generic_arguments: Vec<dir::GenericArgumentBinding>,
    /// The substituted backing type.
    pub(in crate::sema) backing: dir::GlobalTypeId,
}

impl NewtypeInstance {
    /// Convert this application into an implicit receiver adjustment.
    pub(in crate::sema) fn into_receiver_adjustment(
        self,
        ty: dir::GlobalTypeId,
    ) -> dir::ReceiverAdjustment {
        dir::ReceiverAdjustment::NewtypePayload {
            key: dir::InstanceKey::new(self.symbol, self.generic_arguments),
            ty,
        }
    }

    /// Convert this application into a runtime payload projection.
    pub(in crate::sema) fn into_projection(self) -> dir::Projection {
        dir::Projection::NewtypePayload {
            key: dir::InstanceKey::new(self.symbol, self.generic_arguments),
            ty: self.backing,
        }
    }
}

impl CheckState<'_> {
    /// Decompose one nominal newtype instance.
    pub(in crate::sema) fn decompose_newtype(
        &mut self,
        _origin: Origin,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<NewtypeInstance>> {
        // resolve the solved value before reading its head
        let value = self.shallow_resolve(value)?;
        let dir::Type::Application(instance) = self.ty(value)? else {
            return Ok(None);
        };
        let declared = self.definition(instance.symbol)?;
        let Some(dir::Definition::Newtype(definition)) = declared.as_deref() else {
            return Ok(None);
        };
        let symbol = instance.symbol;
        let instance = dir::GenericApplication { symbol, ..instance };
        let declared_backing = definition.backing;

        // apply the written arguments to the declared backing
        let arguments: SmallVec<[_; 8]> =
            self.type_ids(value.module_id, instance.arguments)?.into();
        let substitution = self
            .instance_substitution(value.module_id, &instance)?
            .with_receiver(value);
        let backing = self.substitute_type(declared_backing, &substitution)?;
        let generic_arguments = self.symbol_generic_argument_bindings(symbol, &arguments)?;

        Ok(Some(NewtypeInstance {
            symbol,
            generic_arguments,
            backing,
        }))
    }
}

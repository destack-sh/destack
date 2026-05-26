use destack_dir as dir;

use crate::check::{
    CheckComponentState, CheckError, CheckModuleState, Obligation, Place, PlaceTarget,
    ShapeMemberTerm, TypeTerm, VariableId,
};
use crate::{CompilerError, CompilerResult};

impl CheckModuleState {
    /// Require one place to accept a write.
    pub(in crate::check) fn require_writable_place(&mut self, place: Place) {
        self.add_obligation(Obligation::WritablePlace { place });
    }
}

impl CheckComponentState<'_> {
    /// Check one writable place requirement.
    pub(in crate::check) fn check_writable_place(&mut self, place: Place) -> CompilerResult<()> {
        if self.is_writable_place(place)? {
            return Ok(());
        }

        let (module, anchor) = self.source_anchor(place.source)?;
        let diagnostic = CheckError::NotWritable { anchor, module };

        self.module_mut(place.source.module_id)?
            .work
            .diagnostics
            .push(diagnostic);

        Ok(())
    }

    /// Return whether one place is writable.
    fn is_writable_place(&self, place: Place) -> CompilerResult<bool> {
        let is_writable = match place.target {
            PlaceTarget::Binding { symbol } => self.is_writable_binding(symbol)?,
            PlaceTarget::Member { owner, key } => self.is_writable_member(owner, key)?,
            PlaceTarget::Index { .. } => true,
            PlaceTarget::Dereference { .. } => true,
        };

        Ok(is_writable)
    }

    /// Return whether one local binding can be assigned.
    fn is_writable_binding(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<bool> {
        let check_module = self.module(symbol.module_id)?;
        let bindings = check_module.input.binding_table();
        let local_symbol = bindings.get_symbol(symbol.local_id);
        let is_writable = local_symbol.binding_mutability.is_some_and(|mutability| {
            matches!(
                mutability,
                dir::Mutability::Mutable | dir::Mutability::Exclusive
            )
        }) && check_module
            .input
            .resolved
            .imports
            .symbol_target(symbol.local_id)
            .is_none();

        Ok(is_writable)
    }

    /// Return whether one member target can be assigned.
    fn is_writable_member(&self, owner: VariableId, key: dir::StaticKey) -> CompilerResult<bool> {
        let Some(owner) = self.solved_type_term(owner)? else {
            return Err(CompilerError::Internal {
                message: format!("place owner {owner:?} was not solved"),
            });
        };

        // structural fields carry their write access directly
        if let TypeTerm::Shape { members } = owner {
            for member in members {
                if let ShapeMemberTerm::Field {
                    key: member_key,
                    is_readonly,
                    ..
                } = member
                    && member_key.matches(&key)
                {
                    return Ok(!is_readonly);
                }
            }

            return Ok(true);
        }

        Ok(true)
    }
}

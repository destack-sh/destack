use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CheckError, CheckState, Condition, Decision, Obligation, Place, PlaceTarget, ShapeMember,
    TypeOperand, TypeTerm,
};

impl CheckState<'_> {
    /// Require one place to accept a write.
    pub(in crate::check) fn require_writable_place(&mut self, place: Place, condition: Condition) {
        self.require(Obligation::WritablePlace { place, condition });
    }
}

impl CheckState<'_> {
    /// Check one writable place requirement.
    pub(in crate::check) fn check_writable_place(
        &mut self,
        place: Place,
    ) -> CompilerResult<Option<CheckError>> {
        let diagnostic = match self.decide_writable_place(place)? {
            Decision::Yes => return Ok(None),
            Decision::No => {
                let (module, anchor) = self.source_anchor(place.source);

                CheckError::NotWritable { anchor, module }
            }
            Decision::Undecidable => {
                let (module, anchor) = self.source_anchor(place.source);

                CheckError::CannotSolve { anchor, module }
            }
        };

        Ok(Some(diagnostic))
    }

    /// Return whether one place is writable.
    fn decide_writable_place(&self, place: Place) -> CompilerResult<Decision> {
        let decision = match place.target {
            PlaceTarget::Binding { symbol } => {
                self.decide_writable_binding(place.source, symbol)?
            }
            PlaceTarget::Member { owner, key } => self.decide_writable_member(owner, key)?,
            PlaceTarget::Index { .. } | PlaceTarget::Dereference => Decision::Yes,
        };

        Ok(decision)
    }

    /// Return whether one local binding can be assigned.
    fn decide_writable_binding(
        &self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Decision> {
        if symbol.module_id != source.module_id {
            return Ok(Decision::No);
        }

        let input = self.module(symbol.module_id);
        let bindings = input.binding_table();
        let local_symbol = bindings.get_symbol(symbol.local_id);
        let is_writable = local_symbol.binding_mutability.is_some_and(|mutability| {
            matches!(
                mutability,
                dir::Mutability::Mutable | dir::Mutability::Exclusive
            )
        }) && input
            .resolved
            .imports
            .symbol_target(symbol.local_id)
            .is_none();

        Ok(Decision::from(is_writable))
    }

    /// Return whether one member target can be assigned.
    fn decide_writable_member(
        &self,
        owner: TypeOperand,
        key: dir::StaticKey,
    ) -> CompilerResult<Decision> {
        let Some(owner) = self.type_operand_term(owner)? else {
            return Ok(Decision::Undecidable);
        };

        // structural fields carry their write access directly
        if let TypeTerm::Shape(shape) = owner {
            let members = self.term(shape).members.clone();
            for member in members {
                if let ShapeMember::Field {
                    key: member_key,
                    is_readonly,
                    ..
                } = member
                    && member_key.matches(&key)
                {
                    return Ok(Decision::from(!is_readonly));
                }
            }

            return Ok(Decision::Yes);
        }

        Ok(Decision::Yes)
    }
}

use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckError, CheckState, Condition, Obligation, Place, PlaceTarget, ShapeMember,
    TypeOperand, TypeTerm,
};

impl CheckState<'_> {
    /// Constrain one place to accept a write.
    pub(in crate::check) fn constrain_writable_place(
        &mut self,
        place: Place,
        condition: Condition,
    ) {
        self.push_obligation(Obligation::WritablePlace { place, condition });
    }

    /// Check one writable place requirement.
    pub(in crate::check) fn check_writable_place(
        &mut self,
        place: Place,
    ) -> CompilerResult<Option<CheckError>> {
        let diagnostic = match self.decide_writable_place(place)? {
            Answer::Ready(true) => return Ok(None),
            Answer::Ready(false) => {
                let (module, anchor) = self.source_anchor(place.source);

                CheckError::NotWritable { anchor, module }
            }
            Answer::Pending(_) => {
                let (module, anchor) = self.source_anchor(place.source);

                CheckError::CannotSolve { anchor, module }
            }
        };

        Ok(Some(diagnostic))
    }

    /// Return whether one place is writable.
    fn decide_writable_place(&mut self, place: Place) -> CompilerResult<Answer<bool>> {
        let decision = match place.target {
            PlaceTarget::Binding { symbol } => {
                self.decide_writable_binding(place.source, symbol)?
            }
            PlaceTarget::Member { owner, key } => self.decide_writable_member(owner, key)?,
            PlaceTarget::Index { .. } | PlaceTarget::Dereference => Answer::Ready(true),
        };

        Ok(decision)
    }

    /// Return whether one local binding can be assigned.
    fn decide_writable_binding(
        &self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<bool>> {
        if symbol.module_id != source.module_id {
            return Ok(Answer::Ready(false));
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

        Ok(Answer::from(is_writable))
    }

    /// Return whether one member target can be assigned.
    fn decide_writable_member(
        &mut self,
        owner: TypeOperand,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<bool>> {
        let Some(owner) = self.type_operand_term_id(owner)? else {
            return Ok(Answer::pending(owner.dependencies(self)));
        };

        // structural fields carry their write access directly
        if let TypeTerm::Shape(shape) = self.inference.term(owner) {
            let members = self.inference.term(*shape).members.iter().copied();
            for member in members {
                if let ShapeMember::Field {
                    key: member_key,
                    is_readonly,
                    ..
                } = member
                    && member_key.matches(&key)
                {
                    return Ok(Answer::from(!is_readonly));
                }
            }

            return Ok(Answer::Ready(true));
        }

        Ok(Answer::Ready(true))
    }
}

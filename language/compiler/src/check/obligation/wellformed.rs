use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CheckState, InvalidOperation, ObligationCheck, ObligationFailure, OperationReduction, Origin,
    Relation, WellFormedTypeObligation,
};

impl CheckState<'_> {
    /// Check that one written type operation is well-formed once solved.
    pub(in crate::check) fn check_well_formed_type(
        &mut self,
        origin: Origin,
        obligation: &WellFormedTypeObligation,
    ) -> CompilerResult<ObligationCheck> {
        // explicit placement must agree with an intrinsically placed nominal base
        if let Some(written) = self.type_place(obligation.ty)? {
            let written = self.reduce_type_head(origin, written)?;
            let value = self.strip_form(origin, obligation.ty)?;
            let value = self.reduce_type_head(origin, value)?;
            let symbol = match self.ty(value)? {
                dir::Type::Application(instance) => Some(instance.symbol),
                dir::Type::Reference(reference) => Some(reference.symbol),
                _ => None,
            };
            let written = self.place_space(written)?;
            if let (Some(symbol), Some(written)) = (symbol, written)
                && let Some(declared) = self.nominal_space(symbol)?
                && written != declared
            {
                let failure = ObligationFailure::ConflictingDeclarationPlacement {
                    source: obligation.source,
                    symbol,
                    written,
                    declared,
                };

                return Ok(ObligationCheck::fail(failure));
            }
        }

        // placed types must have a finite representation valid for their storage space
        let representation = self.check_representation(origin, obligation.ty)?;
        if let ObligationCheck::Fails(_) = representation {
            return Ok(representation);
        }

        // operation types validate through their reducer
        let Some(operation) = self.operation_head(obligation.ty)? else {
            return Ok(ObligationCheck::holds());
        };

        let index = match operation {
            dir::TypeOperation::Index(index) => index,
            _ => return Ok(ObligationCheck::holds()),
        };
        let reduction = self.reduce_index(origin, &index)?;
        let invalid = match reduction {
            // parameter receivers index as their bound would; other rigid
            //  keys must prove membership in the receiver's key set
            OperationReduction::Rigid => {
                let bound = match self.ty(index.left)? {
                    dir::Type::Parameter(parameter) => self
                        .generic_parameter(parameter)
                        .and_then(|binding| binding.constraint),
                    _ => None,
                };
                if let Some(bound) = bound {
                    let bounded = dir::IndexType {
                        left: bound,
                        index: index.index,
                    };
                    match self.reduce_index(origin, &bounded)? {
                        OperationReduction::Invalid(_) => {}
                        _ => return Ok(ObligationCheck::holds()),
                    }
                } else {
                    let keys =
                        self.intern_operation(dir::TypeOperation::KeyOf(dir::UnaryType {
                            target: index.left,
                        }))?;
                    let proven =
                        self.decide_relation(origin, Relation::Satisfies, index.index, keys)?;
                    if proven {
                        return Ok(ObligationCheck::holds());
                    }
                }

                InvalidOperation::IndexKey {
                    receiver: index.left,
                    key: index.index,
                }
            }
            OperationReduction::Invalid(invalid) => invalid,
            OperationReduction::Projected(_) => {
                return Ok(ObligationCheck::holds());
            }
        };

        let failure = match invalid {
            InvalidOperation::IndexReceiver { receiver } => {
                ObligationFailure::InvalidIndexReceiver {
                    source: obligation.source,
                    receiver,
                }
            }
            InvalidOperation::IndexKey { receiver, key } => ObligationFailure::InvalidIndexKey {
                source: obligation.source,
                receiver,
                key,
            },
        };

        Ok(ObligationCheck::fail(failure))
    }
}

use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{
    CheckState, InvalidOperation, ObligationCheck, ObligationFailure, OperationReduction, Origin,
    Relation, WellFormedTypeObligation,
};

impl CheckState<'_> {
    /// Check that one written type operation is well-formed once solved.
    pub(in crate::sema) fn check_well_formed_type(
        &mut self,
        origin: Origin,
        obligation: &WellFormedTypeObligation,
    ) -> CompilerResult<ObligationCheck> {
        // explicit placement must agree with an intrinsically placed nominal base
        if let Some(written) = self.type_place(obligation.ty)? {
            let value = self.strip_form(origin, obligation.ty)?;
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
            // index parameter receivers as their bounds would, other rigid keys prove membership
            OperationReduction::Rigid => {
                let bounds = match self.ty(index.left)? {
                    dir::Type::Parameter(parameter) => self.parameter_bounds(origin, parameter)?,
                    _ => SmallVec::new(),
                };
                if !bounds.is_empty() {
                    for bound in bounds {
                        let bounded = dir::IndexType {
                            left: bound,
                            index: index.index,
                        };
                        match self.reduce_index(origin, &bounded)? {
                            OperationReduction::Invalid(_) => {}
                            _ => return Ok(ObligationCheck::holds()),
                        }
                    }
                } else {
                    let keys =
                        self.intern_operation(dir::TypeOperation::KeyOf(dir::UnaryType {
                            target: index.left,
                        }))?;
                    let proven = self
                        .evaluate_relation(origin, Relation::Satisfies, index.index, keys)?
                        .holds();
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

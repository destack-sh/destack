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
        // require nested written placements to agree with one another
        if let Some((written, nested)) = self.conflicting_places(origin, obligation.ty)? {
            let failure = ObligationFailure::ConflictingPlacement {
                source: obligation.source,
                written,
                declared: nested,
                declaration: None,
            };

            return Ok(ObligationCheck::fail(failure));
        }

        // require explicit placement to agree with an intrinsically placed nominal base
        if let Some(written) = self.type_place(obligation.ty)? {
            let value = self.strip_form(origin, obligation.ty)?;
            let symbol = match self.ty(value)? {
                dir::Type::Application(instance) => Some(instance.symbol),
                _ => None,
            };
            let written = self.place_space(written)?;
            if let (Some(symbol), Some(written)) = (symbol, written)
                && let Some(declared) = self.nominal_space(symbol)?
                && written != declared
            {
                let failure = ObligationFailure::ConflictingPlacement {
                    source: obligation.source,
                    written,
                    declared,
                    declaration: Some(symbol),
                };

                return Ok(ObligationCheck::fail(failure));
            }
        }

        // require placed types to have a finite representation for their storage space
        let representation = self.check_representation(origin, obligation.ty)?;
        if let ObligationCheck::Fails(_) = representation {
            return Ok(representation);
        }

        // validate operation types through their reducer
        let Some(operation) = self.operation_head(obligation.ty)? else {
            return Ok(ObligationCheck::holds());
        };

        // only index operations validate further
        let index = match operation {
            dir::TypeOperation::Index(index) => index,
            _ => return Ok(ObligationCheck::holds()),
        };

        // reduce the index operation and translate what it refuses
        let reduction = self.reduce_index(origin, &index)?;
        let invalid = match reduction {
            // index a rigid receiver the way its bounds or its key set would
            OperationReduction::Rigid => {
                let bounds = match self.ty(index.left)? {
                    dir::Type::Parameter(parameter) => self.parameter_bounds(origin, parameter)?,
                    _ => SmallVec::new(),
                };

                // accept a parameter receiver when any bound indexes the key
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
                }
                // otherwise require the key to satisfy the receiver's key set
                else {
                    let keys =
                        self.intern_operation(dir::TypeOperation::KeyOf(dir::UnaryType {
                            target: index.left,
                        }))?;
                    let is_known_key = self
                        .decide_relation(origin, Relation::Subtype, index.index, keys)?
                        .holds();
                    if is_known_key {
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

        // translate the invalid operation into its obligation failure
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

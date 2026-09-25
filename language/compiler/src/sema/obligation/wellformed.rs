use smallvec::SmallVec;
use tspp_dir as dir;

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
        // require a finite representation for the type's storage space
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

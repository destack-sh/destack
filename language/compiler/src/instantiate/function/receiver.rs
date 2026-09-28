use tspp_mir as mir;

use crate::instantiate::function::Specialization;
use crate::{CompilerError, CompilerResult};

impl Specialization<'_, '_> {
    /// Borrow a witness call's receiver at the form its implementation takes.
    pub(super) fn witness_receiver(
        &mut self,
        function: mir::FunctionId,
        arguments: mir::ValueSlice,
    ) -> CompilerResult<Option<mir::ValueSlice>> {
        let values = self.state.tree.get_values(arguments).to_vec();
        let (Some(receiver), Some(parameter)) = (
            values.first().copied(),
            self.state
                .tree
                .get(function)
                .parameters
                .first()
                .map(|parameter| parameter.ty),
        ) else {
            return Ok(None);
        };

        // keep a matching receiver
        let passed = self.value_type(receiver)?;
        if mir::erase_lifetimes(&self.state.tree, passed)
            == mir::erase_lifetimes(&self.state.tree, parameter)
        {
            return Ok(None);
        }

        // borrow the receiver place
        let mir::Type::Reference { pointee, .. } =
            self.state.tree.type_definition(parameter).clone()
        else {
            return Err(self.unreachable_receiver(passed, parameter));
        };
        let place = match self.state.tree.type_definition(passed).clone() {
            mir::Type::Reference {
                pointee: referent, ..
            } if referent == pointee => {
                mir::Place::value(receiver).with_projection(mir::Projection::Deref)
            }
            _ if passed == pointee => {
                let local = self.state.tree.insert(mir::Local {
                    ty: passed,
                    mutability: mir::Mutability::Mutable,
                });
                self.added_locals.push(local);
                self.prefix.push(mir::Instruction::Store {
                    place: mir::Place::local(local),
                    value: receiver,
                });

                mir::Place::local(local)
            }
            _ => return Err(self.unreachable_receiver(passed, parameter)),
        };
        let result_type = mir::erase_lifetimes(&self.state.tree, parameter);
        let borrowed = self.add_value(result_type);
        self.prefix.push(mir::Instruction::Address {
            destination: borrowed,
            place,
            result_type,
        });

        let mut values = values;
        values[0] = borrowed;

        Ok(Some(self.state.tree.add_values(&values)))
    }

    /// Build the error for a receiver the implementation cannot take.
    fn unreachable_receiver(&self, passed: mir::TypeId, parameter: mir::TypeId) -> CompilerError {
        CompilerError::Internal {
            message: format!(
                "a witness receiver {passed:?} its implementation's receiver {parameter:?} cannot take"
            ),
        }
    }
}

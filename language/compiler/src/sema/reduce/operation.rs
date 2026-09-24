use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, OperationReduction, Origin, TryProjection};

impl CheckState<'_> {
    /// Return one type operation's reduced value type.
    pub(in crate::sema) fn reduce_operation_type(
        &mut self,
        origin: Origin,
        operation: dir::TypeOperation,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.intern_operation(operation)?;
        let ty = self.normalize(origin, ty)?;

        Ok(ty)
    }

    /// Reduce one type operation when its inputs allow.
    pub(super) fn reduce_operation(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        operation: &dir::TypeOperation,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match operation {
            // conditionals select by the extends relation
            dir::TypeOperation::Conditional(conditional) => {
                self.reduce_conditional(origin, *conditional)
            }

            // guard narrowings filter runtime-tested sources
            dir::TypeOperation::Narrow(narrow) => self.reduce_narrowing(origin, *narrow),

            // string mappings transform string literals and distribute over unions
            dir::TypeOperation::StringMapping { mapping, target } => {
                self.reduce_string_mapping_operation(origin, id, *mapping, *target)
            }

            // indexed access projects element or member types
            dir::TypeOperation::Index(index) => {
                match self.reduce_index(origin, index)? {
                    OperationReduction::Projected(ty) => Ok(Some(ty)),
                    OperationReduction::Rigid => Ok(None),
                    // ill-formed accesses poison consumers; well-formedness reports
                    OperationReduction::Invalid(_) => {
                        let error = self.intern_type(dir::Type::Error)?;

                        Ok(Some(error))
                    }
                }
            }

            // typeof lifts one stable value-reference type
            dir::TypeOperation::TypeOf(query) => self.reduce_typeof(query.symbol),

            // apply arguments after a value type becomes available
            dir::TypeOperation::Instantiation(application) => {
                let arguments = self.type_ids(id.module_id, application.arguments)?.to_vec();
                let instantiated = self.instantiate_type(origin, application.target, &arguments)?;

                Ok(instantiated.map(|(ty, _)| ty))
            }

            // keyof projects the key union of one closed type
            dir::TypeOperation::KeyOf(unary) => self.reduce_keyof(origin, id, unary.target),

            // erase inference barriers only in the signature that infers
            dir::TypeOperation::NoInfer(_) => Ok(None),

            // awaited types unwrap promise representations recursively
            dir::TypeOperation::Awaited(unary) => self.reduce_awaited(origin, unary.target),

            // spaces settle by the declaration of the value stored
            dir::TypeOperation::SpaceOf(unary) => self.reduce_space_of(origin, unary.target),

            // try projections split nullish values from representations
            dir::TypeOperation::TryOutput { value } => {
                self.reduce_try_projection(origin, *value, TryProjection::Output)
            }
            dir::TypeOperation::TryResidual { value } => {
                self.reduce_try_projection(origin, *value, TryProjection::Residual)
            }
            dir::TypeOperation::TryFailure { value } => {
                self.reduce_try_projection(origin, *value, TryProjection::Failure)
            }

            // static operations evaluate over literal operands
            dir::TypeOperation::StaticBinary(binary) => {
                self.reduce_static_binary_operation(origin, *binary)
            }
            dir::TypeOperation::StaticUnary(unary) => {
                self.reduce_static_unary_operation(origin, *unary)
            }

            // template literals concatenate once every span closes
            dir::TypeOperation::TemplateLiteral(template) => {
                self.reduce_template_literal(origin, id, template)
            }

            // mapped types project their closed key sources field by field
            dir::TypeOperation::Mapped(mapped) => self.reduce_mapped(origin, mapped),

            // bare binders stay symbolic until applied
            dir::TypeOperation::Infer(_) => Ok(None),
        }
    }
}

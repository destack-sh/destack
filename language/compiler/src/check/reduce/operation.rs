use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

impl CheckState<'_> {
    /// Return one type operation's reduced value type.
    pub(in crate::check) fn reduce_operation_type(
        &mut self,
        origin: Origin,
        operation: dir::TypeOperation,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let ty = self.intern_type(origin.module(), dir::Type::Operation(operation))?;
        let ty = answer!(self.reduce_type_head(origin, ty)?);

        Ok(Answer::Ready(ty))
    }

    /// Reduce one type operation when its inputs allow.
    /// Returns ready none when the operation must stay symbolic.
    pub(super) fn reduce_operation(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        operation: &dir::TypeOperation,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        match operation {
            // conditionals select by the extends relation
            dir::TypeOperation::Conditional(conditional) => {
                self.reduce_conditional(origin, *conditional)
            }

            // guard narrowings filter runtime-tested sources
            dir::TypeOperation::Narrow(narrow) => self.reduce_narrow(origin, *narrow),

            // string mappings transform string literals and distribute over unions
            dir::TypeOperation::StringMapping { mapping, target } => {
                self.reduce_string_mapping_operation(origin, id, *mapping, *target)
            }

            // indexed access projects element or member types
            dir::TypeOperation::Index(index) => self.reduce_index(origin, index),

            // typeof lifts one stable value-reference type
            dir::TypeOperation::TypeOf(query) => self.reduce_typeof(origin, query.value),

            // keyof projects the key union of one closed type
            dir::TypeOperation::KeyOf(unary) => self.reduce_keyof(origin, id, unary.target),

            // inference barriers erase only in the signature that owns inference
            dir::TypeOperation::NoInfer(_) => Ok(Answer::Ready(None)),

            // awaited types unwrap promise carriers recursively
            dir::TypeOperation::Awaited(unary) => self.reduce_awaited(origin, unary.target),

            // try projections split nullish values from carriers
            dir::TypeOperation::TryOutput { value } => {
                self.reduce_try_projection(origin, *value, false)
            }
            dir::TypeOperation::TryResidual { value } => {
                self.reduce_try_projection(origin, *value, true)
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
                self.reduce_template_literal(origin, template)
            }

            // mapped types project their closed key sources field by field
            dir::TypeOperation::Mapped(mapped) => self.reduce_mapped(origin, mapped),

            // bare binders stay symbolic until applied
            dir::TypeOperation::Infer(_) => Ok(Answer::Ready(None)),
        }
    }
}

use destack_base::StringId;
use destack_dir::{Expression, LocalNodeId};
use destack_mir as mir;

use crate::{LowerError, LowerResult};

use super::super::BlockLowerer;

impl BlockLowerer<'_, '_> {
    /// Lower a member access expression to a field_get.
    pub(crate) fn lower_member_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        field_name: StringId,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the aggregate value
        let (aggregate_value, aggregate_type) = self.lower_value_expression(left_id)?;

        // resolve field index through the type lowerer
        let field_index = self
            .type_lowerer
            .field_index_for_type(
                aggregate_type,
                field_name,
                self.strings,
                self.builder.tree(),
            )
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "field not found in aggregate type".to_string(),
            })?;

        // get the result type
        let result_type =
            self.mir_type_for_expression(expression_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                })?;

        // emit field_get
        let value = self.builder.field_get(aggregate_value, field_index as u32);
        Ok((value, result_type))
    }

    /// Lower an index expression to an element_get.
    pub(crate) fn lower_index_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        index_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the array value and index
        let (array_value, _array_type) = self.lower_value_expression(left_id)?;
        let (index_value, _index_type) = self.lower_value_expression(index_id)?;

        // get the result type (element type)
        let result_type =
            self.mir_type_for_expression(expression_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                })?;

        // emit element_get
        let value = self.builder.element_get(array_value, index_value);
        Ok((value, result_type))
    }
}

use destack_dir::{
    AnchoredGlobalNodeId, EnumBackingType, EnumFieldValue, Expression, GlobalSymbolId, LocalNodeId,
};
use destack_mir as mir;

use crate::{LowerError, LowerResult};

use crate::lower::emit::FunctionContext;
use crate::lower::r#type::{EnumFieldValueDescriptor, enum_field_value_for_symbol};

impl FunctionContext<'_> {
    /// Lower enum member access into a constant value when possible.
    pub(crate) fn lower_enum_field_member(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        member_symbol: GlobalSymbolId,
    ) -> LowerResult<Option<(mir::Value, mir::LocalNodeId<mir::Type>)>> {
        // anchor diagnostics to the current expression
        let node = expression_id
            .into_global_any(self.env.module_id)
            .into_anchored(Some(self.env.profile));

        // resolve the enum field value when this symbol is a field
        let Some(EnumFieldValueDescriptor { backing, value }) =
            enum_field_value_for_symbol(self.env.program, self.env.profile, member_symbol, node)?
        else {
            return Ok(None);
        };

        // resolve the expression result type
        let result_type = self.lower_type_for_expression(expression_id)?;

        // build the literal value for the backing type
        let value = match (backing, value) {
            (EnumBackingType::Int(_), EnumFieldValue::Int(value)) => {
                self.enum_int_constant(value, backing, node)?
            }
            (EnumBackingType::String, EnumFieldValue::String(value)) => {
                let literal = self.env.strings.get(value);
                self.state.builder.sconst(literal.to_string())
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node,
                    message: "enum field value does not match backing type".to_string(),
                });
            }
        };

        Ok(Some((value, result_type)))
    }

    /// Build an integer constant for an enum backing type.
    fn enum_int_constant(
        &mut self,
        value: i64,
        backing: EnumBackingType,
        node: AnchoredGlobalNodeId,
    ) -> LowerResult<mir::Value> {
        // ensure the enum backing type is integer
        let scalar = self
            .env
            .type_lowerer
            .scalar_type_for_enum_backing(backing)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node,
                message: "enum backing type is not an integer".to_string(),
            })?;

        // resolve the width and signedness
        let (width, signed) = match scalar {
            crate::ScalarType::SignedInt { width } => (width, true),
            crate::ScalarType::UnsignedInt { width } => (width, false),
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node,
                    message: "enum backing type is not an integer".to_string(),
                });
            }
        };
        let width = u8::try_from(width).map_err(|_| LowerError::UnsupportedConstruct {
            node,
            message: "enum backing type width is too large".to_string(),
        })?;

        Ok(self.state.builder.iconst(value, width, signed))
    }
}

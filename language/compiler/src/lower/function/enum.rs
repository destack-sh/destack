use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerError, CompilerResult, LowerError, ScalarType};

use crate::lower::FunctionLowerer;
use crate::lower::r#type::{EnumFieldValueDescriptor, enum_field_value_for_symbol};

impl FunctionLowerer<'_> {
    /// Lower enum member access into a constant value when possible.
    ///
    /// ```ds
    /// enum Status: int32 {
    ///     Active = 1;
    /// }
    ///
    /// function read(): int32 {
    ///     return Status.Active;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v0: int32 = const 1
    /// ```
    pub(crate) fn lower_enum_field_member(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        member_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<(mir::Value, mir::LocalNodeId<mir::Type>)>> {
        // anchor diagnostics to the current expression
        let node = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));
        let anchor = self.diagnostic_anchor(node);

        // resolve the enum field value when this symbol is a field
        let Some(EnumFieldValueDescriptor { backing, value }) = enum_field_value_for_symbol(
            self.context.compiler,
            self.context.provider,
            self.context.profile,
            member_symbol,
            anchor,
        )?
        else {
            return Ok(None);
        };

        // resolve the expression result type
        let result_type = self.lower_type_for_expression(expression_id)?;

        // build the literal value for the backing type
        let value = match (backing, value) {
            (dir::EnumBackingType::Integer(_), dir::EnumFieldValue::Int(value)) => {
                self.enum_int_constant(value, backing, node)?
            }
            (dir::EnumBackingType::String, dir::EnumFieldValue::String(value)) => {
                let (value, _) = self.string_literal_value_for_id(value, Some(node))?;
                value
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "enum field value does not match backing type".to_string(),
                }
                .into());
            }
        };

        // wrap the backing payload into the nominal enum type when needed
        let value = if matches!(
            self.state.builder.tree().get(result_type),
            mir::Type::Newtype { .. }
        ) {
            self.state.builder.bitcast(value, result_type)
        } else {
            value
        };

        Ok(Some((value, result_type)))
    }

    /// Build an integer constant for an enum backing type.
    fn enum_int_constant(
        &mut self,
        value: i64,
        backing: dir::EnumBackingType,
        node: dir::AnchoredGlobalNodeId,
    ) -> CompilerResult<mir::Value> {
        // ensure the enum backing type is integer
        let scalar = self
            .context
            .type_lowerer
            .scalar_type_for_enum_backing(backing)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
                message: "enum backing type is not an integer".to_string(),
            })
            .map_err(CompilerError::from)?;

        // resolve the width and signedness
        let (width, signed) = match scalar {
            ScalarType::SignedInt { width } => (width, true),
            ScalarType::UnsignedInt { width } => (width, false),
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "enum backing type is not an integer".to_string(),
                }
                .into());
            }
        };
        Ok(self.state.builder.iconst(i128::from(value), width, signed))
    }
}

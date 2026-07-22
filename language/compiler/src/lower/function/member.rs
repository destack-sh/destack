use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one member read through its checked resolution.
    pub(in crate::lower) fn lower_member(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // construct the case value for enum member reads
        if let dir::Type::EnumMember(member) = self.node_type(expression)? {
            return self.lower_enum_member(&member);
        }

        let index = self.member_field_index(expression)?;
        let receiver = self.coerced_type_id(left)?;
        let result_type = self.lower_type(self.node_type_id(expression)?)?;
        let value = self.lower_expression(left)?;

        // load fields through addresses for reference receivers
        if let Some(layer) = self.lowerer.peel_reference(receiver)? {
            let address = self.emit_field_address(value, index, result_type, layer.access);

            return Ok(self.builder.load(address, result_type));
        }

        Ok(self.builder.field_get(value, index))
    }

    /// Lower one enum member read to its case construction.
    fn lower_enum_member(&mut self, member: &dir::EnumMemberType) -> CompilerResult<mir::Value> {
        // materialize the owner carrier and select the declared case
        let ty = self.lower_type(member.owner)?;
        let case = self.lowerer.enum_case_index(member)?;

        Ok(self.builder.variant_new(ty, case, None))
    }

    /// Return the declaration field index behind one member expression.
    pub(in crate::lower) fn member_field_index(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<u32> {
        let resolution = self.member_resolution(expression)?;

        // read tuple elements by position
        if let dir::MemberTarget::Element(index) = resolution.target {
            return Ok(index as u32);
        }

        // read struct fields by declaration index behind any form
        let receiver = resolution.receiver;
        let stored = match self.lowerer.peel_reference(receiver)? {
            Some(layer) => layer.stored,
            None => self.lowerer.peel_owned(receiver)?,
        };
        let dir::Type::Application(ref instance) = self.lowerer.ty(stored)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a member read on a structural receiver".to_string(),
            }
            .into());
        };
        let fields = self.lowerer.nominal_fields(instance.symbol)?;
        let index = match &resolution.target {
            // point.x through the field key
            dir::MemberTarget::Field(key) => fields.iter().position(|field| field.key == *key),
            // point.x through the field declaration
            dir::MemberTarget::Symbol(candidate) => {
                if !candidate.adjustments.is_empty() {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "an adjusted member read".to_string(),
                    }
                    .into());
                }
                let symbol = candidate.symbol.local_id;

                fields.iter().position(|field| field.symbol == symbol)
            }
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!("a member read through {other:?}"),
                }
                .into());
            }
        };

        index
            .map(|index| index as u32)
            .ok_or_else(|| CompilerError::Internal {
                message: "checked DIR selected a member missing from its nominal".to_string(),
            })
    }
}

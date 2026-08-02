use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one scalar literal node to a constant at its carrier.
    pub(in crate::lower) fn lower_scalar_literal(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        literal: dir::ScalarLiteral,
    ) -> CompilerResult<mir::Value> {
        let carrier = self.literal_carrier(expression, literal)?;

        self.lower_constant(literal, carrier)
    }

    /// Lower one literal to a constant of one concrete carrier type.
    pub(in crate::lower) fn lower_constant(
        &mut self,
        literal: dir::ScalarLiteral,
        carrier: mir::Type,
    ) -> CompilerResult<mir::Value> {
        match (literal, carrier) {
            // pick the single boolean carrier
            (dir::ScalarLiteral::Boolean(value), _) => Ok(self.builder.bconst(value)),

            // materialize integers at their selected width and sign
            (dir::ScalarLiteral::Integer(value), mir::Type::Int { width, is_signed }) => {
                Ok(self.builder.iconst(value as i128, width, is_signed))
            }
            // materialize integers directly in float contexts
            (dir::ScalarLiteral::Integer(value), mir::Type::Float(float)) => {
                Ok(self.builder.fconst(value as f64, float))
            }

            // materialize floats at their selected format
            (dir::ScalarLiteral::Float(value), mir::Type::Float(float)) => {
                Ok(self.builder.fconst(value, float))
            }

            // reject string literals, which carry no scalar constant
            (dir::ScalarLiteral::String(_), _) => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a string literal constant".to_string(),
            }
            .into()),

            (literal, carrier) => Err(CompilerError::Internal {
                message: format!("carrier {carrier:?} for literal {literal:?}"),
            }),
        }
    }

    /// Return the concrete type carrying one literal node's value.
    fn literal_carrier(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        literal: dir::ScalarLiteral,
    ) -> CompilerResult<mir::Type> {
        // use the node's own type when concretely typed
        let ty = self.node_type_id(expression)?;
        if !matches!(self.lowerer.ty(ty)?, dir::Type::Literal(_)) {
            let carrier = self.lower_type(ty)?;

            return Ok(self.builder.tree().get(carrier).clone());
        }

        match literal {
            // pick the single boolean carrier
            dir::ScalarLiteral::Boolean(_) => Ok(mir::Type::Boolean),

            // require numeric literals to enter through a concrete value target
            dir::ScalarLiteral::Integer(_) | dir::ScalarLiteral::Float(_) => {
                Err(CompilerError::Internal {
                    message: format!(
                        "numeric literal expression {} reached lowering without a concrete target",
                        expression.id
                    ),
                })
            }

            // reject literal domains without scalar carriers
            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("{} literals", other.variant_name()),
            }
            .into()),
        }
    }
}

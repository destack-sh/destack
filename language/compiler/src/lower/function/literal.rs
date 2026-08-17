use destack_core::StringId;
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
        // a string or bigint at its singleton type is const: zero sized,
        //  its content lives in the type and widening materializes it
        let is_const = matches!(self.node_type(expression)?, dir::Type::Literal(_));
        match literal {
            dir::ScalarLiteral::String(string) if !is_const => {
                return self.lower_string_literal(string);
            }
            dir::ScalarLiteral::Bigint(bigint) if !is_const => {
                return self.lower_bigint_literal(bigint);
            }
            dir::ScalarLiteral::String(_) | dir::ScalarLiteral::Bigint(_) => {
                let void = self.builder.tree_mut().intern_type(mir::Type::Void);

                return Ok(self.builder.constant(mir::Constant::Undefined, void));
            }
            _ => {}
        }

        // materialize the literal at the carrier its node commits to
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
            // materialize integers at the pointer-sized carriers
            (dir::ScalarLiteral::Integer(value), mir::Type::Usize) => {
                Ok(self.builder.usize_const(value as u128))
            }
            (dir::ScalarLiteral::Integer(value), mir::Type::Isize) => {
                Ok(self.builder.isize_const(value as i128))
            }

            // materialize floats at their selected format
            (dir::ScalarLiteral::Float(value), mir::Type::Float(float)) => {
                Ok(self.builder.fconst(value, float))
            }

            // read string and bigint literals from their declared immortal objects
            (dir::ScalarLiteral::String(string), _) => self.lower_string_literal(string),
            (dir::ScalarLiteral::Bigint(bigint), _) => self.lower_bigint_literal(bigint),

            // materialize undefined as the void unit in void positions
            (dir::ScalarLiteral::Undefined, carrier @ mir::Type::Void) => {
                let ty = self.builder.tree_mut().intern_type(carrier);

                Ok(self.builder.constant(mir::Constant::Undefined, ty))
            }

            (literal, carrier) => Err(CompilerError::Internal {
                message: format!("carrier {carrier:?} for literal {literal:?}"),
            }),
        }
    }

    /// Lower one string literal to its immortal String object reference.
    fn lower_string_literal(&mut self, string: StringId) -> CompilerResult<mir::Value> {
        // read the declared immortal object, cascading its declare diagnostic
        match self.lowerer.string_literals.get(&string) {
            Some(Ok((global, value))) => {
                let global = *global;
                let value = *value;

                Ok(self.builder.global_addr(global, value))
            }
            Some(Err(diagnostic)) => Err(CompilerError::Diagnostic(Box::new(diagnostic.clone()))),
            None => Err(CompilerError::Internal {
                message: format!("string literal {} reached lowering undeclared", string.0),
            }),
        }
    }

    /// Lower one bigint literal to its immortal BigInt object reference.
    fn lower_bigint_literal(&mut self, bigint: i64) -> CompilerResult<mir::Value> {
        // read the declared immortal object, cascading its declare diagnostic
        match self.lowerer.bigint_literals.get(&bigint) {
            Some(Ok((global, value))) => {
                let global = *global;
                let value = *value;

                Ok(self.builder.global_addr(global, value))
            }
            Some(Err(diagnostic)) => Err(CompilerError::Diagnostic(Box::new(diagnostic.clone()))),
            None => Err(CompilerError::Internal {
                message: format!("bigint literal {bigint} reached lowering undeclared"),
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

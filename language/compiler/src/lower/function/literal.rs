use destack_core::StringId;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one scalar literal node to a constant at its representation.
    pub(in crate::lower) fn lower_scalar_literal(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        literal: dir::Literal,
    ) -> CompilerResult<mir::Value> {
        // a string or bigint at its singleton type stays zero sized: the content lives
        //  in the type and widening materializes it
        let is_const = matches!(self.node_type(expression)?, dir::Type::Literal(_));

        // settle the object-backed literals before the scalar representations
        match literal {
            // materialize a widened string through its constant object
            dir::Literal::String(string) if !is_const => {
                return self.lower_string_literal(string);
            }
            // materialize a widened bigint through its constant object
            dir::Literal::Bigint(bigint) if !is_const => {
                return self.lower_bigint_literal(bigint);
            }
            // leave a singleton string or bigint void
            dir::Literal::String(_) | dir::Literal::Bigint(_) => {
                let void = self.builder.tree_mut().intern_type(mir::Type::Void);

                return Ok(self.builder.constant(mir::Constant::Undefined, void));
            }
            // carry every scalar literal on through
            _ => {}
        }

        // materialize the literal at the representation its node commits to
        let representation = self.literal_representation(expression, literal)?;

        self.lower_constant(literal, representation)
    }

    /// Lower one literal to a constant of one concrete representation type.
    pub(in crate::lower) fn lower_constant(
        &mut self,
        literal: dir::Literal,
        representation: mir::Type,
    ) -> CompilerResult<mir::Value> {
        match (literal, representation) {
            // pick the single boolean representation
            (dir::Literal::Boolean(value), _) => Ok(self.builder.bconst(value)),

            // materialize integers at their selected width and sign
            (dir::Literal::Integer(value), mir::Type::Int { width, is_signed }) => {
                Ok(self.builder.iconst(value as i128, width, is_signed))
            }
            // materialize integers directly in float contexts
            (dir::Literal::Integer(value), mir::Type::Float(float)) => {
                Ok(self.builder.fconst(value as f64, float))
            }
            // materialize integers at the pointer-sized representations
            (dir::Literal::Integer(value), mir::Type::Usize) => {
                Ok(self.builder.usize_const(value as u128))
            }
            (dir::Literal::Integer(value), mir::Type::Isize) => {
                Ok(self.builder.isize_const(value as i128))
            }

            // materialize floats at their selected format
            (dir::Literal::Float(value), mir::Type::Float(float)) => {
                Ok(self.builder.fconst(value, float))
            }

            // read string and bigint literals from their declared constant objects
            (dir::Literal::String(string), _) => self.lower_string_literal(string),
            (dir::Literal::Bigint(bigint), _) => self.lower_bigint_literal(bigint),

            // materialize undefined as the void unit in void positions
            (dir::Literal::Undefined, representation @ mir::Type::Void) => {
                let ty = self.builder.tree_mut().intern_type(representation);

                Ok(self.builder.constant(mir::Constant::Undefined, ty))
            }

            // reject every literal reaching a representation that cannot hold it
            (literal, representation) => Err(CompilerError::Internal {
                message: format!("representation {representation:?} for literal {literal:?}"),
            }),
        }
    }

    /// Lower one string literal to its constant String object reference.
    fn lower_string_literal(&mut self, string: StringId) -> CompilerResult<mir::Value> {
        // read the declared constant object, cascading its declare diagnostic
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

    /// Lower one bigint literal to its constant BigInt object reference.
    fn lower_bigint_literal(&mut self, bigint: i64) -> CompilerResult<mir::Value> {
        // read the declared constant object, cascading its declare diagnostic
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
    fn literal_representation(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        literal: dir::Literal,
    ) -> CompilerResult<mir::Type> {
        // use the node's own type when concretely typed
        let ty = self.node_type_id(expression)?;
        if !matches!(self.lowerer.ty(ty)?, dir::Type::Literal(_)) {
            let representation = self.lower_type(ty)?;

            return Ok(self.builder.tree().get(representation).clone());
        }

        match literal {
            // pick the single boolean representation
            dir::Literal::Boolean(_) => Ok(mir::Type::Boolean),

            // require numeric literals to enter through a concrete value target
            dir::Literal::Integer(_) | dir::Literal::Float(_) => Err(CompilerError::Internal {
                message: format!(
                    "numeric literal expression {} reached lowering without a concrete target",
                    expression.id
                ),
            }),

            // reject literal domains without scalar representations
            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("{} literals", other.variant_name()),
            }
            .into()),
        }
    }
}

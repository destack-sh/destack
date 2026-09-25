use destack_core::StringId;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::operand::Operand;
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one scalar literal node to a constant at its representation.
    pub(in crate::lower) fn lower_scalar_literal(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        literal: dir::Literal,
    ) -> CompilerResult<mir::Value> {
        // a node typed by its literal is the value of that type
        let ty = self.node_type_id(expression)?;
        if let dir::Type::Literal(_) = self.lower.ty(ty)? {
            let singleton = self.lower_type(ty)?;

            return Ok(self.builder.constant(mir::Constant::Zeroed, singleton));
        }

        // materialize the literal at the representation its node commits to
        let representation = self.lower_type(ty)?;

        self.lower_constant(literal, representation)
    }

    /// Lower one literal to a constant at one representation.
    pub(in crate::lower) fn lower_constant(
        &mut self,
        literal: dir::Literal,
        representation: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        if self.is_singleton_representation(representation) {
            return Ok(self.builder.constant(mir::Constant::Zeroed, representation));
        }

        // a singleton literal at a variant lives in the case holding its type
        let stored = self.builder.tree_mut().storage_type(representation);
        if let Some(singleton) = self.singleton_representation(&literal)
            && let mir::Type::Variant { .. } = self.builder.tree().type_definition(stored)
        {
            let case = self.variant_case(stored, singleton)?;

            return Ok(self.builder.variant_new(representation, case, None));
        }

        // lower the literal at its representation
        match (
            literal,
            self.builder.tree().type_definition(representation).clone(),
        ) {
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

            // materialize characters as their unicode scalar values
            (dir::Literal::Character(value), mir::Type::Int { width, is_signed }) => Ok(self
                .builder
                .iconst(i128::from(value as u32), width, is_signed)),

            // leave a nullish literal at a parameter representation to its instance
            (dir::Literal::Null, mir::Type::Parameter { .. }) => {
                Ok(self.builder.constant(mir::Constant::Null, representation))
            }
            (dir::Literal::Undefined, mir::Type::Parameter { .. }) => Ok(self
                .builder
                .constant(mir::Constant::Undefined, representation)),

            // read string and bigint literals from their declared constant objects
            (dir::Literal::String(string), _) => self.lower_string_literal(string),
            (dir::Literal::Bigint(bigint), _) => self.lower_bigint_literal(bigint),

            // reject every literal outside its representation
            (literal, _) => Err(CompilerError::Internal {
                message: format!(
                    "a '{}' literal outside its representation",
                    literal.variant_name()
                ),
            }),
        }
    }

    /// Lower one string literal to its constant String object reference.
    pub(in crate::lower) fn lower_string_literal(
        &mut self,
        string: StringId,
    ) -> CompilerResult<mir::Value> {
        // declare the constant object on its first use
        if !self.lower.string_literals.contains_key(&string) {
            self.lower
                .declare_string_literals(self.builder.tree_mut(), [string])?;
        }

        // read the declared constant object, cascading its declare diagnostic
        match self.lower.string_literals.get(&string) {
            Some(Ok((global, value))) => {
                let global = *global;
                let value = *value;

                Ok(self.builder.address(mir::Place::global(global), value))
            }
            Some(Err(diagnostic)) => Err(CompilerError::Diagnostic(Box::new(diagnostic.clone()))),
            None => Err(CompilerError::Internal {
                message: format!("an undeclared string literal {}", string.0),
            }),
        }
    }

    /// Lower one bigint literal to its constant BigInt object reference.
    fn lower_bigint_literal(&mut self, bigint: i64) -> CompilerResult<mir::Value> {
        // declare the constant object on its first use
        if !self.lower.bigint_literals.contains_key(&bigint) {
            self.lower
                .declare_bigint_literals(self.builder.tree_mut(), [bigint])?;
        }

        // read the declared constant object, cascading its declare diagnostic
        match self.lower.bigint_literals.get(&bigint) {
            Some(Ok((global, value))) => {
                let global = *global;
                let value = *value;

                Ok(self.builder.address(mir::Place::global(global), value))
            }
            Some(Err(diagnostic)) => Err(CompilerError::Diagnostic(Box::new(diagnostic.clone()))),
            None => Err(CompilerError::Internal {
                message: format!("an undeclared bigint literal {bigint}"),
            }),
        }
    }

    /// Return the representation of one singleton literal, absent for a scalar literal.
    fn singleton_representation(&mut self, literal: &dir::Literal) -> Option<mir::TypeId> {
        match literal {
            dir::Literal::Undefined => Some(self.builder.tree_mut().intern_type(mir::Type::Void)),
            dir::Literal::Null => Some(self.lower.singleton_type(self.builder.tree_mut(), literal)),
            _ => None,
        }
    }

    /// Return whether one representation holds a single value and no bytes, through newtypes.
    pub(in crate::lower) fn is_singleton_representation(
        &mut self,
        representation: mir::TypeId,
    ) -> bool {
        let tree = self.builder.tree_mut();
        let mut representation = mir::Substitution::resolve(representation, tree);
        loop {
            match tree.get(representation) {
                mir::Type::Void | mir::Type::Null => return true,
                mir::Type::Struct { fields, .. } => return fields.is_empty(),
                mir::Type::Newtype { value, .. } => {
                    representation = mir::Substitution::resolve(*value, tree)
                }
                _ => return false,
            }
        }
    }

    /// Lower one interpolated template through the calls it renders and joins with.
    pub(in crate::lower) fn lower_template_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        chunks: &[dir::TemplateChunk],
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<mir::Value> {
        // read the calls this template renders and joins through
        let node = expression.into_global_any(self.source);
        let Some(decision) = self
            .lower
            .state(self.source)?
            .decisions
            .template_decision(node)
            .cloned()
        else {
            return Err(CompilerError::Internal {
                message: "an interpolated template without its recorded calls".to_string(),
            });
        };

        // render each interpolation in source order
        if arguments.len() != decision.spans.len() {
            return Err(CompilerError::Internal {
                message: "an interpolated template recording a call for every span".to_string(),
            });
        }
        let mut spans = Vec::with_capacity(arguments.len());
        for (argument, call) in arguments.iter().zip(&decision.spans) {
            let value = self.lower_argument(argument.into_global_any(self.source))?;
            let Some(rendered) = self.lower_target_call(Operand::Value(value), call)? else {
                return Err(CompilerError::Internal {
                    message: "a template span producing no text".to_string(),
                });
            };
            spans.push(rendered);
        }

        // materialize the literal chunks the template writes between its spans
        let mut texts = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            let Some(text) = chunk.cooked else {
                return Err(CompilerError::Internal {
                    message: "an interpolated template chunk without its text".to_string(),
                });
            };
            texts.push(self.lower_string_literal(text)?);
        }

        // join the chunks with the rendered spans
        let dir::CallableTarget::Symbol { function, .. } = &decision.build.target else {
            return Err(CompilerError::Internal {
                message: "a template join outside a direct symbol target".to_string(),
            });
        };
        let declared = self.resolve_callee(&function.key)?;
        let parameters = self.signature_parameters(declared.signature)?;
        let [chunk_slot, span_slot] = parameters.as_slice() else {
            return Err(CompilerError::Internal {
                message: "a template join without its chunk and span slots".to_string(),
            });
        };
        let chunks = self.slot_frame_slice(texts, *chunk_slot)?;
        let spans = self.slot_frame_slice(spans, *span_slot)?;
        let Some(joined) = self.call(&declared, vec![chunks, spans]) else {
            return Err(CompilerError::Internal {
                message: "a template join producing no text".to_string(),
            });
        };

        // adopt the owned text into the managed string the template reads as
        let ty = self.node_type_id(expression)?;
        let target = self.lower_type(ty)?;

        self.adopt(joined, target)
    }

    /// Store one value sequence in a frame slot, viewed at the slice slot it fills.
    pub(in crate::lower) fn slot_frame_slice(
        &mut self,
        values: Vec<mir::Value>,
        slot: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        let view = self.temporary_slice(slot)?;
        let view = self.frame_slice(values, view)?;

        self.adopt(view, slot)
    }
}

use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::place::{Place, PlaceRoot};
use crate::{CompilerError, CompilerResult, LowerError};

/// One expression lowered short of a value.
pub(in crate::lower) enum Operand {
    /// A materialized value.
    Value(mir::Value),
    /// A value living in the expression's type, materialized by the adjustment consuming it.
    Constant(Constant),
    /// A place the consumer reads, borrows, or writes.
    Place(Place),
}

/// One value living in its type.
pub(in crate::lower) enum Constant {
    /// A scalar literal.
    Literal(dir::Literal),
    /// A declared function or class constructor.
    Callable {
        /// The referencing expression.
        expression: dir::LocalNodeId<dir::Expression>,
        /// The referenced callable.
        symbol: dir::GlobalSymbolId,
    },
}

impl Place {
    /// Return the place of one whole local.
    pub(in crate::lower) fn local(local: mir::LocalNodeId<mir::Local>) -> Self {
        Self {
            root: PlaceRoot::Local(local),
            path: Vec::new(),
        }
    }
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one expression as an operand, its recorded coercion applied and a named place kept.
    pub(in crate::lower) fn lower_operand(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Operand> {
        self.lower_anchored(expression, |lower| {
            let operand = lower.lower_source(expression)?;
            match lower.coercion(expression) {
                Some(coercion) => lower.convert(operand, &coercion, Some(expression)),
                None => Ok(operand),
            }
        })
    }

    /// Lower one expression to a value, reading the place it names.
    pub(in crate::lower) fn lower_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let operand = self.lower_operand(expression)?;
        let ty = self.node_type_id(expression)?;

        self.lower_anchored(expression, |lower| lower.as_value(operand, ty))
    }

    /// Lower one expression into a destination place, returning whether control falls through.
    pub(in crate::lower) fn lower_into(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        destination: &Place,
    ) -> CompilerResult<bool> {
        // a joining expression writes each arm into the destination itself
        if self.coercion(expression).is_none() {
            match self.source().tree().get(expression).clone() {
                dir::Expression::If {
                    condition,
                    then_expression,
                    else_expression: Some(else_expression),
                    ..
                } => {
                    return self.lower_if_into(
                        &condition,
                        then_expression,
                        else_expression,
                        destination,
                    );
                }
                dir::Expression::Block(block) => return self.lower_block_into(block, destination),
                dir::Expression::Match { value, arms } => {
                    return self.lower_match_into(value, &arms, destination);
                }
                dir::Expression::Loop { label, body } => {
                    return self.lower_loop_into(label, body, destination);
                }
                dir::Expression::Try {
                    body,
                    catch,
                    finally,
                } => {
                    return self.lower_try(expression, body, catch, finally, Some(destination));
                }
                _ => {}
            }
        }

        // every other expression writes its value
        let value = self.lower_value(expression)?;
        self.write_place(destination, value)?;

        Ok(true)
    }

    /// Lower one expression as the place it names, spilling a value into a fresh local.
    pub(in crate::lower) fn lower_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Place> {
        let operand = self.lower_operand(expression)?;
        let ty = self.node_type_id(expression)?;

        self.as_place(operand, ty)
    }

    /// Read one operand out as a value at its type.
    pub(in crate::lower) fn as_value(
        &mut self,
        operand: Operand,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        match operand {
            Operand::Value(value) => Ok(value),
            Operand::Constant(Constant::Literal(literal)) => {
                let representation = self.lower_type(ty)?;

                self.lower_constant(literal, representation)
            }
            Operand::Constant(Constant::Callable { expression, symbol }) => {
                self.read_symbol(expression, symbol)
            }
            Operand::Place(place) => self.read_place(&place),
        }
    }

    /// Address one operand as a place, homing a value in a fresh local.
    pub(in crate::lower) fn as_place(
        &mut self,
        operand: Operand,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Place> {
        match operand {
            Operand::Place(place) => Ok(place),
            operand => {
                let value = self.as_value(operand, ty)?;
                let representation = self.lower_type(ty)?;
                let local = self.builder.local(representation, mir::Mutability::Mutable);
                self.builder.local_set(local, value);

                Ok(Place::local(local))
            }
        }
    }

    /// Allocate the local one joining expression writes its arms into.
    pub(in crate::lower) fn join_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Place> {
        let ty = self.node_type_id(expression)?;
        let representation = self.lower_type(ty)?;
        let local = self.builder.local(representation, mir::Mutability::Mutable);

        Ok(Place::local(local))
    }

    /// Lower one joining expression as a value through a fresh join local.
    pub(in crate::lower) fn lower_joined(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let destination = self.join_place(expression)?;
        if !self.lower_into(expression, &destination)? {
            return self.dead_value(expression);
        }

        self.read_place(&destination)
    }

    /// Lower one value if into a destination, each arm writing it.
    fn lower_if_into(
        &mut self,
        condition: &dir::Condition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: dir::LocalNodeId<dir::Expression>,
        destination: &Place,
    ) -> CompilerResult<bool> {
        let condition = self.lower_condition(condition)?;
        let then_block = self.builder.block();
        let else_block = self.builder.block();
        let join = self.builder.block();
        self.builder.branch(condition, then_block, else_block);

        // write each arm that falls through and jump to the join
        let mut falls_through = false;
        for (block, arm) in [(then_block, then_expression), (else_block, else_expression)] {
            self.builder.switch_to_block(block);
            if self.lower_into(arm, destination)? {
                self.builder.jump(join);
                falls_through = true;
            }
        }

        // continue at the join when any arm reaches it
        if falls_through {
            self.builder.switch_to_block(join);
        }

        Ok(falls_through)
    }

    /// Lower one value block into a destination, its tail writing it.
    pub(in crate::lower) fn lower_block_into(
        &mut self,
        block: dir::LocalNodeId<dir::Block>,
        destination: &Place,
    ) -> CompilerResult<bool> {
        let depth = self.open_disposals();
        let falls_through = self.lower_block_statements_into(block, destination)?;
        self.close_disposals(depth, !falls_through)?;

        Ok(falls_through)
    }

    /// Lower one block's statements, then its tail into the destination.
    fn lower_block_statements_into(
        &mut self,
        block: dir::LocalNodeId<dir::Block>,
        destination: &Place,
    ) -> CompilerResult<bool> {
        // lower the statements until one terminates the block
        let statements = self.source().tree().get(block).leading_expressions.clone();
        for statement in statements {
            if self.lower_statement(statement)? {
                return Ok(false);
            }
        }

        // write the tail, a valueless tail running for its control flow alone
        let tail_expression = self.source().tree().get(block).tail_expression;
        match tail_expression {
            Some(tail) if self.is_valueless(tail)? => Ok(!self.lower_statement(tail)?),
            Some(tail) => self.lower_into(tail, destination),
            None => Ok(true),
        }
    }

    /// Lower one loop into a destination, its valued breaks writing it.
    fn lower_loop_into(
        &mut self,
        label: Option<tspp_core::StringId>,
        body: dir::LocalNodeId<dir::Block>,
        destination: &Place,
    ) -> CompilerResult<bool> {
        let body_block = self.builder.block();
        let exit = self.builder.block();
        self.builder.jump(body_block);
        self.builder.switch_to_block(body_block);

        // loop back to the body when it falls through
        let terminated = self.lower_loop_body(
            label,
            body_block,
            exit,
            body,
            None,
            Some(destination.clone()),
        )?;
        if !terminated {
            self.builder.jump(body_block);
        }

        // continue after the loop when a break reaches the exit
        let is_exited = self.builder.is_entered(exit);
        self.builder.switch_to_block(exit);

        Ok(is_exited)
    }

    /// Return whether one expression yields no value.
    pub(in crate::lower) fn is_valueless(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        let ty = self.node_type(expression)?;

        Ok(matches!(ty, dir::Type::Never | dir::Type::Void))
    }

    /// Return the dead value standing where a diverging expression produced none.
    pub(in crate::lower) fn dead_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // continue in a fresh block after the divergence
        let continuation = self.builder.block();
        self.builder.switch_to_block(continuation);
        let ty = self.node_type_id(expression)?;
        let representation = self.lower_type(ty)?;

        Ok(self.builder.constant(mir::Constant::Uninit, representation))
    }

    /// Lower one expression's own syntax to an operand, without its coercion.
    fn lower_source(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Operand> {
        // a value living in its type stays a constant until an adjustment materializes it
        match self.node_type(expression)? {
            dir::Type::Literal(literal) => {
                self.lower_const_expression(expression)?;

                return Ok(Operand::Constant(Constant::Literal(literal)));
            }
            dir::Type::Null => {
                self.lower_const_expression(expression)?;

                return Ok(Operand::Constant(Constant::Literal(dir::Literal::Null)));
            }
            dir::Type::Undefined => {
                self.lower_const_expression(expression)?;

                return Ok(Operand::Constant(Constant::Literal(
                    dir::Literal::Undefined,
                )));
            }
            _ => {}
        }

        // retain callable declarations until their values are read
        if let Some(symbol) = self.referenced_callable(expression)? {
            return Ok(Operand::Constant(Constant::Callable { expression, symbol }));
        }

        // an expression naming owned storage stays a place, a reference reads as its value, a
        // read narrowed onto several cases converts as a value
        if self.is_place_expression(expression)
            && self.names_storage(expression)?
            && !self.indirect_storage(expression)?
            && !self.narrows_onto_cases(expression)?
        {
            return Ok(Operand::Place(self.receiver_place(expression)?));
        }

        // everything else materializes through its syntax
        Ok(Operand::Value(self.lower_expression_value(expression)?))
    }

    /// Return the function or class constructor selected for an expression.
    fn referenced_callable(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        // read the declaration selected for this complete expression
        let named = expression.into_global_any(self.source);
        let symbol = self.source().decisions.function_symbol(named).or_else(|| {
            self.source()
                .resolutions
                .name_resolution(named)
                .and_then(|resolution| resolution.single_symbol())
        });
        let Some(symbol) = symbol else {
            return Ok(None);
        };

        // keep stored values in their existing bindings
        if (symbol.module_id == self.source && self.values.contains_key(&symbol.local_id))
            || self.constant_global(symbol)?.is_some()
        {
            return Ok(None);
        }

        // select function and class constructor declarations
        let kind = self
            .lower
            .state(symbol.module_id)?
            .bindings
            .get_symbol(symbol.local_id)
            .kind;
        let is_callable = matches!(kind, dir::SymbolKind::Class | dir::SymbolKind::Function);

        Ok(is_callable.then_some(symbol))
    }

    /// Report one broken invariant of the lowered input as an internal error.
    pub(in crate::lower) fn internal(&self, message: impl Into<String>) -> CompilerError {
        CompilerError::Internal {
            message: message.into(),
        }
    }

    /// Report one unsupported construct, anchored at the node being lowered.
    pub(in crate::lower) fn unsupported(&self, construct: impl Into<String>) -> CompilerError {
        let anchor = match self.builder.source() {
            Some((_, span)) => span.into(),
            None => self.lower.module.into(),
        };

        LowerError::Unsupported {
            anchor,
            construct: construct.into(),
        }
        .into()
    }
}

use destack_dir::{Expression, LocalNodeId};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::emit::FunctionContext;
use crate::lower::item::lower_mutability;

impl FunctionContext<'_> {
    /// Lower a borrow expression to a reference value.
    pub(crate) fn lower_reference_of_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        mutability: Option<dir::Mutability>,
        right: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve the reference result type
        let pointee_type = self.lower_type_for_expression(right)?;
        let mutability = mutability
            .map(lower_mutability)
            .unwrap_or(mir::Mutability::Immutable);
        let result_type = self.state.builder.type_reference(
            mir::ReferenceKind::Borrowed,
            pointee_type,
            mutability,
            mir::AddressSpace::Generic,
            false,
        );

        // FUGU #Incomplete: support local addr so direct reference borrows can lower
        match self.env.dir_tree.get(right) {
            Expression::Member {
                left,
                name,
                static_arguments,
            } => {
                // reject static arguments on member borrows
                if static_arguments.is_some() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "static arguments on member borrows are not supported".to_string(),
                    });
                }

                // lower the aggregate value
                let (aggregate_value, aggregate_type) = self.lower_value_expression(*left)?;

                // resolve field index through the type lowerer
                let field_index = self
                    .env
                    .type_lowerer
                    .field_index_for_type(
                        aggregate_type,
                        *name,
                        self.env.strings,
                        self.state.builder.tree(),
                    )
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "field not found in aggregate type".to_string(),
                    })?;

                // emit field.addr
                let value =
                    self.state
                        .builder
                        .field_addr(aggregate_value, field_index as u32, result_type);
                Ok((value, result_type))
            }
            Expression::Index { left, right } => {
                let index_expr = right.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "missing index expression".to_string(),
                })?;

                // lower array and index expressions
                let (array_value, _) = self.lower_value_expression(*left)?;
                let (index_value, _) = self.lower_value_expression(index_expr)?;

                // emit element.addr
                let value = self
                    .state
                    .builder
                    .element_addr(array_value, index_value, result_type);
                Ok((value, result_type))
            }
            _ => Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "unsupported borrow target".to_string(),
            }),
        }
    }

    /// Lower an ownership conversion to an owned reference.
    pub(crate) fn lower_value_of_expression(
        &mut self,
        _expression_id: LocalNodeId<Expression>,
        mutability: Option<dir::Mutability>,
        right: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the owned value expression
        let (value, pointee_type) = self.lower_value_expression(right)?;

        // resolve the owned reference type
        let mutability = mutability
            .map(lower_mutability)
            .unwrap_or(mir::Mutability::Immutable);
        let result_type = self.state.builder.type_reference(
            mir::ReferenceKind::Owned,
            pointee_type,
            mutability,
            mir::AddressSpace::Generic,
            false,
        );

        // allocate owned storage and store the value
        let pointer = self.state.builder.raw_alloc(pointee_type, result_type);
        self.state.builder.store(pointer, value);

        Ok((pointer, result_type))
    }
}

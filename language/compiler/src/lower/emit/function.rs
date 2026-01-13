use std::collections::HashMap;

use destack_base::StringPool;
use destack_dir::{Expression, GlobalSymbolId, LocalNodeId};
use destack_source::ModuleId;
use destack_workspace::ProfileId;
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::{BreakContext, LocalBinding, LoopContext, Terminates};
use crate::lower::item::GlobalBinding;
use crate::lower::r#type::TypeLowerer;

/// Mutable context for lowering a single function body into MIR.
///
/// Owns the function builder and local state (variables, loops).
/// All expression and statement lowering methods are implemented on this type.
pub(crate) struct FunctionContext<'a> {
    /// Identify the module being lowered.
    pub(crate) module_id: ModuleId,
    /// Identify the profile used for DIR access.
    pub(crate) profile: ProfileId,
    /// Provide access to the DIR tree for expression lookup.
    pub(crate) dir_tree: &'a dir::NodeTree,
    /// Provide access to symbol metadata for type resolution.
    pub(crate) symbols: &'a dir::SymbolTable,
    /// Provide access to inferred and declared types.
    pub(crate) types: &'a dir::TypeTable,
    /// Provide access to the program string pool for name resolution.
    pub(crate) strings: &'a StringPool,
    /// Resolve direct calls for known function symbols.
    pub(crate) functions_by_symbol: &'a HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
    /// Resolve globals by symbol for module-level variable references.
    pub(crate) globals_by_symbol: &'a HashMap<GlobalSymbolId, GlobalBinding>,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: &'a TypeLowerer,
    /// Emit MIR into the current function builder.
    pub(crate) builder: mir::FunctionBuilder<'a>,
    /// Track locals by symbol for variable resolution.
    pub(crate) locals_by_symbol: HashMap<GlobalSymbolId, LocalBinding>,
    /// Track loop contexts by symbol for labeled break/continue.
    pub(crate) loops_by_symbol: HashMap<GlobalSymbolId, LoopContext>,
    /// Track labelled block contexts by symbol for labeled breaks.
    pub(crate) labels_by_symbol: HashMap<GlobalSymbolId, BreakContext>,
    /// Track loop nesting for unlabeled break/continue.
    pub(crate) loop_stack: Vec<LoopContext>,
    /// Track breakable contexts for unlabeled breaks.
    pub(crate) break_stack: Vec<BreakContext>,
    /// Binding for `this` in method bodies.
    pub(crate) this_binding: Option<LocalBinding>,
}

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionContext<'a> {
    /// Create a new function context with the given builder.
    pub(crate) fn new(
        module_id: ModuleId,
        profile: ProfileId,
        dir_tree: &'a dir::NodeTree,
        symbols: &'a dir::SymbolTable,
        types: &'a dir::TypeTable,
        strings: &'a StringPool,
        functions_by_symbol: &'a HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
        globals_by_symbol: &'a HashMap<GlobalSymbolId, GlobalBinding>,
        type_lowerer: &'a TypeLowerer,
        builder: mir::FunctionBuilder<'a>,
    ) -> Self {
        Self {
            module_id,
            profile,
            dir_tree,
            symbols,
            types,
            strings,
            functions_by_symbol,
            globals_by_symbol,
            type_lowerer,
            builder,
            locals_by_symbol: HashMap::new(),
            loops_by_symbol: HashMap::new(),
            labels_by_symbol: HashMap::new(),
            loop_stack: Vec::new(),
            break_stack: Vec::new(),
            this_binding: None,
        }
    }

    /// Lower a function body to MIR and return whether it terminates.
    pub(crate) fn lower_body(
        &mut self,
        body_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<Terminates> {
        self.lower_statement_expression(body_id)
    }

    /// Create an UnsupportedConstruct error for the given expression.
    #[allow(dead_code)]
    pub(crate) fn error(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        message: impl Into<String>,
    ) -> LowerError {
        LowerError::UnsupportedConstruct {
            node: expression_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile)),
            message: message.into(),
        }
    }

    /// Create a MissingType error for the given expression.
    #[allow(dead_code)]
    pub(crate) fn missing_type_error(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerError {
        LowerError::MissingType {
            node: expression_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile)),
        }
    }

    /// Lower a value expression to its result value and type.
    pub(crate) fn lower_value_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let expression = self.dir_tree.get(expression_id);
        match expression {
            Expression::Parenthesized { expression } => self.lower_value_expression(*expression),

            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                self.lower_reference_expression(expression_id, *target_symbol)
            }

            Expression::ScalarLiteral { value } => self.lower_scalar_literal(expression_id, value),

            Expression::Cast {
                operator,
                value,
                target_type: _,
                source: _,
            } => self.lower_cast_expression(expression_id, *operator, *value),

            Expression::Binary {
                left,
                operator,
                right,
            } => self.lower_binary_expression(expression_id, *left, *operator, *right),

            Expression::Assign { left, right } => {
                self.lower_assign_expression(expression_id, *left, *right)
            }

            Expression::Call {
                left,
                dynamic_arguments,
                static_arguments,
            } => {
                self.lower_call_expression(expression_id, left, dynamic_arguments, static_arguments)
            }

            Expression::TupleExpression { elements } => {
                self.lower_tuple_expression(expression_id, elements)
            }

            Expression::ArrayExpression { elements } => {
                self.lower_array_expression(expression_id, elements)
            }

            Expression::Member {
                left,
                name,
                static_arguments,
            } => {
                if static_arguments.is_some() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "static arguments on member access are not supported".to_string(),
                    })?;
                }
                self.lower_member_expression(expression_id, *left, *name)
            }

            Expression::Index { left, right } => {
                let index_expr = right.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "missing index expression".to_string(),
                })?;
                self.lower_index_expression(expression_id, *left, index_expr)
            }

            Expression::Unary { operator, right } => {
                self.lower_unary_expression(expression_id, *operator, *right)
            }

            Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => self.lower_conditional_expression(
                expression_id,
                *condition,
                *then_expression,
                *else_expression,
            ),

            Expression::TaggedObjectExpression { ty, properties } => {
                self.lower_tagged_object_expression(expression_id, *ty, properties)
            }

            Expression::This => self.lower_this_expression(expression_id),

            _ => Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: format!("unsupported value expression '{}'", expression.kind_name()),
            })?,
        }
    }

    /// Lower a variable reference expression.
    fn lower_reference_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: dir::GlobalSymbolId,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // first check locals
        if let Some(binding) = self.locals_by_symbol.get(&target_symbol) {
            let value = self.builder.use_variable(binding.variable);
            return Ok((value, binding.ty));
        }

        // use global_const for immutable, global_addr + load for mutable
        if let Some(global_binding) = self.globals_by_symbol.get(&target_symbol) {
            if global_binding.mutability == mir::Mutability::Mutable {
                let addr = self.builder.global_addr(global_binding.global);
                let value = self.builder.load(addr);
                Ok((value, global_binding.ty))
            } else {
                let value = self.builder.global_const(global_binding.global);
                Ok((value, global_binding.ty))
            }
        }
        // some unresolved symbol reference
        else {
            Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "unresolved symbol reference".to_string(),
            })
        }
    }

    /// Lower a cast expression.
    fn lower_cast_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        operator: dir::CastOperator,
        value_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the cast input first
        let (value, _) = self.lower_value_expression(value_id)?;

        // resolve the target type for the cast
        let target_type =
            self.mir_type_for_expression(expression_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                })?;

        // pick the mir cast operator
        let mir_operator = self.lower_cast_operator(expression_id, operator, value_id)?;

        // emit the cast when needed
        let value = if let Some(mir_operator) = mir_operator {
            self.builder.cast(mir_operator, value, target_type)
        } else {
            value
        };

        Ok((value, target_type))
    }

    /// Lower a binary expression.
    fn lower_binary_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: dir::BinaryOperator,
        right: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // short-circuit logical operators need special control flow
        if matches!(operator, dir::BinaryOperator::And | dir::BinaryOperator::Or) {
            return self.lower_logical_operator(expression_id, operator, left, right);
        }

        // lower operands
        let (left_value, _) = self.lower_value_expression(left)?;
        let (right_value, _) = self.lower_value_expression(right)?;

        // get result type
        let result_type =
            self.mir_type_for_expression(expression_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                })?;

        // emit binary operation
        let op = self.lower_binary_operator(expression_id, operator, left)?;
        let value = self.builder.binary_op(op, left_value, right_value);

        // comparisons produce bool, others preserve operand type
        let ty = if op.is_comparison() {
            self.type_lowerer.ty_bool
        } else {
            result_type
        };

        Ok((value, ty))
    }

    /// Lower an assignment expression.
    fn lower_assign_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve the assignment target
        let target_symbol = match self.dir_tree.get(left) {
            Expression::LocalReference { target_symbol, .. } => *target_symbol,
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "unsupported assignment target".to_string(),
                })?;
            }
        };
        let binding = self.local_binding_for_symbol(left, target_symbol)?;

        // lower the assigned value
        let (value, value_type) = self.lower_value_expression(right)?;

        // update the variable binding
        self.builder.define_variable(binding.variable, value);

        Ok((value, value_type))
    }

    /// Lower a unary expression.
    fn lower_unary_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        operator: dir::UnaryOperator,
        right: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower operand and emit unary operation
        let (operand_value, operand_type) = self.lower_value_expression(right)?;
        let op = self.lower_unary_operator(expression_id, operator, right)?;
        let value = self.builder.unary_op(op, operand_value);

        // logical NOT produces bool, other unary ops preserve type
        let ty = if matches!(operator, dir::UnaryOperator::Not) {
            self.type_lowerer.ty_bool
        } else {
            operand_type
        };

        Ok((value, ty))
    }

    /// Lower a `this` expression.
    fn lower_this_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get this binding set by lower_method
        let binding = self
            .this_binding
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "this reference outside of method context".to_string(),
            })?;

        let value = self.builder.use_variable(binding.variable);
        Ok((value, binding.ty))
    }
}

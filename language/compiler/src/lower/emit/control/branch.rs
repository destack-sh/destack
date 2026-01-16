use destack_dir::{Expression, LocalNodeId};
use destack_mir as mir;

use crate::{LowerError, LowerResult};

use crate::lower::emit::FunctionContext;

impl FunctionContext<'_> {
    /// Lower a conditional (ternary) expression.
    ///
    /// Conditional expressions like `a ? b : c` evaluate condition `a`,
    /// then evaluate either `b` or `c` based on the result.
    pub(crate) fn lower_conditional_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        condition_id: LocalNodeId<Expression>,
        then_id: LocalNodeId<Expression>,
        else_id: Option<LocalNodeId<Expression>>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // require else branch for value expressions
        let else_id = else_id.ok_or_else(|| LowerError::UnsupportedConstruct {
            node: expression_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile)),
            message: "conditional expression requires else branch".to_string(),
        })?;

        // evaluate condition first
        let (condition_value, condition_type) = self.lower_value_expression(condition_id)?;
        self.check_type_is_bool(condition_id, condition_type, "conditional expression")?;

        // get the result type from the expression
        let result_type = self.mir_type_for_expression(expression_id)?;

        // create blocks for each branch
        let then_block = self.builder.create_block();
        let else_block = self.builder.create_block();
        let merge_block = self.builder.create_block();

        // create a variable to hold the result
        let result_variable = self.builder.create_variable(result_type);

        // branch based on condition
        self.builder.branch(condition_value, then_block, else_block);

        // then block: evaluate then expression and set result
        self.builder.switch_to_block(then_block);
        let (then_value, _) = self.lower_value_expression(then_id)?;
        self.builder.define_variable(result_variable, then_value);
        self.builder.jump(merge_block);

        // else block: evaluate else expression and set result
        self.builder.switch_to_block(else_block);
        let (else_value, _) = self.lower_value_expression(else_id)?;
        self.builder.define_variable(result_variable, else_value);
        self.builder.jump(merge_block);

        // merge block: use the result variable (SSA will create block parameter)
        self.builder.switch_to_block(merge_block);
        let result_value = self.builder.use_variable(result_variable);

        Ok((result_value, result_type))
    }
}

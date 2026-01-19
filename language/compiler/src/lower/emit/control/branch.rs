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
                .into_global_any(self.env.module_id)
                .into_anchored(Some(self.env.profile)),
            message: "conditional expression requires else branch".to_string(),
        })?;

        // get the result type from the expression
        let result_type = self.mir_type_for_expression(expression_id)?;

        // create blocks for each branch
        let then_block = self.state.builder.create_block();
        let else_block = self.state.builder.create_block();
        let merge_block = self.state.builder.create_block();

        // create a variable to hold the result
        let result_variable = self.state.builder.create_variable(result_type);

        // branch based on condition (with union tag checks when possible)
        let did_check = self.lower_union_tag_check(condition_id, then_block, else_block)?;
        if !did_check {
            // evaluate condition first
            let (condition_value, condition_type) = self.lower_value_expression(condition_id)?;
            self.check_type_is_bool(condition_id, condition_type, "conditional expression")?;

            // branch
            self.state
                .builder
                .branch(condition_value, then_block, else_block);
        }

        // then block: evaluate then expression and set result
        self.state.builder.switch_to_block(then_block);
        let (then_value, _) = self.lower_value_expression(then_id)?;
        self.state
            .builder
            .define_variable(result_variable, then_value);
        self.state.builder.jump(merge_block);

        // else block: evaluate else expression and set result
        self.state.builder.switch_to_block(else_block);
        let (else_value, _) = self.lower_value_expression(else_id)?;
        self.state
            .builder
            .define_variable(result_variable, else_value);
        self.state.builder.jump(merge_block);

        // merge block: use the result variable (SSA will create block parameter)
        self.state.builder.switch_to_block(merge_block);
        let result_value = self.state.builder.use_variable(result_variable);

        Ok((result_value, result_type))
    }
}

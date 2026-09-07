use destack_dir as dir;
use destack_js as js;

use crate::EmitError;
use crate::emit::js::ScriptEmitter;

impl ScriptEmitter<'_> {
    /// Emit one DIR function body.
    pub(crate) fn emit_function_body(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<js::Block>, EmitError> {
        // wrap an expression body in a return statement
        let dir::Expression::Block(block) = self.tree.get(expression) else {
            let value = self.emit_expression(expression)?;
            let statement = js::Statement::Return { value: Some(value) };
            let statement = self.insert_from_source(statement, expression);
            let block = js::Block {
                statements: vec![statement],
            };

            return Ok(self.insert_from_source(block, expression));
        };
        let block = *block;
        let source = self.tree.get(block);
        let mut statements = Vec::with_capacity(source.len());

        // emit leading expressions
        for leading in source.leading_expressions.iter().copied() {
            if let Some(statement) = self.emit_statement(leading)? {
                statements.push(statement);
            }
        }

        // emit the function tail
        if let Some(tail) = source.tail_expression
            && !self.expression_is_erased(tail)
        {
            let statement = if self.expression_is_valueless(tail)? {
                // retain a valueless statement
                let Some(statement) = self.emit_statement(tail)? else {
                    return Err(self.internal_error(
                        "erased expression reached JavaScript function tail".to_string(),
                    ));
                };

                statement
            } else {
                // return a produced value
                let expression = self.emit_expression(tail)?;
                let statement = js::Statement::Return {
                    value: Some(expression),
                };
                self.insert_from_source(statement, tail)
            };
            statements.push(statement);
        }

        let output = self.insert_from_source(js::Block { statements }, block);

        Ok(output)
    }
}

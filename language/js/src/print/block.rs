use crate::{Block, CatchClause, Module, Precedence, SwitchCase};

use super::printer::{PrintError, PrintNode, Printer};

impl PrintNode for Block {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        printer.token("{")?;
        for statement in self.statements.iter().copied() {
            printer.node(statement, module)?;
        }
        printer.token("}")
    }
}

impl PrintNode for CatchClause {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        printer.word("catch")?;
        if let Some(pattern) = self.pattern {
            printer.token("(")?;
            printer.node(pattern, module)?;
            printer.token(")")?;
        }
        printer.node(self.body, module)
    }
}

impl PrintNode for SwitchCase {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        if let Some(value) = self.value {
            printer.word("case")?;
            printer.expression(value, Precedence::Lowest, module)?;
        } else {
            printer.word("default")?;
        }
        printer.token(":")?;

        for statement in self.body.iter().copied() {
            printer.node(statement, module)?;
        }

        Ok(())
    }
}

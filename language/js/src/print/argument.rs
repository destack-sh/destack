use crate::{Argument, ArrayElement, Module, Precedence};

use super::printer::{PrintError, PrintNode, Printer};

impl PrintNode for Argument {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Positional { value } => {
                printer.expression(*value, Precedence::Assignment, module)
            }
            Self::Spread { value } => {
                printer.token("...")?;
                printer.expression(*value, Precedence::Assignment, module)
            }
        }
    }
}

impl PrintNode for ArrayElement {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Expression { value } => {
                printer.expression(*value, Precedence::Assignment, module)
            }
            Self::Spread { value } => {
                printer.token("...")?;
                printer.expression(*value, Precedence::Assignment, module)
            }
            Self::Elision => Ok(()),
        }
    }
}

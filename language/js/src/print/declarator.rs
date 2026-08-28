use crate::{Declarator, Module};

use super::printer::{PrintError, PrintNode, Printer};

impl PrintNode for Declarator {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        printer.node(self.pattern, module)?;
        printer.default(self.value, module)
    }
}

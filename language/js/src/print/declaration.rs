use crate::{Asynchrony, Declaration, Module, Precedence};

use super::printer::{PrintError, PrintNode, Printer};

impl PrintNode for Declaration {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Class(class) => {
                printer.word("class")?;
                if let Some(name) = class.name {
                    printer.identifier(name, module)?;
                }
                if let Some(extends) = class.extends_expression {
                    printer.word("extends")?;
                    printer.expression(extends, Precedence::Call, module)?;
                }

                printer.token("{")?;
                for member in class.members.iter().copied() {
                    printer.node(member, module)?;
                }
                printer.token("}")?;
            }
            Self::Function(function) => {
                let signature = &function.signature;
                if signature.asynchrony == Asynchrony::Async {
                    printer.word("async")?;
                }
                printer.word("function")?;
                if signature.is_generator {
                    printer.token("*")?;
                }
                if let Some(name) = function.name {
                    printer.identifier(name, module)?;
                }
                printer.parameters(&signature.parameters, signature.rest, module)?;
                printer.node(function.body, module)?;
            }
        }

        Ok(())
    }
}

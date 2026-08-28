use crate::{ClassElementName, LocalNodeId, Member, Module, Precedence, Property, PropertyName};

use super::printer::{PrintError, PrintNode, Printer};

impl PrintNode for Property {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Field { key, value } => {
                printer.property_name(*key, module)?;
                printer.token(":")?;
                printer.expression(*value, Precedence::Assignment, module)?;
            }
            Self::Shorthand { value } => printer.shorthand(*value, module)?,
            Self::Method {
                key,
                signature,
                body,
            } => printer.method(false, *key, signature, *body, module)?,
            Self::Getter { key, body } => printer.getter(false, *key, *body, module)?,
            Self::Setter {
                key,
                parameter,
                body,
            } => printer.setter(false, *key, *parameter, *body, module)?,
            Self::Spread { value } => {
                printer.token("...")?;
                printer.expression(*value, Precedence::Assignment, module)?;
            }
        }

        Ok(())
    }
}

impl PrintNode for Member {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Field {
                key,
                default,
                is_static,
            } => {
                printer.static_modifier(*is_static)?;
                printer.class_element_name(*key, module)?;
                if let Some(default) = default {
                    printer.token("=")?;
                    printer.expression(*default, Precedence::Assignment, module)?;
                }
                printer.token(";")?;
            }
            Self::Method {
                key,
                signature,
                body,
                is_static,
            } => printer.method(*is_static, *key, signature, *body, module)?,
            Self::Getter {
                key,
                body,
                is_static,
            } => printer.getter(*is_static, *key, *body, module)?,
            Self::Setter {
                key,
                parameter,
                body,
                is_static,
            } => printer.setter(*is_static, *key, *parameter, *body, module)?,
            Self::Constructor {
                parameters,
                rest,
                body,
            } => {
                printer.word("constructor")?;
                printer.parameters(parameters, *rest, module)?;
                printer.node(*body, module)?;
            }
            Self::StaticBlock { body } => {
                printer.word("static")?;
                printer.node(*body, module)?;
            }
        }

        Ok(())
    }
}

impl Printer {
    /// Print one comma separated object property list.
    pub(super) fn properties(
        &mut self,
        properties: &[LocalNodeId<Property>],
        module: &Module,
    ) -> Result<(), PrintError> {
        for (index, property) in properties.iter().copied().enumerate() {
            if index > 0 {
                self.token(",")?;
            }
            self.node(property, module)?;
        }

        Ok(())
    }

    /// Print one ECMAScript property name.
    pub(super) fn property_name(
        &mut self,
        name: PropertyName,
        module: &Module,
    ) -> Result<(), PrintError> {
        match name {
            PropertyName::Identifier(name) => self.identifier_name(name, module),
            PropertyName::String(name) => self.string(name, module),
            PropertyName::Computed(expression) => {
                self.token("[")?;
                self.expression(expression, Precedence::Assignment, module)?;
                self.token("]")
            }
        }
    }

    /// Print one ECMAScript class element name.
    pub(super) fn class_element_name(
        &mut self,
        name: ClassElementName,
        module: &Module,
    ) -> Result<(), PrintError> {
        match name {
            ClassElementName::Public(name) => self.property_name(name, module),
            ClassElementName::Private(identifier) => {
                self.token("#")?;
                self.identifier(identifier, module)
            }
        }
    }

    /// Print one static modifier.
    pub(super) fn static_modifier(&mut self, is_static: bool) -> Result<(), PrintError> {
        if is_static {
            self.word("static")?;
        }

        Ok(())
    }
}

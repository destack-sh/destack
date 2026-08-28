use crate::{
    Asynchrony, ClassElementName, FunctionSignature, LocalNodeId, Module, Parameter, Pattern,
    PropertyName,
};

use super::printer::{PrintError, PrintNode, Printer};

impl PrintNode for Parameter {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Named { name, default } => {
                printer.identifier(*name, module)?;
                printer.default(*default, module)
            }
            Self::Pattern { pattern, default } => {
                printer.node(*pattern, module)?;
                printer.default(*default, module)
            }
        }
    }
}

impl Printer {
    /// Print one compact arrow function parameter list.
    pub(super) fn arrow_parameters(
        &mut self,
        parameters: &[LocalNodeId<Parameter>],
        rest: Option<LocalNodeId<Pattern>>,
        module: &Module,
    ) -> Result<(), PrintError> {
        if rest.is_none()
            && let [parameter] = parameters
        {
            let parameter = module.tree.get(*parameter);
            if matches!(parameter, Parameter::Named { default: None, .. }) {
                return self.node(parameters[0], module);
            }
        }

        self.parameters(parameters, rest, module)
    }

    /// Print one complete function parameter list.
    pub(super) fn parameters(
        &mut self,
        parameters: &[LocalNodeId<Parameter>],
        rest: Option<LocalNodeId<Pattern>>,
        module: &Module,
    ) -> Result<(), PrintError> {
        self.token("(")?;
        self.nodes(parameters, module)?;
        if let Some(rest) = rest {
            if !parameters.is_empty() {
                self.token(",")?;
            }
            self.token("...")?;
            self.node(rest, module)?;
        }
        self.token(")")
    }

    /// Print one ordinary method.
    pub(super) fn method<N: MethodName>(
        &mut self,
        is_static: bool,
        key: N,
        signature: &FunctionSignature,
        body: LocalNodeId<crate::Block>,
        module: &Module,
    ) -> Result<(), PrintError> {
        self.static_modifier(is_static)?;
        self.method_prefix(signature)?;
        key.print(self, module)?;
        self.parameters(&signature.parameters, signature.rest, module)?;
        self.node(body, module)
    }

    /// Print one getter.
    pub(super) fn getter<N: MethodName>(
        &mut self,
        is_static: bool,
        key: N,
        body: LocalNodeId<crate::Block>,
        module: &Module,
    ) -> Result<(), PrintError> {
        self.static_modifier(is_static)?;
        self.word("get")?;
        key.print(self, module)?;
        self.token("(")?;
        self.token(")")?;
        self.node(body, module)
    }

    /// Print one setter.
    pub(super) fn setter<N: MethodName>(
        &mut self,
        is_static: bool,
        key: N,
        parameter: LocalNodeId<crate::Parameter>,
        body: LocalNodeId<crate::Block>,
        module: &Module,
    ) -> Result<(), PrintError> {
        self.static_modifier(is_static)?;
        self.word("set")?;
        key.print(self, module)?;
        self.token("(")?;
        self.node(parameter, module)?;
        self.token(")")?;
        self.node(body, module)
    }

    /// Print the modifiers preceding one ordinary method name.
    fn method_prefix(&mut self, signature: &FunctionSignature) -> Result<(), PrintError> {
        if signature.asynchrony == Asynchrony::Async {
            self.word("async")?;
        }
        if signature.is_generator {
            self.token("*")?;
        }

        Ok(())
    }
}

/// One object or class method name.
pub(super) trait MethodName: Copy {
    /// Print the name.
    fn print(self, printer: &mut Printer, module: &Module) -> Result<(), PrintError>;
}

impl MethodName for PropertyName {
    fn print(self, printer: &mut Printer, module: &Module) -> Result<(), PrintError> {
        printer.property_name(self, module)
    }
}

impl MethodName for ClassElementName {
    fn print(self, printer: &mut Printer, module: &Module) -> Result<(), PrintError> {
        printer.class_element_name(self, module)
    }
}

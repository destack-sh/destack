use crate::{
    ArrayAssignPatternField, ArrayPatternField, AssignPattern, Identifier, LocalNodeId, Module,
    ObjectAssignPatternField, ObjectPatternField, Pattern, Place, Precedence,
};

use super::printer::{PrintError, PrintNode, Printer};

impl PrintNode for Place {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Identifier { identifier } => printer.identifier(*identifier, module),
            Self::Member { object, property } => {
                let needs_extra_dot = printer.place_object(*object, module)?;
                if needs_extra_dot {
                    printer.token(".")?;
                }
                printer.token(".")?;
                printer.identifier_name(*property, module)
            }
            Self::PrivateMember { object, property } => {
                let needs_extra_dot = printer.place_object(*object, module)?;
                if needs_extra_dot {
                    printer.token(".")?;
                }
                printer.token(".")?;
                printer.token("#")?;
                printer.identifier(*property, module)
            }
            Self::Index { object, key } => {
                let needs_parentheses = module.tree.get(*object).is_optional_chain(&module.tree);
                if needs_parentheses {
                    printer.token("(")?;
                }
                printer.expression(*object, Precedence::Call, module)?;
                if needs_parentheses {
                    printer.token(")")?;
                }
                printer.token("[")?;
                printer.expression(*key, Precedence::Lowest, module)?;
                printer.token("]")
            }
        }
    }
}

impl PrintNode for AssignPattern {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Place { place } => printer.node(*place, module),
            Self::Array { fields, rest } => printer.array_assign_pattern(fields, *rest, module),
            Self::Object { fields, rest } => printer.object_assign_pattern(fields, *rest, module),
        }
    }
}

impl PrintNode for ArrayAssignPatternField {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Positional { pattern, default } => {
                printer.node(*pattern, module)?;
                printer.default(*default, module)
            }
            Self::Elision => Ok(()),
        }
    }
}

impl PrintNode for ObjectAssignPatternField {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Named {
                name,
                pattern,
                default,
            } => {
                printer.property_name(*name, module)?;
                printer.token(":")?;
                printer.node(*pattern, module)?;
                printer.default(*default, module)
            }
            Self::Shorthand {
                identifier,
                default,
            } => {
                printer.shorthand(*identifier, module)?;
                printer.default(*default, module)
            }
        }
    }
}

impl PrintNode for Pattern {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Binding { identifier } => printer.identifier(*identifier, module),
            Self::Array { fields, rest } => printer.array_pattern(fields, *rest, module),
            Self::Object { fields, rest } => printer.object_pattern(fields, *rest, module),
        }
    }
}

impl PrintNode for ArrayPatternField {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Positional { pattern, default } => {
                printer.node(*pattern, module)?;
                printer.default(*default, module)
            }
            Self::Elision => Ok(()),
        }
    }
}

impl PrintNode for ObjectPatternField {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self {
            Self::Named {
                name,
                pattern,
                default,
            } => {
                printer.property_name(*name, module)?;
                printer.token(":")?;
                printer.node(*pattern, module)?;
                printer.default(*default, module)
            }
            Self::Shorthand {
                identifier,
                default,
            } => {
                printer.shorthand(*identifier, module)?;
                printer.default(*default, module)
            }
        }
    }
}

impl Printer {
    /// Print one optional default value.
    pub(super) fn default(
        &mut self,
        default: Option<LocalNodeId<crate::Expression>>,
        module: &Module,
    ) -> Result<(), PrintError> {
        if let Some(default) = default {
            self.token("=")?;
            self.expression(default, Precedence::Assignment, module)?;
        }

        Ok(())
    }

    /// Print one array binding pattern.
    fn array_pattern(
        &mut self,
        fields: &[LocalNodeId<ArrayPatternField>],
        rest: Option<LocalNodeId<Pattern>>,
        module: &Module,
    ) -> Result<(), PrintError> {
        self.token("[")?;
        self.nodes(fields, module)?;
        self.rest(fields, rest, module)?;
        if rest.is_none()
            && fields
                .last()
                .is_some_and(|field| matches!(module.tree.get(*field), ArrayPatternField::Elision))
        {
            self.token(",")?;
        }
        self.token("]")
    }

    /// Print one object binding pattern.
    fn object_pattern(
        &mut self,
        fields: &[LocalNodeId<ObjectPatternField>],
        rest: Option<Identifier>,
        module: &Module,
    ) -> Result<(), PrintError> {
        self.token("{")?;
        self.nodes(fields, module)?;
        if let Some(rest) = rest {
            if !fields.is_empty() {
                self.token(",")?;
            }
            self.token("...")?;
            self.identifier(rest, module)?;
        }
        self.token("}")
    }

    /// Print comma separated nodes.
    pub(super) fn nodes<T>(
        &mut self,
        nodes: &[LocalNodeId<T>],
        module: &Module,
    ) -> Result<(), PrintError>
    where
        T: PrintNode,
        crate::Tree: crate::TreeStore<T>,
    {
        for (index, node) in nodes.iter().copied().enumerate() {
            if index > 0 {
                self.token(",")?;
            }
            self.node(node, module)?;
        }

        Ok(())
    }

    /// Print one final rest node.
    pub(super) fn rest<T, R>(
        &mut self,
        fields: &[LocalNodeId<T>],
        rest: Option<LocalNodeId<R>>,
        module: &Module,
    ) -> Result<(), PrintError>
    where
        T: crate::Node,
        R: PrintNode,
        crate::Tree: crate::TreeStore<R>,
    {
        if let Some(rest) = rest {
            if !fields.is_empty() {
                self.token(",")?;
            }
            self.token("...")?;
            self.node(rest, module)?;
        }

        Ok(())
    }

    /// Print one array assignment pattern.
    fn array_assign_pattern(
        &mut self,
        fields: &[LocalNodeId<ArrayAssignPatternField>],
        rest: Option<LocalNodeId<AssignPattern>>,
        module: &Module,
    ) -> Result<(), PrintError> {
        self.token("[")?;
        self.nodes(fields, module)?;
        self.rest(fields, rest, module)?;
        if rest.is_none()
            && fields.last().is_some_and(|field| {
                matches!(module.tree.get(*field), ArrayAssignPatternField::Elision)
            })
        {
            self.token(",")?;
        }
        self.token("]")
    }

    /// Print one object assignment pattern.
    fn object_assign_pattern(
        &mut self,
        fields: &[LocalNodeId<ObjectAssignPatternField>],
        rest: Option<LocalNodeId<Place>>,
        module: &Module,
    ) -> Result<(), PrintError> {
        self.token("{")?;
        self.nodes(fields, module)?;
        self.rest(fields, rest, module)?;
        self.token("}")
    }

    /// Print one non-optional place object.
    fn place_object(
        &mut self,
        object: LocalNodeId<crate::Expression>,
        module: &Module,
    ) -> Result<bool, PrintError> {
        let expression = module.tree.get(object);
        let needs_parentheses = expression.is_optional_chain(&module.tree);
        if needs_parentheses {
            self.token("(")?;
        }
        self.expression(object, Precedence::Call, module)?;
        if needs_parentheses {
            self.token(")")?;
        }

        Ok(!needs_parentheses && expression.needs_decimal_member_dot(false))
    }
}

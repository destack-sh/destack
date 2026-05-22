use destack_dir as dir;

use super::VariableId;

/// Field collected for an object-like construction.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct FieldTerm {
    /// The resolved field key.
    pub(in crate::check) key: dir::StaticKey,
    /// The field value type variable.
    pub(in crate::check) value: VariableId,
}

/// Term used to bind one variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Term {
    /// Type term.
    Type(TypeTerm),
    /// Static term.
    Static(StaticTerm),
    /// Layout term.
    Layout(LayoutTerm),
    /// Resolution term.
    Resolution(ResolutionTerm),
}

/// Term used to bind a type variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum TypeTerm {
    /// Concrete type.
    Type(dir::Type),
    /// Type variable.
    Variable(VariableId),
    /// Source type expression normalized by the solver.
    TypeExpression(dir::GlobalNodeId<dir::TypeExpression>),
}

/// Term used to bind a static variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum StaticTerm {
    /// Concrete static term.
    Value(dir::StaticTerm),
    /// Static variable.
    Variable(VariableId),
    /// Expression evaluated as a static term by the solver.
    Expression(dir::GlobalNodeId<dir::Expression>),
}

/// Term used to bind a layout variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum LayoutTerm {
    /// Concrete layout id.
    Layout(dir::LocalLayoutId),
    /// Layout variable.
    Variable(VariableId),
}

/// Term used to bind a resolution variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ResolutionTerm {
    /// Resolved name.
    Name(dir::NameResolution),
    /// Resolved call.
    Call(dir::CallResolution),
    /// Resolved member.
    Member(dir::MemberResolution),
}

/// Check-local memory form term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum FormTerm {
    /// Automatically managed value form.
    Managed,
    /// Owned value form.
    Owned,
    /// Borrowed value form.
    Borrowed {
        /// The solved lifetime value.
        lifetime: VariableId,
        /// The solved access value.
        access: VariableId,
    },
    /// Raw pointer form.
    Raw,
    /// Placed value form.
    Placed {
        /// The solved place value.
        place: VariableId,
    },
    /// Readonly view form.
    Readonly,
}

/// Argument supplied to a generic application or call.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ArgumentTerm {
    /// Type argument.
    Type(VariableId),
    /// Static argument.
    Static(VariableId),
    /// Dynamic argument.
    Dynamic(VariableId),
    /// Spread type argument.
    SpreadType(VariableId),
    /// Static spread argument.
    SpreadStatic(VariableId),
    /// Dynamic spread argument.
    SpreadDynamic(VariableId),
}

impl ArgumentTerm {
    /// Return the variable referenced by this argument.
    pub(in crate::check) fn variable(&self) -> VariableId {
        match self {
            Self::Type(variable)
            | Self::Static(variable)
            | Self::Dynamic(variable)
            | Self::SpreadType(variable)
            | Self::SpreadStatic(variable)
            | Self::SpreadDynamic(variable) => *variable,
        }
    }
}

/// Operand supplied to an operator resolution.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum OperandTerm {
    /// Type-space operand.
    Type(VariableId),
    /// Static-space operand.
    Static(VariableId),
    /// Dynamic operand.
    Dynamic(VariableId),
}

impl OperandTerm {
    /// Return the variable referenced by this operand.
    pub(in crate::check) fn variable(&self) -> VariableId {
        match self {
            Self::Type(variable) | Self::Static(variable) | Self::Dynamic(variable) => *variable,
        }
    }
}

/// Operator resolution input.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum OperatorTerm {
    /// Unary operator.
    Unary(dir::UnaryOperator),
    /// Binary operator.
    Binary(dir::BinaryOperator),
    /// Await operator.
    Await,
    /// Maybe unwrap operator.
    Maybe,
    /// Must unwrap operator.
    Must,
}

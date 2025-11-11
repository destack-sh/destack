use dyst_dir::{self as dir};

/// Error when transpiling something into JS/TS
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum TranspileError {
    /// Unsupported expression.
    UnsupportedExpression { node: dir::NodeId<dir::Expression> } = 1,
    /// Unsupported definition.
    UnsupportedDefinition { node: dir::NodeId<dir::Definition> } = 2,
    /// Unsupported path.
    UnsupportedPath { path: dir::Path } = 3,
    /// Unsupported type.
    UnsupportedType { node: dir::NodeId<dir::Type> } = 4,
    /// Unsupported variant.
    UnsupportedVariant { node: dir::NodeId<dir::Variant> } = 5,
    /// Unsupported field.
    UnsupportedField { node: dir::NodeId<dir::Field> } = 6,
    /// Unsupported dependency item.
    UnsupportedDependencyItem {
        node: dir::NodeId<dir::DependencyItem>,
    } = 7,
    /// Unsupported parameter.
    UnsupportedParameter { node: dir::NodeId<dir::Parameter> } = 8,
    /// Unsupported argument.
    UnsupportedArgument { node: dir::NodeId<dir::Argument> } = 9,
    /// Unsupported pattern.
    UnsupportedPattern { node: dir::NodeId<dir::Pattern> } = 10,
    /// Unsupported pattern field.
    UnsupportedPatternField {
        node: dir::NodeId<dir::PatternField>,
    } = 11,
    /// Unsupported annotation.
    UnsupportedAnnotation { node: dir::NodeId<dir::Annotation> } = 12,

    /// Print error.
    PrintError { message: String } = 100,
}

pub type TranspileResult<T> = Result<T, TranspileError>;

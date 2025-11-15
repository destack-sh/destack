use dyst_dir::{self as dir};

use crate::TranspileDiagnostic;

/// Error when transpiling something into JS/TS
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum TranspileError {
    /// Unsupported expression.
    UnsupportedExpression { node: dir::NodeId<dir::Expression> } = 1,
    /// Unsupported definition.
    UnsupportedDefinition { node: dir::NodeId<dir::Definition> } = 2,
    /// Unsupported path.
    UnsupportedPath {
        node: dir::NodeIdAny,
        path: dir::Path,
    } = 3,
    /// Unsupported type.
    UnsupportedType { node: dir::NodeId<dir::Type> } = 4,
    /// Unsupported property.
    UnsupportedProperty { node: dir::NodeId<dir::Property> } = 5,
    /// Unsupported enum field.
    UnsupportedEnumField { node: dir::NodeId<dir::EnumField> } = 6,
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
}

impl TranspileError {
    /// Get the message of the error.
    pub fn message(&self) -> &'static str {
        match self {
            Self::UnsupportedExpression { .. } => "unsupported expression",
            Self::UnsupportedDefinition { .. } => "unsupported definition",
            Self::UnsupportedPath { .. } => "unsupported path",
            Self::UnsupportedType { .. } => "unsupported type",
            Self::UnsupportedProperty { .. } => "unsupported property",
            Self::UnsupportedEnumField { .. } => "unsupported enum field",
            Self::UnsupportedDependencyItem { .. } => "unsupported dependency item",
            Self::UnsupportedParameter { .. } => "unsupported parameter",
            Self::UnsupportedArgument { .. } => "unsupported argument",
            Self::UnsupportedPattern { .. } => "unsupported pattern",
            Self::UnsupportedPatternField { .. } => "unsupported pattern field",
            Self::UnsupportedAnnotation { .. } => "unsupported annotation",
        }
    }

    /// Get the number of the error.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnsupportedExpression { .. } => 1,
            Self::UnsupportedDefinition { .. } => 2,
            Self::UnsupportedPath { .. } => 3,
            Self::UnsupportedType { .. } => 4,
            Self::UnsupportedProperty { .. } => 5,
            Self::UnsupportedEnumField { .. } => 6,
            Self::UnsupportedDependencyItem { .. } => 7,
            Self::UnsupportedParameter { .. } => 8,
            Self::UnsupportedArgument { .. } => 9,
            Self::UnsupportedPattern { .. } => 10,
            Self::UnsupportedPatternField { .. } => 11,
            Self::UnsupportedAnnotation { .. } => 12,
        }
    }

    /// Get the full code of the error. See CompileError.
    pub fn full_code(&self) -> String {
        format!("TE{:03}", self.sub_code())
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> dir::NodeIdAny {
        match self {
            Self::UnsupportedExpression { node } => node.into_any(),
            Self::UnsupportedDefinition { node } => node.into_any(),
            Self::UnsupportedPath { node, .. } => *node,
            Self::UnsupportedType { node } => node.into_any(),
            Self::UnsupportedProperty { node } => node.into_any(),
            Self::UnsupportedEnumField { node } => node.into_any(),
            Self::UnsupportedDependencyItem { node } => node.into_any(),
            Self::UnsupportedParameter { node } => node.into_any(),
            Self::UnsupportedArgument { node } => node.into_any(),
            Self::UnsupportedPattern { node } => node.into_any(),
            Self::UnsupportedPatternField { node } => node.into_any(),
            Self::UnsupportedAnnotation { node } => node.into_any(),
        }
    }
}

impl From<TranspileError> for TranspileDiagnostic {
    fn from(error: TranspileError) -> Self {
        TranspileDiagnostic::Error(error)
    }
}

pub type TranspileResult<T> = Result<T, TranspileError>;

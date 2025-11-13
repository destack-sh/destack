use dyst_dir::{self as dir, Session};
use dyst_source::{Diagnostic, DiagnosticSeverity, LabeledSpan};

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
}

impl TranspileError {
    /// Get the family letter of the error. See CompilerError.
    pub fn family_letter(&self) -> &'static str {
        "T"
    }

    /// Get the family number of the error. See CompilerError.
    pub fn family_number(&self) -> u8 {
        8
    }

    /// Get the message of the error.
    pub fn message(&self) -> &'static str {
        match self {
            Self::UnsupportedExpression { .. } => "unsupported expression",
            Self::UnsupportedDefinition { .. } => "unsupported definition",
            Self::UnsupportedPath { .. } => "unsupported path",
            Self::UnsupportedType { .. } => "unsupported type",
            Self::UnsupportedVariant { .. } => "unsupported variant",
            Self::UnsupportedField { .. } => "unsupported field",
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
            Self::UnsupportedVariant { .. } => 5,
            Self::UnsupportedField { .. } => 6,
            Self::UnsupportedDependencyItem { .. } => 7,
            Self::UnsupportedParameter { .. } => 8,
            Self::UnsupportedArgument { .. } => 9,
            Self::UnsupportedPattern { .. } => 10,
            Self::UnsupportedPatternField { .. } => 11,
            Self::UnsupportedAnnotation { .. } => 12,
        }
    }

    /// Get the full code of the error. See CompilerError.
    pub fn full_code(&self) -> String {
        format!("{}{:03}", self.family_letter(), self.sub_code())
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> dir::NodeIdAny {
        match self {
            Self::UnsupportedExpression { node } => node.into_any(),
            Self::UnsupportedDefinition { node } => node.into_any(),
            Self::UnsupportedPath { node, .. } => *node,
            Self::UnsupportedType { node } => node.into_any(),
            Self::UnsupportedVariant { node } => node.into_any(),
            Self::UnsupportedField { node } => node.into_any(),
            Self::UnsupportedDependencyItem { node } => node.into_any(),
            Self::UnsupportedParameter { node } => node.into_any(),
            Self::UnsupportedArgument { node } => node.into_any(),
            Self::UnsupportedPattern { node } => node.into_any(),
            Self::UnsupportedPatternField { node } => node.into_any(),
            Self::UnsupportedAnnotation { node } => node.into_any(),
        }
    }

    /// Get the message of the error.
    pub fn to_diagnostic<'a>(&self, session: &'a Session<'a>) -> Diagnostic {
        // get source information
        let node_id = self.node_id();
        let (module_id, ast_id) = session.tree.get_source(node_id.id);
        let module = session
            .modules
            .get(module_id)
            .unwrap_or_else(|| panic!("module not found: {module_id:?}"));
        let file_id = module.file_id;
        let file = session
            .files
            .get(file_id)
            .unwrap_or_else(|| panic!("file not found: {file_id:?}"));

        // make diagnostic
        let message = self.message().to_string();
        let code = self.full_code();
        let primary_span = ast_id
            .map(|ast_id| module.ast.get_span_by_id(ast_id))
            .unwrap_or_else(|| file.span());
        let primary_span = LabeledSpan {
            span: primary_span,
            label: message.clone(),
        };

        Diagnostic {
            code,
            severity: DiagnosticSeverity::Error,
            message,
            file_id,
            primary_span,
            secondary_spans: None,
            suggestions: None,
        }
    }
}

pub type TranspileResult<T> = Result<T, TranspileError>;

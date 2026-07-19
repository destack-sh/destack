use core::fmt;
use std::error::Error;

use destack_dir::{NodeType, Token, TokenSpan, TokenType};
use destack_source::{
    ByteRange, ContentId, Diagnostic, DiagnosticDefinition, DiagnosticLabel, DiagnosticTarget,
    FileId, Span,
};

/// All parser diagnostic definitions.
const PARSER_DIAGNOSTICS: &[DiagnosticDefinition] = &[
    ParserDiagnostic::UnexpectedSyntax.definition(),
    ParserDiagnostic::UnexpectedToken.definition(),
    ParserDiagnostic::ExpectedToken.definition(),
    ParserDiagnostic::Expected(NodeType::Expression).definition(),
    ParserDiagnostic::Expected(NodeType::TypeExpression).definition(),
    ParserDiagnostic::Expected(NodeType::Block).definition(),
    ParserDiagnostic::Expected(NodeType::Catch).definition(),
    ParserDiagnostic::Expected(NodeType::Declaration).definition(),
    ParserDiagnostic::Expected(NodeType::Declarator).definition(),
    ParserDiagnostic::Expected(NodeType::Property).definition(),
    ParserDiagnostic::Expected(NodeType::TypeMember).definition(),
    ParserDiagnostic::Expected(NodeType::TypeMappedParameter).definition(),
    ParserDiagnostic::Expected(NodeType::Member).definition(),
    ParserDiagnostic::Expected(NodeType::EnumField).definition(),
    ParserDiagnostic::Expected(NodeType::WhereClause).definition(),
    ParserDiagnostic::Expected(NodeType::DependencyItem).definition(),
    ParserDiagnostic::Expected(NodeType::GenericParameter).definition(),
    ParserDiagnostic::Expected(NodeType::Parameter).definition(),
    ParserDiagnostic::Expected(NodeType::GenericArgument).definition(),
    ParserDiagnostic::Expected(NodeType::TupleElement).definition(),
    ParserDiagnostic::Expected(NodeType::Argument).definition(),
    ParserDiagnostic::Expected(NodeType::TreeAttribute).definition(),
    ParserDiagnostic::Expected(NodeType::TreeChild).definition(),
    ParserDiagnostic::Expected(NodeType::MatchCase).definition(),
    ParserDiagnostic::Expected(NodeType::Pattern).definition(),
    ParserDiagnostic::Expected(NodeType::PatternField).definition(),
    ParserDiagnostic::Expected(NodeType::AssignPattern).definition(),
    ParserDiagnostic::Expected(NodeType::AssignPatternField).definition(),
    ParserDiagnostic::Expected(NodeType::Decorator).definition(),
    ParserDiagnostic::InvalidAssignmentTarget.definition(),
];

/// One parser diagnostic.
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
enum ParserDiagnostic {
    /// Source syntax with no more specific token classification.
    UnexpectedSyntax,
    /// A source token rejected by its grammar position.
    UnexpectedToken,
    /// One required source token.
    ExpectedToken,
    /// One required grammar node.
    Expected(NodeType),
    /// An expression that cannot be assigned to.
    InvalidAssignmentTarget,
}

impl ParserDiagnostic {
    /// Return the static definition of this parser diagnostic.
    const fn definition(self) -> DiagnosticDefinition {
        let (id, description) = match self {
            Self::UnexpectedSyntax => ("unexpected-syntax", "Unexpected source syntax."),
            Self::UnexpectedToken => ("unexpected-token", "Unexpected source token."),
            Self::ExpectedToken => ("expected-token", "Expected a specific source token."),
            Self::Expected(NodeType::Expression) => {
                ("expected-expression", "Expected an expression.")
            }
            Self::Expected(NodeType::TypeExpression) => {
                ("expected-type-expression", "Expected a type expression.")
            }
            Self::Expected(NodeType::Block) => ("expected-block", "Expected a block."),
            Self::Expected(NodeType::Catch) => ("expected-catch", "Expected a catch clause."),
            Self::Expected(NodeType::Declaration) => {
                ("expected-declaration", "Expected a declaration.")
            }
            Self::Expected(NodeType::Declarator) => {
                ("expected-declarator", "Expected a declarator.")
            }
            Self::Expected(NodeType::Property) => ("expected-property", "Expected a property."),
            Self::Expected(NodeType::TypeMember) => {
                ("expected-type-member", "Expected a type member.")
            }
            Self::Expected(NodeType::TypeMappedParameter) => (
                "expected-mapped-type-parameter",
                "Expected a mapped type parameter.",
            ),
            Self::Expected(NodeType::Member) => ("expected-member", "Expected a member."),
            Self::Expected(NodeType::EnumField) => {
                ("expected-enum-field", "Expected an enum field.")
            }
            Self::Expected(NodeType::WhereClause) => {
                ("expected-where-clause", "Expected a where clause.")
            }
            Self::Expected(NodeType::DependencyItem) => {
                ("expected-dependency-item", "Expected a dependency item.")
            }
            Self::Expected(NodeType::GenericParameter) => (
                "expected-generic-parameter",
                "Expected a generic parameter.",
            ),
            Self::Expected(NodeType::Parameter) => ("expected-parameter", "Expected a parameter."),
            Self::Expected(NodeType::GenericArgument) => {
                ("expected-generic-argument", "Expected a generic argument.")
            }
            Self::Expected(NodeType::TupleElement) => {
                ("expected-tuple-element", "Expected a tuple element.")
            }
            Self::Expected(NodeType::Argument) => ("expected-argument", "Expected an argument."),
            Self::Expected(NodeType::TreeAttribute) => {
                ("expected-tree-attribute", "Expected a tree attribute.")
            }
            Self::Expected(NodeType::TreeChild) => {
                ("expected-tree-child", "Expected a tree child.")
            }
            Self::Expected(NodeType::MatchCase) => {
                ("expected-match-case", "Expected a match case.")
            }
            Self::Expected(NodeType::Pattern) => ("expected-pattern", "Expected a pattern."),
            Self::Expected(NodeType::PatternField) => {
                ("expected-pattern-field", "Expected a pattern field.")
            }
            Self::Expected(NodeType::AssignPattern) => (
                "expected-assignment-pattern",
                "Expected an assignment pattern.",
            ),
            Self::Expected(NodeType::AssignPatternField) => (
                "expected-assignment-pattern-field",
                "Expected an assignment pattern field.",
            ),
            Self::Expected(NodeType::Decorator) => ("expected-decorator", "Expected a decorator."),
            Self::InvalidAssignmentTarget => {
                ("invalid-assignment-target", "Invalid assignment target.")
            }
        };

        DiagnosticDefinition::error(id, description)
    }
}

/// One parser error used for recovery and diagnostics.
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub struct ParserError {
    /// The source byte range of the error.
    range: ByteRange,
    /// The actual token type at the error span.
    actual: Option<TokenType>,
    /// The immediate parser failure.
    kind: ParserErrorKind,
    /// The grammar node expected at the error.
    expected_node: Option<NodeType>,
}

/// Immediate parser failure independent of its grammar context.
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub enum ParserErrorKind {
    /// The source location is unexpected.
    Unexpected,
    /// The source location requires one token.
    Expected(TokenType),
    /// The source expression is not an assignment target.
    InvalidAssignmentTarget,
}

/// The source location of one parser error.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ParserErrorLocation {
    /// The source byte range of the error.
    range: ByteRange,
    /// The actual token type at the error span.
    actual: Option<TokenType>,
}

/// One parser operation result.
pub type ParserResult<T> = Result<T, ParserError>;

/// Extension methods for parser operation results.
pub trait ParserResultExt<T> {
    /// Attach the grammar node required by this operation.
    fn in_node(self, node_type: NodeType) -> Result<T, ParserError>;
}

impl From<ByteRange> for ParserErrorLocation {
    /// Create a parser error location from a source byte range.
    #[inline]
    fn from(range: ByteRange) -> Self {
        Self {
            range,
            actual: None,
        }
    }
}

impl From<Token> for ParserErrorLocation {
    /// Create a parser error location from a compact source token.
    #[inline]
    fn from(token: Token) -> Self {
        Self {
            range: token.range(),
            actual: Some(token.ty()),
        }
    }
}

impl From<TokenSpan> for ParserErrorLocation {
    /// Create a parser error location from a token span.
    #[inline]
    fn from(token: TokenSpan) -> Self {
        Self {
            range: token.span.range(),
            actual: Some(token.token.ty()),
        }
    }
}

impl<T> ParserResultExt<T> for Result<T, ParserError> {
    /// Attach a grammar node when the error has no expected node.
    #[inline]
    fn in_node(self, node_type: NodeType) -> Self {
        match self {
            Err(error) if error.expected_node.is_none() => Err(error.in_node(node_type)),
            result => result,
        }
    }
}

impl ParserError {
    /// All parser diagnostic definitions.
    pub const ALL: &'static [DiagnosticDefinition] = PARSER_DIAGNOSTICS;

    /// Create an error for an unexpected source location.
    pub fn unexpected(location: impl Into<ParserErrorLocation>) -> Self {
        let location = location.into();

        Self {
            range: location.range,
            actual: location.actual,
            kind: ParserErrorKind::Unexpected,
            expected_node: None,
        }
    }

    /// Create an error for one required token.
    pub fn expected(location: impl Into<ParserErrorLocation>, expected: TokenType) -> Self {
        let location = location.into();

        Self {
            range: location.range,
            actual: location.actual,
            kind: ParserErrorKind::Expected(expected),
            expected_node: None,
        }
    }

    /// Create an invalid assignment target error.
    pub fn invalid_assignment_target(range: ByteRange) -> Self {
        Self {
            range,
            actual: None,
            kind: ParserErrorKind::InvalidAssignmentTarget,
            expected_node: None,
        }
    }

    /// Return the source range of this error.
    pub fn range(self) -> ByteRange {
        self.range
    }

    /// Return the actual token type when the error has one.
    pub fn actual_token(self) -> Option<TokenType> {
        self.actual
    }

    /// Return the expected token type when the error requires one.
    pub fn expected_token(self) -> Option<TokenType> {
        match self.kind {
            ParserErrorKind::Expected(expected) => Some(expected),
            ParserErrorKind::Unexpected | ParserErrorKind::InvalidAssignmentTarget => None,
        }
    }

    /// Return the grammar node expected at this error.
    pub fn expected_node(self) -> Option<NodeType> {
        self.expected_node
    }

    /// Return the immediate parser failure.
    pub fn kind(self) -> ParserErrorKind {
        self.kind
    }

    /// Return this error range as a span in the parsed file.
    #[inline]
    pub fn span(self, file_id: FileId) -> Span {
        let range = self.range();

        Span::new(file_id, range.start, range.end)
    }

    /// Set the grammar node expected at this error.
    pub fn in_node(mut self, node_type: NodeType) -> Self {
        self.expected_node = Some(node_type);

        self
    }

    /// Convert this parser error into one source diagnostic.
    pub fn to_diagnostic(&self, content: ContentId, file_id: FileId) -> Diagnostic {
        let definition = self.diagnostic().definition();
        let (message, label) = self.message();
        let primary =
            DiagnosticLabel::message(content, DiagnosticTarget::Span(self.span(file_id)), label);

        Diagnostic::error(definition.id, message, primary)
    }

    /// Return the diagnostic for this parser error.
    fn diagnostic(&self) -> ParserDiagnostic {
        match (self.kind, self.expected_node, self.actual) {
            (ParserErrorKind::InvalidAssignmentTarget, _, _) => {
                ParserDiagnostic::InvalidAssignmentTarget
            }
            (_, Some(node_type), _) => ParserDiagnostic::Expected(node_type),
            (ParserErrorKind::Expected(_), None, _) => ParserDiagnostic::ExpectedToken,
            (ParserErrorKind::Unexpected, None, Some(_)) => ParserDiagnostic::UnexpectedToken,
            (ParserErrorKind::Unexpected, None, None) => ParserDiagnostic::UnexpectedSyntax,
        }
    }

    /// Return the diagnostic header and primary label messages.
    fn message(&self) -> (String, String) {
        if let Some(node_type) = self.expected_node {
            let expected = node_type.name();
            let message = format!("expected {expected}");
            let label = match self.actual {
                Some(actual) => format!("expected {expected}, found {actual}"),
                None => message.clone(),
            };

            return (message, label);
        }

        match self.kind {
            ParserErrorKind::Unexpected => match self.actual {
                Some(actual) => {
                    let message = format!("unexpected {actual}");

                    (message.clone(), message)
                }
                None => {
                    let message = "unexpected syntax".to_string();

                    (message.clone(), message)
                }
            },
            ParserErrorKind::Expected(expected) => {
                let message = format!("expected {expected}");
                let label = match self.actual {
                    Some(actual) => format!("expected {expected}, found {actual}"),
                    None => message.clone(),
                };

                (message, label)
            }
            ParserErrorKind::InvalidAssignmentTarget => {
                let message = "invalid assignment target".to_string();

                (message.clone(), message)
            }
        }
    }
}

impl fmt::Display for ParserError {
    /// Format one parser error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (message, _) = self.message();

        write!(formatter, "{message} at {:?}", self.range())
    }
}

impl Error for ParserError {}

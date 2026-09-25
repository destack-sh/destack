use clap::ValueEnum;
use tspp_dir as dir;

/// A DIR node type accepted by structural commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum NodeTypeArg {
    /// Expression.
    Expression,
    /// Type expression.
    TypeExpression,
    /// Block.
    Block,
    /// Catch clause.
    Catch,
    /// Declaration.
    Declaration,
    /// Declarator.
    Declarator,
    /// Property.
    Property,
    /// Type member.
    TypeMember,
    /// Mapped type parameter.
    TypeMappedParameter,
    /// Member.
    Member,
    /// Enum field.
    EnumField,
    /// Where clause.
    WhereClause,
    /// Dependency item.
    DependencyItem,
    /// Generic parameter.
    GenericParameter,
    /// Parameter.
    Parameter,
    /// Generic argument.
    GenericArgument,
    /// Tuple element.
    TupleElement,
    /// Call argument.
    Argument,
    /// Tree attribute.
    TreeAttribute,
    /// Tree child.
    TreeChild,
    /// Match arm.
    MatchArm,
    /// Destructuring or selection pattern.
    Pattern,
    /// Pattern field.
    PatternField,
    /// Assignment pattern.
    AssignPattern,
    /// Assignment pattern field.
    AssignPatternField,
    /// Decorator.
    Decorator,
    /// Switch case.
    SwitchCase,
}

impl From<NodeTypeArg> for dir::NodeType {
    /// Convert a CLI node type into its DIR node type.
    fn from(value: NodeTypeArg) -> Self {
        match value {
            NodeTypeArg::Expression => Self::Expression,
            NodeTypeArg::TypeExpression => Self::TypeExpression,
            NodeTypeArg::Block => Self::Block,
            NodeTypeArg::Catch => Self::Catch,
            NodeTypeArg::Declaration => Self::Declaration,
            NodeTypeArg::Declarator => Self::Declarator,
            NodeTypeArg::Property => Self::Property,
            NodeTypeArg::TypeMember => Self::TypeMember,
            NodeTypeArg::TypeMappedParameter => Self::TypeMappedParameter,
            NodeTypeArg::Member => Self::Member,
            NodeTypeArg::EnumField => Self::EnumField,
            NodeTypeArg::WhereClause => Self::WhereClause,
            NodeTypeArg::DependencyItem => Self::DependencyItem,
            NodeTypeArg::GenericParameter => Self::GenericParameter,
            NodeTypeArg::Parameter => Self::Parameter,
            NodeTypeArg::GenericArgument => Self::GenericArgument,
            NodeTypeArg::TupleElement => Self::TupleElement,
            NodeTypeArg::Argument => Self::Argument,
            NodeTypeArg::TreeAttribute => Self::TreeAttribute,
            NodeTypeArg::TreeChild => Self::TreeChild,
            NodeTypeArg::MatchArm => Self::MatchArm,
            NodeTypeArg::Pattern => Self::Pattern,
            NodeTypeArg::PatternField => Self::PatternField,
            NodeTypeArg::AssignPattern => Self::AssignPattern,
            NodeTypeArg::AssignPatternField => Self::AssignPatternField,
            NodeTypeArg::Decorator => Self::Decorator,
            NodeTypeArg::SwitchCase => Self::SwitchCase,
        }
    }
}

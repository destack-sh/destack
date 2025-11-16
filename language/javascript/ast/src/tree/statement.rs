use crate::{
    Argument, AssignOperator, Block, Definition, DependencyItem, DependencyKind, ExportType,
    Expression, Mutability, Node, NodeId, NodeType, Parameter, Pattern, StringId, Type,
};

/// A Statement is a JS/TS top-level statement in some container/block.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Import items (including type items).
    Import {
        kind: DependencyKind,
        target: StringId,
        alias: Option<StringId>,
        items: Vec<NodeId<DependencyItem>>,
        arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Export items (including type items).
    Export {
        mode: ExportType,
        kind: DependencyKind,
        target: Option<StringId>,
        alias: Option<StringId>,
        items: Vec<NodeId<DependencyItem>>,
    },
    /// Export value.
    ExportValue { value: NodeId<Expression> },

    /// Definition statement.
    Definition { definition: NodeId<Definition> },
    /// Block of statements.
    Block { block: NodeId<Block> },

    /// Let binding.
    Let {
        mutability: Mutability,
        pattern: NodeId<Pattern>,
        ty: Option<NodeId<Type>>,
        value: Option<NodeId<Expression>>,
    },
    /// Let type alias.
    LetType {
        name: StringId,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        value: NodeId<Type>,
    },
    /// Assignment operation.
    Assign {
        left: NodeId<Expression>,
        operator: AssignOperator,
        right: NodeId<Expression>,
    },
    /// Expression statement.
    Expression { expression: NodeId<Expression> },

    /// If statement.
    If {
        condition: NodeId<Expression>,
        then_block: NodeId<Block>,
        else_block: Option<NodeId<Block>>,
    },
    /// While statement.
    While {
        condition: NodeId<Expression>,
        body: NodeId<Block>,
    },
    /// For statement.
    For {
        initialization: Option<NodeId<Expression>>,
        condition: Option<NodeId<Expression>>,
        increment: Option<NodeId<Expression>>,
        body: NodeId<Block>,
    },
    /// For in statement.
    ForIn {
        name: StringId,
        iterator: NodeId<Expression>,
        body: NodeId<Block>,
    },
    /// For of statement.
    ForOf {
        pattern: NodeId<Pattern>,
        iterator: NodeId<Expression>,
        body: NodeId<Block>,
    },

    /// Try statement.
    Try {
        try_block: NodeId<Block>,
        catch_pattern: Option<NodeId<Pattern>>,
        catch_block: NodeId<Block>,
        finally_block: Option<NodeId<Block>>,
    },
    /// Await statement.
    Await { value: NodeId<Expression> },
    /// Yield statement.
    Yield { value: NodeId<Expression> },
    /// Throw statement.
    Throw { value: NodeId<Expression> },
    /// Continue statement.
    Continue { label: Option<StringId> },
    /// Break statement.
    Break { label: Option<StringId> },
    /// Return statement.
    Return { value: Option<NodeId<Expression>> },
}

impl Node for Statement {
    const TYPE: NodeType = NodeType::Statement;
}

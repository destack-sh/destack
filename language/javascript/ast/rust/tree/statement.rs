use crate::{
    Argument, AssignOperator, Block, DependencyItem, Expression, Mutability, Node, NodeId,
    NodeType, Parameter, Pattern, StringId, Type,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Import.
    Import {
        source: StringId,
        items: Vec<NodeId<DependencyItem>>,
        arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Export.
    Export { items: Vec<NodeId<DependencyItem>> },

    /// Block of statements.
    Block { block: NodeId<Block> },

    /// Let.
    Let {
        mutability: Mutability,
        pattern: NodeId<Pattern>,
        ty: Option<NodeId<Type>>,
        value: Option<NodeId<Expression>>,
    },
    /// Let type.
    LetType {
        name: StringId,
        static_parameters: Option<Vec<NodeId<Parameter>>>,
        value: NodeId<Type>,
    },
    /// Assignment.
    Assign {
        left: NodeId<Expression>,
        operator: AssignOperator,
        right: NodeId<Expression>,
    },
    /// Expression.
    Expression { expression: NodeId<Expression> },

    /// If.
    If {
        condition: NodeId<Expression>,
        then_block: NodeId<Block>,
        else_block: Option<NodeId<Block>>,
    },
    /// While loop.
    While {
        condition: NodeId<Expression>,
        body: NodeId<Block>,
    },
    /// For three-part loop.
    For {
        initialization: Option<NodeId<Expression>>,
        condition: NodeId<Expression>,
        increment: Option<NodeId<Expression>>,
        body: NodeId<Block>,
    },
    /// For in loop.
    ForIn {
        name: StringId,
        iterator: NodeId<Expression>,
        body: NodeId<Block>,
    },
    /// For of loop.
    ForOf {
        pattern: NodeId<Pattern>,
        iterator: NodeId<Expression>,
        body: NodeId<Block>,
    },

    /// Try.
    Try {
        try_block: NodeId<Block>,
        catch_pattern: Option<NodeId<Pattern>>,
        catch_block: NodeId<Block>,
        finally_block: Option<NodeId<Block>>,
    },
    /// Await.
    Await { value: NodeId<Expression> },
    /// Yield.
    Yield { value: NodeId<Expression> },
    /// Throw.
    Throw { value: NodeId<Expression> },
    /// Continue.
    Continue { label: Option<StringId> },
    /// Break.
    Break { label: Option<StringId> },
    /// Return.
    Return { value: Option<NodeId<Expression>> },
}

impl Node for Statement {
    const TYPE: NodeType = NodeType::Statement;
}

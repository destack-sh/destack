use crate::{
    Argument, AssignOperator, Block, Declaration, DeclarationDescriptor, DependencyItem,
    DependencyKind, Expression, LocalNodeId, Mutability, Node, NodeType, Pattern, StringId, Type,
};

/// A Statement is a JS/TS top-level statement in some container/block.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Import items (including type items).
    Import {
        kind: DependencyKind,
        target: StringId,
        items: Vec<LocalNodeId<DependencyItem>>,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Export items (including type items).
    Export {
        kind: DependencyKind,
        target: Option<StringId>,
        items: Vec<LocalNodeId<DependencyItem>>,
    },
    /// Export value.
    ExportValue { value: LocalNodeId<Expression> },

    /// Declaration statement.
    Declaration {
        declaration: LocalNodeId<Declaration>,
    },
    /// Block of statements.
    Block { block: LocalNodeId<Block> },

    /// Let binding.
    Let {
        descriptor: DeclarationDescriptor,
        mutability: Mutability,
        pattern: LocalNodeId<Pattern>,
        ty: Option<LocalNodeId<Type>>,
        value: Option<LocalNodeId<Expression>>,
    },
    /// Assignment operation.
    Assign {
        left: LocalNodeId<Expression>,
        operator: AssignOperator,
        right: LocalNodeId<Expression>,
    },
    /// Expression statement.
    Expression { expression: LocalNodeId<Expression> },

    /// If statement.
    If {
        condition: LocalNodeId<Expression>,
        then_block: LocalNodeId<Block>,
        else_block: Option<LocalNodeId<Block>>,
    },
    /// While statement.
    While {
        condition: LocalNodeId<Expression>,
        body: LocalNodeId<Block>,
    },
    /// For statement.
    For {
        initialization: Option<LocalNodeId<Expression>>,
        condition: Option<LocalNodeId<Expression>>,
        increment: Option<LocalNodeId<Expression>>,
        body: LocalNodeId<Block>,
    },
    /// For in statement.
    ForIn {
        name: StringId,
        iterator: LocalNodeId<Expression>,
        body: LocalNodeId<Block>,
    },
    /// For of statement.
    ForOf {
        pattern: LocalNodeId<Pattern>,
        iterator: LocalNodeId<Expression>,
        body: LocalNodeId<Block>,
    },

    /// Try statement.
    Try {
        try_block: LocalNodeId<Block>,
        catch_pattern: Option<LocalNodeId<Pattern>>,
        catch_block: LocalNodeId<Block>,
        finally_block: Option<LocalNodeId<Block>>,
    },
    /// Await statement.
    Await { value: LocalNodeId<Expression> },
    /// Yield statement.
    Yield { value: LocalNodeId<Expression> },
    /// Throw statement.
    Throw { value: LocalNodeId<Expression> },
    /// Continue statement.
    Continue { label: Option<StringId> },
    /// Break statement.
    Break { label: Option<StringId> },
    /// Return statement.
    Return {
        value: Option<LocalNodeId<Expression>>,
    },
}

impl Node for Statement {
    const TYPE: NodeType = NodeType::Statement;
}

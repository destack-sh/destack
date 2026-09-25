use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    AssignOperator, Asynchrony, Block, CatchClause, Declaration, Declarator, DependencyItem,
    Expression, LocalNodeId, Mutability, Node, NodeType, Pattern, Property, StringId, SwitchCase,
};
use tspp_source::ModuleId;

/// The kind of one dependency attribute clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DependencyAttributeClauseKind {
    /// The standard `with` attribute clause keyword.
    With,
    /// The legacy `assert` attribute clause keyword.
    Assert,
}

/// One dependency attribute clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DependencyAttributeClause {
    /// The clause introducer.
    pub kind: DependencyAttributeClauseKind,
    /// The attribute entries inside the clause body.
    pub properties: Vec<LocalNodeId<Property>>,
}

/// A JavaScript statement in some container or block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Statement {
    /// Import items.
    Import {
        target: StringId,
        target_module: Option<ModuleId>,
        items: Option<Vec<LocalNodeId<DependencyItem>>>,
        attributes: Option<DependencyAttributeClause>,
    },
    /// Export items.
    Export {
        target: Option<StringId>,
        target_module: Option<ModuleId>,
        items: Vec<LocalNodeId<DependencyItem>>,
        attributes: Option<DependencyAttributeClause>,
    },
    /// Default export.
    ExportDefault { value: LocalNodeId<Expression> },

    /// Declaration statement.
    Declaration {
        declaration: LocalNodeId<Declaration>,
    },
    /// Block of statements.
    Block { block: LocalNodeId<Block> },
    /// Labelled statement (like `label: stmt`).
    Labelled {
        label: StringId,
        body: LocalNodeId<Statement>,
    },

    /// Let binding.
    Let {
        is_exported: bool,
        mutability: Mutability,
        declarators: Vec<LocalNodeId<Declarator>>,
    },
    /// Var binding.
    Var {
        is_exported: bool,
        declarators: Vec<LocalNodeId<Declarator>>,
    },
    /// Using binding.
    Using {
        asynchrony: Asynchrony,
        is_exported: bool,
        declarators: Vec<LocalNodeId<Declarator>>,
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
    /// Do while statement.
    DoWhile {
        body: LocalNodeId<Block>,
        condition: LocalNodeId<Expression>,
    },
    /// For statement.
    For {
        initialization: Option<ForInitialization>,
        condition: Option<LocalNodeId<Expression>>,
        increment: Option<LocalNodeId<Expression>>,
        body: LocalNodeId<Block>,
    },
    /// For of statement.
    ForOf {
        asynchrony: Asynchrony,
        keyword: Option<BindingKeyword>,
        pattern: LocalNodeId<Pattern>,
        iterator: LocalNodeId<Expression>,
        body: LocalNodeId<Block>,
    },
    /// Switch statement.
    Switch {
        value: LocalNodeId<Expression>,
        cases: Vec<LocalNodeId<SwitchCase>>,
    },

    /// Try statement.
    Try {
        try_block: LocalNodeId<Block>,
        catch_clause: Option<LocalNodeId<CatchClause>>,
        finally_block: Option<LocalNodeId<Block>>,
    },
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
    /// Debugger statement.
    Debugger,
}

impl Node for Statement {
    const TYPE: NodeType = NodeType::Statement;
}

/// The binding keyword used by a for each binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum BindingKeyword {
    /// `var` binding keyword.
    Var,
    /// `let` binding keyword.
    Let,
    /// `const` binding keyword.
    Const,
}

/// The initializer of one for statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ForInitialization {
    /// One expression initializer.
    Expression(LocalNodeId<Expression>),
    /// One declaration initializer.
    Declaration {
        keyword: BindingKeyword,
        declarators: Vec<LocalNodeId<Declarator>>,
    },
}

impl Statement {
    /// Returns true if this statement needs a trailing semicolon.
    pub fn needs_semicolon(&self) -> bool {
        match self {
            // block-based statements don't need semicolons
            Statement::If { .. }
            | Statement::While { .. }
            | Statement::For { .. }
            | Statement::ForOf { .. }
            | Statement::Switch { .. }
            | Statement::Try { .. }
            | Statement::Block { .. }
            | Statement::Labelled { .. } => false,

            // declarations (function, class, etc.) typically don't need semicolons
            Statement::Declaration { .. } => false,

            // all other statements need semicolons
            Statement::Import { .. }
            | Statement::Export { .. }
            | Statement::ExportDefault { .. }
            | Statement::Let { .. }
            | Statement::Var { .. }
            | Statement::Using { .. }
            | Statement::Assign { .. }
            | Statement::Expression { .. }
            | Statement::DoWhile { .. }
            | Statement::Throw { .. }
            | Statement::Continue { .. }
            | Statement::Break { .. }
            | Statement::Return { .. }
            | Statement::Debugger => true,
        }
    }
}

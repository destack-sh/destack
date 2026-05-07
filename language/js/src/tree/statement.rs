use serde::{Deserialize, Serialize};

use crate::{
    AssignOperator, Asynchrony, Block, CatchClause, Declaration, Declarator, DependencyBinding,
    DependencyItem, DependencySpace, Expression, LocalNodeId, Mutability, Node, NodeType, Pattern,
    Property, StringId, SwitchCase,
};
use destack_source::ModuleId;

/// The kind of one dependency attribute clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyAttributeClauseKind {
    /// The standard `with` attribute clause keyword.
    With,
    /// The legacy `assert` attribute clause keyword.
    Assert,
}

/// One dependency attribute clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DependencyAttributeClause {
    /// The clause introducer.
    pub kind: DependencyAttributeClauseKind,
    /// The attribute entries inside the clause body.
    pub properties: Vec<LocalNodeId<Property>>,
}

/// A Statement is a JS/TS top-level statement in some container/block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    /// Import items (including type items).
    Import {
        space: DependencySpace,
        target: StringId,
        target_module: Option<ModuleId>,
        items: Option<Vec<LocalNodeId<DependencyItem>>>,
        attributes: Option<DependencyAttributeClause>,
    },
    /// Export items (including type items).
    Export {
        space: DependencySpace,
        target: Option<StringId>,
        target_module: Option<ModuleId>,
        items: Vec<LocalNodeId<DependencyItem>>,
        attributes: Option<DependencyAttributeClause>,
    },
    /// Export value.
    ExportValue { value: LocalNodeId<Expression> },

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
        export: Option<DependencyBinding>,
        is_ambient: bool,
        mutability: Mutability,
        declarators: Vec<LocalNodeId<Declarator>>,
    },
    /// Var binding.
    Var {
        export: Option<DependencyBinding>,
        is_ambient: bool,
        declarators: Vec<LocalNodeId<Declarator>>,
    },
    /// Using binding.
    Using {
        asynchrony: Asynchrony,
        export: Option<DependencyBinding>,
        is_ambient: bool,
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
    /// For in statement.
    ForIn {
        keyword: Option<BindingKeyword>,
        pattern: LocalNodeId<Pattern>,
        iterator: LocalNodeId<Expression>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingKeyword {
    /// `var` binding keyword.
    Var,
    /// `let` binding keyword.
    Let,
    /// `const` binding keyword.
    Const,
}

/// The initializer of one for statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Return whether this statement is type only in plain js output.
    pub fn is_type_only(&self, tree: &crate::Tree) -> bool {
        match self {
            Self::Declaration { declaration } => {
                let declaration = tree.get(*declaration);
                declaration.is_type_only()
            }
            Self::Expression { expression } => {
                let expression = tree.get(*expression);
                expression.is_type_only(tree)
            }
            Self::Labelled { body, .. } => {
                let body = tree.get(*body);
                body.is_type_only(tree)
            }
            Self::Let { is_ambient, .. }
            | Self::Var { is_ambient, .. }
            | Self::Using { is_ambient, .. } => *is_ambient,
            _ => false,
        }
    }

    /// Returns true if this statement needs a trailing semicolon.
    pub fn needs_semicolon(&self) -> bool {
        match self {
            // block-based statements don't need semicolons
            Statement::If { .. }
            | Statement::While { .. }
            | Statement::For { .. }
            | Statement::ForIn { .. }
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
            | Statement::ExportValue { .. }
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

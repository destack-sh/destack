use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    AssignPattern, Asynchrony, Block, CatchClause, Declaration, Declarator, ExportKind,
    ExportSpecifier, Expression, Identifier, ImportAttribute, ImportClause, LocalNodeId,
    ModuleExportName, Mutability, Node, NodeType, Pattern, ReExportSpecifier, StringLiteral,
    SwitchCase,
};

/// One JavaScript statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Statement {
    /// Import items.
    Import {
        source: StringLiteral,
        clause: Option<ImportClause>,
        attributes: Option<Vec<LocalNodeId<ImportAttribute>>>,
    },
    /// Export local bindings.
    Export {
        specifiers: Vec<LocalNodeId<ExportSpecifier>>,
    },
    /// Re-export bindings from another module.
    ReExport {
        source: StringLiteral,
        specifiers: Vec<LocalNodeId<ReExportSpecifier>>,
        attributes: Option<Vec<LocalNodeId<ImportAttribute>>>,
    },
    /// Re-export every binding from another module.
    ExportAll {
        exported: Option<ModuleExportName>,
        source: StringLiteral,
        attributes: Option<Vec<LocalNodeId<ImportAttribute>>>,
    },
    /// Default export.
    ExportDefault { value: LocalNodeId<Expression> },

    /// Declaration statement.
    Declaration {
        /// The declaration export.
        export: Option<ExportKind>,
        /// The declared class or function.
        declaration: LocalNodeId<Declaration>,
    },
    /// Block of statements.
    Block { block: LocalNodeId<Block> },
    /// Labelled statement (like `label: stmt`).
    Labelled {
        label: Identifier,
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
        declarators: Vec<LocalNodeId<Declarator>>,
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
        target: IterationTarget,
        iterator: LocalNodeId<Expression>,
        body: LocalNodeId<Block>,
    },
    /// For of statement.
    ForOf {
        asynchrony: Asynchrony,
        target: IterationTarget,
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
    Continue { label: Option<Identifier> },
    /// Break statement.
    Break { label: Option<Identifier> },
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

/// The target of one iteration statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum IterationTarget {
    /// One declared binding.
    Binding {
        keyword: BindingKeyword,
        pattern: LocalNodeId<Pattern>,
    },
    /// One assignment target.
    Assignment { pattern: LocalNodeId<AssignPattern> },
    /// One `for of` resource binding.
    Using {
        asynchrony: Asynchrony,
        pattern: LocalNodeId<Pattern>,
    },
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
    /// Return whether this statement needs a trailing semicolon.
    pub(crate) fn needs_semicolon(&self) -> bool {
        match self {
            // statements ending in blocks
            Statement::If { .. }
            | Statement::While { .. }
            | Statement::For { .. }
            | Statement::ForIn { .. }
            | Statement::ForOf { .. }
            | Statement::Switch { .. }
            | Statement::Try { .. }
            | Statement::Block { .. }
            | Statement::Labelled { .. } => false,

            // declarations
            Statement::Declaration { .. } => false,

            // statements ending in semicolons
            Statement::Import { .. }
            | Statement::Export { .. }
            | Statement::ReExport { .. }
            | Statement::ExportAll { .. }
            | Statement::ExportDefault { .. }
            | Statement::Let { .. }
            | Statement::Var { .. }
            | Statement::Using { .. }
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

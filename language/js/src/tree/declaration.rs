use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    Block, ExportKind, Expression, FunctionSignature, LocalNodeId, Member, Name, Node, NodeType,
};

/// A class declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ClassDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export binding of the declaration.
    pub export: Option<ExportKind>,
    /// The optional extended class expression.
    pub extends_expression: Option<LocalNodeId<Expression>>,
    /// The class members.
    pub members: Vec<LocalNodeId<Member>>,
}

/// A function declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FunctionDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export binding of the declaration.
    pub export: Option<ExportKind>,
    /// The function signature.
    pub signature: FunctionSignature,
    /// The function body.
    pub body: LocalNodeId<Block>,
}

/// A declaration item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Declaration {
    /// Class declaration.
    Class(ClassDeclaration),
    /// Function declaration.
    Function(FunctionDeclaration),
}

impl Node for Declaration {
    const TYPE: NodeType = NodeType::Declaration;
}

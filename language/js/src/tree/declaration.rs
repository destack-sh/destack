use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Block, Expression, FunctionSignature, Identifier, LocalNodeId, Member, Node, NodeType,
};

/// One class declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ClassDeclaration {
    /// The declared name.
    pub name: Option<Identifier>,
    /// The optional extended class expression.
    pub extends_expression: Option<LocalNodeId<Expression>>,
    /// The class members.
    pub members: Vec<LocalNodeId<Member>>,
}

/// One function declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FunctionDeclaration {
    /// The declared name.
    pub name: Option<Identifier>,
    /// The function signature.
    pub signature: FunctionSignature,
    /// The function body.
    pub body: LocalNodeId<Block>,
}

/// One class or function declaration.
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

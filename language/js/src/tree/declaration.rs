use serde::{Deserialize, Serialize};

use crate::{
    Block, DependencyBinding, Expression, FunctionSignature, GenericParameter, LocalNodeId, Member,
    Name, Node, NodeType, Statement, StringId, TypeExpression, TypeMember,
};

/// A global augmentation declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalDeclaration {
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
    /// The statements inside the global body.
    pub statements: Vec<LocalNodeId<Statement>>,
}

/// A namespace declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamespaceDeclaration {
    /// The namespace name.
    pub name: Option<Name>,
    /// The export binding of the declaration.
    pub export: Option<DependencyBinding>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
    /// The statements inside the namespace body.
    pub statements: Vec<LocalNodeId<Statement>>,
}

/// A type alias declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export binding of the declaration.
    pub export: Option<DependencyBinding>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The declared type value.
    pub value: LocalNodeId<TypeExpression>,
}

/// A class declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export binding of the declaration.
    pub export: Option<DependencyBinding>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
    /// Whether the declaration is abstract.
    pub is_abstract: bool,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The optional extended class expression.
    pub extends_expression: Option<LocalNodeId<Expression>>,
    /// The generic arguments applied to the extended class expression.
    pub extends_generic_arguments: Vec<LocalNodeId<TypeExpression>>,
    /// The implemented interface types.
    pub implements_types: Vec<LocalNodeId<TypeExpression>>,
    /// The class members.
    pub members: Vec<LocalNodeId<Member>>,
}

/// One interface heritage clause item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceHeritage {
    /// The extended interface expression.
    pub expression: LocalNodeId<Expression>,
    /// The type arguments applied to the extended interface expression.
    pub type_arguments: Vec<LocalNodeId<TypeExpression>>,
}

/// An interface declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export binding of the declaration.
    pub export: Option<DependencyBinding>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The extended interfaces.
    pub extends: Vec<InterfaceHeritage>,
    /// The interface members.
    pub members: Vec<LocalNodeId<TypeMember>>,
}

/// An enum declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export binding of the declaration.
    pub export: Option<DependencyBinding>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
    /// The enum fields.
    pub fields: Vec<LocalNodeId<EnumField>>,
}

/// A function declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export binding of the declaration.
    pub export: Option<DependencyBinding>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
    /// Whether the declaration is abstract.
    pub is_abstract: bool,
    /// The function signature.
    pub signature: FunctionSignature,
    /// The optional function body.
    pub body: Option<LocalNodeId<Block>>,
}

/// A Declaration is a declaration in some namespace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Declaration {
    /// Global augmentation declaration.
    Global(GlobalDeclaration),
    /// Namespace declaration.
    Namespace(NamespaceDeclaration),
    /// Type alias declaration.
    Type(TypeDeclaration),
    /// Class declaration.
    Class(ClassDeclaration),
    /// Interface declaration.
    Interface(InterfaceDeclaration),
    /// Enum declaration.
    Enum(EnumDeclaration),
    /// Function declaration.
    Function(FunctionDeclaration),
}

impl Node for Declaration {
    const TYPE: NodeType = NodeType::Declaration;
}

impl Declaration {
    /// Return whether this declaration is type only in plain js output.
    pub fn is_type_only(&self) -> bool {
        match self {
            Self::Global(GlobalDeclaration { is_ambient, .. })
            | Self::Namespace(NamespaceDeclaration { is_ambient, .. })
            | Self::Type(TypeDeclaration { is_ambient, .. })
            | Self::Class(ClassDeclaration { is_ambient, .. })
            | Self::Interface(InterfaceDeclaration { is_ambient, .. })
            | Self::Enum(EnumDeclaration { is_ambient, .. })
            | Self::Function(FunctionDeclaration { is_ambient, .. }) => {
                *is_ambient || matches!(self, Self::Type(_) | Self::Interface(_))
            }
        }
    }
}

/// An EnumField is a named field of an enum declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumField {
    /// The name of the enum field.
    pub name: StringId,
    /// The value of the enum field.
    pub value: Option<LocalNodeId<Expression>>,
}

impl Node for EnumField {
    const TYPE: NodeType = NodeType::EnumField;
}

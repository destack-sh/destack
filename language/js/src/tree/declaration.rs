use crate::{
    BindingAnchor, Block, DependencyMode, Expression, FunctionSignature, GenericParameter,
    LocalNodeId, Member, Name, Node, NodeType, Statement, StringId, TypeExpression, TypeMember,
};

/// The kind of declaration.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DeclarationKind {
    /// Declare.
    Declaration,
    /// Definition.
    Definition,
}

/// The abstraction level of a declaration.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DeclarationAbstraction {
    /// Abstract declaration.
    Abstract,
    /// Concrete declaration.
    Concrete,
}

/// The descriptor for a declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclarationDescriptor {
    /// The kind of declaration.
    pub kind: DeclarationKind,
    /// The abstraction level of the declaration.
    pub abstraction: DeclarationAbstraction,
    /// The anchor of the declaration.
    pub anchor: BindingAnchor,
    /// The name of the declaration.
    pub name: Option<Name>,
    /// The export type of the declaration.
    pub export: Option<DependencyMode>,
}

/// A global augmentation declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalDeclaration {
    /// The declaration descriptor.
    pub descriptor: DeclarationDescriptor,
    /// The statements inside the global body.
    pub statements: Vec<LocalNodeId<Statement>>,
}

/// A namespace declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct NamespaceDeclaration {
    /// The declaration descriptor.
    pub descriptor: DeclarationDescriptor,
    /// The statements inside the namespace body.
    pub statements: Vec<LocalNodeId<Statement>>,
}

/// A type alias declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDeclaration {
    /// The declaration descriptor.
    pub descriptor: DeclarationDescriptor,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The declared type value.
    pub value: LocalNodeId<TypeExpression>,
}

/// A class declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDeclaration {
    /// The declaration descriptor.
    pub descriptor: DeclarationDescriptor,
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

/// An interface declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDeclaration {
    /// The declaration descriptor.
    pub descriptor: DeclarationDescriptor,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The extended interface types.
    pub extends_types: Vec<LocalNodeId<TypeExpression>>,
    /// The interface members.
    pub members: Vec<LocalNodeId<TypeMember>>,
}

/// An enum declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDeclaration {
    /// The declaration descriptor.
    pub descriptor: DeclarationDescriptor,
    /// The enum fields.
    pub fields: Vec<LocalNodeId<EnumField>>,
}

/// A function declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDeclaration {
    /// The declaration descriptor.
    pub descriptor: DeclarationDescriptor,
    /// The function signature.
    pub signature: FunctionSignature,
    /// The optional function body.
    pub body: Option<LocalNodeId<Block>>,
}

/// A Declaration is a declaration in some namespace.
#[derive(Debug, Clone, PartialEq)]
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
            Self::Global(GlobalDeclaration { descriptor, .. })
            | Self::Namespace(NamespaceDeclaration { descriptor, .. })
            | Self::Type(TypeDeclaration { descriptor, .. })
            | Self::Class(ClassDeclaration { descriptor, .. })
            | Self::Interface(InterfaceDeclaration { descriptor, .. })
            | Self::Enum(EnumDeclaration { descriptor, .. })
            | Self::Function(FunctionDeclaration { descriptor, .. }) => {
                descriptor.kind == DeclarationKind::Declaration
                    || matches!(self, Self::Type(_) | Self::Interface(_))
            }
        }
    }
}

/// An EnumField is a named field of an enum declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumField {
    /// The name of the enum field.
    pub name: StringId,
    /// The value of the enum field.
    pub value: Option<LocalNodeId<Expression>>,
}

impl Node for EnumField {
    const TYPE: NodeType = NodeType::EnumField;
}

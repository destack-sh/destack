use serde::{Deserialize, Serialize};

use crate::{
    Ambientness, DependencyKind, ExportMode, Expression, FunctionSignature, GenericParameter,
    LocalNodeId, Member, Mutability, Name, Node, NodeType, Path, StringId, TypeExpression,
    WhereClause,
};

/// The source keyword used for a namespace declaration.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NamespaceKind {
    /// `namespace Foo {}`.
    #[default]
    Namespace,
    /// `module Foo {}` or `module "foo" {}`.
    Module,
}

/// The target of an import-alias declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImportAliasTarget {
    /// A require-based alias target.
    Require { target: StringId },
    /// A qualified path alias target.
    Path { path: Path },
}

/// A global augmentation declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalDeclaration {
    /// Whether the declaration is ambient.
    pub ambient: Ambientness,
    /// The expressions inside the global augmentation body.
    pub expressions: Vec<LocalNodeId<Expression>>,
}

/// A namespace declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamespaceDeclaration {
    /// The namespace name.
    pub name: Name,
    /// The export mode of the declaration.
    pub export: Option<ExportMode>,
    /// Whether the declaration is ambient.
    pub ambient: Ambientness,
    /// The source namespace keyword.
    pub kind: NamespaceKind,
    /// The generic parameters of the namespace.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the namespace.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The expressions inside the namespace body.
    pub expressions: Vec<LocalNodeId<Expression>>,
}

/// A type declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeDeclaration {
    /// The declared name.
    pub name: Name,
    /// The export mode of the declaration.
    pub export: Option<ExportMode>,
    /// Whether the declaration is ambient.
    pub ambient: Ambientness,
    /// Whether the declaration is nominal.
    pub is_nominal: bool,
    /// The optional mutability qualifier.
    pub mutability: Option<Mutability>,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the declaration.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The declared type expression.
    pub value: LocalNodeId<TypeExpression>,
}

/// An import alias declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportAliasDeclaration {
    /// The declared name.
    pub name: Name,
    /// The export mode of the declaration.
    pub export: Option<ExportMode>,
    /// Whether the declaration is ambient.
    pub ambient: Ambientness,
    /// The import alias dependency kind.
    pub kind: DependencyKind,
    /// The alias target.
    pub target: ImportAliasTarget,
}

/// A struct declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructDeclaration {
    /// The declared name.
    pub name: Name,
    /// The export mode of the declaration.
    pub export: Option<ExportMode>,
    /// Whether the declaration is ambient.
    pub ambient: Ambientness,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the declaration.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The implemented interfaces.
    pub implements_types: Vec<LocalNodeId<TypeExpression>>,
    /// The embedded types.
    pub embedded_types: Vec<LocalNodeId<TypeExpression>>,
    /// The struct members.
    pub members: Vec<LocalNodeId<Member>>,
}

/// A class declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export mode of the declaration.
    pub export: Option<ExportMode>,
    /// Whether the declaration is ambient.
    pub ambient: Ambientness,
    /// Whether the declaration is abstract.
    pub is_abstract: bool,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the declaration.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The extended class.
    pub extends_type: Option<LocalNodeId<TypeExpression>>,
    /// The implemented interfaces.
    pub implements_types: Vec<LocalNodeId<TypeExpression>>,
    /// The class members.
    pub members: Vec<LocalNodeId<Member>>,
}

/// The kind of an enum declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum EnumKind {
    /// A regular enum.
    #[default]
    Enum,
    /// A const enum.
    Const,
}

/// An enum declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export mode of the declaration.
    pub export: Option<ExportMode>,
    /// Whether the declaration is ambient.
    pub ambient: Ambientness,
    /// The enum kind.
    pub kind: EnumKind,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the declaration.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The implemented interfaces.
    pub implements_types: Vec<LocalNodeId<TypeExpression>>,
    /// The enum fields.
    pub fields: Vec<LocalNodeId<EnumField>>,
    /// The enum members.
    pub members: Vec<LocalNodeId<Member>>,
}

/// An interface declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export mode of the declaration.
    pub export: Option<ExportMode>,
    /// Whether the declaration is ambient.
    pub ambient: Ambientness,
    /// Whether the interface is nominal.
    pub is_nominal: bool,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the declaration.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The extended interfaces.
    pub extends_types: Vec<LocalNodeId<TypeExpression>>,
    /// The interface members.
    pub members: Vec<LocalNodeId<Member>>,
}

/// An extension declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export mode of the declaration.
    pub export: Option<ExportMode>,
    /// Whether the declaration is ambient.
    pub ambient: Ambientness,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the declaration.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The extended target type.
    pub target_type: LocalNodeId<TypeExpression>,
    /// The implemented interfaces.
    pub implements_types: Vec<LocalNodeId<TypeExpression>>,
    /// The extension members.
    pub members: Vec<LocalNodeId<Member>>,
}

/// A function declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export mode of the declaration.
    pub export: Option<ExportMode>,
    /// Whether the declaration is ambient.
    pub ambient: Ambientness,
    /// The function signature.
    pub signature: FunctionSignature,
    /// The optional function body.
    pub body: Option<LocalNodeId<Expression>>,
}

/// Declaration introduces a type or such into a scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Declaration {
    /// Global augmentation declaration.
    Global(GlobalDeclaration),
    /// Namespace declaration.
    Namespace(NamespaceDeclaration),
    /// Type declaration.
    Type(TypeDeclaration),
    /// Import-alias declaration.
    ImportAlias(ImportAliasDeclaration),
    /// Struct declaration.
    Struct(StructDeclaration),
    /// Class declaration.
    Class(ClassDeclaration),
    /// Enum declaration.
    Enum(EnumDeclaration),
    /// Interface declaration.
    Interface(InterfaceDeclaration),
    /// Extension declaration.
    Extension(ExtensionDeclaration),
    /// Function declaration.
    Function(FunctionDeclaration),
}

impl Node for Declaration {
    const TYPE: NodeType = NodeType::Declaration;
}

impl Declaration {
    /// Get the name of the declaration.
    #[inline]
    pub fn name(&self) -> Option<Name> {
        match self {
            Declaration::Global(_) => None,
            Declaration::Namespace(declaration) => Some(declaration.name),
            Declaration::Type(declaration) => Some(declaration.name),
            Declaration::ImportAlias(declaration) => Some(declaration.name),
            Declaration::Struct(declaration) => Some(declaration.name),
            Declaration::Class(declaration) => declaration.name,
            Declaration::Enum(declaration) => declaration.name,
            Declaration::Interface(declaration) => declaration.name,
            Declaration::Extension(declaration) => declaration.name,
            Declaration::Function(declaration) => declaration.name,
        }
    }

    /// Get the generic parameters of the declaration.
    #[inline]
    pub fn generic_parameters(&self) -> Option<&Vec<LocalNodeId<GenericParameter>>> {
        match self {
            Declaration::Namespace(declaration) => Some(&declaration.generic_parameters),
            Declaration::Type(declaration) => Some(&declaration.generic_parameters),
            Declaration::Struct(declaration) => Some(&declaration.generic_parameters),
            Declaration::Class(declaration) => Some(&declaration.generic_parameters),
            Declaration::Enum(declaration) => Some(&declaration.generic_parameters),
            Declaration::Interface(declaration) => Some(&declaration.generic_parameters),
            Declaration::Extension(declaration) => Some(&declaration.generic_parameters),
            Declaration::Function(declaration) => Some(&declaration.signature.generic_parameters),
            Declaration::Global(_) | Declaration::ImportAlias(_) => None,
        }
    }
}

/// An enum field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumField {
    /// The name of the enum field.
    pub name: Name,
    /// The default value of the enum field.
    pub value: Option<LocalNodeId<Expression>>,
}

impl Node for EnumField {
    const TYPE: NodeType = NodeType::EnumField;
}

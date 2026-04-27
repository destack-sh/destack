use serde::{Deserialize, Serialize};

use crate::{
    Ambientness, DependencyKind, ExportMode, Expression, FunctionSignature, GenericArgument,
    GenericParameter, GlobalSymbolId, LocalNodeId, LocalScopeId, LocalSymbolId, Member, Mutability,
    Name, Node, NodeType, Path, StringId, TypeExpression, TypeMember, WhereClause,
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
    /// The declaration symbol.
    pub symbol: LocalSymbolId,
    /// The declaration scope.
    pub scope: LocalScopeId,
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
    /// The declaration symbol.
    pub symbol: LocalSymbolId,
    /// The source namespace keyword.
    pub kind: NamespaceKind,
    /// The generic parameters of the namespace.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the namespace.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The declaration scope.
    pub scope: LocalScopeId,
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
    /// The declaration symbol.
    pub symbol: LocalSymbolId,
    /// The declaration scope.
    pub scope: LocalScopeId,
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

/// An import-alias declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportAliasDeclaration {
    /// The declared name.
    pub name: Name,
    /// The export mode of the declaration.
    pub export: Option<ExportMode>,
    /// Whether the declaration is ambient.
    pub ambient: Ambientness,
    /// The declaration symbol.
    pub symbol: LocalSymbolId,
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
    /// The declaration symbol.
    pub symbol: LocalSymbolId,
    /// The declaration scope.
    pub scope: LocalScopeId,
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
    /// The declaration symbol.
    pub symbol: LocalSymbolId,
    /// The optional symbol for `self`.
    pub self_symbol: Option<LocalSymbolId>,
    /// The declaration scope.
    pub scope: LocalScopeId,
    /// Whether the declaration is abstract.
    pub is_abstract: bool,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the declaration.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The extended class expression.
    pub extends_expression: Option<LocalNodeId<Expression>>,
    /// The generic arguments applied to the extended class expression.
    pub extends_generic_arguments: Vec<LocalNodeId<GenericArgument>>,
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
    /// The declaration symbol.
    pub symbol: LocalSymbolId,
    /// The declaration scope.
    pub scope: LocalScopeId,
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

/// One interface heritage clause item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceHeritage {
    /// The extended interface expression.
    pub expression: LocalNodeId<Expression>,
    /// The generic arguments applied to the extended interface expression.
    pub generic_arguments: Vec<LocalNodeId<GenericArgument>>,
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
    /// The declaration symbol.
    pub symbol: LocalSymbolId,
    /// The declaration scope.
    pub scope: LocalScopeId,
    /// Whether the interface is nominal.
    pub is_nominal: bool,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the declaration.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The extended interfaces.
    pub extends: Vec<InterfaceHeritage>,
    /// The interface members.
    pub members: Vec<LocalNodeId<TypeMember>>,
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
    /// The declaration symbol.
    pub symbol: LocalSymbolId,
    /// The declaration scope.
    pub scope: LocalScopeId,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the declaration.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The extended target type.
    pub target_type: LocalNodeId<TypeExpression>,
    /// The resolved target symbol.
    pub target_symbol: Option<GlobalSymbolId>,
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
    /// The declaration symbol.
    pub symbol: LocalSymbolId,
    /// The optional symbol for `self`.
    pub self_symbol: Option<LocalSymbolId>,
    /// The declaration scope.
    pub scope: LocalScopeId,
    /// The function signature.
    pub signature: FunctionSignature,
    /// The optional function body.
    pub body: Option<LocalNodeId<Expression>>,
}

/// Declaration introduces a type or function into its scope.
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

    /// Get the name of this kind of declaration.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Declaration::Global(_) => "global",
            Declaration::Namespace(declaration) => match declaration.kind {
                NamespaceKind::Namespace => "namespace",
                NamespaceKind::Module => "module",
            },
            Declaration::Type(_) => "type",
            Declaration::ImportAlias(_) => "import alias",
            Declaration::Struct(_) => "struct",
            Declaration::Class(_) => "class",
            Declaration::Enum(_) => "enum",
            Declaration::Interface(_) => "interface",
            Declaration::Extension(_) => "extension",
            Declaration::Function(_) => "function",
        }
    }

    /// Get the declaration symbol.
    pub fn symbol(&self) -> LocalSymbolId {
        match self {
            Declaration::Global(declaration) => declaration.symbol,
            Declaration::Namespace(declaration) => declaration.symbol,
            Declaration::Type(declaration) => declaration.symbol,
            Declaration::ImportAlias(declaration) => declaration.symbol,
            Declaration::Struct(declaration) => declaration.symbol,
            Declaration::Class(declaration) => declaration.symbol,
            Declaration::Enum(declaration) => declaration.symbol,
            Declaration::Interface(declaration) => declaration.symbol,
            Declaration::Extension(declaration) => declaration.symbol,
            Declaration::Function(declaration) => declaration.symbol,
        }
    }

    /// Get the symbol that owns the declaration name when one exists.
    pub fn name_symbol(&self) -> LocalSymbolId {
        match self {
            Declaration::Class(ClassDeclaration {
                self_symbol: Some(self_symbol),
                ..
            })
            | Declaration::Function(FunctionDeclaration {
                self_symbol: Some(self_symbol),
                ..
            }) => *self_symbol,
            _ => self.symbol(),
        }
    }

    /// Get the declaration scope when one exists.
    pub fn scope(&self) -> Option<LocalScopeId> {
        match self {
            Declaration::Global(declaration) => Some(declaration.scope),
            Declaration::Namespace(declaration) => Some(declaration.scope),
            Declaration::Type(declaration) => Some(declaration.scope),
            Declaration::ImportAlias(_) => None,
            Declaration::Struct(declaration) => Some(declaration.scope),
            Declaration::Class(declaration) => Some(declaration.scope),
            Declaration::Enum(declaration) => Some(declaration.scope),
            Declaration::Interface(declaration) => Some(declaration.scope),
            Declaration::Extension(declaration) => Some(declaration.scope),
            Declaration::Function(declaration) => Some(declaration.scope),
        }
    }

    /// Get the declaration-body member ids for structured declarations.
    pub fn member_ids(&self) -> Option<&[LocalNodeId<Member>]> {
        match self {
            Declaration::Struct(declaration) => Some(&declaration.members),
            Declaration::Class(declaration) => Some(&declaration.members),
            Declaration::Enum(declaration) => Some(&declaration.members),
            Declaration::Extension(declaration) => Some(&declaration.members),
            Declaration::Function(_) => None,
            _ => None,
        }
    }

    /// Get the type-surface member ids for interface declarations.
    pub fn type_member_ids(&self) -> Option<&[LocalNodeId<TypeMember>]> {
        match self {
            Declaration::Interface(declaration) => Some(&declaration.members),
            _ => None,
        }
    }

    /// Get the generic parameters of the declaration.
    pub fn generic_parameters(&self) -> Option<&[LocalNodeId<GenericParameter>]> {
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

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    ExportKind, Expression, FunctionSignature, GenericParameter, LocalNodeId, Member, Mutability,
    Name, Node, NodeFold, NodeType, ScopeKind, SymbolKind, SymbolRole, TypeExpression, TypeMember,
    Visibility, WhereClause,
};

/// A global declaration block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct GlobalDeclaration {
    /// The expressions inside the global body.
    pub expressions: Vec<LocalNodeId<Expression>>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
}

/// A module declaration block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct ModuleDeclaration {
    /// The expressions inside the module body.
    pub expressions: Vec<LocalNodeId<Expression>>,
}

/// A type declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct TypeDeclaration {
    /// The declared name.
    pub name: Name,
    /// The export kind of the declaration.
    pub export: Option<ExportKind>,
    /// Whether the declaration places its values in shared storage.
    pub is_shared: bool,
    /// The optional mutability qualifier.
    pub mutability: Option<Mutability>,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the declaration.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The backing visibility written ahead of a newtype value.
    pub backing_visibility: Option<Visibility>,
    /// The declared type expression.
    pub value: LocalNodeId<TypeExpression>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
    /// Whether the declaration is nominal.
    pub is_nominal: bool,
}

/// A struct declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct StructDeclaration {
    /// The declared name.
    pub name: Name,
    /// The export kind of the declaration.
    pub export: Option<ExportKind>,
    /// Whether the declaration places its values in shared storage.
    pub is_shared: bool,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the declaration.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The implemented interfaces.
    pub implements_types: Vec<LocalNodeId<TypeExpression>>,
    /// The struct members.
    pub members: Vec<LocalNodeId<Member>>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
}

/// A class declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct ClassDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export kind of the declaration.
    pub export: Option<ExportKind>,
    /// Whether the declaration places its values in shared storage.
    pub is_shared: bool,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the declaration.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The extended class type.
    pub extends_type: Option<LocalNodeId<TypeExpression>>,
    /// The implemented interfaces.
    pub implements_types: Vec<LocalNodeId<TypeExpression>>,
    /// The class members.
    pub members: Vec<LocalNodeId<Member>>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
    /// Whether the declaration is abstract.
    pub is_abstract: bool,
    /// Whether the declaration is final.
    pub is_final: bool,
}

/// An enum declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct EnumDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export kind of the declaration.
    pub export: Option<ExportKind>,
    /// Whether the declaration places its values in shared storage.
    pub is_shared: bool,
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
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
}

/// An interface declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct InterfaceDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export kind of the declaration.
    pub export: Option<ExportKind>,
    /// Whether the declaration places its values in shared storage.
    pub is_shared: bool,
    /// The generic parameters of the declaration.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the declaration.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The extended interface types.
    pub extends_types: Vec<LocalNodeId<TypeExpression>>,
    /// The interface members.
    pub members: Vec<LocalNodeId<TypeMember>>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
    /// Whether the interface is nominal.
    pub is_nominal: bool,
}

/// An extension declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct ExtensionDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export kind of the declaration.
    pub export: Option<ExportKind>,
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
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
}

/// A function declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct FunctionDeclaration {
    /// The declared name.
    pub name: Option<Name>,
    /// The export kind of the declaration.
    pub export: Option<ExportKind>,
    /// The function signature.
    pub signature: FunctionSignature,
    /// The optional function body.
    pub body: Option<LocalNodeId<Expression>>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
}

/// Declaration introduces a type or such into a scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub enum Declaration {
    /// Global declaration block.
    Global(GlobalDeclaration),
    /// Module declaration block.
    Module(ModuleDeclaration),
    /// Type declaration.
    Type(TypeDeclaration),
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
    /// Return the export kind on this declaration.
    pub fn export(&self) -> Option<ExportKind> {
        match self {
            Declaration::Type(declaration) => declaration.export,
            Declaration::Struct(declaration) => declaration.export,
            Declaration::Class(declaration) => declaration.export,
            Declaration::Enum(declaration) => declaration.export,
            Declaration::Interface(declaration) => declaration.export,
            Declaration::Extension(declaration) => declaration.export,
            Declaration::Function(declaration) => declaration.export,
            Declaration::Global(_) | Declaration::Module(_) => None,
        }
    }

    /// Return whether this declaration is ambient.
    pub fn is_ambient(&self) -> bool {
        match self {
            Declaration::Global(declaration) => declaration.is_ambient,
            Declaration::Type(declaration) => declaration.is_ambient,
            Declaration::Struct(declaration) => declaration.is_ambient,
            Declaration::Class(declaration) => declaration.is_ambient,
            Declaration::Enum(declaration) => declaration.is_ambient,
            Declaration::Interface(declaration) => declaration.is_ambient,
            Declaration::Extension(declaration) => declaration.is_ambient,
            Declaration::Function(declaration) => declaration.is_ambient,
            Declaration::Module(_) => false,
        }
    }

    /// Return whether this declaration places its values in shared storage.
    pub fn is_shared(&self) -> bool {
        match self {
            Declaration::Type(declaration) => declaration.is_shared,
            Declaration::Struct(declaration) => declaration.is_shared,
            Declaration::Class(declaration) => declaration.is_shared,
            Declaration::Enum(declaration) => declaration.is_shared,
            Declaration::Interface(declaration) => declaration.is_shared,
            Declaration::Global(_)
            | Declaration::Module(_)
            | Declaration::Extension(_)
            | Declaration::Function(_) => false,
        }
    }

    /// Return whether this declaration is abstract.
    pub fn is_abstract(&self) -> bool {
        match self {
            Declaration::Class(declaration) => declaration.is_abstract,
            Declaration::Function(declaration) => declaration.signature.is_abstract,
            _ => false,
        }
    }

    /// Return the symbol kind introduced by this declaration.
    pub fn symbol_kind(&self) -> Option<SymbolKind> {
        match self {
            Declaration::Global(_) | Declaration::Module(_) => None,
            Declaration::Type(declaration) => {
                if declaration.is_nominal {
                    Some(SymbolKind::Newtype)
                } else {
                    Some(SymbolKind::TypeAlias)
                }
            }
            Declaration::Struct(_) => Some(SymbolKind::Struct),
            Declaration::Class(_) => Some(SymbolKind::Class),
            Declaration::Enum(_) => Some(SymbolKind::Enum),
            Declaration::Interface(declaration) => {
                if declaration.is_nominal {
                    Some(SymbolKind::NewtypeInterface)
                } else {
                    Some(SymbolKind::Interface)
                }
            }
            Declaration::Extension(_) => Some(SymbolKind::Extension),
            Declaration::Function(_) => Some(SymbolKind::Function),
        }
    }

    /// Return the symbol role introduced by this declaration.
    pub fn symbol_role(&self) -> Option<SymbolRole> {
        match self {
            Declaration::Global(_) | Declaration::Module(_) => None,
            Declaration::Function(_) | Declaration::Type(_) => Some(SymbolRole::Item),
            _ => Some(SymbolRole::Namespace),
        }
    }

    /// Return the owned scope kind for this declaration symbol.
    pub fn symbol_scope_kind(&self) -> Option<ScopeKind> {
        match self {
            Declaration::Global(_) | Declaration::Module(_) => None,
            Declaration::Function(_) => Some(ScopeKind::Function),
            Declaration::Type(_) => Some(ScopeKind::Type),
            _ => Some(ScopeKind::Namespace),
        }
    }

    /// Return the name of the declaration.
    #[inline]
    pub fn name(&self) -> Option<Name> {
        match self {
            Declaration::Global(_) => None,
            Declaration::Module(_) => None,
            Declaration::Type(declaration) => Some(declaration.name),
            Declaration::Struct(declaration) => Some(declaration.name),
            Declaration::Class(declaration) => declaration.name,
            Declaration::Enum(declaration) => declaration.name,
            Declaration::Interface(declaration) => declaration.name,
            Declaration::Extension(declaration) => declaration.name,
            Declaration::Function(declaration) => declaration.name,
        }
    }

    /// Return the mutable declaration name.
    #[inline]
    pub fn name_mut(&mut self) -> Option<&mut Name> {
        match self {
            Declaration::Global(_) | Declaration::Module(_) => None,
            Declaration::Type(declaration) => Some(&mut declaration.name),
            Declaration::Struct(declaration) => Some(&mut declaration.name),
            Declaration::Class(declaration) => declaration.name.as_mut(),
            Declaration::Enum(declaration) => declaration.name.as_mut(),
            Declaration::Interface(declaration) => declaration.name.as_mut(),
            Declaration::Extension(declaration) => declaration.name.as_mut(),
            Declaration::Function(declaration) => declaration.name.as_mut(),
        }
    }

    /// Return the declaration kind name for display.
    #[inline]
    pub fn kind_name(&self) -> &'static str {
        match self {
            Declaration::Global(_) => "global",
            Declaration::Module(_) => "module",
            Declaration::Type(_) => "type",
            Declaration::Struct(_) => "struct",
            Declaration::Class(_) => "class",
            Declaration::Enum(_) => "enum",
            Declaration::Interface(_) => "interface",
            Declaration::Extension(_) => "extension",
            Declaration::Function(_) => "function",
        }
    }

    /// Return the body member ids for structured declarations.
    #[inline]
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

    /// Return the type member ids for interface declarations.
    #[inline]
    pub fn type_member_ids(&self) -> Option<&[LocalNodeId<TypeMember>]> {
        match self {
            Declaration::Interface(declaration) => Some(&declaration.members),
            _ => None,
        }
    }

    /// Return the generic parameters of the declaration.
    #[inline]
    pub fn generic_parameters(&self) -> Option<&[LocalNodeId<GenericParameter>]> {
        match self {
            Declaration::Type(declaration) => Some(&declaration.generic_parameters),
            Declaration::Struct(declaration) => Some(&declaration.generic_parameters),
            Declaration::Class(declaration) => Some(&declaration.generic_parameters),
            Declaration::Enum(declaration) => Some(&declaration.generic_parameters),
            Declaration::Interface(declaration) => Some(&declaration.generic_parameters),
            Declaration::Extension(declaration) => Some(&declaration.generic_parameters),
            Declaration::Function(declaration) => Some(&declaration.signature.generic_parameters),
            Declaration::Global(_) | Declaration::Module(_) => None,
        }
    }

    /// Return the where clauses of the declaration.
    #[inline]
    pub fn where_clauses(&self) -> Option<&[LocalNodeId<WhereClause>]> {
        match self {
            Declaration::Type(declaration) => Some(&declaration.where_clauses),
            Declaration::Struct(declaration) => Some(&declaration.where_clauses),
            Declaration::Class(declaration) => Some(&declaration.where_clauses),
            Declaration::Enum(declaration) => Some(&declaration.where_clauses),
            Declaration::Interface(declaration) => Some(&declaration.where_clauses),
            Declaration::Extension(declaration) => Some(&declaration.where_clauses),
            Declaration::Function(declaration) => Some(&declaration.signature.where_clauses),
            Declaration::Global(_) | Declaration::Module(_) => None,
        }
    }

    /// Return the implemented interface types of the declaration.
    #[inline]
    pub fn implements_types(&self) -> Option<&[LocalNodeId<TypeExpression>]> {
        match self {
            Declaration::Struct(declaration) => Some(&declaration.implements_types),
            Declaration::Class(declaration) => Some(&declaration.implements_types),
            Declaration::Enum(declaration) => Some(&declaration.implements_types),
            Declaration::Extension(declaration) => Some(&declaration.implements_types),
            Declaration::Global(_)
            | Declaration::Module(_)
            | Declaration::Type(_)
            | Declaration::Interface(_)
            | Declaration::Function(_) => None,
        }
    }

    /// Return the mutable generic parameters of the declaration.
    #[inline]
    pub fn generic_parameters_mut(&mut self) -> Option<&mut Vec<LocalNodeId<GenericParameter>>> {
        match self {
            Declaration::Type(declaration) => Some(&mut declaration.generic_parameters),
            Declaration::Struct(declaration) => Some(&mut declaration.generic_parameters),
            Declaration::Class(declaration) => Some(&mut declaration.generic_parameters),
            Declaration::Enum(declaration) => Some(&mut declaration.generic_parameters),
            Declaration::Interface(declaration) => Some(&mut declaration.generic_parameters),
            Declaration::Extension(declaration) => Some(&mut declaration.generic_parameters),
            Declaration::Function(declaration) => {
                Some(&mut declaration.signature.generic_parameters)
            }
            Declaration::Global(_) | Declaration::Module(_) => None,
        }
    }
}

/// An enum field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
pub struct EnumField {
    /// The name of the enum field.
    pub name: Name,
    /// The default value of the enum field.
    pub value: Option<LocalNodeId<Expression>>,
}

impl Node for EnumField {
    const TYPE: NodeType = NodeType::EnumField;
}

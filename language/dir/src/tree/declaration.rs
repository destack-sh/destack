use destack_source::AdaptImage;
use serde::{Deserialize, Serialize};

use crate::{
    BindingAnchor, DependencyKind, DependencyMode, Expression, FunctionSignature, Generics,
    GlobalSymbolId, Heritage, LocalNodeId, LocalScopeId, LocalSymbolId, Member, Mutability, Name,
    Node, NodeType, Parameter, StringId, TypeKind,
};

/// The kind of declaration.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum DeclarationKind {
    /// Declare.
    Declaration,
    /// Definition.
    Definition,
}

/// The source keyword used for a namespace declaration.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, AdaptImage)]
pub enum NamespaceKind {
    /// `namespace Foo {}`.
    Namespace,
    /// `module Foo {}` or `module "foo" {}`.
    Module,
}

/// The abstraction level of a declaration.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum DeclarationAbstraction {
    /// Abstract declaration.
    Abstract,
    /// Concrete declaration.
    Concrete,
}

/// The meta data for a declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
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
    /// The symbol of the declaration.
    pub symbol: LocalSymbolId,
}

/// The target of an import-alias declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum ImportAliasTarget {
    /// A require-based alias target.
    Require { target: StringId },
    /// A qualified path alias target.
    Path { value: LocalNodeId<Expression> },
}

/// Declaration introduces a type or function into its scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum Declaration {
    /// Global augmentation declaration.
    Global {
        descriptor: DeclarationDescriptor,
        scope: LocalScopeId,
        expressions: Vec<LocalNodeId<Expression>>,
    },
    /// Namespace declaration.
    Namespace {
        descriptor: DeclarationDescriptor,
        kind: NamespaceKind,
        generics: Generics,
        scope: LocalScopeId,
        expressions: Vec<LocalNodeId<Expression>>,
    },
    /// Type alias declaration.
    Type {
        descriptor: DeclarationDescriptor,
        kind: TypeKind,
        mutability: Option<Mutability>,
        static_parameters: Option<Vec<LocalNodeId<Parameter>>>,
        value: LocalNodeId<Expression>,
    },
    /// Import-alias declaration (TypeScript `import A = B.C`).
    ImportAlias {
        descriptor: DeclarationDescriptor,
        kind: DependencyKind,
        target: ImportAliasTarget,
    },
    /// Struct declaration: nominal value type with fixed layout.
    Struct {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        heritage: Heritage,
        scope: LocalScopeId,
        members: Vec<LocalNodeId<Member>>,
    },
    /// Class declaration with reference semantics.
    Class {
        descriptor: DeclarationDescriptor,
        self_symbol: Option<LocalSymbolId>,
        generics: Generics,
        heritage: Heritage,
        scope: LocalScopeId,
        members: Vec<LocalNodeId<Member>>,
    },
    /// Enum declaration.
    Enum {
        descriptor: DeclarationDescriptor,
        kind: EnumKind,
        generics: Generics,
        heritage: Heritage,
        scope: LocalScopeId,
        fields: Vec<LocalNodeId<EnumField>>,
        members: Vec<LocalNodeId<Member>>,
    },
    /// Interface declaration.
    Interface {
        descriptor: DeclarationDescriptor,
        kind: TypeKind,
        generics: Generics,
        heritage: Heritage,
        scope: LocalScopeId,
        members: Vec<LocalNodeId<Member>>,
    },
    /// Function declaration. Nested declarations are lifted from the body.
    Function {
        descriptor: DeclarationDescriptor,
        self_symbol: Option<LocalSymbolId>,
        signature: FunctionSignature,
        scope: LocalScopeId,
        body: Option<LocalNodeId<Expression>>,
    },
    /// Extension declaration.
    Extension {
        descriptor: DeclarationDescriptor,
        generics: Generics,
        target_type: LocalNodeId<Expression>,
        target_symbol: Option<GlobalSymbolId>,
        heritage: Heritage,
        scope: LocalScopeId,
        members: Vec<LocalNodeId<Member>>,
    },
}

impl Node for Declaration {
    const TYPE: NodeType = NodeType::Declaration;
}

impl Declaration {
    /// Get the name of this kind of declaration.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Declaration::Global { .. } => "global",
            Declaration::Namespace { kind, .. } => match kind {
                NamespaceKind::Namespace => "namespace",
                NamespaceKind::Module => "module",
            },
            Declaration::Type { .. } => "type",
            Declaration::ImportAlias { .. } => "import alias",
            Declaration::Struct { .. } => "struct",
            Declaration::Class { .. } => "class",
            Declaration::Enum { .. } => "enum",
            Declaration::Interface { .. } => "interface",
            Declaration::Function { .. } => "function",
            Declaration::Extension { .. } => "extension",
        }
    }

    /// Get the descriptor of the declaration.
    pub fn descriptor(&self) -> &DeclarationDescriptor {
        match self {
            Declaration::Global { descriptor, .. } => descriptor,
            Declaration::Namespace { descriptor, .. } => descriptor,
            Declaration::Type { descriptor, .. } => descriptor,
            Declaration::ImportAlias { descriptor, .. } => descriptor,
            Declaration::Struct { descriptor, .. } => descriptor,
            Declaration::Class { descriptor, .. } => descriptor,
            Declaration::Enum { descriptor, .. } => descriptor,
            Declaration::Interface { descriptor, .. } => descriptor,
            Declaration::Function { descriptor, .. } => descriptor,
            Declaration::Extension { descriptor, .. } => descriptor,
        }
    }

    /// Get the symbol of the declaration.
    pub fn symbol(&self) -> LocalSymbolId {
        self.descriptor().symbol
    }

    /// Get the symbol that owns the declaration name when one exists.
    pub fn name_symbol(&self) -> LocalSymbolId {
        match self {
            Declaration::Class {
                self_symbol: Some(self_symbol),
                ..
            }
            | Declaration::Function {
                self_symbol: Some(self_symbol),
                ..
            } => *self_symbol,
            _ => self.symbol(),
        }
    }

    /// Get the scope of the declaration.
    /// Returns None for Declaration::Type which has no scope.
    pub fn scope(&self) -> Option<LocalScopeId> {
        match self {
            Declaration::Global { scope, .. } => Some(*scope),
            Declaration::Namespace { scope, .. } => Some(*scope),
            Declaration::Type { .. } | Declaration::ImportAlias { .. } => None,
            Declaration::Struct { scope, .. } => Some(*scope),
            Declaration::Class { scope, .. } => Some(*scope),
            Declaration::Enum { scope, .. } => Some(*scope),
            Declaration::Interface { scope, .. } => Some(*scope),
            Declaration::Function { scope, .. } => Some(*scope),
            Declaration::Extension { scope, .. } => Some(*scope),
        }
    }

    /// Get the member ids for structured declarations.
    pub fn member_ids(&self) -> Option<&[LocalNodeId<Member>]> {
        match self {
            Declaration::Struct { members, .. }
            | Declaration::Class { members, .. }
            | Declaration::Enum { members, .. }
            | Declaration::Interface { members, .. }
            | Declaration::Extension { members, .. } => Some(members),
            Declaration::Global { .. }
            | Declaration::Namespace { .. }
            | Declaration::Type { .. }
            | Declaration::ImportAlias { .. }
            | Declaration::Function { .. } => None,
        }
    }

    /// Get the heritage block for declarations that carry one.
    pub fn heritage(&self) -> Option<&Heritage> {
        match self {
            Declaration::Struct { heritage, .. }
            | Declaration::Class { heritage, .. }
            | Declaration::Enum { heritage, .. }
            | Declaration::Interface { heritage, .. }
            | Declaration::Extension { heritage, .. } => Some(heritage),
            Declaration::Global { .. }
            | Declaration::Namespace { .. }
            | Declaration::Type { .. }
            | Declaration::ImportAlias { .. }
            | Declaration::Function { .. } => None,
        }
    }

    /// Get the static parameters of the declaration.
    #[inline]
    pub fn static_parameters(&self) -> Option<&Vec<LocalNodeId<Parameter>>> {
        match self {
            Declaration::Function { signature, .. } => signature
                .generics
                .as_ref()
                .and_then(|g| g.static_parameters.as_ref()),
            Declaration::Namespace { generics, .. }
            | Declaration::Struct { generics, .. }
            | Declaration::Class { generics, .. }
            | Declaration::Interface { generics, .. }
            | Declaration::Enum { generics, .. }
            | Declaration::Extension { generics, .. } => generics.static_parameters.as_ref(),
            Declaration::Type {
                static_parameters, ..
            } => static_parameters.as_ref(),
            Declaration::ImportAlias { .. } => None,
            Declaration::Global { .. } => None,
        }
    }
}

/// The kind of an enum declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, AdaptImage)]
pub enum EnumKind {
    /// A regular enum.
    #[default]
    Enum,
    /// A const enum.
    Const,
}

/// An enum field is a named field of an enum declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct EnumField {
    /// The name of the enum field.
    pub name: StringId,
    /// The value of the enum field.
    pub value: Option<LocalNodeId<Expression>>,
    /// The symbol of the enum field.
    pub symbol: LocalSymbolId,
}

impl Node for EnumField {
    const TYPE: NodeType = NodeType::EnumField;
}

use destack_core::{FxIndexMap, FxIndexSet, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{LocalNodeId, LocalNodeIdAny, Node, StaticKey};

/// One keyed member of a canonical language item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct LanguageMember {
    /// The stable key of the canonical owner.
    pub owner: StringId,
    /// The member key.
    pub key: StaticKey,
}

/// Canonical language identities retained by one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct LanguageTable {
    /// Canonical items keyed by their MIR declarations.
    items: FxIndexMap<LocalNodeIdAny, StringId>,
    /// Canonical members declared by MIR declarations.
    declared_members: FxIndexMap<LocalNodeIdAny, LanguageMember>,
    /// Canonical members implemented by MIR declarations.
    implemented_members: FxIndexSet<(LocalNodeIdAny, LanguageMember)>,
}

impl LanguageTable {
    /// Create an empty language table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the canonical item represented by one declaration.
    pub fn set_item<T>(&mut self, node: LocalNodeId<T>, item: StringId) -> Option<StringId>
    where
        T: Node,
    {
        self.items.insert(node.into_any(), item)
    }

    /// Return the canonical item represented by one declaration.
    pub fn item<T>(&self, node: LocalNodeId<T>) -> Option<StringId>
    where
        T: Node,
    {
        self.items.get(&node.into_any()).copied()
    }

    /// Record one canonical member declared by a MIR declaration.
    pub fn declare_member<T>(
        &mut self,
        node: LocalNodeId<T>,
        member: LanguageMember,
    ) -> Option<LanguageMember>
    where
        T: Node,
    {
        self.declared_members.insert(node.into_any(), member)
    }

    /// Record one canonical member implemented by a MIR declaration.
    pub fn implement_member<T>(&mut self, node: LocalNodeId<T>, member: LanguageMember)
    where
        T: Node,
    {
        self.implemented_members.insert((node.into_any(), member));
    }

    /// Return whether one declaration declares the canonical member.
    pub fn declares_member<T>(&self, node: LocalNodeId<T>, member: LanguageMember) -> bool
    where
        T: Node,
    {
        self.declared_members.get(&node.into_any()) == Some(&member)
    }

    /// Return whether one declaration implements the canonical member.
    pub fn implements_member<T>(&self, node: LocalNodeId<T>, member: LanguageMember) -> bool
    where
        T: Node,
    {
        let node = node.into_any();

        self.implemented_members.contains(&(node, member))
            || self.declared_members.get(&node) == Some(&member)
    }
}

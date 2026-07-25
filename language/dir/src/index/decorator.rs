use crate::{
    Decorator, DecoratorTarget, Expression, GlobalNodeId, GlobalNodeIdAny, LocalDecoratorId,
    Postings,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Indexed decorator applications.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DecoratorIndex {
    /// The decorator applications in stable order.
    entries: Vec<DecoratorEntry>,
}

/// Decorator postings by name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct DecoratorPostings {
    /// Named decorator postings.
    pub names: Postings<String>,
}

/// One indexed decorator application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DecoratorEntry {
    /// The decorator name when syntactically known.
    pub name: Option<String>,
    /// The local decorator application id.
    pub application: LocalDecoratorId,
    /// The decorator node.
    pub decorator: GlobalNodeId<Decorator>,
    /// The decorator expression target.
    pub expression: GlobalNodeId<Expression>,
    /// The decorated owner node.
    pub owner: GlobalNodeIdAny,
    /// The resolved decorator declaration.
    pub target: DecoratorTarget,
}

impl DecoratorIndex {
    /// Create a decorator index from entries.
    pub fn new(entries: Vec<DecoratorEntry>) -> Self {
        let mut index = Self { entries };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.entries.sort_by(DecoratorEntry::compare_by_index);
        self.entries.dedup();
    }

    /// Return decorators matching one optional decorator name.
    pub fn search(&self, name: Option<&str>) -> impl Iterator<Item = &DecoratorEntry> {
        self.entries
            .iter()
            .filter(move |entry| name.is_none_or(|name| entry.name.as_deref() == Some(name)))
    }

    /// Return all indexed decorators.
    pub fn entries(&self) -> &[DecoratorEntry] {
        &self.entries
    }
}

impl DecoratorPostings {
    /// Build decorator postings from module index sections.
    pub fn build(indexes: &[&DecoratorIndex]) -> Self {
        let names = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .filter_map(move |entry| entry.name.clone().map(|name| (name, module)))
        }));

        Self { names }
    }
}

impl DecoratorEntry {
    /// Compare two decorators in stable index order.
    fn compare_by_index(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.name.as_deref(),
            self.decorator.module_id,
            self.owner.local_id.id,
            self.decorator.local_id.id,
            self.expression.local_id.id,
        );
        let right = (
            other.name.as_deref(),
            other.decorator.module_id,
            other.owner.local_id.id,
            other.decorator.local_id.id,
            other.expression.local_id.id,
        );

        left.cmp(&right)
    }
}

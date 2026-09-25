use std::collections::BTreeMap;
use std::fmt;

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use tspp_serde::{Reflect, Schema, Type};

/// One runtime label key-value pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Label {
    /// Label key.
    pub key: Box<str>,
    /// Label value.
    pub value: Box<str>,
}

/// Sparse label set for runtime-owned metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct LabelSet {
    /// Stored labels when this set is non-empty.
    labels: Option<Box<Labels>>,
}

impl Reflect for LabelSet {
    /// Reflect the serialized label map.
    fn reflect(schema: &mut Schema) -> Type {
        Type::Map {
            key: Box::new(String::reflect(schema)),
            value: Box::new(String::reflect(schema)),
        }
    }
}

/// Non-empty label payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Labels {
    /// First label stored without a vector allocation.
    first: Label,
    /// Additional labels when this set has more than one label.
    rest: Vec<Label>,
}

impl LabelSet {
    /// Empty label set.
    pub const EMPTY: Self = Self { labels: None };

    /// Create one empty label set.
    pub const fn new() -> Self {
        Self::EMPTY
    }

    /// Return whether this label set is empty.
    pub fn is_empty(&self) -> bool {
        self.labels.is_none()
    }

    /// Return the number of labels.
    pub fn len(&self) -> usize {
        match self.labels.as_ref() {
            Some(labels) => labels.len(),
            None => 0,
        }
    }

    /// Insert or replace one label.
    pub fn insert(&mut self, key: impl Into<Box<str>>, value: impl Into<Box<str>>) {
        let key = key.into();
        let value = value.into();

        match self.labels.as_mut() {
            Some(labels) => labels.insert(key, value),
            None => self.labels = Some(Box::new(Labels::new(key, value))),
        }
    }

    /// Return one label value by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.labels.as_ref()?.get(key)
    }

    /// Return whether this label set contains one key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Return an iterator over all labels.
    pub fn iter(&self) -> impl Iterator<Item = &Label> {
        self.labels.iter().flat_map(|labels| labels.iter())
    }
}

impl Labels {
    /// Create one non-empty label payload.
    fn new(key: Box<str>, value: Box<str>) -> Self {
        Self {
            first: Label { key, value },
            rest: Vec::new(),
        }
    }

    /// Insert or replace one label.
    fn insert(&mut self, key: Box<str>, value: Box<str>) {
        // replace inline label
        if self.first.key.as_ref() == key.as_ref() {
            self.first.value = value;
            return;
        }

        // replace overflow label
        if let Some(label) = self
            .rest
            .iter_mut()
            .find(|label| label.key.as_ref() == key.as_ref())
        {
            label.value = value;
            return;
        }

        // append new label
        self.rest.push(Label { key, value });
    }

    /// Return the number of labels.
    fn len(&self) -> usize {
        1 + self.rest.len()
    }

    /// Return one label value by key.
    fn get(&self, key: &str) -> Option<&str> {
        // check inline label
        if self.first.key.as_ref() == key {
            return Some(self.first.value.as_ref());
        }

        // scan overflow labels
        let label = self.rest.iter().find(|label| label.key.as_ref() == key)?;

        Some(label.value.as_ref())
    }

    /// Return an iterator over all labels.
    fn iter(&self) -> impl Iterator<Item = &Label> {
        std::iter::once(&self.first).chain(self.rest.iter())
    }
}

impl From<BTreeMap<String, String>> for LabelSet {
    fn from(labels: BTreeMap<String, String>) -> Self {
        let mut set = Self::new();

        for (key, value) in labels {
            set.insert(key, value);
        }

        set
    }
}

impl Serialize for LabelSet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;

        for label in self.iter() {
            map.serialize_entry(label.key.as_ref(), label.value.as_ref())?;
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for LabelSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(LabelSetVisitor)
    }
}

/// Serde visitor for compact label sets.
struct LabelSetVisitor;

impl<'de> Visitor<'de> for LabelSetVisitor {
    type Value = LabelSet;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a label map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut labels = LabelSet::new();

        while let Some((key, value)) = map.next_entry::<String, String>()? {
            labels.insert(key, value);
        }

        Ok(labels)
    }
}

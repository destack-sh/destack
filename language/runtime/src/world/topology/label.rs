use std::collections::BTreeMap;
use std::fmt;

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
    entries: Option<Box<LabelStorage>>,
}

/// Non-empty label storage.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LabelStorage {
    /// First label stored without a vector allocation.
    first: Label,
    /// Additional labels when this set has more than one label.
    rest: Vec<Label>,
}

/// Iterator over one compact label set.
#[derive(Debug)]
pub struct LabelSetIter<'a> {
    /// First label to yield.
    first: Option<&'a Label>,
    /// Remaining labels to yield.
    rest: std::slice::Iter<'a, Label>,
}

impl LabelSet {
    /// Empty label set.
    pub const EMPTY: Self = Self { entries: None };

    /// Create one empty label set.
    pub const fn new() -> Self {
        Self::EMPTY
    }

    /// Return whether this label set is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_none()
    }

    /// Return the number of labels.
    pub fn len(&self) -> usize {
        match self.entries.as_ref() {
            Some(entries) => entries.len(),
            None => 0,
        }
    }

    /// Insert or replace one label.
    pub fn insert(&mut self, key: impl Into<Box<str>>, value: impl Into<Box<str>>) {
        let key = key.into();
        let value = value.into();

        // replace first label
        match self.entries.as_mut() {
            Some(entries) if entries.first.key.as_ref() == key.as_ref() => {
                entries.first.value = value;
            }
            // replace or append remaining label
            Some(entries) => {
                if let Some(label) = entries
                    .rest
                    .iter_mut()
                    .find(|label| label.key.as_ref() == key.as_ref())
                {
                    label.value = value;
                } else {
                    entries.rest.push(Label { key, value });
                }
            }
            // create compact one-label storage
            None => {
                self.entries = Some(Box::new(LabelStorage {
                    first: Label { key, value },
                    rest: Vec::new(),
                }));
            }
        }
    }

    /// Return one label value by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        let entries = self.entries.as_ref()?;

        // check inline label
        if entries.first.key.as_ref() == key {
            return Some(entries.first.value.as_ref());
        }

        // scan overflow labels
        let label = entries
            .rest
            .iter()
            .find(|label| label.key.as_ref() == key)?;

        Some(label.value.as_ref())
    }

    /// Return whether this label set contains one key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Return an iterator over all labels.
    pub fn iter(&self) -> LabelSetIter<'_> {
        match self.entries.as_ref() {
            Some(entries) => LabelSetIter {
                first: Some(&entries.first),
                rest: entries.rest.iter(),
            },
            None => LabelSetIter {
                first: None,
                rest: [].iter(),
            },
        }
    }
}

impl<'a> Iterator for LabelSetIter<'a> {
    type Item = &'a Label;

    fn next(&mut self) -> Option<Self::Item> {
        if self.first.is_some() {
            return self.first.take();
        }

        self.rest.next()
    }
}

impl LabelStorage {
    /// Return the number of labels.
    fn len(&self) -> usize {
        1 + self.rest.len()
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

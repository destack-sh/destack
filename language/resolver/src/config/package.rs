use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ImportsExportsEntry<'a>(pub &'a Value);

impl<'a> ImportsExportsEntry<'a> {
    pub fn kind(&self) -> ImportsExportsKind {
        match self.0 {
            Value::String(_) => ImportsExportsKind::String,
            Value::Array(_) => ImportsExportsKind::Array,
            Value::Object(_) => ImportsExportsKind::Map,
            _ => ImportsExportsKind::Invalid,
        }
    }

    pub fn as_string(&self) -> Option<&'a str> {
        match self.0 {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<ImportsExportsArray<'a>> {
        match self.0 {
            Value::Array(arr) => Some(ImportsExportsArray(arr)),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<ImportsExportsMap<'a>> {
        match self.0 {
            Value::Object(obj) => Some(ImportsExportsMap(obj)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportsExportsArray<'a>(&'a [Value]);

impl<'a> ImportsExportsArray<'a> {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = ImportsExportsEntry<'a>> {
        ImportsExportsArrayIter {
            slice: self.0,
            index: 0,
        }
    }
}

struct ImportsExportsArrayIter<'a> {
    slice: &'a [Value],
    index: usize,
}

impl<'a> Iterator for ImportsExportsArrayIter<'a> {
    type Item = ImportsExportsEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.slice.get(self.index).map(|value| {
            self.index += 1;
            ImportsExportsEntry(value)
        })
    }
}

#[derive(Debug, Clone)]
pub struct ImportsExportsMap<'a>(pub(crate) &'a serde_json::Map<String, Value>);

impl<'a> ImportsExportsMap<'a> {
    pub(crate) fn get(&self, key: &str) -> Option<ImportsExportsEntry<'a>> {
        self.0.get(key).map(ImportsExportsEntry)
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = &'a str> {
        self.0.keys().map(String::as_str)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&'a str, ImportsExportsEntry<'a>)> {
        self.0
            .iter()
            .map(|(k, v)| (k.as_str(), ImportsExportsEntry(v)))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportsExportsKind {
    String,
    Array,
    Map,
    Invalid,
}

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::BuildHasher;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use smallvec::{Array, SmallVec};

/// Type that can describe its Destack serialization schema.
pub trait Schema {
    /// Register this type in one schema registry.
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef;
}

/// Registry of named schema items.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaRegistry {
    /// Named schema items keyed by module and name.
    pub items: BTreeMap<SchemaName, SchemaItem>,
    /// Item names grouped by module path.
    pub modules: BTreeMap<Vec<String>, Vec<SchemaName>>,
    /// Items that are currently being declared.
    #[serde(skip)]
    declaring: BTreeSet<SchemaName>,
}

impl SchemaRegistry {
    /// Include one schema type in this registry.
    pub fn include<T: Schema>(&mut self) -> SchemaRef {
        let reference = T::schema(self);
        if let SchemaRef::Named(name) = &reference {
            self.move_to_end(name);
        }

        reference
    }

    /// Register one named schema item and return a reference to it.
    pub fn register(
        &mut self,
        module: &'static str,
        name: &'static str,
        docs: Vec<String>,
        declare: impl FnOnce(&mut Self) -> SchemaShape,
    ) -> SchemaRef {
        self.register_with(module, name, docs, Vec::new(), declare)
    }

    /// Register one named schema item with consumer attributes.
    pub fn register_with(
        &mut self,
        module: &'static str,
        name: &'static str,
        docs: Vec<String>,
        attributes: Vec<String>,
        declare: impl FnOnce(&mut Self) -> SchemaShape,
    ) -> SchemaRef {
        let name = SchemaName::new(module, name);
        if self.items.contains_key(&name) || self.declaring.contains(&name) {
            return SchemaRef::Named(name);
        }

        self.modules
            .entry(name.module.clone())
            .or_default()
            .push(name.clone());

        self.declaring.insert(name.clone());
        let shape = declare(self);
        self.declaring.remove(&name);

        let item = SchemaItem {
            name: name.clone(),
            docs,
            attributes,
            shape,
        };
        self.items.insert(name.clone(), item);

        SchemaRef::Named(name)
    }

    /// Move one registered item to the end of its module order.
    fn move_to_end(&mut self, name: &SchemaName) {
        let Some(names) = self.modules.get_mut(&name.module) else {
            unreachable!("schema item module is missing");
        };
        let Some(index) = names.iter().position(|item| item == name) else {
            unreachable!("schema item module order is missing");
        };

        let name = names.remove(index);
        names.push(name);
    }
}

/// Name of one schema item.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SchemaName {
    /// Module path segments.
    pub module: Vec<String>,
    /// Item name.
    pub name: String,
}

impl SchemaName {
    /// Build one schema name from a Rust module path and item name.
    pub fn new(module: &str, name: &str) -> Self {
        let module = module.split("::").map(str::to_string).collect();

        Self {
            module,
            name: name.to_string(),
        }
    }
}

/// Reference to a schema type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaRef {
    /// Unit value.
    Unit,
    /// Boolean value.
    Bool,
    /// Signed integer value.
    Signed { bits: u16 },
    /// Unsigned integer value.
    Unsigned { bits: u16 },
    /// Machine-sized unsigned integer value.
    Usize,
    /// Floating point value.
    Float { bits: u16 },
    /// Unicode scalar value.
    Char,
    /// UTF-8 string value.
    String,
    /// Optional value.
    Option(Box<SchemaRef>),
    /// Sequence value.
    Sequence(Box<SchemaRef>),
    /// Fixed-length array value.
    Array {
        /// Element type.
        item: Box<SchemaRef>,
        /// Element count.
        len: usize,
    },
    /// Fixed-length tuple value.
    Tuple(Vec<SchemaRef>),
    /// Map value.
    Map {
        /// Key type.
        key: Box<SchemaRef>,
        /// Value type.
        value: Box<SchemaRef>,
    },
    /// Dynamic JSON value.
    Json,
    /// Named schema item.
    Named(SchemaName),
}

/// One named schema item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaItem {
    /// Schema item name.
    pub name: SchemaName,
    /// Documentation lines.
    pub docs: Vec<String>,
    /// Consumer attributes attached to this item.
    pub attributes: Vec<String>,
    /// Schema item shape.
    pub shape: SchemaShape,
}

/// Shape of one named schema item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaShape {
    /// Struct with named or positional fields.
    Struct(Vec<SchemaField>),
    /// Enum with variants.
    Enum(Vec<SchemaVariant>),
}

/// One schema struct or variant field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaField {
    /// Field name.
    pub name: String,
    /// Documentation lines.
    pub docs: Vec<String>,
    /// Field type.
    pub ty: SchemaRef,
}

/// One enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVariant {
    /// Variant name.
    pub name: String,
    /// Documentation lines.
    pub docs: Vec<String>,
    /// Variant payload.
    pub payload: SchemaPayload,
}

/// Payload of one enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaPayload {
    /// Variant without payload.
    Unit,
    /// Variant with one unnamed payload.
    Tuple(SchemaRef),
    /// Variant with fields.
    Struct(Vec<SchemaField>),
}

impl Schema for () {
    fn schema(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Unit
    }
}

impl Schema for bool {
    fn schema(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Bool
    }
}

impl Schema for char {
    fn schema(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Char
    }
}

impl Schema for String {
    fn schema(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::String
    }
}

impl Schema for str {
    fn schema(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::String
    }
}

impl Schema for PathBuf {
    fn schema(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::String
    }
}

impl<T: Schema> Schema for Option<T> {
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Option(Box::new(T::schema(registry)))
    }
}

impl<T: Schema> Schema for Vec<T> {
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Sequence(Box::new(T::schema(registry)))
    }
}

impl<T: Schema> Schema for BTreeSet<T> {
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Sequence(Box::new(T::schema(registry)))
    }
}

impl<K: Schema, V: Schema, S: BuildHasher> Schema for HashMap<K, V, S> {
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Map {
            key: Box::new(K::schema(registry)),
            value: Box::new(V::schema(registry)),
        }
    }
}

impl<A> Schema for SmallVec<A>
where
    A: Array,
    A::Item: Schema,
{
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Sequence(Box::new(A::Item::schema(registry)))
    }
}

impl<T: Schema, const N: usize> Schema for [T; N] {
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Array {
            item: Box::new(T::schema(registry)),
            len: N,
        }
    }
}

impl<T: Schema> Schema for [T] {
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Sequence(Box::new(T::schema(registry)))
    }
}

impl<K: Schema, V: Schema> Schema for BTreeMap<K, V> {
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Map {
            key: Box::new(K::schema(registry)),
            value: Box::new(V::schema(registry)),
        }
    }
}

impl<K: Schema, V: Schema> Schema for IndexMap<K, V> {
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Map {
            key: Box::new(K::schema(registry)),
            value: Box::new(V::schema(registry)),
        }
    }
}

impl<T: Schema> Schema for IndexSet<T> {
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Sequence(Box::new(T::schema(registry)))
    }
}

impl<A: Schema, B: Schema> Schema for (A, B) {
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Tuple(vec![A::schema(registry), B::schema(registry)])
    }
}

impl<T: Schema + ?Sized> Schema for Arc<T> {
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef {
        T::schema(registry)
    }
}

impl<T: Schema + ?Sized> Schema for Box<T> {
    fn schema(registry: &mut SchemaRegistry) -> SchemaRef {
        T::schema(registry)
    }
}

impl Schema for NonZeroU32 {
    fn schema(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Unsigned { bits: 32 }
    }
}

impl Schema for serde_json::Value {
    fn schema(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Json
    }
}

macro_rules! unsigned_schema {
    ($($ty:ty => $bits:literal),* $(,)?) => {
        $(
            impl Schema for $ty {
                fn schema(_registry: &mut SchemaRegistry) -> SchemaRef {
                    SchemaRef::Unsigned { bits: $bits }
                }
            }
        )*
    };
}

macro_rules! signed_schema {
    ($($ty:ty => $bits:literal),* $(,)?) => {
        $(
            impl Schema for $ty {
                fn schema(_registry: &mut SchemaRegistry) -> SchemaRef {
                    SchemaRef::Signed { bits: $bits }
                }
            }
        )*
    };
}

macro_rules! float_schema {
    ($($ty:ty => $bits:literal),* $(,)?) => {
        $(
            impl Schema for $ty {
                fn schema(_registry: &mut SchemaRegistry) -> SchemaRef {
                    SchemaRef::Float { bits: $bits }
                }
            }
        )*
    };
}

unsigned_schema! {
    u8 => 8,
    u16 => 16,
    u32 => 32,
    u64 => 64,
    u128 => 128,
}

impl Schema for usize {
    fn schema(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Usize
    }
}

signed_schema! {
    i8 => 8,
    i16 => 16,
    i32 => 32,
    i64 => 64,
    i128 => 128,
    isize => 64,
}

float_schema! {
    f32 => 32,
    f64 => 64,
}

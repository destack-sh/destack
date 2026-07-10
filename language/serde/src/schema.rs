use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::BuildHasher;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use im::OrdMap;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use smallvec::{Array, SmallVec};

/// Type that can describe its Destack serialization schema.
pub trait Reflect {
    /// Register this type in one schema registry.
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef;
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
    /// Register one schema type in this registry.
    pub fn register<T: Reflect>(&mut self) -> SchemaRef {
        let reference = T::reflect(self);
        if let SchemaRef::Named(name) = &reference {
            self.move_to_end(name);
        }

        reference
    }

    /// Declare one named schema item and return a reference to it.
    pub fn declare(
        &mut self,
        module: &'static str,
        name: &'static str,
        docs: Vec<String>,
        declare: impl FnOnce(&mut Self) -> SchemaShape,
    ) -> SchemaRef {
        self.declare_with(module, name, docs, Vec::new(), declare)
    }

    /// Declare one named schema item with consumer attributes.
    pub fn declare_with(
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
    /// Reflect item name.
    pub name: SchemaName,
    /// Documentation lines.
    pub docs: Vec<String>,
    /// Consumer attributes attached to this item.
    pub attributes: Vec<String>,
    /// Reflect item shape.
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

impl Reflect for () {
    fn reflect(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Unit
    }
}

impl Reflect for bool {
    fn reflect(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Bool
    }
}

impl Reflect for char {
    fn reflect(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Char
    }
}

impl Reflect for String {
    fn reflect(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::String
    }
}

impl Reflect for str {
    fn reflect(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::String
    }
}

impl Reflect for PathBuf {
    fn reflect(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::String
    }
}

impl<T: Reflect> Reflect for Option<T> {
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Option(Box::new(T::reflect(registry)))
    }
}

impl<T: Reflect> Reflect for Vec<T> {
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Sequence(Box::new(T::reflect(registry)))
    }
}

impl<T: Reflect> Reflect for BTreeSet<T> {
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Sequence(Box::new(T::reflect(registry)))
    }
}

impl<K: Reflect, V: Reflect, S: BuildHasher> Reflect for HashMap<K, V, S> {
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Map {
            key: Box::new(K::reflect(registry)),
            value: Box::new(V::reflect(registry)),
        }
    }
}

impl<A> Reflect for SmallVec<A>
where
    A: Array,
    A::Item: Reflect,
{
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Sequence(Box::new(A::Item::reflect(registry)))
    }
}

impl<T: Reflect, const N: usize> Reflect for [T; N] {
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Array {
            item: Box::new(T::reflect(registry)),
            len: N,
        }
    }
}

impl<T: Reflect> Reflect for [T] {
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Sequence(Box::new(T::reflect(registry)))
    }
}

impl<K: Reflect, V: Reflect> Reflect for BTreeMap<K, V> {
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Map {
            key: Box::new(K::reflect(registry)),
            value: Box::new(V::reflect(registry)),
        }
    }
}

impl<K: Reflect, V: Reflect> Reflect for OrdMap<K, V> {
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Map {
            key: Box::new(K::reflect(registry)),
            value: Box::new(V::reflect(registry)),
        }
    }
}

impl<K: Reflect, V: Reflect, S> Reflect for IndexMap<K, V, S> {
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Map {
            key: Box::new(K::reflect(registry)),
            value: Box::new(V::reflect(registry)),
        }
    }
}

impl<T: Reflect> Reflect for IndexSet<T> {
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Sequence(Box::new(T::reflect(registry)))
    }
}

impl<A: Reflect, B: Reflect> Reflect for (A, B) {
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Tuple(vec![A::reflect(registry), B::reflect(registry)])
    }
}

impl<T: Reflect + ?Sized> Reflect for Arc<T> {
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        T::reflect(registry)
    }
}

impl<T: Reflect + ?Sized> Reflect for Box<T> {
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        T::reflect(registry)
    }
}

impl Reflect for NonZeroU32 {
    fn reflect(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Unsigned { bits: 32 }
    }
}

impl Reflect for serde_json::Value {
    fn reflect(_registry: &mut SchemaRegistry) -> SchemaRef {
        SchemaRef::Json
    }
}

macro_rules! unsigned_schema {
    ($($ty:ty => $bits:literal),* $(,)?) => {
        $(
            impl Reflect for $ty {
                fn reflect(_registry: &mut SchemaRegistry) -> SchemaRef {
                    SchemaRef::Unsigned { bits: $bits }
                }
            }
        )*
    };
}

macro_rules! signed_schema {
    ($($ty:ty => $bits:literal),* $(,)?) => {
        $(
            impl Reflect for $ty {
                fn reflect(_registry: &mut SchemaRegistry) -> SchemaRef {
                    SchemaRef::Signed { bits: $bits }
                }
            }
        )*
    };
}

macro_rules! float_schema {
    ($($ty:ty => $bits:literal),* $(,)?) => {
        $(
            impl Reflect for $ty {
                fn reflect(_registry: &mut SchemaRegistry) -> SchemaRef {
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

impl Reflect for usize {
    fn reflect(_registry: &mut SchemaRegistry) -> SchemaRef {
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

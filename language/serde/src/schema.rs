use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::BuildHasher;
use std::num::{NonZeroU32, NonZeroU64};
use std::path::PathBuf;
use std::sync::Arc;

use im::OrdMap;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use smallvec::{Array, SmallVec};

/// Type that can describe its TS++ serialization schema.
pub trait Reflect {
    /// Register this type in one schema.
    fn reflect(schema: &mut Schema) -> Type;
}

/// One collection of named serialization types.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schema {
    /// Named schema items keyed by module and name.
    pub items: BTreeMap<Name, Item>,
    /// Item names grouped by module path.
    pub modules: BTreeMap<Vec<String>, Vec<Name>>,
    /// Items that are currently being declared.
    #[serde(skip)]
    declaring: BTreeSet<Name>,
}

impl Schema {
    /// Register one type in this schema.
    pub fn register<T: Reflect>(&mut self) -> Type {
        let ty = T::reflect(self);
        if let Type::Named(name) = &ty {
            self.move_to_end(name);
        }

        ty
    }

    /// Merge another schema and return the first conflicting item name.
    pub fn merge(&mut self, schema: Self) -> Result<(), Name> {
        // reject conflicts before changing either declaration index
        let conflict = schema.items.iter().find_map(|(name, item)| {
            self.items
                .get(name)
                .filter(|existing| *existing != item)
                .map(|_| name.clone())
        });
        if let Some(name) = conflict {
            return Err(name);
        }

        // merge exact declarations
        for (name, item) in schema.items {
            self.items.entry(name).or_insert(item);
        }

        // merge module declaration order
        for (module, names) in schema.modules {
            let existing = self.modules.entry(module).or_default();
            for name in names {
                // preserve each name once
                if !existing.contains(&name) {
                    existing.push(name);
                }
            }
        }

        Ok(())
    }

    /// Declare one named schema item and return its named type.
    pub fn declare(
        &mut self,
        module: &'static str,
        name: &'static str,
        docs: Vec<String>,
        declare: impl FnOnce(&mut Self) -> Type,
    ) -> Type {
        let name = Name::new(module, name);
        if self.items.contains_key(&name) || self.declaring.contains(&name) {
            return Type::Named(name);
        }

        self.modules
            .entry(name.module.clone())
            .or_default()
            .push(name.clone());

        self.declaring.insert(name.clone());
        let ty = declare(self);
        self.declaring.remove(&name);

        let item = Item {
            name: name.clone(),
            docs,
            ty,
        };
        self.items.insert(name.clone(), item);

        Type::Named(name)
    }

    /// Move one registered item to the end of its module order.
    fn move_to_end(&mut self, name: &Name) {
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
pub struct Name {
    /// Module path segments.
    pub module: Vec<String>,
    /// Item name.
    pub name: String,
}

impl Name {
    /// Build one schema name from a Rust module path and item name.
    pub fn new(module: &str, name: &str) -> Self {
        let module = module.split("::").map(str::to_string).collect();

        Self {
            module,
            name: name.to_string(),
        }
    }
}

/// One serialization type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Type {
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
    Option(Box<Type>),
    /// Sequence value.
    Sequence(Box<Type>),
    /// Fixed-length array value.
    Array {
        /// Element type.
        item: Box<Type>,
        /// Element count.
        len: usize,
    },
    /// Fixed-length tuple value.
    Tuple(Vec<Type>),
    /// Map value.
    Map {
        /// Key type.
        key: Box<Type>,
        /// Value type.
        value: Box<Type>,
    },
    /// Named schema item.
    Named(Name),
    /// Struct with named fields.
    Struct(Vec<Field>),
    /// Enum with variants.
    Enum(Vec<Variant>),
}

/// One named schema item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// Reflect item name.
    pub name: Name,
    /// Documentation lines.
    pub docs: Vec<String>,
    /// Reflected serialization type.
    pub ty: Type,
}

/// One schema struct or variant field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    /// Field name.
    pub name: String,
    /// Documentation lines.
    pub docs: Vec<String>,
    /// Field type.
    pub ty: Type,
}

/// One enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant {
    /// Variant name.
    pub name: String,
    /// Documentation lines.
    pub docs: Vec<String>,
    /// Variant payload.
    pub payload: Payload,
}

/// Payload of one enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Payload {
    /// Variant without payload.
    Unit,
    /// Variant with an unnamed payload.
    Value(Type),
    /// Variant with named fields.
    Struct(Vec<Field>),
}

impl Reflect for () {
    fn reflect(_schema: &mut Schema) -> Type {
        Type::Unit
    }
}

impl Reflect for bool {
    fn reflect(_schema: &mut Schema) -> Type {
        Type::Bool
    }
}

impl Reflect for char {
    fn reflect(_schema: &mut Schema) -> Type {
        Type::Char
    }
}

impl Reflect for String {
    fn reflect(_schema: &mut Schema) -> Type {
        Type::String
    }
}

impl Reflect for str {
    fn reflect(_schema: &mut Schema) -> Type {
        Type::String
    }
}

impl Reflect for PathBuf {
    fn reflect(_schema: &mut Schema) -> Type {
        Type::String
    }
}

impl<T: Reflect> Reflect for Option<T> {
    fn reflect(schema: &mut Schema) -> Type {
        Type::Option(Box::new(T::reflect(schema)))
    }
}

impl<T: Reflect> Reflect for Vec<T> {
    fn reflect(schema: &mut Schema) -> Type {
        Type::Sequence(Box::new(T::reflect(schema)))
    }
}

impl<T: Reflect> Reflect for BTreeSet<T> {
    fn reflect(schema: &mut Schema) -> Type {
        Type::Sequence(Box::new(T::reflect(schema)))
    }
}

impl<K: Reflect, V: Reflect, S: BuildHasher> Reflect for HashMap<K, V, S> {
    fn reflect(schema: &mut Schema) -> Type {
        Type::Map {
            key: Box::new(K::reflect(schema)),
            value: Box::new(V::reflect(schema)),
        }
    }
}

impl<A> Reflect for SmallVec<A>
where
    A: Array,
    A::Item: Reflect,
{
    fn reflect(schema: &mut Schema) -> Type {
        Type::Sequence(Box::new(A::Item::reflect(schema)))
    }
}

impl<T: Reflect, const N: usize> Reflect for [T; N] {
    fn reflect(schema: &mut Schema) -> Type {
        Type::Array {
            item: Box::new(T::reflect(schema)),
            len: N,
        }
    }
}

impl<T: Reflect> Reflect for [T] {
    fn reflect(schema: &mut Schema) -> Type {
        Type::Sequence(Box::new(T::reflect(schema)))
    }
}

impl<K: Reflect, V: Reflect> Reflect for BTreeMap<K, V> {
    fn reflect(schema: &mut Schema) -> Type {
        Type::Map {
            key: Box::new(K::reflect(schema)),
            value: Box::new(V::reflect(schema)),
        }
    }
}

impl<K: Reflect, V: Reflect> Reflect for OrdMap<K, V> {
    fn reflect(schema: &mut Schema) -> Type {
        Type::Map {
            key: Box::new(K::reflect(schema)),
            value: Box::new(V::reflect(schema)),
        }
    }
}

impl<K: Reflect, V: Reflect, S> Reflect for IndexMap<K, V, S> {
    fn reflect(schema: &mut Schema) -> Type {
        Type::Map {
            key: Box::new(K::reflect(schema)),
            value: Box::new(V::reflect(schema)),
        }
    }
}

impl<T: Reflect, S> Reflect for IndexSet<T, S> {
    fn reflect(schema: &mut Schema) -> Type {
        Type::Sequence(Box::new(T::reflect(schema)))
    }
}

impl<A: Reflect, B: Reflect> Reflect for (A, B) {
    fn reflect(schema: &mut Schema) -> Type {
        Type::Tuple(vec![A::reflect(schema), B::reflect(schema)])
    }
}

impl<T: Reflect + ?Sized> Reflect for Arc<T> {
    fn reflect(schema: &mut Schema) -> Type {
        T::reflect(schema)
    }
}

impl<T: Reflect + ?Sized> Reflect for Box<T> {
    fn reflect(schema: &mut Schema) -> Type {
        T::reflect(schema)
    }
}

impl Reflect for NonZeroU32 {
    fn reflect(_schema: &mut Schema) -> Type {
        Type::Unsigned { bits: 32 }
    }
}

impl Reflect for NonZeroU64 {
    fn reflect(_schema: &mut Schema) -> Type {
        Type::Unsigned { bits: 64 }
    }
}

macro_rules! unsigned_schema {
    ($($ty:ty => $bits:literal),* $(,)?) => {
        $(
            impl Reflect for $ty {
                fn reflect(_schema: &mut Schema) -> Type {
                    Type::Unsigned { bits: $bits }
                }
            }
        )*
    };
}

macro_rules! signed_schema {
    ($($ty:ty => $bits:literal),* $(,)?) => {
        $(
            impl Reflect for $ty {
                fn reflect(_schema: &mut Schema) -> Type {
                    Type::Signed { bits: $bits }
                }
            }
        )*
    };
}

macro_rules! float_schema {
    ($($ty:ty => $bits:literal),* $(,)?) => {
        $(
            impl Reflect for $ty {
                fn reflect(_schema: &mut Schema) -> Type {
                    Type::Float { bits: $bits }
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
    fn reflect(_schema: &mut Schema) -> Type {
        Type::Usize
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

use std::fmt;
use std::mem::{ManuallyDrop, size_of};
use std::sync::Arc;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tspp_serde::{Field, Reflect, Schema, Type};

use super::{TypeId, Word};

const INLINE_WORD_COUNT: usize = 2;
const _: () = assert!(size_of::<Value>() == 3 * size_of::<Word>());

/// One type-erased value crossing a Program execution boundary.
#[repr(C)]
pub struct Value {
    /// The concrete Program type.
    ty: TypeId,
    /// The active execution-word storage.
    kind: ValueStorageKind,
    /// Inline or indirect execution words.
    storage: ValueStorage,
}

/// Inline words or shared indirect storage selected by `Value::kind`.
#[repr(C)]
union ValueStorage {
    /// Up to two inline execution words.
    inline: [Word; INLINE_WORD_COUNT],
    /// Shared words copied only when one World mutates them.
    indirect: ManuallyDrop<Arc<[Word]>>,
}

/// Active storage representation for one value.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum ValueStorageKind {
    /// No execution words.
    Empty,
    /// One inline execution word.
    Single,
    /// Two inline execution words.
    Pair,
    /// Three or more shared execution words.
    Indirect,
}

/// Serialized fields for one value.
#[derive(Deserialize)]
struct ValueFields {
    /// The concrete Program type.
    ty: TypeId,
    /// The exact execution words.
    words: Vec<Word>,
}

impl Value {
    /// Create one exact Program value.
    pub(crate) fn new(ty: TypeId, words: impl IntoIterator<Item = Word>) -> Self {
        let mut words = words.into_iter();
        let Some(first) = words.next() else {
            return Self::inline(ty, ValueStorageKind::Empty, [Word::ZERO; INLINE_WORD_COUNT]);
        };
        let Some(second) = words.next() else {
            return Self::inline(ty, ValueStorageKind::Single, [first, Word::ZERO]);
        };
        let Some(third) = words.next() else {
            return Self::inline(ty, ValueStorageKind::Pair, [first, second]);
        };

        // allocate one immutable shared buffer for larger values
        let mut values = Vec::with_capacity(words.size_hint().0 + 3);
        values.extend([first, second, third]);
        values.extend(words);

        Self::indirect(ty, values.into())
    }

    /// Return this value's concrete Program type.
    pub const fn ty(&self) -> TypeId {
        self.ty
    }

    /// Return this value's exact execution words.
    pub fn words(&self) -> &[Word] {
        if let Some(word_count) = self.kind.inline_word_count() {
            // SAFETY: inline storage kinds select initialized inline words
            let words = unsafe { &self.storage.inline };

            &words[..word_count]
        } else {
            // SAFETY: the indirect storage kind selects an initialized Arc
            unsafe { &self.storage.indirect }
        }
    }

    /// Return this value's exact execution words mutably.
    pub(crate) fn words_mut(&mut self) -> &mut [Word] {
        if let Some(word_count) = self.kind.inline_word_count() {
            // SAFETY: inline storage kinds select initialized inline words
            let words = unsafe { &mut self.storage.inline };

            &mut words[..word_count]
        } else {
            // SAFETY: the indirect storage kind selects an initialized Arc
            let words = unsafe { &mut self.storage.indirect };

            Arc::make_mut(words)
        }
    }

    /// Fork this value for one forked World.
    pub fn fork(&self) -> Self {
        if self.kind != ValueStorageKind::Indirect {
            // SAFETY: inline storage kinds select initialized inline words
            let words = unsafe { self.storage.inline };

            Self::inline(self.ty, self.kind, words)
        } else {
            // SAFETY: the indirect storage kind selects an initialized Arc
            let words = unsafe { &self.storage.indirect };

            Self::indirect(self.ty, (**words).clone())
        }
    }

    /// Create one inline value.
    const fn inline(ty: TypeId, kind: ValueStorageKind, words: [Word; INLINE_WORD_COUNT]) -> Self {
        Self {
            ty,
            kind,
            storage: ValueStorage { inline: words },
        }
    }

    /// Create one indirect value.
    fn indirect(ty: TypeId, words: Arc<[Word]>) -> Self {
        Self {
            ty,
            kind: ValueStorageKind::Indirect,
            storage: ValueStorage {
                indirect: ManuallyDrop::new(words),
            },
        }
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        if self.kind == ValueStorageKind::Indirect {
            // SAFETY: the indirect storage kind selects an initialized Arc
            unsafe { ManuallyDrop::drop(&mut self.storage.indirect) };
        }
    }
}

impl ValueStorageKind {
    /// Return the active inline word count, or `None` for indirect storage.
    const fn inline_word_count(self) -> Option<usize> {
        match self {
            Self::Empty => Some(0),
            Self::Single => Some(1),
            Self::Pair => Some(2),
            Self::Indirect => None,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty && self.words() == other.words()
    }
}

impl Eq for Value {}

impl fmt::Debug for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Value")
            .field("ty", &self.ty)
            .field("words", &self.words())
            .finish()
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut value = serializer.serialize_struct("Value", 2)?;
        value.serialize_field("ty", &self.ty)?;
        value.serialize_field("words", self.words())?;

        value.end()
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = ValueFields::deserialize(deserializer)?;

        Ok(Self::new(value.ty, value.words))
    }
}

impl Reflect for Value {
    fn reflect(schema: &mut Schema) -> Type {
        schema.declare(
            module_path!(),
            "Value",
            vec!["One type-erased value crossing a Program execution boundary.".to_string()],
            |schema| {
                Type::Struct(vec![
                    Field {
                        name: "ty".to_string(),
                        docs: vec!["The concrete Program type.".to_string()],
                        ty: TypeId::reflect(schema),
                    },
                    Field {
                        name: "words".to_string(),
                        docs: vec!["The exact execution words.".to_string()],
                        ty: Vec::<Word>::reflect(schema),
                    },
                ])
            },
        )
    }
}

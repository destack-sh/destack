use destack_serde::Reflect;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{GlobalSymbolId, LanguageItem};

/// One runtime scalar family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ScalarFamily {
    /// One plain scalar value domain.
    Domain(ScalarDomain),
    /// Members of one enum type.
    Enum(GlobalSymbolId),
}

impl ScalarFamily {
    /// Return whether this family holds builtin numerics.
    fn is_numeric(self) -> bool {
        matches!(self, Self::Domain(domain) if domain.is_numeric())
    }

    /// Return whether this family holds only integers.
    fn is_integral(self) -> bool {
        matches!(self, Self::Domain(domain) if domain.is_integral())
    }
}

/// Distinct runtime scalar families held by one type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct ScalarFamilySet {
    /// The distinct families in stable insertion order.
    families: SmallVec<[ScalarFamily; 4]>,
}

impl PartialEq for ScalarFamilySet {
    /// Compare scalar families without regard to insertion order.
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .families
                .iter()
                .all(|family| other.families.contains(family))
    }
}

impl Eq for ScalarFamilySet {}

impl ScalarFamilySet {
    /// Create an empty family set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert one family when it is not already present.
    pub fn insert(&mut self, family: ScalarFamily) {
        if !self.families.contains(&family) {
            self.families.push(family);
        }
    }

    /// Insert every family from another set.
    pub fn extend(&mut self, other: Self) {
        for family in other.families {
            self.insert(family);
        }
    }

    /// Retain the families also present in another set.
    pub fn intersect(&mut self, other: &Self) {
        self.families
            .retain(|family| other.families.contains(family));
    }

    /// Return whether no scalar family is present.
    pub fn is_empty(&self) -> bool {
        self.families.is_empty()
    }

    /// Return the number of distinct scalar families.
    pub fn len(&self) -> usize {
        self.families.len()
    }

    /// Return whether this set contains one family.
    pub fn contains(&self, family: ScalarFamily) -> bool {
        self.families.contains(&family)
    }

    /// Return whether this set contains exactly one plain domain.
    pub fn is_only_domain(&self, domain: ScalarDomain) -> bool {
        self.families.as_slice() == [ScalarFamily::Domain(domain)]
    }

    /// Return whether every family holds builtin numerics.
    pub fn is_numeric(&self) -> bool {
        !self.is_empty() && self.families.iter().all(|family| family.is_numeric())
    }

    /// Return whether every family holds only integers.
    pub fn is_integral(&self) -> bool {
        !self.is_empty() && self.families.iter().all(|family| family.is_integral())
    }

    /// Iterate the distinct scalar families.
    pub fn iter(&self) -> impl Iterator<Item = &ScalarFamily> {
        self.families.iter()
    }
}

impl From<ScalarFamily> for ScalarFamilySet {
    /// Create a family set containing one family.
    fn from(family: ScalarFamily) -> Self {
        let mut families = SmallVec::new();
        families.push(family);

        Self { families }
    }
}

/// One scalar value domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ScalarDomain {
    /// Machine integer scalar values.
    Integer,
    /// Machine float scalar values.
    Float,
    /// Bigint scalar values.
    Bigint,
    /// Character scalar values.
    Character,
    /// String scalar values.
    String,
    /// Symbol scalar values.
    Symbol,
    /// Boolean scalar values.
    Boolean,
    /// Null singleton values.
    Null,
    /// Undefined singleton values.
    Undefined,
}

impl ScalarDomain {
    /// Return the language item that owns this domain's members.
    pub fn member_owner_item(self) -> Option<LanguageItem> {
        match self {
            Self::Integer | Self::Float => Some(LanguageItem::Number),
            Self::Bigint => Some(LanguageItem::BigInt),
            Self::String => Some(LanguageItem::String),
            Self::Character | Self::Symbol | Self::Boolean | Self::Null | Self::Undefined => None,
        }
    }

    /// Return the language item that carries this domain at runtime.
    pub fn representation_item(self) -> Option<LanguageItem> {
        match self {
            Self::Bigint => Some(LanguageItem::BigInt),
            Self::String => Some(LanguageItem::String),
            Self::Integer
            | Self::Float
            | Self::Character
            | Self::Symbol
            | Self::Boolean
            | Self::Null
            | Self::Undefined => None,
        }
    }

    /// Return whether this domain holds builtin numerics.
    pub fn is_numeric(self) -> bool {
        matches!(self, Self::Integer | Self::Float | Self::Bigint)
    }

    /// Return whether this domain holds only integers.
    pub fn is_integral(self) -> bool {
        matches!(self, Self::Integer | Self::Bigint)
    }
}

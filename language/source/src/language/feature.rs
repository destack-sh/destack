use std::fmt;

/// Destack language features that can be enabled/disabled.
///
/// These are opt-in extensions beyond standard TypeScript.
/// By default in `.ds` files, all features are enabled.
/// In `.ts`/`.js` files, features are disabled unless explicitly enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum LanguageFeature {
    /// Precise numeric types, raw strings, byte literals (`int32`, `r#"..."#`, `b"..."`).
    Primitives = 1 << 0,
    /// Range literals (`1..10`, `1..=10`).
    Ranges = 1 << 1,
    /// Tuple types and literals (`(a, b, c)`).
    Tuples = 1 << 2,
    /// Tree literals (`<Node>...</Node>`).
    Trees = 1 << 3,
    /// Nominal (distinct) types with `newtype`.
    Newtypes = 1 << 4,
    /// Struct declarations for value-oriented data types.
    Structs = 1 << 5,
    /// Explicit ownership and reference semantics (`&T`, `&mut T`, `^T`).
    Ownership = 1 << 6,
    /// Constraint guards with `where` clauses.
    Constraints = 1 << 7,
    /// Type extensions for organizing implementations.
    Extensions = 1 << 8,
    /// Function and operator overloading.
    Overloading = 1 << 9,
    /// Pattern matching with `match` expressions.
    Patterns = 1 << 10,
    /// Effect declarations with `with` clauses.
    Effects = 1 << 11,
    /// Defer statements for cleanup (`defer file.close()`).
    Defer = 1 << 12,
}

impl LanguageFeature {
    /// All language features.
    pub const ALL: &[LanguageFeature] = &[
        Self::Primitives,
        Self::Ranges,
        Self::Tuples,
        Self::Trees,
        Self::Newtypes,
        Self::Structs,
        Self::Ownership,
        Self::Constraints,
        Self::Extensions,
        Self::Overloading,
        Self::Patterns,
        Self::Effects,
        Self::Defer,
    ];

    /// The config key for this feature (e.g., `"allowOverloading"`).
    pub fn options_key(&self) -> &'static str {
        match self {
            Self::Primitives => "allowPrimitives",
            Self::Ranges => "allowRanges",
            Self::Tuples => "allowTuples",
            Self::Trees => "allowTrees",
            Self::Newtypes => "allowNewtypes",
            Self::Structs => "allowStructs",
            Self::Ownership => "allowOwnership",
            Self::Constraints => "allowConstraints",
            Self::Extensions => "allowExtensions",
            Self::Overloading => "allowOverloading",
            Self::Patterns => "allowPatterns",
            Self::Effects => "allowEffects",
            Self::Defer => "allowDefer",
        }
    }

    /// Human-readable name for error messages.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Primitives => "primitives",
            Self::Ranges => "ranges",
            Self::Tuples => "tuples",
            Self::Trees => "trees",
            Self::Newtypes => "newtypes",
            Self::Structs => "structs",
            Self::Ownership => "ownership",
            Self::Constraints => "constraints",
            Self::Extensions => "extensions",
            Self::Overloading => "overloading",
            Self::Patterns => "patterns",
            Self::Effects => "effects",
            Self::Defer => "defer",
        }
    }

    /// Convert to bitmask value.
    #[inline]
    pub const fn as_bit(&self) -> u16 {
        *self as u16
    }
}

impl fmt::Display for LanguageFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

/// A set of enabled language features, stored as a bitset.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LanguageFeatureSet {
    bits: u16,
}

impl fmt::Debug for LanguageFeatureSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl Default for LanguageFeatureSet {
    fn default() -> Self {
        Self::none()
    }
}

impl LanguageFeatureSet {
    /// Create an empty feature set (no features enabled).
    #[inline]
    pub const fn none() -> Self {
        Self { bits: 0 }
    }

    /// Create a feature set with all features enabled.
    #[inline]
    pub const fn all() -> Self {
        let mut bits = 0u16;
        let mut i = 0;
        while i < LanguageFeature::ALL.len() {
            bits |= LanguageFeature::ALL[i].as_bit();
            i += 1;
        }
        Self { bits }
    }

    /// Create a feature set from a single feature.
    #[inline]
    pub const fn from_feature(feature: LanguageFeature) -> Self {
        Self {
            bits: feature.as_bit(),
        }
    }

    /// Create a feature set from raw bits.
    #[inline]
    pub const fn from_bits(bits: u16) -> Self {
        Self { bits }
    }

    /// Get the raw bits.
    #[inline]
    pub const fn bits(&self) -> u16 {
        self.bits
    }

    /// Check if a feature is enabled.
    #[inline]
    pub const fn is_enabled(&self, feature: LanguageFeature) -> bool {
        self.bits & feature.as_bit() != 0
    }

    /// Check if all features are enabled.
    #[inline]
    pub const fn is_all(&self) -> bool {
        self.bits == Self::all().bits
    }

    /// Check if no features are enabled.
    #[inline]
    pub const fn is_none(&self) -> bool {
        self.bits == 0
    }

    /// Enable a feature.
    #[inline]
    pub fn enable(&mut self, feature: LanguageFeature) {
        self.bits |= feature.as_bit();
    }

    /// Disable a feature.
    #[inline]
    pub fn disable(&mut self, feature: LanguageFeature) {
        self.bits &= !feature.as_bit();
    }

    /// Set a feature to enabled or disabled.
    #[inline]
    pub fn set(&mut self, feature: LanguageFeature, enabled: bool) {
        if enabled {
            self.enable(feature);
        } else {
            self.disable(feature);
        }
    }

    /// Builder: enable a feature.
    #[inline]
    pub const fn with(mut self, feature: LanguageFeature) -> Self {
        self.bits |= feature.as_bit();
        self
    }

    /// Builder: disable a feature.
    #[inline]
    pub const fn without(mut self, feature: LanguageFeature) -> Self {
        self.bits &= !feature.as_bit();
        self
    }

    /// Union of two feature sets.
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }

    /// Intersection of two feature sets.
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self {
            bits: self.bits & other.bits,
        }
    }

    /// Iterate over enabled features.
    pub fn iter(&self) -> impl Iterator<Item = LanguageFeature> + '_ {
        LanguageFeature::ALL
            .iter()
            .copied()
            .filter(|f| self.is_enabled(*f))
    }

    /// Count of enabled features.
    #[inline]
    pub const fn len(&self) -> usize {
        self.bits.count_ones() as usize
    }

    /// Whether no features are enabled.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.bits == 0
    }
}

impl FromIterator<LanguageFeature> for LanguageFeatureSet {
    fn from_iter<T: IntoIterator<Item = LanguageFeature>>(iter: T) -> Self {
        let mut set = Self::none();
        for feature in iter {
            set.enable(feature);
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_set_operations() {
        let mut set = LanguageFeatureSet::none();
        assert!(set.is_none());
        assert!(!set.is_enabled(LanguageFeature::Ownership));

        set.enable(LanguageFeature::Ownership);
        assert!(set.is_enabled(LanguageFeature::Ownership));
        assert!(!set.is_enabled(LanguageFeature::Effects));

        set.disable(LanguageFeature::Ownership);
        assert!(!set.is_enabled(LanguageFeature::Ownership));
    }

    #[test]
    fn test_feature_set_all() {
        let all = LanguageFeatureSet::all();
        for feature in LanguageFeature::ALL {
            assert!(all.is_enabled(*feature));
        }
    }

    #[test]
    fn test_feature_set_builder() {
        let set = LanguageFeatureSet::none()
            .with(LanguageFeature::Ownership)
            .with(LanguageFeature::Patterns);

        assert!(set.is_enabled(LanguageFeature::Ownership));
        assert!(set.is_enabled(LanguageFeature::Patterns));
        assert!(!set.is_enabled(LanguageFeature::Effects));
    }

    #[test]
    fn test_feature_config_keys() {
        assert_eq!(
            LanguageFeature::Overloading.options_key(),
            "allowOverloading"
        );
        assert_eq!(LanguageFeature::Ownership.options_key(), "allowOwnership");
    }
}

use std::fmt;

/// Destack language features that can be enabled/disabled.
///
/// These are opt-in extensions beyond standard TypeScript.
/// By default in `.ds` files, all features are enabled.
/// In `.ts`/`.js` files, features are disabled unless explicitly enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LanguageFeature {
    /// Expression extensions: implicit returns, `loop`, `defer`, ranges, tuples, patterns.
    Expressions = 1 << 0,
    /// Tree literals: TSX-like syntax generalized for any tree-shaped data.
    Trees = 1 << 1,
    /// Annotations: decorators (`@`) extended to any expression.
    Annotations = 1 << 2,
    /// Type system extensions: newtypes, primitives, structs, constraints.
    Types = 1 << 3,
    /// Reflection: types as values, runtime type descriptors, decorator metadata.
    Reflection = 1 << 4,
    /// Dispatch: extensions and overloading (type-based method/function dispatch).
    Dispatch = 1 << 5,
    /// Ownership: value ownership (`&T`, `^T`), mutability (`const`/`var`), and explicit dispatch.
    Ownership = 1 << 6,
}

impl LanguageFeature {
    /// All language features.
    pub const ALL: &[LanguageFeature] = &[
        Self::Expressions,
        Self::Trees,
        Self::Annotations,
        Self::Types,
        Self::Reflection,
        Self::Dispatch,
        Self::Ownership,
    ];

    /// Human-readable name for error messages.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Expressions => "expressions",
            Self::Trees => "trees",
            Self::Annotations => "annotations",
            Self::Types => "types",
            Self::Reflection => "reflection",
            Self::Dispatch => "dispatch",
            Self::Ownership => "ownership",
        }
    }

    /// Convert to bitmask value.
    #[inline]
    pub const fn as_bit(&self) -> u8 {
        *self as u8
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
    bits: u8,
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
        let mut bits = 0u8;
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
    pub const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    /// Get the raw bits.
    #[inline]
    pub const fn bits(&self) -> u8 {
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
        assert!(!set.is_enabled(LanguageFeature::Types));

        set.enable(LanguageFeature::Types);
        assert!(set.is_enabled(LanguageFeature::Types));
        assert!(!set.is_enabled(LanguageFeature::Ownership));

        set.disable(LanguageFeature::Types);
        assert!(!set.is_enabled(LanguageFeature::Types));
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
            .with(LanguageFeature::Types)
            .with(LanguageFeature::Dispatch);

        assert!(set.is_enabled(LanguageFeature::Types));
        assert!(set.is_enabled(LanguageFeature::Dispatch));
        assert!(!set.is_enabled(LanguageFeature::Ownership));
    }
}

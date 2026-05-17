/// Result of an alias query.
///
/// Ordered from strongest to weakest: MustAlias < PartialAlias < MayAlias < NoAlias.
/// NoAlias is the "best" result for optimization (proves non-interference).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AliasResult {
    /// The locations definitely refer to the same memory.
    MustAlias,
    /// The locations partially overlap (one contains part of the other).
    PartialAlias,
    /// The locations might refer to the same memory (conservative).
    MayAlias,
    /// The locations definitely do not overlap.
    NoAlias,
}

impl AliasResult {
    /// Check if this result indicates definite non-aliasing.
    pub fn is_no_alias(self) -> bool {
        matches!(self, AliasResult::NoAlias)
    }

    /// Check if the locations may alias (not definitely disjoint).
    pub fn may_alias(self) -> bool {
        !matches!(self, AliasResult::NoAlias)
    }

    /// Check if this is a definite must-alias.
    pub fn is_must_alias(self) -> bool {
        matches!(self, AliasResult::MustAlias)
    }

    /// Merge two alias results, returning the most conservative.
    ///
    /// Used when combining results from multiple code paths.
    pub fn merge(self, other: AliasResult) -> AliasResult {
        // if either says NoAlias and the other says MustAlias, something's wrong
        // but we conservatively return MayAlias
        match (self, other) {
            (AliasResult::NoAlias, AliasResult::NoAlias) => AliasResult::NoAlias,
            (AliasResult::MustAlias, AliasResult::MustAlias) => AliasResult::MustAlias,
            _ => AliasResult::MayAlias,
        }
    }
}

/// Mod/ref behavior flags for memory operations.
///
/// Indicates whether an instruction reads (Ref), writes (Mod), or both.
/// Uses a bitfield representation for efficient intersection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModRefInfo(u8);

impl ModRefInfo {
    /// No memory access.
    pub const NO_MOD_REF: Self = Self(0);
    /// Reads memory only.
    pub const REF: Self = Self(1);
    /// Writes memory only.
    pub const MOD: Self = Self(2);
    /// Reads and writes memory.
    pub const MOD_REF: Self = Self(3);

    /// Check if this includes a read.
    pub fn is_ref(self) -> bool {
        self.0 & 1 != 0
    }

    /// Check if this includes a write.
    pub fn is_mod(self) -> bool {
        self.0 & 2 != 0
    }

    /// Check if this has no memory effects.
    pub fn is_no_mod_ref(self) -> bool {
        self.0 == 0
    }

    /// Intersect with another ModRefInfo (both must agree).
    ///
    /// Used when multiple analyses provide mod/ref information.
    /// If any analysis says "no mod", the result has no mod.
    pub fn intersect(self, other: ModRefInfo) -> ModRefInfo {
        Self(self.0 & other.0)
    }

    /// Union with another ModRefInfo.
    ///
    /// Used when combining effects from multiple operations.
    pub fn union(self, other: ModRefInfo) -> ModRefInfo {
        Self(self.0 | other.0)
    }

    /// Create from separate read/write flags.
    pub fn from_flags(reads: bool, writes: bool) -> Self {
        let mut val = 0;
        if reads {
            val |= 1;
        }
        if writes {
            val |= 2;
        }
        Self(val)
    }
}

impl Default for ModRefInfo {
    fn default() -> Self {
        Self::MOD_REF
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alias_result_merge() {
        // same results preserve
        assert_eq!(
            AliasResult::NoAlias.merge(AliasResult::NoAlias),
            AliasResult::NoAlias
        );
        assert_eq!(
            AliasResult::MustAlias.merge(AliasResult::MustAlias),
            AliasResult::MustAlias
        );
        assert_eq!(
            AliasResult::MayAlias.merge(AliasResult::MayAlias),
            AliasResult::MayAlias
        );
        assert_eq!(
            AliasResult::PartialAlias.merge(AliasResult::PartialAlias),
            AliasResult::MayAlias
        );

        // conflicting results go to MayAlias
        assert_eq!(
            AliasResult::NoAlias.merge(AliasResult::MayAlias),
            AliasResult::MayAlias
        );
        assert_eq!(
            AliasResult::MustAlias.merge(AliasResult::NoAlias),
            AliasResult::MayAlias
        );
        assert_eq!(
            AliasResult::PartialAlias.merge(AliasResult::MustAlias),
            AliasResult::MayAlias
        );
    }

    #[test]
    fn test_alias_result_predicates() {
        assert!(AliasResult::NoAlias.is_no_alias());
        assert!(!AliasResult::MayAlias.is_no_alias());

        assert!(AliasResult::MustAlias.is_must_alias());
        assert!(!AliasResult::PartialAlias.is_must_alias());

        assert!(AliasResult::MayAlias.may_alias());
        assert!(AliasResult::MustAlias.may_alias());
        assert!(!AliasResult::NoAlias.may_alias());
    }

    #[test]
    fn test_mod_ref_info() {
        assert!(!ModRefInfo::NO_MOD_REF.is_ref());
        assert!(!ModRefInfo::NO_MOD_REF.is_mod());
        assert!(ModRefInfo::NO_MOD_REF.is_no_mod_ref());

        assert!(ModRefInfo::REF.is_ref());
        assert!(!ModRefInfo::REF.is_mod());

        assert!(!ModRefInfo::MOD.is_ref());
        assert!(ModRefInfo::MOD.is_mod());

        assert!(ModRefInfo::MOD_REF.is_ref());
        assert!(ModRefInfo::MOD_REF.is_mod());
    }

    #[test]
    fn test_mod_ref_intersect() {
        assert_eq!(
            ModRefInfo::MOD_REF.intersect(ModRefInfo::REF),
            ModRefInfo::REF
        );
        assert_eq!(
            ModRefInfo::MOD_REF.intersect(ModRefInfo::MOD),
            ModRefInfo::MOD
        );
        assert_eq!(
            ModRefInfo::REF.intersect(ModRefInfo::MOD),
            ModRefInfo::NO_MOD_REF
        );
    }

    #[test]
    fn test_mod_ref_union() {
        assert_eq!(ModRefInfo::REF.union(ModRefInfo::MOD), ModRefInfo::MOD_REF);
        assert_eq!(
            ModRefInfo::NO_MOD_REF.union(ModRefInfo::REF),
            ModRefInfo::REF
        );
        assert_eq!(
            ModRefInfo::MOD_REF.union(ModRefInfo::MOD_REF),
            ModRefInfo::MOD_REF
        );
    }

    #[test]
    fn test_mod_ref_from_flags() {
        assert_eq!(ModRefInfo::from_flags(false, false), ModRefInfo::NO_MOD_REF);
        assert_eq!(ModRefInfo::from_flags(true, false), ModRefInfo::REF);
        assert_eq!(ModRefInfo::from_flags(false, true), ModRefInfo::MOD);
        assert_eq!(ModRefInfo::from_flags(true, true), ModRefInfo::MOD_REF);
    }

    #[test]
    fn test_mod_ref_default() {
        // default is conservative MOD_REF
        assert_eq!(ModRefInfo::default(), ModRefInfo::MOD_REF);
    }
}

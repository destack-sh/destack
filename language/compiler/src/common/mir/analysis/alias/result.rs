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

/// Memory behavior summary for a function.
///
/// Describes what kinds of memory a function may access.
/// Used to quickly filter out impossible aliasing at call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FunctionModRefBehavior {
    flags: u8,
}

impl FunctionModRefBehavior {
    /// Function reads memory.
    const READS_MEMORY: u8 = 1;
    /// Function writes memory.
    const WRITES_MEMORY: u8 = 2;
    /// Function accesses memory through argument pointers.
    const ACCESSES_ARG_MEM: u8 = 4;
    /// Function accesses global memory.
    const ACCESSES_GLOBAL_MEM: u8 = 8;
    /// Function accesses inaccessible memory (e.g., errno, internal state).
    const ACCESSES_INACCESSIBLE_MEM: u8 = 16;

    /// Function does not access any memory (readnone).
    pub const DOES_NOT_ACCESS_MEMORY: Self = Self { flags: 0 };

    /// Function only reads memory (readonly).
    pub fn only_reads_memory() -> Self {
        Self {
            flags: Self::READS_MEMORY
                | Self::ACCESSES_ARG_MEM
                | Self::ACCESSES_GLOBAL_MEM
                | Self::ACCESSES_INACCESSIBLE_MEM,
        }
    }

    /// Function only reads argument pointees (argmemonly + readonly).
    pub fn only_reads_arg_mem() -> Self {
        Self {
            flags: Self::READS_MEMORY | Self::ACCESSES_ARG_MEM,
        }
    }

    /// Function only accesses argument pointees (argmemonly).
    pub fn only_accesses_arg_mem() -> Self {
        Self {
            flags: Self::READS_MEMORY | Self::WRITES_MEMORY | Self::ACCESSES_ARG_MEM,
        }
    }

    /// Function may access any memory (conservative default).
    pub fn may_access_any_memory() -> Self {
        Self {
            flags: Self::READS_MEMORY
                | Self::WRITES_MEMORY
                | Self::ACCESSES_ARG_MEM
                | Self::ACCESSES_GLOBAL_MEM
                | Self::ACCESSES_INACCESSIBLE_MEM,
        }
    }

    /// Check if function reads memory.
    pub fn reads_memory(self) -> bool {
        self.flags & Self::READS_MEMORY != 0
    }

    /// Check if function writes memory.
    pub fn writes_memory(self) -> bool {
        self.flags & Self::WRITES_MEMORY != 0
    }

    /// Check if function only accesses memory through arguments.
    pub fn is_arg_mem_only(self) -> bool {
        let mem_mask = Self::ACCESSES_GLOBAL_MEM | Self::ACCESSES_INACCESSIBLE_MEM;
        self.flags & mem_mask == 0
    }

    /// Check if function accesses global memory.
    pub fn accesses_global_mem(self) -> bool {
        self.flags & Self::ACCESSES_GLOBAL_MEM != 0
    }

    /// Get ModRefInfo from this behavior.
    pub fn get_mod_ref(self) -> ModRefInfo {
        ModRefInfo::from_flags(self.reads_memory(), self.writes_memory())
    }
}

/// Attributes for a function parameter relevant to alias analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ParameterAttributes {
    flags: u8,
}

impl ParameterAttributes {
    /// Parameter does not alias other accessible memory.
    const NOALIAS: u8 = 1;
    /// Parameter pointer is not captured (stored or returned).
    const NOCAPTURE: u8 = 2;
    /// Parameter pointee is only read, not written.
    const READONLY: u8 = 4;
    /// Parameter pointee is only written, not read.
    const WRITEONLY: u8 = 8;
    /// Parameter pointer is known non-null.
    #[allow(dead_code)]
    const NONNULL: u8 = 16;

    /// No special attributes.
    pub const NONE: Self = Self { flags: 0 };

    /// Parameter does not alias other arguments or globals.
    pub fn noalias() -> Self {
        Self {
            flags: Self::NOALIAS,
        }
    }

    /// Parameter is not captured (stored to memory or returned).
    pub fn nocapture() -> Self {
        Self {
            flags: Self::NOCAPTURE,
        }
    }

    /// Parameter is only read through (not written).
    pub fn readonly() -> Self {
        Self {
            flags: Self::READONLY,
        }
    }

    /// Parameter is only written through (not read).
    pub fn writeonly() -> Self {
        Self {
            flags: Self::WRITEONLY,
        }
    }

    /// Check if noalias.
    pub fn is_noalias(self) -> bool {
        self.flags & Self::NOALIAS != 0
    }

    /// Check if nocapture.
    pub fn is_nocapture(self) -> bool {
        self.flags & Self::NOCAPTURE != 0
    }

    /// Check if readonly.
    pub fn is_readonly(self) -> bool {
        self.flags & Self::READONLY != 0
    }

    /// Check if writeonly.
    pub fn is_writeonly(self) -> bool {
        self.flags & Self::WRITEONLY != 0
    }

    /// Combine with another set of attributes.
    pub fn with(self, other: ParameterAttributes) -> Self {
        Self {
            flags: self.flags | other.flags,
        }
    }

    /// Get ModRefInfo for accesses through this parameter.
    pub fn get_mod_ref(self) -> ModRefInfo {
        if self.is_readonly() {
            ModRefInfo::REF
        } else if self.is_writeonly() {
            ModRefInfo::MOD
        } else {
            ModRefInfo::MOD_REF
        }
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

    #[test]
    fn test_function_mod_ref_behavior() {
        // readnone: no memory access
        let readnone = FunctionModRefBehavior::DOES_NOT_ACCESS_MEMORY;
        assert!(!readnone.reads_memory());
        assert!(!readnone.writes_memory());
        assert!(readnone.get_mod_ref().is_no_mod_ref());

        // readonly: reads but doesn't write
        let readonly = FunctionModRefBehavior::only_reads_memory();
        assert!(readonly.reads_memory());
        assert!(!readonly.writes_memory());
        assert!(readonly.get_mod_ref().is_ref());
        assert!(!readonly.get_mod_ref().is_mod());

        // argmemonly + readonly
        let argmem_readonly = FunctionModRefBehavior::only_reads_arg_mem();
        assert!(argmem_readonly.is_arg_mem_only());
        assert!(argmem_readonly.reads_memory());
        assert!(!argmem_readonly.writes_memory());

        // argmemonly: only accesses argument pointees
        let argmemonly = FunctionModRefBehavior::only_accesses_arg_mem();
        assert!(argmemonly.is_arg_mem_only());
        assert!(!argmemonly.accesses_global_mem());
        assert!(argmemonly.reads_memory());
        assert!(argmemonly.writes_memory());

        // may access anything
        let any = FunctionModRefBehavior::may_access_any_memory();
        assert!(!any.is_arg_mem_only());
        assert!(any.accesses_global_mem());
        assert!(any.reads_memory());
        assert!(any.writes_memory());
    }

    #[test]
    fn test_param_attrs() {
        // noalias
        let noalias = ParameterAttributes::noalias();
        assert!(noalias.is_noalias());
        assert!(!noalias.is_readonly());
        assert!(!noalias.is_nocapture());

        // nocapture
        let nocapture = ParameterAttributes::nocapture();
        assert!(nocapture.is_nocapture());
        assert!(!nocapture.is_noalias());

        // readonly
        let readonly = ParameterAttributes::readonly();
        assert!(readonly.is_readonly());
        assert!(!readonly.is_writeonly());
        assert_eq!(readonly.get_mod_ref(), ModRefInfo::REF);

        // writeonly
        let writeonly = ParameterAttributes::writeonly();
        assert!(writeonly.is_writeonly());
        assert!(!writeonly.is_readonly());
        assert_eq!(writeonly.get_mod_ref(), ModRefInfo::MOD);

        // combined attributes
        let combined = ParameterAttributes::noalias().with(ParameterAttributes::readonly());
        assert!(combined.is_noalias());
        assert!(combined.is_readonly());
        assert_eq!(combined.get_mod_ref(), ModRefInfo::REF);

        // none has no attributes
        assert!(!ParameterAttributes::NONE.is_noalias());
        assert!(!ParameterAttributes::NONE.is_nocapture());
        assert!(!ParameterAttributes::NONE.is_readonly());
        assert_eq!(ParameterAttributes::NONE.get_mod_ref(), ModRefInfo::MOD_REF);
    }
}

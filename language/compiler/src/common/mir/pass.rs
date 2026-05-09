/// Requirements for running a MIR pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassRequirements {
    /// Bitmask storing requirement flags.
    bits: u32,
}

impl PassRequirements {
    /// No special requirements.
    pub const NONE: Self = Self { bits: 0 };
    /// Call instructions must carry call effects metadata.
    pub const CALL_EFFECTS: Self = Self { bits: 1 << 0 };
    /// Memory access instructions must carry memory access metadata.
    pub const MEMORY_ACCESS_METADATA: Self = Self { bits: 1 << 1 };
    /// Profile data must be present in the pipeline context.
    pub const PROFILE_DATA: Self = Self { bits: 1 << 2 };
    /// Aggregate types must carry layout metadata.
    pub const TYPE_LAYOUTS: Self = Self { bits: 1 << 3 };

    /// Return true when no requirements are set.
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// Return true when all bits in other are present.
    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }

    /// Return the union of two requirement sets.
    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }
}

impl std::ops::BitOr for PassRequirements {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            bits: self.bits | rhs.bits,
        }
    }
}

impl std::ops::BitOrAssign for PassRequirements {
    fn bitor_assign(&mut self, rhs: Self) {
        self.bits |= rhs.bits;
    }
}

/// Static metadata about a MIR pass.
#[derive(Debug, Clone, Copy)]
pub struct PassMetadata {
    /// Pass ID like "constant-fold".
    pub id: &'static str,
    /// Pass name like "ConstantFold".
    pub name: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// Metadata required before this pass can run.
    pub requirements: PassRequirements,
}

/// Base trait for MIR passes.
pub trait Pass: Send + Sync {
    /// Return the static metadata for this pass.
    fn metadata(&self) -> &'static PassMetadata;
}

/// Declare one MIR pass and its static metadata.
#[macro_export]
macro_rules! declare_mir_pass {
    (
        $(#[doc = $doc:literal])*
        #[pass(id = $id:literal $(, requires($($requirement:ident),* $(,)?))?)]
        $visibility:vis $name:ident,
        $description:literal $(,)?
    ) => {
        $(#[doc = $doc])*
        #[derive(Debug, Clone, Copy)]
        $visibility struct $name;

        impl $crate::common::mir::Pass for $name {
            fn metadata(&self) -> &'static $crate::common::mir::PassMetadata {
                Self::metadata()
            }
        }

        impl $name {
            /// Static metadata for this pass.
            $visibility const METADATA: $crate::common::mir::PassMetadata =
                $crate::common::mir::PassMetadata {
                    id: $id,
                    name: stringify!($name),
                    description: $description,
                    requirements: $crate::declare_mir_pass!(@requirements $($($requirement),*)?),
            };

            /// Return the pass metadata.
            $visibility const fn metadata() -> &'static $crate::common::mir::PassMetadata {
                &Self::METADATA
            }
        }
    };

    (@requirements) => {
        $crate::common::mir::PassRequirements::NONE
    };

    (@requirements $first:ident $(, $rest:ident)*) => {
        $crate::declare_mir_pass!(@requirement $first)$(.union($crate::declare_mir_pass!(@requirement $rest)))*
    };

    (@requirement call_effects) => {
        $crate::common::mir::PassRequirements::CALL_EFFECTS
    };

    (@requirement memory_access_metadata) => {
        $crate::common::mir::PassRequirements::MEMORY_ACCESS_METADATA
    };

    (@requirement profile_data) => {
        $crate::common::mir::PassRequirements::PROFILE_DATA
    };

    (@requirement type_layouts) => {
        $crate::common::mir::PassRequirements::TYPE_LAYOUTS
    };
}

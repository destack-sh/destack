use serde::{Deserialize, Serialize};

use crate::{DEFAULT_SIZE_CLASS_BYTES, DEFAULT_SIZE_CLASS_TABLE_NAME};

/// One fixed-size small allocation class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SizeClass {
    /// The slot payload size in bytes.
    pub bytes: usize,
}

impl SizeClass {
    /// Create one size class.
    pub const fn new(bytes: usize) -> Self {
        Self { bytes }
    }
}

/// One canonical size-class table used by the heap.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SizeClassTable {
    /// The stable table name for configuration and image compatibility.
    pub name: String,
    /// The ordered size classes in bytes.
    pub classes: Vec<SizeClass>,
}

impl SizeClassTable {
    /// Create one validated size-class table.
    pub fn new(
        name: impl Into<String>,
        classes: impl IntoIterator<Item = usize>,
    ) -> Result<Self, SizeClassTableError> {
        let name = name.into();
        let classes = classes.into_iter().collect::<Vec<_>>();

        Self::validate(&classes)?;

        Ok(Self {
            name,
            classes: classes.into_iter().map(SizeClass::new).collect(),
        })
    }

    /// Return the built-in default size-class table.
    pub fn default_table() -> Self {
        debug_assert!(Self::validate(DEFAULT_SIZE_CLASS_BYTES).is_ok());

        Self {
            name: DEFAULT_SIZE_CLASS_TABLE_NAME.to_string(),
            classes: DEFAULT_SIZE_CLASS_BYTES
                .iter()
                .copied()
                .map(SizeClass::new)
                .collect(),
        }
    }

    /// Return the largest span-allocated payload size in bytes.
    pub fn max_small_allocation_bytes(&self) -> usize {
        self.classes
            .last()
            .map(|class| class.bytes)
            .unwrap_or_default()
    }

    /// Return the smallest span-allocated payload size in bytes.
    pub fn min_small_allocation_bytes(&self) -> usize {
        self.classes.first().map(|class| class.bytes).unwrap_or(1)
    }

    /// Return one best-fit size class for the given payload size.
    pub fn class_for(&self, bytes: usize) -> Option<SizeClass> {
        self.classes
            .iter()
            .copied()
            .find(|class| class.bytes >= bytes)
    }

    /// Return one best-fit size class index for the given payload size.
    pub fn class_index_for(&self, bytes: usize) -> Option<usize> {
        self.classes.iter().position(|class| class.bytes >= bytes)
    }

    /// Return the retained bytes owned by this size-class table.
    pub fn retained_bytes(&self) -> usize {
        self.name.capacity() + self.classes.capacity() * std::mem::size_of::<SizeClass>()
    }


    /// Resolve one configured size-class selection.
    pub fn from_selection(
        selection: &SizeClassTableSelection,
    ) -> Result<Self, SizeClassTableError> {
        match selection {
            SizeClassTableSelection::Default => Ok(Self::default_table()),
            SizeClassTableSelection::Named(name) => match name.as_str() {
                "default" => Ok(Self::default_table()),
                _ => Err(SizeClassTableError::UnknownPreset { name: name.clone() }),
            },
            SizeClassTableSelection::Explicit(classes) => Self::new("explicit", classes.clone()),
        }
    }

    /// Validate one raw size-class list.
    fn validate(classes: &[usize]) -> Result<(), SizeClassTableError> {
        if classes.is_empty() {
            return Err(SizeClassTableError::Empty);
        }

        let mut previous = 0usize;
        for &bytes in classes {
            if bytes == 0 {
                return Err(SizeClassTableError::ZeroClass);
            }

            if bytes % 8 != 0 {
                return Err(SizeClassTableError::Misaligned { bytes });
            }

            if bytes <= previous {
                return Err(SizeClassTableError::NotStrictlyIncreasing { previous, bytes });
            }

            previous = bytes;
        }

        Ok(())
    }
}

impl Default for SizeClassTable {
    fn default() -> Self {
        Self::default_table()
    }
}

/// One configured size-class table selection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SizeClassTableSelection {
    /// The built-in default table.
    #[default]
    Default,
    /// One named built-in table.
    Named(String),
    /// One explicit table from configuration.
    Explicit(Vec<usize>),
}

/// Validation error for one configured size-class table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SizeClassTableError {
    /// The configured table is empty.
    Empty,
    /// One size class was zero.
    ZeroClass,
    /// One size class violated 8-byte alignment.
    Misaligned { bytes: usize },
    /// The configured classes were not strictly increasing.
    NotStrictlyIncreasing { previous: usize, bytes: usize },
    /// The configured built-in preset name was unknown.
    UnknownPreset { name: String },
}

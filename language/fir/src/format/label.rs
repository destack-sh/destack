/// Unique identifier for a label in the formatting system.
#[derive(Debug, Eq, Copy, Clone)]
pub struct LabelId {
    value: u64,
    #[cfg(debug_assertions)]
    name: &'static str,
}

impl PartialEq for LabelId {
    fn eq(&self, other: &Self) -> bool {
        let is_equal = self.value == other.value;

        #[cfg(debug_assertions)]
        {
            if is_equal {
                assert_eq!(
                    self.name, other.name,
                    "Two `LabelId`s with different names have the same `value`. Are you mixing labels of two different `LabelDeclaration` or are the values returned by the `LabelDeclaration` not unique?"
                );
            }
        }

        is_equal
    }
}

impl LabelId {
    /// Create a LabelId from a LabelDeclaration.
    #[expect(clippy::needless_pass_by_value)]
    pub fn of<T: LabelDeclaration>(label: T) -> Self {
        Self {
            value: label.value(),
            #[cfg(debug_assertions)]
            name: label.name(),
        }
    }
}

/// Defines the valid labels of a language.
/// You want to have at most one implementation per formatter project.
pub trait LabelDeclaration {
    /// Gets the `u64` uniquely identifying this specific label.
    fn value(&self) -> u64;

    /// Gets the name of the label that is shown in debug builds.
    fn name(&self) -> &'static str;
}

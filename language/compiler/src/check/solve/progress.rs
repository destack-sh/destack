use smallvec::SmallVec;

use crate::check::VariableId;

/// Variable changes produced by one solver reduction.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Progress {
    /// The reduction did not change any variable.
    Unchanged,
    /// The work item changed these variables.
    Changed(SmallVec<[VariableId; 4]>),
}

impl Progress {
    /// Return one changed variable.
    pub(in crate::check) fn changed(variable: VariableId) -> Self {
        Self::Changed(smallvec::smallvec![variable])
    }

    /// Return progress from a conditional variable change.
    pub(in crate::check) fn from_change(variable: VariableId, is_changed: bool) -> Self {
        if is_changed {
            Self::changed(variable)
        } else {
            Self::Unchanged
        }
    }

    /// Return whether this progress did not change any variable.
    pub(in crate::check) fn is_unchanged(&self) -> bool {
        matches!(self, Self::Unchanged)
    }

    /// Merge two progress values.
    pub(in crate::check) fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unchanged, progress) | (progress, Self::Unchanged) => progress,
            (Self::Changed(mut left), Self::Changed(right)) => {
                left.extend(right);

                Self::Changed(left)
            }
        }
    }
}

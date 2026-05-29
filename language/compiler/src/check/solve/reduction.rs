use crate::check::Progress;

/// Optional reduced value plus the variable changes made while reducing it.
pub(in crate::check) struct Reduction<T> {
    /// The reduced value when the term is ready.
    pub(in crate::check) value: Option<T>,
    /// The variable changes produced while reducing.
    pub(in crate::check) progress: Progress,
}

impl<T> Reduction<T> {
    /// Return a reduced value with no side progress.
    pub(in crate::check) fn value(value: T) -> Self {
        Self {
            value: Some(value),
            progress: Progress::Unchanged,
        }
    }

    /// Return a reduced value with side progress.
    pub(in crate::check) fn with_progress(value: T, progress: Progress) -> Self {
        Self {
            value: Some(value),
            progress,
        }
    }

    /// Return a pending reduction with no side progress.
    pub(in crate::check) fn pending() -> Self {
        Self {
            value: None,
            progress: Progress::Unchanged,
        }
    }

    /// Return a pending reduction with side progress.
    pub(in crate::check) fn progress(progress: Progress) -> Self {
        Self {
            value: None,
            progress,
        }
    }
}

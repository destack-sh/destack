use smallvec::SmallVec;

use crate::check::Dependency;

/// One answer that may need more solver progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum Answer<T> {
    /// The answer is available.
    Ready(T),
    /// The answer is waiting on unresolved dependencies.
    Pending(SmallVec<[Dependency; 2]>),
}

impl<T> Answer<T> {
    /// Return a pending answer blocked by one dependency stream.
    pub(in crate::check) fn pending(dependencies: impl IntoIterator<Item = Dependency>) -> Self {
        let mut pending = SmallVec::new();

        for dependency in dependencies {
            if !pending.contains(&dependency) {
                pending.push(dependency);
            }
        }

        Self::Pending(pending)
    }

    /// Return whether this answer is pending.
    pub(in crate::check) fn is_pending(&self) -> bool {
        matches!(self, Self::Pending(_))
    }

    /// Map a ready answer while preserving pending.
    pub(in crate::check) fn map<U>(self, f: impl FnOnce(T) -> U) -> Answer<U> {
        match self {
            Self::Ready(value) => Answer::Ready(f(value)),
            Self::Pending(dependencies) => Answer::Pending(dependencies),
        }
    }
}

impl Answer<bool> {
    /// Return this answer's stable trace label.
    pub(in crate::check) fn label(&self) -> &'static str {
        match self {
            Self::Ready(true) => "yes",
            Self::Ready(false) => "no",
            Self::Pending(_) => "pending",
        }
    }

    /// Combine boolean answers that must both hold.
    pub(in crate::check) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Ready(false), _) | (_, Self::Ready(false)) => Self::Ready(false),
            (Self::Pending(left), Self::Pending(right)) => {
                Self::pending(left.into_iter().chain(right))
            }
            (Self::Pending(dependencies), _) | (_, Self::Pending(dependencies)) => {
                Self::Pending(dependencies)
            }
            (Self::Ready(true), Self::Ready(true)) => Self::Ready(true),
        }
    }

    /// Combine boolean answers where either one may hold.
    pub(in crate::check) fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Ready(true), _) | (_, Self::Ready(true)) => Self::Ready(true),
            (Self::Pending(left), Self::Pending(right)) => {
                Self::pending(left.into_iter().chain(right))
            }
            (Self::Pending(dependencies), _) | (_, Self::Pending(dependencies)) => {
                Self::Pending(dependencies)
            }
            (Self::Ready(false), Self::Ready(false)) => Self::Ready(false),
        }
    }
}

impl Answer<()> {
    /// Return this answer's stable trace label.
    pub(in crate::check) fn label(&self) -> &'static str {
        match self {
            Self::Ready(()) => "done",
            Self::Pending(_) => "pending",
        }
    }
}

impl From<bool> for Answer<bool> {
    fn from(value: bool) -> Self {
        Self::Ready(value)
    }
}

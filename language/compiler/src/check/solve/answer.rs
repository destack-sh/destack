use destack_dir as dir;
use smallvec::SmallVec;

/// One answer that may need more solver progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum Answer<T> {
    /// The answer is available.
    Ready(T),
    /// The answer is waiting on unresolved dependencies.
    Pending(SmallVec<[Dependency; 2]>),
}

/// One dependency that can wake solver work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Dependency {
    /// A variable was solved.
    Variable(dir::TypeVariableId),
    /// A symbol type was written.
    SymbolType(dir::GlobalSymbolId),
}

impl<T> Answer<T> {
    /// Return the ready value, if this answer has settled.
    pub(in crate::check) fn ready(self) -> Option<T> {
        match self {
            Self::Ready(value) => Some(value),
            Self::Pending(_) => None,
        }
    }

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

    /// Return a ready value unless dependencies still block it.
    pub(in crate::check) fn ready_unless_blocked(
        value: T,
        dependencies: impl IntoIterator<Item = Dependency>,
    ) -> Self {
        match Self::pending(dependencies) {
            Self::Pending(pending) if pending.is_empty() => Self::Ready(value),
            pending => pending,
        }
    }
}

impl Answer<bool> {
    /// Return whether this answer is ready true.
    pub(in crate::check) fn is_ready_true(&self) -> bool {
        matches!(self, Self::Ready(true))
    }

    /// Return whether this answer is ready false.
    pub(in crate::check) fn is_ready_false(&self) -> bool {
        matches!(self, Self::Ready(false))
    }

    /// Return `Some(value)` when this answer is true.
    pub(in crate::check) fn then_some<T>(self, value: T) -> Answer<Option<T>> {
        match self {
            Self::Ready(true) => Answer::Ready(Some(value)),
            Self::Ready(false) => Answer::Ready(None),
            Self::Pending(blockers) => Answer::Pending(blockers),
        }
    }

    /// Combine two boolean answers conjunctively.
    pub(in crate::check) fn and(self, other: Self) -> Self {
        match (self, other) {
            // definite failure wins over pending
            (Self::Ready(false), _) | (_, Self::Ready(false)) => Self::Ready(false),
            (Self::Ready(true), other) => other,
            (pending, Self::Ready(true)) => pending,
            (Self::Pending(left), Self::Pending(right)) => {
                Self::pending(left.into_iter().chain(right))
            }
        }
    }

    /// Combine two boolean answers disjunctively.
    pub(in crate::check) fn or(self, other: Self) -> Self {
        match (self, other) {
            // definite success wins over pending
            (Self::Ready(true), _) | (_, Self::Ready(true)) => Self::Ready(true),
            (Self::Ready(false), other) => other,
            (pending, Self::Ready(false)) => pending,
            (Self::Pending(left), Self::Pending(right)) => {
                Self::pending(left.into_iter().chain(right))
            }
        }
    }
}

/// Unwrap one ready solver answer or propagate its pending blockers.
macro_rules! answer {
    ($answer:expr $(,)?) => {
        match $answer {
            Answer::Ready(value) => value,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        }
    };
}

/// Export `answer!` to every check module.
pub(in crate::check) use answer;

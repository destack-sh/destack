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
}

impl Answer<bool> {
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

/// One dependency that can wake solver work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Dependency {
    /// A variable was solved.
    Variable(dir::TypeVariableId),
    /// A node decision was made.
    Decision(dir::GlobalNodeIdAny),
}

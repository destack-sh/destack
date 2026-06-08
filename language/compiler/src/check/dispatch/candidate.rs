/// Candidate cardinality for speculative dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CandidateCardinality {
    /// No candidate exists.
    Empty,
    /// One candidate can keep pending inference work.
    One,
    /// Multiple candidates must not commit branch-local pending work.
    Many,
}

impl CandidateCardinality {
    /// Return the candidate cardinality from a candidate count.
    pub(in crate::check) fn from_len(len: usize) -> Self {
        match len {
            0 => Self::Empty,
            1 => Self::One,
            _ => Self::Many,
        }
    }

    /// Return whether this represents many candidates.
    pub(in crate::check) fn is_many(self) -> bool {
        self == Self::Many
    }
}

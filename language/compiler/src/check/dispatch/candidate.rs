/// Candidate set shape for speculative dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CandidateSet {
    /// One candidate can keep pending inference work.
    Single,
    /// Multiple candidates must not commit branch-local pending work.
    Overload,
}

impl CandidateSet {
    /// Return the candidate set shape from a candidate count.
    pub(in crate::check) fn from_len(len: usize) -> Self {
        match len {
            1 => Self::Single,
            _ => Self::Overload,
        }
    }

    /// Return whether pending probe work belongs to a unique candidate.
    pub(in crate::check) fn keeps_pending_probe(self) -> bool {
        self == Self::Single
    }
}

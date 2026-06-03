/// Representation constraints attached to one declaration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::check) struct RepresentationConstraint {
    /// The minimum aggregate alignment in bytes.
    pub(in crate::check) minimum_alignment: Option<u32>,
    /// The maximum field alignment in bytes.
    pub(in crate::check) field_alignment_limit: Option<u32>,
}

impl RepresentationConstraint {
    /// Return the field alignment after packing constraints.
    pub(in crate::check) fn field_alignment(self, alignment: u32) -> u32 {
        match self.field_alignment_limit {
            Some(limit) => alignment.min(limit).max(1),
            None => alignment.max(1),
        }
    }

    /// Return the aggregate alignment after alignment constraints.
    pub(in crate::check) fn aggregate_alignment(self, alignment: u32) -> u32 {
        match self.minimum_alignment {
            Some(minimum) => alignment.max(minimum).max(1),
            None => alignment.max(1),
        }
    }
}

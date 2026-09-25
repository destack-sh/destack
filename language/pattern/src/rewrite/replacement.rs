use std::sync::Arc;

use tspp_source::File;

use crate::Fragment;

/// A parsed replacement fragment and its authored source.
#[derive(Debug)]
pub struct Replacement {
    /// The authored replacement source.
    pub(crate) file: Arc<File>,
    /// The parsed replacement fragment.
    pub(crate) fragment: Fragment,
}

impl Replacement {
    /// Return the authored replacement source.
    pub fn file(&self) -> &File {
        &self.file
    }

    /// Return the parsed replacement fragment.
    pub fn fragment(&self) -> &Fragment {
        &self.fragment
    }
}

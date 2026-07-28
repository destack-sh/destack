use crate::{ModuleQueryContext, ProgramQueryContext};

macro_rules! formatted {
    ($expression:expr) => {
        match $expression? {
            Some(text) => text,
            None => return Ok(None),
        }
    };
}

pub(super) use formatted;

/// Formatter for language text from one program module.
pub(crate) struct Formatter<'owner, 'module, 'program> {
    /// The module that owns local ids read by the formatter.
    pub(super) module: &'owner ModuleQueryContext<'module>,
    /// The program used for global DIR reads.
    pub(super) program: &'owner ProgramQueryContext<'program>,
}

impl<'owner, 'module, 'program> Formatter<'owner, 'module, 'program> {
    /// Create a formatter for one module.
    pub(crate) fn new(
        module: &'owner ModuleQueryContext<'module>,
        program: &'owner ProgramQueryContext<'program>,
    ) -> Self {
        Self { module, program }
    }
}

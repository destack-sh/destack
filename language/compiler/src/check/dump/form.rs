use destack_dir as dir;

use crate::check::{Dump, DumpContext, FormTerm};

use super::format::dump_record;

impl Dump for FormTerm {
    /// Render one memory form term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Managed => dump_record("FormTerm.Managed", []),
            Self::Owned => dump_record("FormTerm.Owned", []),
            Self::Borrowed { lifetime, access } => dump_record(
                "FormTerm.Borrowed",
                [
                    ("lifetime", lifetime.dump(context)),
                    ("access", access.dump(context)),
                ],
            ),
            Self::Raw => dump_record("FormTerm.Raw", []),
            Self::Placed { place } => {
                dump_record("FormTerm.Placed", [("place", place.dump(context))])
            }
            Self::Readonly => dump_record("FormTerm.Readonly", []),
        }
    }
}

impl Dump for dir::Access {
    /// Render one access value.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Readonly => "readonly".to_string(),
            Self::Mutable => "mutable".to_string(),
            Self::Exclusive => "exclusive".to_string(),
        }
    }
}

impl Dump for dir::Space {
    /// Render one storage space.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Local => "local".to_string(),
            Self::Shared => "shared".to_string(),
            Self::Static => "static".to_string(),
            Self::Frame => "frame".to_string(),
        }
    }
}

impl Dump for dir::Place {
    /// Render one memory place.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Ambient => "ambient".to_string(),
            Self::Space(space) => space.dump(context),
        }
    }
}

impl Dump for dir::Lifetime {
    /// Render one lifetime.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Static => "static".to_string(),
            Self::Symbol(symbol) => context.symbol_label(*symbol),
        }
    }
}

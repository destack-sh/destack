use tspp_dir as dir;

use crate::{DocResult, Generator, Module};

/// One byte interval inside formatted text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FormattedRange {
    /// The inclusive byte offset.
    pub(crate) start: u32,
    /// The exclusive byte offset.
    pub(crate) end: u32,
}

/// One declaration signature with its declared name interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FormattedSignature {
    /// The complete signature text.
    pub(crate) text: String,
    /// The declared name interval when one exists.
    pub(crate) name: Option<FormattedRange>,
}

impl FormattedSignature {
    /// Create text without a declared name.
    pub(crate) fn plain(text: String) -> Self {
        Self { text, name: None }
    }

    /// Join one prefix, declared name, and suffix.
    pub(crate) fn named(prefix: String, name: &str, suffix: String) -> Self {
        let start = prefix.len() as u32;
        let end = start + name.len() as u32;
        let text = format!("{prefix}{name}{suffix}");

        Self {
            text,
            name: Some(FormattedRange { start, end }),
        }
    }

    /// Prepend text while retaining the declared name interval.
    pub(crate) fn prepend(mut self, prefix: &str) -> Self {
        if let Some(name) = &mut self.name {
            let length = prefix.len() as u32;
            name.start += length;
            name.end += length;
        }
        self.text.insert_str(0, prefix);

        self
    }
}

/// Printer for checked declarations and types.
pub(crate) struct Printer<'owner, 'module, 'program> {
    /// The module that owns local ids read by the printer.
    pub(super) module: &'owner Module<'module>,
    /// The checked program.
    pub(super) program: &'owner Generator<'program>,
}

impl<'owner, 'module, 'program> Printer<'owner, 'module, 'program> {
    /// Create a printer for one module.
    pub(crate) fn new(
        module: &'owner Module<'module>,
        program: &'owner Generator<'program>,
    ) -> Self {
        Self { module, program }
    }

    /// Return the type table visible to this printer.
    pub(super) fn types(&self) -> &dir::TypeTable<'_> {
        self.module.types()
    }

    /// Read one type through the table that owns its local id.
    pub(super) fn read_type<R>(
        &self,
        type_id: dir::GlobalTypeId,
        read: impl FnOnce(&dir::Type, &Printer<'_, '_, '_>) -> DocResult<R>,
    ) -> DocResult<R> {
        if type_id.module_id == self.module.module_id() {
            let type_value = self.types().get_type(type_id.local_id);

            read(&type_value, self)
        } else {
            self.program.read_type(type_id, |type_value, module| {
                let formatter = Printer::new(module, self.program);

                read(type_value, &formatter)
            })
        }
    }
}

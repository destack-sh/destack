use crate::format::{Instruction, PrintResult};
use crate::print::call::PrintArgs;

/// The line suffix instructions awaiting a line break.
#[derive(Debug, Default)]
pub(super) struct LineSuffixes<'a> {
    /// The pending suffix entries.
    suffixes: Vec<LineSuffixEntry<'a>>,
}

impl<'a> LineSuffixes<'a> {
    /// Append suffix instructions and their print arguments.
    pub(super) fn extend<I>(&mut self, args: PrintArgs, instructions: I) -> PrintResult<()>
    where
        I: IntoIterator<Item = PrintResult<Instruction<'a>>>,
    {
        for instruction in instructions {
            self.suffixes.push(LineSuffixEntry::Suffix(instruction?));
        }

        self.suffixes.push(LineSuffixEntry::Args(args));

        Ok(())
    }

    /// Drain all pending suffix entries.
    pub(super) fn take_pending(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = LineSuffixEntry<'a>> + '_ + ExactSizeIterator {
        self.suffixes.drain(..)
    }

    /// Return whether any suffix is pending.
    pub(super) fn has_pending(&self) -> bool {
        !self.suffixes.is_empty()
    }
}

/// One pending line suffix entry.
#[derive(Debug, Copy, Clone)]
pub(super) enum LineSuffixEntry<'a> {
    /// One suffix instruction.
    Suffix(Instruction<'a>),
    /// The print arguments for following suffix instructions.
    Args(PrintArgs),
}

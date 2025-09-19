use crate::format::FormatNode;
use crate::print::call_stack::PrintNodeArgs;

/// Stores the queued line suffixes.
#[derive(Debug, Default)]
pub(super) struct LineSuffixes<'a> {
    suffixes: Vec<LineSuffixEntry<'a>>,
}

impl<'a> LineSuffixes<'a> {
    /// Extend the line suffixes with `nodes`, storing their call stack arguments with them.
    pub(super) fn extend<I>(&mut self, args: PrintNodeArgs, nodes: I)
    where
        I: IntoIterator<Item = &'a FormatNode>,
    {
        self.suffixes
            .extend(nodes.into_iter().map(LineSuffixEntry::Suffix));
        self.suffixes.push(LineSuffixEntry::Args(args));
    }

    /// Take all the pending line suffixes.
    pub(super) fn take_pending<'l>(
        &'l mut self,
    ) -> impl DoubleEndedIterator<Item = LineSuffixEntry<'a>> + 'l + ExactSizeIterator {
        self.suffixes.drain(..)
    }

    /// Check if there are any line suffixes.
    pub(super) fn has_pending(&self) -> bool {
        !self.suffixes.is_empty()
    }
}

#[derive(Debug, Copy, Clone)]
pub(super) enum LineSuffixEntry<'a> {
    /// Line suffix to print.
    Suffix(&'a FormatNode),

    /// Potentially changed call arguments that should be used to format any following items.
    Args(PrintNodeArgs),
}

use std::fmt::Debug;

use crate::format::{
    FormatTagKind, IndentStyle, Indentation, InvalidDocumentError, PrintError, PrintMode,
    PrintResult,
};
use crate::print::mode::MeasureMode;
use crate::print::stack::{Stack, StackedStack};

/// The structural scope represented by one print stack frame.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum StackFrameKind {
    /// The document root.
    Root,
    /// One structural formatting tag.
    Tag(FormatTagKind),
}

/// One structural scope and its active print arguments.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct StackFrame {
    /// The structural scope.
    kind: StackFrameKind,
    /// The active print arguments.
    args: PrintArgs,
}

/// The print and measurement modes active in one structural scope.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) struct PrintArgs {
    /// The active print mode.
    mode: PrintMode,
    /// The active measurement mode.
    measure_mode: MeasureMode,
}

impl PrintArgs {
    /// Create the root print arguments.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Return the active print mode.
    pub(crate) fn mode(self) -> PrintMode {
        self.mode
    }

    /// Return the active measurement mode.
    pub(crate) fn measure_mode(self) -> MeasureMode {
        self.measure_mode
    }

    /// Return these arguments with a new print mode.
    pub(crate) fn with_print_mode(mut self, mode: PrintMode) -> Self {
        self.mode = mode;
        self
    }

    /// Return these arguments with a new measurement mode.
    pub(crate) fn with_measure_mode(mut self, mode: MeasureMode) -> Self {
        self.measure_mode = mode;
        self
    }
}

impl Default for PrintArgs {
    fn default() -> Self {
        Self {
            mode: PrintMode::Expanded,
            measure_mode: MeasureMode::FirstLine,
        }
    }
}

/// A structural scope stack carrying the active [`PrintArgs`].
pub(crate) trait CallStack {
    /// The concrete stack storage.
    type Stack: Stack<StackFrame> + Debug;

    /// Return the structural stack.
    fn stack(&self) -> &Self::Stack;

    /// Return the mutable structural stack.
    fn stack_mut(&mut self) -> &mut Self::Stack;

    /// Pop one matching structural scope.
    fn pop(&mut self, kind: FormatTagKind) -> PrintResult<PrintArgs> {
        let last = self.stack_mut().pop();

        match last {
            Some(StackFrame {
                kind: StackFrameKind::Tag(actual_kind),
                args,
            }) if actual_kind == kind => Ok(args),

            // report mismatched structural tags
            Some(StackFrame {
                kind: StackFrameKind::Tag(expected_kind),
                ..
            }) => Err(PrintError::InvalidDocument(Self::invalid_document_error(
                kind,
                Some(expected_kind),
            ))),

            // preserve the root frame while reporting an unmatched end tag
            Some(
                frame @ StackFrame {
                    kind: StackFrameKind::Root,
                    ..
                },
            ) => {
                self.stack_mut().push(frame);
                Err(PrintError::InvalidDocument(Self::invalid_document_error(
                    kind, None,
                )))
            }

            // report an invalid empty stack
            None => Err(PrintError::InvalidDocument(Self::invalid_document_error(
                kind, None,
            ))),
        }
    }

    #[cold]
    fn invalid_document_error(
        end_kind: FormatTagKind,
        start_kind: Option<FormatTagKind>,
    ) -> InvalidDocumentError {
        match start_kind {
            None => InvalidDocumentError::StartTagMissing { kind: end_kind },
            Some(start_kind) => InvalidDocumentError::StartEndTagMismatch {
                start_kind,
                end_kind,
            },
        }
    }

    /// Return the arguments for the current scope.
    fn top(&self) -> PrintArgs {
        let frame = self.stack().top();
        debug_assert!(frame.is_some());

        // safety: every call stack retains its permanent root frame
        unsafe { frame.unwrap_unchecked().args }
    }

    /// Return the current tag kind, or `None` at the document root.
    fn top_kind(&self) -> Option<FormatTagKind> {
        let frame = self.stack().top();
        debug_assert!(frame.is_some());

        // safety: every call stack retains its permanent root frame
        match unsafe { frame.unwrap_unchecked().kind } {
            StackFrameKind::Root => None,
            StackFrameKind::Tag(kind) => Some(kind),
        }
    }

    /// Push one structural scope.
    fn push(&mut self, kind: FormatTagKind, args: PrintArgs) {
        self.stack_mut().push(StackFrame {
            kind: StackFrameKind::Tag(kind),
            args,
        });
    }
}

/// Indentation stack shared by printing and fit measuring.
pub(crate) trait IndentStack {
    /// The active indentation stack.
    type Stack: Stack<Indentation> + Debug;
    /// Temporarily removed indentation frames.
    type HistoryStack: Stack<Indentation> + Debug;

    /// Return the active indentation stack.
    fn current_stack(&self) -> &Self::Stack;

    /// Return the mutable active indentation stack.
    fn current_stack_mut(&mut self) -> &mut Self::Stack;

    /// Return the mutable history indentation stack.
    fn history_stack_mut(&mut self) -> &mut Self::HistoryStack;

    /// Temporarily remove the current indentation frame.
    fn start_dedent(&mut self) {
        if let Some(indent) = self.current_stack_mut().pop() {
            self.history_stack_mut().push(indent);
        }
    }

    /// Restore the last temporarily removed indentation frame.
    fn end_dedent(&mut self) {
        if let Some(indent) = self.history_stack_mut().pop() {
            self.current_stack_mut().push(indent);
        }
    }

    /// Pop the current indentation frame.
    fn pop(&mut self) {
        self.current_stack_mut().pop();
    }

    /// Return the current indentation.
    fn indentation(&self) -> Indentation {
        self.current_stack().top().copied().unwrap_or_default()
    }

    /// Reset indentation to the root indentation.
    fn reset_indent(&mut self) {
        self.current_stack_mut().push(Indentation::default());
    }

    /// Push one normal indentation frame.
    fn indent(&mut self, indent_style: IndentStyle) {
        let next_indent = self.indentation().increment_level(indent_style);
        self.current_stack_mut().push(next_indent);
    }

    /// Push one aligned indentation frame.
    fn align(&mut self, count: u8) {
        let next_indent = self.indentation().set_align(count);
        self.current_stack_mut().push(next_indent);
    }
}

/// Stack for line suffix indentation frames.
pub(crate) trait SuffixStack {
    /// The suffix indentation stack.
    type SuffixStack: Stack<Indentation> + Debug;

    /// Return the mutable suffix indentation stack.
    fn suffix_stack_mut(&mut self) -> &mut Self::SuffixStack;

    /// Push one suffix indentation frame.
    fn push_suffix(&mut self, indentation: Indentation) {
        self.suffix_stack_mut().push(indentation);
    }
}

/// The structural scope stack used while printing.
#[derive(Debug, Clone)]
pub(crate) struct PrintCallStack(Vec<StackFrame>);

impl PrintCallStack {
    /// Create one print stack with its permanent root frame.
    pub(crate) fn new(args: PrintArgs) -> Self {
        Self(vec![StackFrame {
            kind: StackFrameKind::Root,
            args,
        }])
    }
}

impl CallStack for PrintCallStack {
    type Stack = Vec<StackFrame>;

    fn stack(&self) -> &Self::Stack {
        &self.0
    }

    fn stack_mut(&mut self) -> &mut Self::Stack {
        &mut self.0
    }
}

/// The indentation state used while printing.
#[derive(Debug, Clone)]
pub(crate) struct PrintIndentStack {
    /// The active indentation frames.
    indentations: Vec<Indentation>,
    /// The temporarily removed dedentation frames.
    history_indentations: Vec<Indentation>,
    /// The indentation frames retained for pending line suffixes.
    suffix_indentations: Vec<Indentation>,
}

impl PrintIndentStack {
    /// Create one print indentation stack.
    pub(crate) fn new(indentation: Indentation) -> Self {
        Self {
            indentations: vec![indentation],
            history_indentations: Vec::new(),
            suffix_indentations: Vec::new(),
        }
    }

    /// Restore suffix indentation frames before flushing suffix instructions.
    pub(crate) fn flush_suffixes(&mut self) {
        self.indentations
            .extend(self.suffix_indentations.drain(..).rev());
    }
}

impl IndentStack for PrintIndentStack {
    type HistoryStack = Vec<Indentation>;
    type Stack = Vec<Indentation>;

    fn current_stack(&self) -> &Self::Stack {
        &self.indentations
    }

    fn current_stack_mut(&mut self) -> &mut Self::Stack {
        &mut self.indentations
    }

    fn history_stack_mut(&mut self) -> &mut Self::HistoryStack {
        &mut self.history_indentations
    }
}

impl SuffixStack for PrintIndentStack {
    type SuffixStack = Vec<Indentation>;

    fn suffix_stack_mut(&mut self) -> &mut Self::SuffixStack {
        &mut self.suffix_indentations
    }
}

/// A temporary structural stack used while measuring whether content fits.
#[must_use]
pub(crate) struct FitsCallStack<'print> {
    /// The borrowed print stack and owned measurement frames.
    stack: StackedStack<'print, StackFrame>,
}

impl<'print> FitsCallStack<'print> {
    /// Create a measurement stack over the current print stack.
    pub(crate) fn new(print: &'print PrintCallStack, saved: Vec<StackFrame>) -> Self {
        let stack = StackedStack::with_vec(&print.0, saved);

        Self { stack }
    }

    /// Take the reusable owned stack storage.
    pub(crate) fn take_storage(&mut self) -> Vec<StackFrame> {
        self.stack.take_vec()
    }
}

impl<'a> CallStack for FitsCallStack<'a> {
    type Stack = StackedStack<'a, StackFrame>;

    fn stack(&self) -> &Self::Stack {
        &self.stack
    }

    fn stack_mut(&mut self) -> &mut Self::Stack {
        &mut self.stack
    }
}

/// Indentation stack used while measuring whether instructions fit.
pub(crate) struct FitsIndentStack<'print> {
    /// The borrowed print indentation and owned measurement frames.
    indentations: StackedStack<'print, Indentation>,
    /// The borrowed dedentation history and owned measurement frames.
    history_indentations: StackedStack<'print, Indentation>,
}

impl<'print> FitsIndentStack<'print> {
    /// Create one fit indentation stack on top of the print indentation stack.
    pub(crate) fn new(
        print: &'print PrintIndentStack,
        saved_indentations: Vec<Indentation>,
        saved_history_indentations: Vec<Indentation>,
    ) -> Self {
        let indentations = StackedStack::with_vec(&print.indentations, saved_indentations);
        let history_indentations =
            StackedStack::with_vec(&print.history_indentations, saved_history_indentations);

        Self {
            indentations,
            history_indentations,
        }
    }

    /// Take owned temporary fit stacks.
    pub(crate) fn take_storage(&mut self) -> (Vec<Indentation>, Vec<Indentation>) {
        (
            self.indentations.take_vec(),
            self.history_indentations.take_vec(),
        )
    }
}

impl<'a> IndentStack for FitsIndentStack<'a> {
    type HistoryStack = StackedStack<'a, Indentation>;
    type Stack = StackedStack<'a, Indentation>;

    fn current_stack(&self) -> &Self::Stack {
        &self.indentations
    }

    fn current_stack_mut(&mut self) -> &mut Self::Stack {
        &mut self.indentations
    }

    fn history_stack_mut(&mut self) -> &mut Self::HistoryStack {
        &mut self.history_indentations
    }
}

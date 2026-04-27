use crate::format::{
    FormatTagKind, IndentStyle, Indentation, InvalidDocumentError, PrintError, PrintMode,
    PrintResult,
};
use crate::print::mode::MeasureMode;
use crate::print::stack::{Stack, StackedStack};
use std::fmt::Debug;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum StackFrameKind {
    Root,
    Tag(FormatTagKind),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct StackFrame {
    kind: StackFrameKind,
    args: PrintNodeArgs,
}

/// Store arguments passed to `print_node` call, holding the state specific to printing an node.
///
/// E.g. the `indent` depends on the token the Printer's currently processing.
/// That's why it must be stored outside of the [`PrinterState`] that stores the state common to all nodes.
/// The state is passed by value, which is why it's important that it isn't storing any heavy data structures.
/// Such structures should be stored on the [`PrinterState`] instead.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) struct PrintNodeArgs {
    mode: PrintMode,
    measure_mode: MeasureMode,
}

impl PrintNodeArgs {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn mode(self) -> PrintMode {
        self.mode
    }

    pub(crate) fn measure_mode(self) -> MeasureMode {
        self.measure_mode
    }

    pub(crate) fn with_print_mode(mut self, mode: PrintMode) -> Self {
        self.mode = mode;
        self
    }

    pub(crate) fn with_measure_mode(mut self, mode: MeasureMode) -> Self {
        self.measure_mode = mode;
        self
    }
}

impl Default for PrintNodeArgs {
    fn default() -> Self {
        Self {
            mode: PrintMode::Expanded,
            measure_mode: MeasureMode::FirstLine,
        }
    }
}

/// Call stack that stores the [`PrintNodeCallArgs`].
///
/// New [`PrintNodeCallArgs`] are pushed onto the stack for every [`start`](Tag::is_start) [`Tag`](FormatNode::Tag)
/// and popped when reaching the corresponding [`end`](Tag::is_end) [`Tag`](FormatNode::Tag).
pub(crate) trait CallStack {
    type Stack: Stack<StackFrame> + Debug;

    fn stack(&self) -> &Self::Stack;

    fn stack_mut(&mut self) -> &mut Self::Stack;

    /// Pop the call arguments at the top and assert that they correspond to a start tag of `kind`.
    ///
    /// Returns `Ok` with the arguments if the kind of the top stack frame matches `kind`, otherwise returns `Err`.
    fn pop(&mut self, kind: FormatTagKind) -> PrintResult<PrintNodeArgs> {
        let last = self.stack_mut().pop();

        match last {
            Some(StackFrame {
                kind: StackFrameKind::Tag(actual_kind),
                args,
            }) if actual_kind == kind => Ok(args),

            // start / end kind don't match
            Some(StackFrame {
                kind: StackFrameKind::Tag(expected_kind),
                ..
            }) => Err(PrintError::InvalidDocument(Self::invalid_document_error(
                kind,
                Some(expected_kind),
            ))),

            // tried to pop the outer most stack frame, which is not valid
            Some(
                frame @ StackFrame {
                    kind: StackFrameKind::Root,
                    ..
                },
            ) => {
                // put it back in to guarantee that the stack is never empty
                self.stack_mut().push(frame);
                Err(PrintError::InvalidDocument(Self::invalid_document_error(
                    kind, None,
                )))
            }

            // this should be unreachable but having it for completeness
            // happens if the stack is empty
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

    /// Get the [`PrintNodeArgs`] for the current stack frame.
    fn top(&self) -> PrintNodeArgs {
        self.stack()
            .top()
            .expect("expected `stack` to never be empty")
            .args
    }

    /// Get the [`TagKind`] of the current stack frame or [None] if this is the root stack frame.
    fn top_kind(&self) -> Option<FormatTagKind> {
        match self
            .stack()
            .top()
            .expect("expected `stack` to never be empty")
            .kind
        {
            StackFrameKind::Root => None,
            StackFrameKind::Tag(kind) => Some(kind),
        }
    }

    /// Create a new stack frame for a [`FormatNode::Tag`] of `kind` with `args` as the call arguments.
    fn push(&mut self, kind: FormatTagKind, args: PrintNodeArgs) {
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

/// Call stack used for printing the [`FormatNode`]s.
#[derive(Debug, Clone)]
pub(crate) struct PrintCallStack(Vec<StackFrame>);

impl PrintCallStack {
    pub(crate) fn new(args: PrintNodeArgs) -> Self {
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

/// Indentation stack used for printing format nodes.
#[derive(Debug, Clone)]
pub(crate) struct PrintIndentStack {
    indentations: Vec<Indentation>,
    history_indentations: Vec<Indentation>,
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

    /// Restore suffix indentation frames before flushing suffix nodes.
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

/// Call stack used for measuring if some content fits on the line.
///
/// The stack is a view on top of the [`PrintCallStack`] because the stack frames are still necessary for printing.
#[must_use]
pub(crate) struct FitsCallStack<'print> {
    stack: StackedStack<'print, StackFrame>,
}

impl<'print> FitsCallStack<'print> {
    pub(crate) fn new(print: &'print PrintCallStack, saved: Vec<StackFrame>) -> Self {
        let stack = StackedStack::with_vec(&print.0, saved);

        Self { stack }
    }

    pub(crate) fn finish(self) -> Vec<StackFrame> {
        self.stack.into_vec()
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

/// Indentation stack used for measuring if nodes fit on the line.
pub(crate) struct FitsIndentStack<'print> {
    indentations: StackedStack<'print, Indentation>,
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

    /// Return owned temporary fit stacks.
    pub(crate) fn finish(self) -> (Vec<Indentation>, Vec<Indentation>) {
        (
            self.indentations.into_vec(),
            self.history_indentations.into_vec(),
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

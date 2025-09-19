use crate::print::mode::MeasureMode;
use crate::print::stack::{Stack, StackedStack};
use crate::format::{
    FormatTagKind, IndentStyle, Indentation, InvalidDocumentError, PrintError, PrintMode,
    PrintResult,
};
use std::fmt::Debug;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum StackFrameKind {
    Root,
    Tag(FormatTagKind),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct StackFrame {
    kind: StackFrameKind,
    args: PrintElementArgs,
}

/// Store arguments passed to `print_element` call, holding the state specific to printing an element.
///
/// E.g. the `indent` depends on the token the Printer's currently processing.
/// That's why it must be stored outside of the [`PrinterState`] that stores the state common to all elements.
/// The state is passed by value, which is why it's important that it isn't storing any heavy data structures.
/// Such structures should be stored on the [`PrinterState`] instead.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) struct PrintElementArgs {
    indent: Indentation,
    mode: PrintMode,
    measure_mode: MeasureMode,
}

impl PrintElementArgs {
    pub(crate) fn new(indent: Indentation) -> Self {
        Self {
            indent,
            ..Self::default()
        }
    }

    pub(crate) fn mode(self) -> PrintMode {
        self.mode
    }

    pub(crate) fn measure_mode(self) -> MeasureMode {
        self.measure_mode
    }

    pub(crate) fn indentation(self) -> Indentation {
        self.indent
    }

    pub(crate) fn increment_indent_level(mut self, indent_style: IndentStyle) -> Self {
        self.indent = self.indent.increment_level(indent_style);
        self
    }

    pub(crate) fn decrement_indent(mut self) -> Self {
        self.indent = self.indent.decrement();
        self
    }

    pub(crate) fn reset_indent(mut self) -> Self {
        self.indent = Indentation::default();
        self
    }

    pub(crate) fn set_indent_align(mut self, count: u8) -> Self {
        self.indent = self.indent.set_align(count);
        self
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

impl Default for PrintElementArgs {
    fn default() -> Self {
        Self {
            indent: Indentation::Level(0),
            mode: PrintMode::Expanded,
            measure_mode: MeasureMode::FirstLine,
        }
    }
}

/// Call stack that stores the [`PrintElementCallArgs`].
///
/// New [`PrintElementCallArgs`] are pushed onto the stack for every [`start`](Tag::is_start) [`Tag`](FormatElement::Tag)
/// and popped when reaching the corresponding [`end`](Tag::is_end) [`Tag`](FormatElement::Tag).
pub(crate) trait CallStack {
    type Stack: Stack<StackFrame> + Debug;

    fn stack(&self) -> &Self::Stack;

    fn stack_mut(&mut self) -> &mut Self::Stack;

    /// Pop the call arguments at the top and assert that they correspond to a start tag of `kind`.
    ///
    /// Returns `Ok` with the arguments if the kind of the top stack frame matches `kind`, otherwise returns `Err`.
    fn pop(&mut self, kind: FormatTagKind) -> PrintResult<PrintElementArgs> {
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

    /// Get the [`PrintElementArgs`] for the current stack frame.
    fn top(&self) -> PrintElementArgs {
        self.stack()
            .top()
            .unwrap_or_else(|| panic!("expected `stack` to never be empty"))
            .args
    }

    /// Get the [`TagKind`] of the current stack frame or [None] if this is the root stack frame.
    fn top_kind(&self) -> Option<FormatTagKind> {
        match self
            .stack()
            .top()
            .unwrap_or_else(|| panic!("expected `stack` to never be empty"))
            .kind
        {
            StackFrameKind::Root => None,
            StackFrameKind::Tag(kind) => Some(kind),
        }
    }

    /// Create a new stack frame for a [`FormatElement::Tag`] of `kind` with `args` as the call arguments.
    fn push(&mut self, kind: FormatTagKind, args: PrintElementArgs) {
        self.stack_mut().push(StackFrame {
            kind: StackFrameKind::Tag(kind),
            args,
        });
    }
}

/// Call stack used for printing the [`FormatElement`]s.
#[derive(Debug, Clone)]
pub(crate) struct PrintCallStack(Vec<StackFrame>);

impl PrintCallStack {
    pub(crate) fn new(args: PrintElementArgs) -> Self {
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

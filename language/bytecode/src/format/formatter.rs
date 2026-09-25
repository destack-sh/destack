use std::collections::HashMap;
use std::fmt;

use tspp_fir::format;
use tspp_fir::format::{
    Allocator, FormatContext, FormatError, FormatOptions, FormatResult, Formatter,
};
use tspp_fir::print::{MAX_OUTPUT_BYTES, PrintOptions};
use tspp_source::{File, FileType, IndentStyle, LineEnding};

use crate::{CodeOffset, CodeRange, FunctionId, Label, Object, Relocation};

/// Shared state for formatting one bytecode object.
pub struct BytecodeFormatContext<'a> {
    /// The immutable bytecode object.
    pub(super) object: &'a Object,
    /// The active function while its body is formatted.
    pub(super) function: Option<FunctionId>,
    /// Function-local branch labels keyed by byte offset.
    pub(super) labels: HashMap<CodeOffset, Label>,
    /// Relocations keyed by absolute code-section byte offset.
    pub(super) relocations: HashMap<u32, Relocation>,
    /// Source names in dense object function order.
    function_names: &'a [String],
    /// The formatting options.
    options: BytecodeFormatOptions,
    /// The empty FIR source file.
    file: File,
}

/// Bytecode format options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BytecodeFormatOptions {
    /// The emitted line ending.
    pub line_ending: LineEnding,
    /// The emitted indentation style.
    pub indent_style: IndentStyle,
    /// The emitted indentation width.
    pub indent_width: u8,
    /// The maximum formatted line width.
    pub line_width: u8,
}

impl Default for BytecodeFormatOptions {
    fn default() -> Self {
        Self {
            line_ending: LineEnding::LineFeed,
            indent_style: IndentStyle::Space,
            indent_width: 4,
            line_width: 100,
        }
    }
}

impl BytecodeFormatOptions {
    /// Return equivalent FIR print options.
    pub fn as_print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
            trim_trailing_whitespace: false,
            max_output_bytes: MAX_OUTPUT_BYTES,
        }
    }
}

impl FormatOptions for BytecodeFormatOptions {
    fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    fn indent_width(&self) -> u8 {
        self.indent_width
    }

    fn line_width(&self) -> u8 {
        self.line_width
    }

    fn as_print_options(&self) -> PrintOptions {
        BytecodeFormatOptions::as_print_options(self)
    }
}

/// FIR formatter for bytecode.
pub type BytecodeFormatter<'a, 'buffer> = Formatter<'buffer, 'a, BytecodeFormatContext<'a>>;

impl fmt::Debug for BytecodeFormatContext<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BytecodeFormatContext")
            .field("options", &self.options)
            .finish()
    }
}

impl<'a> BytecodeFormatContext<'a> {
    /// Create one bytecode formatting context.
    fn new(
        object: &'a Object,
        function_names: &'a [String],
        options: BytecodeFormatOptions,
    ) -> Self {
        let relocations = object
            .relocations()
            .iter()
            .map(|relocation| (relocation.byte_offset, *relocation))
            .collect();

        Self {
            object,
            function: None,
            labels: HashMap::new(),
            relocations,
            function_names,
            options,
            file: File::empty_text(FileType::Tspp),
        }
    }

    /// Begin formatting one function body.
    pub(super) fn begin_function(&mut self, id: FunctionId, code: CodeRange) -> FormatResult<()> {
        self.function = Some(id);

        // assign stable labels to every branch target
        self.collect_labels(code)?;

        Ok(())
    }

    /// Finish formatting the active function.
    pub(super) fn end_function(&mut self) -> FormatResult<()> {
        self.function = None;
        self.labels.clear();

        Ok(())
    }

    /// Return the canonical label at one function-local byte offset.
    pub(super) fn label(&self, offset: CodeOffset) -> Option<Label> {
        self.labels.get(&offset).copied()
    }

    /// Return one source function name.
    pub(super) fn function_name(&self, function: FunctionId) -> FormatResult<&str> {
        self.function_names
            .get(function.index())
            .map(String::as_str)
            .ok_or(FormatError::SyntaxError {
                message: "function name is absent",
            })
    }
}

impl FormatContext for BytecodeFormatContext<'_> {
    type Options = BytecodeFormatOptions;

    fn options(&self) -> &Self::Options {
        &self.options
    }

    fn file(&self) -> &File {
        &self.file
    }
}

/// Format one bytecode object.
pub fn format_bytecode(
    object: &Object,
    function_names: &[String],
    options: BytecodeFormatOptions,
) -> FormatResult<String> {
    let context = BytecodeFormatContext::new(object, function_names, options);
    let allocator = Allocator::default();
    let document = format!(&allocator, context, [object])?;
    let printed = document.print()?;

    Ok(printed.as_str().to_string())
}

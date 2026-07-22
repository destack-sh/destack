use std::collections::HashMap;
use std::fmt;

use destack_fir::format;
use destack_fir::format::{
    Allocator, FormatContext, FormatError, FormatOptions, FormatResult, Formatter,
};
use destack_fir::print::{MAX_OUTPUT_BYTES, PrintOptions};
use destack_source::{File, FileType, IndentStyle, LineEnding};

use crate::{
    CodeOffset, CodeRange, DynamicRelocation, Function, FunctionId, Label, Object, RegisterId,
    RegisterRange, Symbol, ValueType,
};

/// Shared state for formatting one bytecode object.
pub struct BytecodeFormatContext<'a> {
    /// The immutable bytecode object.
    pub(super) object: &'a Object,
    /// The active function while its body is formatted.
    pub(super) function: Option<FunctionId>,
    /// Function-local branch labels keyed by byte offset.
    pub(super) labels: HashMap<CodeOffset, Label>,
    /// Symbols keyed by absolute code-section byte offset.
    pub(super) relocations: HashMap<u32, Symbol>,
    /// Dynamic relocations keyed by absolute code-section byte offset.
    pub(super) dynamic_relocations: HashMap<u32, DynamicRelocation>,
    /// Logical value types keyed by their first register word.
    registers: Vec<Option<ValueType>>,
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
            .field("function", &self.function)
            .finish()
    }
}

impl<'a> BytecodeFormatContext<'a> {
    /// Create one bytecode formatting context.
    fn new(object: &'a Object, options: BytecodeFormatOptions) -> Self {
        let relocations = object
            .instruction_relocations()
            .iter()
            .map(|relocation| (relocation.byte_offset, relocation.symbol))
            .collect();
        let dynamic_relocations = object
            .dynamic_relocations()
            .iter()
            .map(|relocation| (relocation.byte_offset, *relocation))
            .collect();

        Self {
            object,
            function: None,
            labels: HashMap::new(),
            relocations,
            dynamic_relocations,
            registers: Vec::new(),
            options,
            file: File::empty_text(FileType::Destack),
        }
    }

    /// Begin formatting one function body.
    pub(super) fn begin_function(
        &mut self,
        id: FunctionId,
        function: &Function,
        code: CodeRange,
    ) -> FormatResult<()> {
        self.function = Some(id);
        self.registers.clear();
        self.registers.resize(function.body.register_count(), None);
        let mut register = RegisterId(0);

        // establish the immutable function register partition
        for ty in function.body.register_types(self.object.value_types()) {
            self.set_register_type(register, *ty)?;
            register.0 += ty.word_count();
        }
        if register.index() != function.body.register_count() {
            return Err(FormatError::SyntaxError {
                message: "function register types do not cover its register file",
            });
        }

        // assign stable labels to every branch target
        self.collect_labels(code)?;

        Ok(())
    }

    /// Finish formatting the active function.
    pub(super) fn end_function(&mut self) {
        self.function = None;
        self.labels.clear();
        self.registers.clear();
    }

    /// Return the canonical label at one function-local byte offset.
    pub(super) fn label(&self, offset: CodeOffset) -> Option<Label> {
        self.labels.get(&offset).copied()
    }

    /// Return one initialized logical register type.
    pub(super) fn register_type(&self, register: RegisterId) -> FormatResult<ValueType> {
        self.registers
            .get(register.index())
            .copied()
            .flatten()
            .ok_or(FormatError::SyntaxError {
                message: "instruction reads an uninitialized register",
            })
    }

    /// Return the logical values packed into one physical register range.
    pub(super) fn register_values(&self, range: RegisterRange) -> FormatResult<Vec<RegisterId>> {
        let mut register = range.start;
        let end = u32::from(range.start.0) + u32::from(range.word_count);
        let mut values = Vec::new();

        // advance by each logical value's physical width
        while u32::from(register.0) < end {
            let ty = self.register_type(register)?;
            values.push(register);
            register.0 += ty.word_count();
        }
        if u32::from(register.0) != end {
            return Err(FormatError::SyntaxError {
                message: "register range ends inside a logical value",
            });
        }

        Ok(values)
    }

    /// Return the logical value types packed into one physical register range.
    pub(super) fn register_types(&self, range: RegisterRange) -> FormatResult<Vec<ValueType>> {
        self.register_values(range)?
            .into_iter()
            .map(|register| self.register_type(register))
            .collect()
    }

    /// Return the active function definition.
    pub(super) fn active_function(&self) -> FormatResult<&'a Function> {
        let function = self.function.ok_or(FormatError::SyntaxError {
            message: "instruction formatted outside a function",
        })?;

        self.object
            .function(function)
            .ok_or(FormatError::SyntaxError {
                message: "active function is missing from its object",
            })
    }

    /// Set one logical value type in the register partition.
    fn set_register_type(&mut self, register: RegisterId, ty: ValueType) -> FormatResult<()> {
        let start = register.index();
        let end = start + ty.word_count() as usize;
        if end > self.registers.len() {
            return Err(FormatError::SyntaxError {
                message: "instruction writes outside its function register file",
            });
        }

        self.registers[start..end].fill(None);
        self.registers[start] = Some(ty);

        Ok(())
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
pub fn format_bytecode(object: &Object, options: BytecodeFormatOptions) -> FormatResult<String> {
    let context = BytecodeFormatContext::new(object, options);
    let allocator = Allocator::default();
    let document = format!(&allocator, context, [object])?;
    let printed = document.print()?;

    Ok(printed.as_str().to_string())
}

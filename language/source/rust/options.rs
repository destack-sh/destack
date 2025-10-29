/// The mode we're parsing, compiling, checking Dyst in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageMode {
    /// Lenient scripting with relaxed checking, conversion, cloning, boxing and more.
    Script,
    /// Strict engineering with explicit context, defaults, typing, behavior and more.
    Library,
}

/// The language compatibility mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageCompatibility {
    /// The JavaScript compatibility mode (like `.js`).
    JavaScript,
    /// The JavaScript XML compatibility mode (like `.jsx`).
    JavaScriptXml,
    /// The TypeScript compatibility mode (like `.ts`).
    TypeScript,
    /// The TypeScript XML compatibility mode (like `.tsx`).
    TypeScriptXml,
}

impl LanguageCompatibility {
    /// Whether the language compatibility is TypeScript-related.
    #[inline]
    pub fn is_typescript(&self) -> bool {
        matches!(
            self,
            LanguageCompatibility::TypeScript | LanguageCompatibility::TypeScriptXml
        )
    }

    /// Whether the language compatibility is JavaScript-related.
    #[inline]
    pub fn is_javascript(&self) -> bool {
        matches!(
            self,
            LanguageCompatibility::JavaScript | LanguageCompatibility::JavaScriptXml
        )
    }

    /// Whether the language compatibility is XML-related.
    #[inline]
    pub fn is_xml(&self) -> bool {
        matches!(
            self,
            LanguageCompatibility::JavaScriptXml | LanguageCompatibility::TypeScriptXml
        )
    }
}

/// The language version.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LanguageVersion {
    /// The first version of the language.
    V1,
}

/// The options for working with the Dyst language.
#[derive(Debug, Copy, Clone, Default)]
pub struct LanguageOptions {
    /// The version of the language.
    pub version: LanguageVersion = LanguageVersion::V1,
    /// The mode we're operating Dyst in.
    pub mode: LanguageMode = LanguageMode::Script,
    /// The compatibility mode.
    pub compatibility: Option<LanguageCompatibility> = None,
    /// The formatting options.
    pub formatting: FormattingOptions,
}

impl LanguageOptions {
    /// Whether the language compatibility is XML-related.
    #[inline]
    pub fn is_compatible_with_xml(&self) -> bool {
        self.compatibility
            .is_some_and(|compatibility| compatibility.is_xml())
    }

    /// Whether the language compatibility is JavaScript-related.
    #[inline]
    pub fn is_compatible_with_javascript(&self) -> bool {
        self.compatibility
            .is_some_and(|compatibility| compatibility.is_javascript())
    }

    /// Whether the language compatibility is TypeScript-related.
    #[inline]
    pub fn is_compatible_with_typescript(&self) -> bool {
        self.compatibility
            .is_some_and(|compatibility| compatibility.is_typescript())
    }

    /// Set the language version.
    pub fn with_version(mut self, version: LanguageVersion) -> Self {
        self.version = version;
        self
    }

    /// Set the language mode.
    pub fn with_mode(mut self, mode: LanguageMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the language compatibility.
    pub fn with_compatibility(mut self, compatibility: LanguageCompatibility) -> Self {
        self.compatibility = Some(compatibility);
        self
    }

    /// Set the language compatibility.
    pub fn without_compatibility(mut self) -> Self {
        self.compatibility = None;
        self
    }

    /// Set the formatting options.
    pub fn with_formatting(mut self, formatting: FormattingOptions) -> Self {
        self.formatting = formatting;
        self
    }

    /// Set the line ending type.
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.formatting.line_ending = line_ending;
        self
    }

    /// Set the indent style.
    pub fn with_indent_style(mut self, indent_style: IndentStyle) -> Self {
        self.formatting.indent_style = indent_style;
        self
    }

    /// Set the indent width.
    pub fn with_indent_width(mut self, indent_width: u8) -> Self {
        self.formatting.indent_width = indent_width;
        self
    }

    /// Set the line width.
    pub fn with_line_width(mut self, line_width: u8) -> Self {
        self.formatting.line_width = line_width;
        self
    }
}

/// The formatting options.
#[derive(Debug, Copy, Clone, Default)]
pub struct FormattingOptions {
    /// The type of line ending to apply to the printed input.  
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u8 = 100,
}

impl FormattingOptions {
    /// Set the line ending type.
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Set the indent style.
    pub fn with_indent_style(mut self, indent_style: IndentStyle) -> Self {
        self.indent_style = indent_style;
        self
    }

    /// Set the indent width.
    pub fn with_indent_width(mut self, indent_width: u8) -> Self {
        self.indent_width = indent_width;
        self
    }

    /// Set the line width.
    pub fn with_line_width(mut self, line_width: u8) -> Self {
        self.line_width = line_width;
        self
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash, Default)]
pub enum IndentStyle {
    /// Use tabs to indent code.
    #[default]
    Tab,
    /// Use [`IndentWidth`] spaces to indent code.
    Space,
}

impl IndentStyle {
    /// Check if this is an [`IndentStyle::Tab`].
    pub const fn is_tab(&self) -> bool {
        matches!(self, IndentStyle::Tab)
    }

    /// Check if this is an [`IndentStyle::Space`].
    pub const fn is_space(&self) -> bool {
        matches!(self, IndentStyle::Space)
    }

    /// Get the string representation of the indent style.
    pub const fn as_str(&self) -> &'static str {
        match self {
            IndentStyle::Tab => "tab",
            IndentStyle::Space => "space",
        }
    }
}

impl std::fmt::Display for IndentStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum LineEnding {
    /// Line Feed only (\n), common on Linux and macOS as well as inside git repos.
    #[default]
    LineFeed,
    /// Carriage Return + Line Feed characters (\r\n), common on Windows.
    CarriageReturnLineFeed,
    /// Carriage Return character only (\r), used very rarely.
    CarriageReturn,
}

impl LineEnding {
    /// Get the string representation of this line ending.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            LineEnding::LineFeed => "\n",
            LineEnding::CarriageReturnLineFeed => "\r\n",
            LineEnding::CarriageReturn => "\r",
        }
    }

    /// Get the string used to configure this line ending.
    ///
    /// See [`LineEnding::as_str`] for the actual string representation of the line ending.
    #[inline]
    pub const fn as_setting_str(&self) -> &'static str {
        match self {
            LineEnding::LineFeed => "lf",
            LineEnding::CarriageReturnLineFeed => "crlf",
            LineEnding::CarriageReturn => "cr",
        }
    }
}

use destack_source::LanguageType;

/// Options for working with the Destack language.
///
/// This contains the language type for parsing.
#[derive(Debug, Copy, Clone, Default)]
pub struct LanguageOptions {
    /// The source language type (determines compatibility behavior).
    pub ty: LanguageType = LanguageType::Destack,
}

impl LanguageOptions {
    /// Whether the language type is Destack-compatible.
    #[inline]
    pub fn is_destack(&self) -> bool {
        self.ty.is_destack()
    }

    /// Whether the language type is JavaScript-compatible.
    #[inline]
    pub fn is_javascript(&self) -> bool {
        self.ty.is_javascript()
    }

    /// Whether the language type is TypeScript-compatible.
    #[inline]
    pub fn is_typescript(&self) -> bool {
        self.ty.is_typescript()
    }

    /// Whether the language supports JSX/tree literal syntax.
    #[inline]
    pub fn supports_jsx(&self) -> bool {
        self.ty.supports_jsx()
    }

    /// Set the language type.
    pub fn with_type(mut self, language_type: LanguageType) -> Self {
        self.ty = language_type;
        self
    }
}

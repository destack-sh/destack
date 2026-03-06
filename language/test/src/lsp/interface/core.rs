use destack_lsp_types as lsp;

use crate::lsp::LspTestState;

/// The tsserver-style `test` facade over one applied-LSP state.
#[derive(Debug)]
pub struct Test<'a> {
    /// The underlying mutable harness state.
    pub(super) state: &'a mut LspTestState,
}

/// The tsserver-style `goTo` facade over one applied-LSP state.
#[derive(Debug)]
pub struct GoTo<'a> {
    /// The underlying mutable harness state.
    pub(super) state: &'a mut LspTestState,
}

/// The tsserver-style `verify` facade over one applied-LSP state.
#[derive(Debug)]
pub struct Verify<'a> {
    /// The underlying mutable harness state.
    pub(super) state: &'a mut LspTestState,
}

/// The tsserver-style negated `verify.not` facade over one applied-LSP state.
#[derive(Debug)]
pub struct VerifyNegatable<'a> {
    /// The underlying mutable harness state.
    pub(super) state: &'a mut LspTestState,
    /// Whether assertions should be negated.
    pub(super) is_negative: bool,
}

/// The tsserver-style `edit` facade over one applied-LSP state.
#[derive(Debug)]
pub struct Edit<'a> {
    /// The underlying mutable harness state.
    pub(super) state: &'a mut LspTestState,
}

/// The tsserver-style `format` facade over one applied-LSP state.
#[derive(Debug)]
pub struct Format<'a> {
    /// The underlying mutable harness state.
    pub(super) state: &'a mut LspTestState,
}

/// The tsserver-style `cancellation` facade over one applied-LSP state.
#[derive(Debug)]
pub struct Cancellation<'a> {
    /// The underlying mutable harness state.
    pub(super) state: &'a mut LspTestState,
}

/// The tsserver-style `debug` facade over one applied-LSP state.
#[derive(Debug)]
pub struct Debug<'a> {
    /// The underlying mutable harness state.
    pub(super) state: &'a mut LspTestState,
}

/// One native formatting option value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatOptionValue {
    /// One boolean formatting value.
    Bool(bool),
    /// One numeric formatting value.
    Number(i32),
    /// One string formatting value.
    String(String),
}

/// One native signature-help trigger description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelpTrigger {
    /// The trigger kind.
    pub kind: lsp::SignatureHelpTriggerKind,
    /// The trigger character when one exists.
    pub character: Option<String>,
    /// Whether the request retriggers an already visible signature-help session.
    pub is_retrigger: bool,
}

impl SignatureHelpTrigger {
    /// Build one manual invocation trigger.
    pub fn invoked() -> Self {
        Self {
            kind: lsp::SignatureHelpTriggerKind::INVOKED,
            character: None,
            is_retrigger: false,
        }
    }

    /// Build one trigger-character request.
    pub fn trigger_character(character: &str) -> Self {
        Self {
            kind: lsp::SignatureHelpTriggerKind::TRIGGER_CHARACTER,
            character: Some(character.to_string()),
            is_retrigger: false,
        }
    }

    /// Build one content-change request.
    pub fn content_change() -> Self {
        Self {
            kind: lsp::SignatureHelpTriggerKind::CONTENT_CHANGE,
            character: None,
            is_retrigger: false,
        }
    }

    /// Convert into an LSP signature-help context.
    pub(super) fn to_lsp_context(&self) -> lsp::SignatureHelpContext {
        lsp::SignatureHelpContext {
            trigger_kind: self.kind.clone(),
            trigger_character: self.character.clone(),
            is_retrigger: self.is_retrigger,
            active_signature_help: None,
        }
    }
}

impl LspTestState {
    /// Return the tsserver-style `test` facade.
    pub fn test(&mut self) -> Test<'_> {
        Test { state: self }
    }

    /// Return the tsserver-style `goTo` facade.
    pub fn go_to(&mut self) -> GoTo<'_> {
        GoTo { state: self }
    }

    /// Return the tsserver-style `verify` facade.
    pub fn verify(&mut self) -> Verify<'_> {
        Verify { state: self }
    }

    /// Return the tsserver-style `edit` facade.
    pub fn edit(&mut self) -> Edit<'_> {
        Edit { state: self }
    }

    /// Return the tsserver-style `format` facade.
    pub fn format(&mut self) -> Format<'_> {
        Format { state: self }
    }

    /// Return the tsserver-style `cancellation` facade.
    pub fn cancellation(&mut self) -> Cancellation<'_> {
        Cancellation { state: self }
    }

    /// Return the tsserver-style `debug` facade.
    pub fn debug(&mut self) -> Debug<'_> {
        Debug { state: self }
    }
}

use std::{error, fmt};

use serde::{Deserialize, Serialize};
use tspp_program::{TypeId, Word};
use tspp_serde::Reflect;

/// One panic payload retained while frames unwind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Panic {
    /// The payload type when the panic carries a value.
    pub ty: Option<TypeId>,
    /// The encoded payload words.
    pub words: Vec<Word>,
    /// The payload text rendered before unwinding when the payload is a string.
    pub message: Option<String>,
}

impl Panic {
    /// Create one payloadless panic.
    pub const fn empty() -> Self {
        Self {
            ty: None,
            words: Vec::new(),
            message: None,
        }
    }

    /// Create one typed panic payload.
    pub const fn new(ty: TypeId, words: Vec<Word>) -> Self {
        Self {
            ty: Some(ty),
            words,
            message: None,
        }
    }

    /// Attach the rendered payload text.
    pub fn message(mut self, message: Option<String>) -> Self {
        self.message = message;

        self
    }
}

impl fmt::Display for Panic {
    /// Format one encoded panic payload.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.message, self.ty) {
            (Some(message), _) => formatter.write_str(message),
            (None, Some(ty)) => write!(formatter, "{ty:?} in {} words", self.words.len()),
            (None, None) => formatter.write_str("no payload"),
        }
    }
}

impl error::Error for Panic {}

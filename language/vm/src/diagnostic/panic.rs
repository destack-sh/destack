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
}

impl Panic {
    /// Create one payloadless panic.
    pub const fn empty() -> Self {
        Self {
            ty: None,
            words: Vec::new(),
        }
    }

    /// Create one typed panic payload.
    pub const fn new(ty: TypeId, words: Vec<Word>) -> Self {
        Self {
            ty: Some(ty),
            words,
        }
    }
}

impl fmt::Display for Panic {
    /// Format one encoded panic payload.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ty {
            Some(ty) => write!(formatter, "{ty:?} in {} words", self.words.len()),
            None => formatter.write_str("no payload"),
        }
    }
}

impl error::Error for Panic {}

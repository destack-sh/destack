pub mod service;

pub use service::{
    DiagnosticSnapshot, FileChange, FileImage, FileUpdate, FileUpdateKind, LanguageService,
    LanguageServiceError, LanguageServiceMessage, LanguageServiceMessageKind,
    LanguageServiceResult, QueryResult, TextChange, TextPosition, TextRange,
};

#[cfg(test)]
mod tests;

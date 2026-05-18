pub mod service;

pub use service::{
    DiagnosticView, FileChange, FileImage, FileUpdate, FileUpdateKind, LanguageService,
    LanguageServiceError, LanguageServiceMessage, LanguageServiceMessageKind,
    LanguageServiceResult, QueryResult, QueryRevision, TextChange, TextPosition, TextRange,
};

#[cfg(test)]
mod tests;

#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![feature(thread_id_value)]

pub mod repository;
pub mod revision;

pub use destack_source::{
    Applicability, Diagnostic, DiagnosticCollection, DiagnosticCollector, DiagnosticSeverity,
    DiagnosticStore, DiagnosticStoreUpdate, LabeledSpan, Suggestion,
};

mod diagnostic;
mod file;
pub mod workspace;

pub use destack_session::{FileChange, FileUpdateKind};
pub use destack_source::{TextChange, TextPosition, TextRange};
pub use diagnostic::{DiagnosticView, Error};
pub use file::{FileImage, FileUpdate, SourceUpdateResult};
pub use workspace::{Message, MessageKind, QueryResult, RevisionPolicy, UpdateBatch, Workspace};

#[cfg(test)]
mod tests;

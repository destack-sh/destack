mod diagnostic;
mod file;
pub mod workspace;

pub use destack_session::Edit;
pub use destack_source::{TextChange, TextPosition, TextRange};
pub use diagnostic::{DiagnosticView, Error};
pub use file::{Commit, FileImage, FileUpdate, UpdateKind};
pub use workspace::{Message, MessageKind, QueryResult, RevisionPolicy, UpdateBatch, Workspace};

#[cfg(test)]
mod tests;

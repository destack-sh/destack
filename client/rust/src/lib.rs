pub use crate as language;

pub use destack_artifact as artifact;
pub use destack_core as core;
pub use destack_dir as dir;
pub use destack_heap as heap;
pub use destack_js as js;
pub use destack_mir as mir;
pub use destack_program as program;
pub use destack_query as query;
pub use destack_repository as repository;
pub use destack_source as source;
pub use destack_workspace as workspace;

pub use destack_artifact::{
    ArtifactDependency, ArtifactFingerprint, ArtifactKey, ArtifactProjection,
    ArtifactProjectionDependency, ArtifactProjectionFingerprint, ArtifactProjectionKey,
    ArtifactRecord, ArtifactSidecar, ArtifactVersion, SourceDependency,
};
pub use destack_core::StringId;
pub use destack_repository::Revision;
pub use destack_source::{
    Applicability, ByteRange, ComponentId, Content, ContentId, Diagnostic, DiagnosticCollection,
    DiagnosticHelp, DiagnosticLabel, DiagnosticNote, DiagnosticSeverity, DiagnosticSuggestion,
    DiagnosticTag, Edit, FileId, FilePatch, FileType, ModuleId, ModuleKey, PackageId, Patch,
    PatchSet, ProductId, ProductKey, ProfileId, Span, TargetId, TargetKey, TextPatch,
};

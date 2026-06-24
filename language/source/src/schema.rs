use destack_serde::SchemaRegistry;

use crate::{
    Applicability, ComponentId, Content, ContentId, Diagnostic, DiagnosticHelp, DiagnosticLabel,
    DiagnosticNote, DiagnosticSeverity, DiagnosticSuggestion, DiagnosticTag, FileId, FilePatch,
    FileType, ModuleId, PackageId, Patch, PatchSet, ProductId, ProfileId, Span, TargetId,
};

/// Include public source schema roots.
pub fn schema(registry: &mut SchemaRegistry) {
    registry.register::<PackageId>();
    registry.register::<ModuleId>();
    registry.register::<ProfileId>();
    registry.register::<ComponentId>();
    registry.register::<TargetId>();
    registry.register::<ProductId>();
    registry.register::<FileId>();
    registry.register::<ContentId>();
    registry.register::<Content>();
    registry.register::<Span>();
    registry.register::<FileType>();

    registry.register::<DiagnosticSeverity>();
    registry.register::<DiagnosticTag>();
    registry.register::<Applicability>();
    registry.register::<DiagnosticLabel>();
    registry.register::<DiagnosticNote>();
    registry.register::<DiagnosticHelp>();
    registry.register::<DiagnosticSuggestion>();
    registry.register::<Diagnostic>();
    registry.register::<Patch>();
    registry.register::<FilePatch>();
    registry.register::<PatchSet>();
}

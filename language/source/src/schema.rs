use destack_serde::SchemaRegistry;

use crate::{
    Applicability, ByteRange, ComponentId, Content, ContentId, Diagnostic, DiagnosticHelp,
    DiagnosticLabel, DiagnosticNote, DiagnosticReference, DiagnosticSeverity, DiagnosticSuggestion,
    DiagnosticTag, Edit, FileId, FilePatch, FileType, ModuleId, PackageId, Patch, PatchSet,
    ProductId, ProfileId, Span, TargetId, TextChange, TextPatch, TextPosition, TextRange,
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
    registry.register::<DiagnosticReference>();
    registry.register::<Patch>();
    registry.register::<FilePatch>();
    registry.register::<PatchSet>();

    registry.register::<TextChange>();
    registry.register::<TextRange>();
    registry.register::<TextPosition>();
    registry.register::<ByteRange>();
    registry.register::<TextPatch>();
    registry.register::<Edit>();
}

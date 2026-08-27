use destack_core::{Blob, BlobId};
use destack_serde::Schema;

use crate::{
    Applicability, ByteRange, ComponentId, Diagnostic, DiagnosticHelp, DiagnosticLabel,
    DiagnosticNote, DiagnosticReference, DiagnosticSeverity, DiagnosticSuggestion, DiagnosticTag,
    Edit, FileId, FilePatch, FileType, Location, LocationId, ModuleId, PackageId, Patch, PatchSet,
    ProductId, ProfileId, ProvenanceId, ProvenanceTable, Span, TargetId, TextChange, TextPatch,
    TextPosition, TextRange, Transform, TransformId,
};

/// Include public source schema roots.
pub fn schema(schema: &mut Schema) {
    schema.register::<PackageId>();
    schema.register::<ModuleId>();
    schema.register::<ProfileId>();
    schema.register::<ComponentId>();
    schema.register::<TargetId>();
    schema.register::<ProductId>();
    schema.register::<FileId>();
    schema.register::<BlobId>();
    schema.register::<Blob>();
    schema.register::<Span>();
    schema.register::<FileType>();

    schema.register::<ProvenanceId>();
    schema.register::<LocationId>();
    schema.register::<TransformId>();
    schema.register::<Location>();
    schema.register::<Transform>();
    schema.register::<ProvenanceTable>();

    schema.register::<DiagnosticSeverity>();
    schema.register::<DiagnosticTag>();
    schema.register::<Applicability>();
    schema.register::<DiagnosticLabel>();
    schema.register::<DiagnosticNote>();
    schema.register::<DiagnosticHelp>();
    schema.register::<DiagnosticSuggestion>();
    schema.register::<Diagnostic>();
    schema.register::<DiagnosticReference>();
    schema.register::<Patch>();
    schema.register::<FilePatch>();
    schema.register::<PatchSet>();

    schema.register::<TextChange>();
    schema.register::<TextRange>();
    schema.register::<TextPosition>();
    schema.register::<ByteRange>();
    schema.register::<TextPatch>();
    schema.register::<Edit>();
}

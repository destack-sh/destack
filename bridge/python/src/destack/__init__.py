# generated bridge target, do not edit

from ._native import VERSION, version
from .workspace import RemoteWorkspace, Workspace, open_workspace
from ._generated.artifact.dependency import (
    ArtifactDependency,
)
from ._generated.artifact.key import (
    ArtifactKey,
)
from ._generated.artifact.output import (
    BuildProfile,
    BuildLinkage,
    EmitFormat,
    FileType,
    SourceMapSource,
    SourceMap,
    Declaration,
    ScriptLanguage,
    Script,
    ObjectFormat,
    Object,
    Asset,
    Build,
    BundleSection,
    BundleMode,
    BundleFile,
    Bundle,
    ProgramFormat,
    ProgramHeader,
    Program,
    Runtime,
    Host,
    ProductTarget,
    Product,
)
from ._generated.artifact.record import (
    ArtifactString,
    ArtifactRecord,
)
from ._generated.artifact.sidecar import (
    ArtifactSidecarLabel,
    ArtifactSidecar,
)
from ._generated.artifact.version import (
    ArtifactVersion,
)
from ._generated.diagnostic.diagnostic import (
    DiagnosticSeverity,
    DiagnosticTag,
    Applicability,
    DiagnosticLabel,
    DiagnosticNote,
    DiagnosticHelp,
    DiagnosticSuggestion,
    Diagnostic,
)
from ._generated.diagnostic.edit import (
    Patch,
    FilePatch,
    PatchSet,
)
from ._generated.dir.checked import (
    DirChecked,
)
from ._generated.dir.parsed import (
    DirParsed,
)
from ._generated.dir.resolved import (
    DirResolved,
)
from ._generated.repository.revision import (
    Revision,
)
from ._generated.repository.trace import (
    TraceReport,
    TraceStage,
    TraceTime,
    TraceArtifact,
    TraceSpan,
    TraceCounter,
)
from ._generated.source.component import (
    ComponentId,
)
from ._generated.source.file import (
    FileId,
    ContentId,
    Content,
)
from ._generated.source.module import (
    ModuleId,
)
from ._generated.source.package import (
    PackageId,
)
from ._generated.source.product import (
    ProductId,
)
from ._generated.source.profile import (
    ProfileId,
)
from ._generated.source.span import (
    Span,
)
from ._generated.source.target import (
    TargetId,
)

__all__ = [
    "Workspace",
    "RemoteWorkspace",
    "open_workspace",
    "ArtifactDependency",
    "ArtifactKey",
    "BuildProfile",
    "BuildLinkage",
    "EmitFormat",
    "FileType",
    "SourceMapSource",
    "SourceMap",
    "Declaration",
    "ScriptLanguage",
    "Script",
    "ObjectFormat",
    "Object",
    "Asset",
    "Build",
    "BundleSection",
    "BundleMode",
    "BundleFile",
    "Bundle",
    "ProgramFormat",
    "ProgramHeader",
    "Program",
    "Runtime",
    "Host",
    "ProductTarget",
    "Product",
    "ArtifactString",
    "ArtifactRecord",
    "ArtifactSidecarLabel",
    "ArtifactSidecar",
    "ArtifactVersion",
    "DiagnosticSeverity",
    "DiagnosticTag",
    "Applicability",
    "DiagnosticLabel",
    "DiagnosticNote",
    "DiagnosticHelp",
    "DiagnosticSuggestion",
    "Diagnostic",
    "Patch",
    "FilePatch",
    "PatchSet",
    "DirChecked",
    "DirParsed",
    "DirResolved",
    "Revision",
    "TraceReport",
    "TraceStage",
    "TraceTime",
    "TraceArtifact",
    "TraceSpan",
    "TraceCounter",
    "ComponentId",
    "FileId",
    "ContentId",
    "Content",
    "ModuleId",
    "PackageId",
    "ProductId",
    "ProfileId",
    "Span",
    "TargetId",
    "VERSION",
    "version",
]

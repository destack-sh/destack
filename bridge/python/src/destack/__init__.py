# generated bridge target, do not edit

from ._native import VERSION, Repository, Session, Workspace, version
from .artifact.dependency import (
    ArtifactDependency,
)
from .artifact.key import (
    ArtifactKey,
)
from .artifact.output import (
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
    ModuleBuildKind,
    BuildRequest,
    BuildOutput,
)
from .artifact.record import (
    ArtifactString,
    ArtifactRecord,
)
from .artifact.sidecar import (
    ArtifactSidecarLabel,
    ArtifactSidecar,
)
from .artifact.version import (
    ArtifactVersion,
)
from .diagnostic.diagnostic import (
    DiagnosticSeverity,
    DiagnosticTag,
    Applicability,
    DiagnosticLabel,
    DiagnosticNote,
    DiagnosticHelp,
    DiagnosticSuggestion,
    Diagnostic,
)
from .diagnostic.edit import (
    Replacement,
    FilePatch,
    BatchEdit,
)
from .dir.checked import (
    DirChecked,
)
from .dir.parsed import (
    DirParsed,
)
from .dir.resolved import (
    DirResolved,
)
from .repository.revision import (
    Revision,
)
from .repository.trace import (
    TraceReport,
    TraceStage,
    TraceTime,
    TraceArtifact,
    TraceSpan,
    TraceCounter,
)
from .session.command.check import (
    CheckOutput,
)
from .session.command.format import (
    Document,
    FormatRequest,
    FormatOutput,
)
from .session.command.lint import (
    Scope,
    LintRequest,
    LintOutput,
)
from .session.command.parse import (
    ParseOutput,
)
from .session.file import (
    SessionFile,
)
from .session.module import (
    Module,
)
from .session.source.file import (
    Change,
)
from .session.source.source import (
    Source,
)
from .session.source.update import (
    TextRange,
    TextEdit,
    Edit,
    Commit,
)
from .source.component import (
    ComponentId,
)
from .source.file import (
    FileId,
    ContentId,
    Content,
)
from .source.module import (
    ModuleId,
)
from .source.package import (
    PackageId,
)
from .source.product import (
    ProductId,
)
from .source.profile import (
    ProfileId,
)
from .source.span import (
    Span,
)
from .source.target import (
    TargetId,
)

__all__ = [
    "Repository",
    "Session",
    "Workspace",
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
    "ModuleBuildKind",
    "BuildRequest",
    "BuildOutput",
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
    "Replacement",
    "FilePatch",
    "BatchEdit",
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
    "CheckOutput",
    "Document",
    "FormatRequest",
    "FormatOutput",
    "Scope",
    "LintRequest",
    "LintOutput",
    "ParseOutput",
    "SessionFile",
    "Module",
    "Change",
    "Source",
    "TextRange",
    "TextEdit",
    "Edit",
    "Commit",
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

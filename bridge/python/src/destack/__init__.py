# generated bridge target, do not edit

from ._native import VERSION, Session, version
from .artifact.dependency import (
    ArtifactPathState,
    ArtifactDirectoryEntry,
    ArtifactSourceDependency,
    ArtifactDependency,
)
from .artifact.key import (
    ArtifactKey,
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
    FileContentId,
    FileContent,
)
from .source.module import (
    ModuleId,
)
from .source.package import (
    PackageId,
)
from .source.profile import (
    ProfileId,
)
from .source.span import (
    Span,
    LabeledSpan,
)
from .source.target import (
    TargetId,
)

__all__ = [
    "Session",
    "ArtifactPathState",
    "ArtifactDirectoryEntry",
    "ArtifactSourceDependency",
    "ArtifactDependency",
    "ArtifactKey",
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
    "FileContentId",
    "FileContent",
    "ModuleId",
    "PackageId",
    "ProfileId",
    "Span",
    "LabeledSpan",
    "TargetId",
    "VERSION",
    "version",
]

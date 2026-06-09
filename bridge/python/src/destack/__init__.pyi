# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

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
    Edit,
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
    FileChange,
    FileChangeKind,
)
from .session.source.source import (
    Source,
)
from .session.source.update import (
    TextRange,
    TextEdit,
    FileEdit,
    FileUpdate,
    FileUpdateResult,
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
VERSION: str

def version() -> str: ...

class Session:
    """Python language session."""

    @staticmethod
    def open(source: Source) -> Session: ...

    def revision(self) -> Revision: ...

    def files(self) -> list[SessionFile]: ...

    def update(self, update: FileUpdate) -> FileUpdateResult: ...

    def reload(self) -> list[FileChange]: ...

    def load_module(self, path: str) -> Module: ...

    def provide(self, revision: Revision, keys: Sequence[ArtifactKey]) -> None: ...

    def require(self, revision: Revision, key: ArtifactKey) -> ArtifactVersion: ...

    def artifact_record(self, revision: Revision, key: ArtifactKey) -> ArtifactRecord: ...

    def parse(self, revision: Revision, module: Module) -> DirParsed: ...

    def resolve(self, revision: Revision, module: Module, profile: ProfileId) -> DirResolved: ...

    def check(self, revision: Revision, module: Module, profile: ProfileId) -> DirChecked: ...

    def diagnostics(self, revision: Revision, key: ArtifactKey | None = None) -> list[Diagnostic]: ...

    def sidecars(self, revision: Revision, key: ArtifactKey) -> list[ArtifactSidecar]: ...


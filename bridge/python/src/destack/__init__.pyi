# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from .artifact.key import (
    ArtifactKey,
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
    FileEdit,
    BatchEdit,
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
    SourceFile,
    SourceFileContent,
    FileUpdate,
    FileUpdateKind,
)
from .session.source.snapshot import (
    SourceSnapshot,
)
from .session.source.update import (
    TextRange,
    TextEdit,
    SourceEdit,
    SourceUpdate,
    SourceUpdateResult,
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
    def open_path(path: str) -> Session: ...

    @staticmethod
    def open_source(root: str, source: SourceSnapshot) -> Session: ...

    def revision(self) -> Revision: ...

    def files(self) -> list[SessionFile]: ...

    def update(self, update: SourceUpdate) -> SourceUpdateResult: ...

    def reload(self) -> list[FileUpdate]: ...

    def load_module(self, path: str) -> Module: ...

    def provide(self, revision: Revision, keys: Sequence[ArtifactKey]) -> None: ...

    def require(self, revision: Revision, key: ArtifactKey) -> ArtifactVersion: ...

    def diagnostics(self, revision: Revision, key: ArtifactKey | None = None) -> list[Diagnostic]: ...

    def sidecars(self, revision: Revision, key: ArtifactKey) -> list[ArtifactSidecar]: ...


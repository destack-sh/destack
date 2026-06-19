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
VERSION: str

def version() -> str: ...

class Repository:
    """Python language repository."""

    @staticmethod
    def open(source: Source) -> Repository: ...

    def root(self) -> str: ...

    def workspace(self) -> Workspace: ...


class Session:
    """Python language session."""

    @staticmethod
    def open(source: Source) -> Session: ...

    def revision(self) -> Revision: ...

    def files(self) -> list[SessionFile]: ...

    def edit(self, edits: Sequence[Edit]) -> Commit: ...

    def edit_if_current(self, revision: Revision, edits: Sequence[Edit]) -> Commit: ...

    def reload(self) -> list[Change]: ...

    def module(self, path: str) -> Module: ...

    def target(self, revision: Revision, package: PackageId, name: str) -> TargetId: ...

    def profile(self, revision: Revision, module: Module, name: str) -> ProfileId: ...

    def provide(self, revision: Revision, keys: Sequence[ArtifactKey]) -> None: ...

    def require(self, revision: Revision, key: ArtifactKey) -> ArtifactVersion: ...

    def artifact_record(self, revision: Revision, key: ArtifactKey) -> ArtifactRecord: ...

    def trace(self, revision: Revision, detailed: bool) -> TraceReport | None: ...

    def build(self, revision: Revision, request: BuildRequest) -> BuildOutput: ...

    def content(self, id: ContentId) -> Content: ...

    def text(self, id: ContentId) -> str: ...

    def bytes(self, id: ContentId) -> bytes: ...

    def parse(self, revision: Revision, module: Module) -> ParseOutput: ...

    def resolve(self, revision: Revision, module: Module, profile: ProfileId) -> DirResolved: ...

    def check(self, revision: Revision, module: Module, profile: ProfileId) -> CheckOutput: ...

    def format(self, revision: Revision, request: FormatRequest) -> FormatOutput: ...

    def lint(self, revision: Revision, request: LintRequest) -> LintOutput: ...

    def diagnostics(self, revision: Revision, key: ArtifactKey | None = None) -> list[Diagnostic]: ...

    def sidecars(self, revision: Revision, key: ArtifactKey) -> list[ArtifactSidecar]: ...


class Workspace:
    """Python language workspace."""

    @staticmethod
    def open(source: Source) -> Workspace: ...

    def root(self) -> str: ...

    def revision(self) -> Revision: ...

    def files(self) -> list[SessionFile]: ...

    def edit(self, edits: Sequence[Edit]) -> Commit: ...

    def edit_if_current(self, revision: Revision, edits: Sequence[Edit]) -> Commit: ...

    def reload(self) -> list[Change]: ...

    def module(self, path: str) -> Module: ...

    def target(self, revision: Revision, package: PackageId, name: str) -> TargetId: ...

    def profile(self, revision: Revision, module: Module, name: str) -> ProfileId: ...

    def provide(self, revision: Revision, keys: Sequence[ArtifactKey]) -> None: ...

    def require(self, revision: Revision, key: ArtifactKey) -> ArtifactVersion: ...

    def artifact_record(self, revision: Revision, key: ArtifactKey) -> ArtifactRecord: ...

    def trace(self, revision: Revision, detailed: bool) -> TraceReport | None: ...

    def build(self, revision: Revision, request: BuildRequest) -> BuildOutput: ...

    def content(self, id: ContentId) -> Content: ...

    def text(self, id: ContentId) -> str: ...

    def bytes(self, id: ContentId) -> bytes: ...

    def parse(self, revision: Revision, module: Module) -> ParseOutput: ...

    def resolve(self, revision: Revision, module: Module, profile: ProfileId) -> DirResolved: ...

    def check(self, revision: Revision, module: Module, profile: ProfileId) -> CheckOutput: ...

    def format(self, revision: Revision, request: FormatRequest) -> FormatOutput: ...

    def lint(self, revision: Revision, request: LintRequest) -> LintOutput: ...

    def diagnostics(self, revision: Revision, key: ArtifactKey | None = None) -> list[Diagnostic]: ...

    def sidecars(self, revision: Revision, key: ArtifactKey) -> list[ArtifactSidecar]: ...

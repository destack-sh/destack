# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.artifact.version import (
    ArtifactVersion,
)

from destack.session.module import (
    Module,
)

from destack.source.file import (
    ContentId,
)

from destack.source.product import (
    ProductId,
)

from destack.source.target import (
    TargetId,
)

class BuildProfile:
    """Build distribution profile crossing bridge boundaries."""

    @property
    def label(self) -> str: ...

class BuildLinkage:
    """Build payload linkage crossing bridge boundaries."""

    @property
    def label(self) -> str: ...

class EmitFormat:
    """Emitted artifact family crossing bridge boundaries."""

    @property
    def label(self) -> str: ...

class FileType:
    """Source file type crossing bridge boundaries."""

    @property
    def label(self) -> str: ...

class SourceMapSource:
    """One emitted or linked source map crossing bridge boundaries."""

    """The mapped source names."""
    @property
    def name(self) -> str: ...

    """The embedded source contents when they exist."""
    @property
    def content(self) -> str | None: ...

class SourceMap:
    """One emitted or linked source map crossing bridge boundaries."""

    """The source map version."""
    @property
    def version(self) -> int: ...

    """The emitted file name when one exists."""
    @property
    def file(self) -> str | None: ...

    """The source root when one exists."""
    @property
    def source_root(self) -> str | None: ...

    """The mapped sources."""
    @property
    def sources(self) -> list[SourceMapSource]: ...

    """The recorded symbol names."""
    @property
    def names(self) -> list[str]: ...

    """The VLQ mapping payload."""
    @property
    def mappings(self) -> str: ...

    """The debug id when one exists."""
    @property
    def debug_id(self) -> str | None: ...

class Declaration:
    """One emitted declaration crossing bridge boundaries."""

    """The declaration text."""
    @property
    def text(self) -> str: ...

class ScriptLanguage:
    """Structured script language crossing bridge boundaries."""

    @property
    def label(self) -> str: ...

class Script:
    """One structured script artifact crossing bridge boundaries."""

    """The target language of this script."""
    @property
    def language(self) -> ScriptLanguage: ...

    """The emitted declaration when one exists."""
    @property
    def declaration(self) -> Declaration | None: ...

    """The source map when one exists."""
    @property
    def map(self) -> SourceMap | None: ...

    """Whether this script has top-level side effects."""
    @property
    def has_top_level_side_effects(self) -> bool: ...

class ObjectFormat:
    """Compiled-code object format crossing bridge boundaries."""

    @property
    def label(self) -> str: ...

class Object:
    """One compiled-code object artifact crossing bridge boundaries."""

    """The compiled-code object format."""
    @property
    def format(self) -> ObjectFormat: ...

    """The encoded object content identity."""
    @property
    def content(self) -> ContentId: ...

    """The source map when one exists."""
    @property
    def map(self) -> SourceMap | None: ...

class Asset:
    """One opaque asset artifact crossing bridge boundaries."""

    """The asset file type."""
    @property
    def file_type(self) -> FileType: ...

    """The asset content identity."""
    @property
    def content(self) -> ContentId: ...

    """The source module URI when one exists."""
    @property
    def source(self) -> str | None: ...

    """The source map when one exists."""
    @property
    def map(self) -> SourceMap | None: ...

class Build:
    """One target-built toolchain payload crossing bridge boundaries."""

    """The build distribution profile."""
    @property
    def profile(self) -> BuildProfile: ...

    """The build linkage."""
    @property
    def linkage(self) -> BuildLinkage: ...

    """The encoded build content."""
    @property
    def content(self) -> ContentId: ...

class BundleSection:
    """One section of a linked bundle crossing bridge boundaries."""

    @property
    def label(self) -> str: ...

class BundleMode:
    """Bundle assembly mode crossing bridge boundaries."""

    @property
    def label(self) -> str: ...

class BundleFile:
    """One derived bundle file crossing bridge boundaries."""

    """The bundle section this file belongs to."""
    @property
    def section(self) -> BundleSection: ...

    """The output URI."""
    @property
    def uri(self) -> str: ...

    """The emitted file type."""
    @property
    def file_type(self) -> FileType: ...

    """The output content identity."""
    @property
    def content(self) -> ContentId: ...

    """The related source URI when one exists."""
    @property
    def source(self) -> str | None: ...

class Bundle:
    """One linked file graph crossing bridge boundaries."""

    """The emitted artifact family."""
    @property
    def emit(self) -> EmitFormat: ...

    """The target-level assembly mode."""
    @property
    def mode(self) -> BundleMode: ...

    """The files in this bundle."""
    @property
    def files(self) -> list[BundleFile]: ...

class ProgramFormat:
    """Program executable format crossing bridge boundaries."""

    @property
    def label(self) -> str: ...

class ProgramHeader:
    """Durable program header crossing bridge boundaries."""

    """Human-facing program name."""
    @property
    def name(self) -> str | None: ...

    """Build fingerprint that produced this program."""
    @property
    def fingerprint(self) -> str | None: ...

    """Target triple or equivalent target identity."""
    @property
    def target(self) -> str | None: ...

class Program:
    """Durable executable program crossing bridge boundaries."""

    """The program identity and compatibility header."""
    @property
    def header(self) -> ProgramHeader: ...

    """The executable format."""
    @property
    def format(self) -> ProgramFormat: ...

    """Content blobs referenced by the executable payload."""
    @property
    def contents(self) -> list[ContentId]: ...

class Runtime:
    """Semantic runtime contract crossing bridge boundaries."""

    @property
    def label(self) -> str: ...

class Host:
    """Host environment crossing bridge boundaries."""

    @property
    def label(self) -> str: ...

class ProductTarget:
    """One linked product target crossing bridge boundaries."""

    """The configured product target name."""
    @property
    def name(self) -> str: ...

    """The repository target assembled into this product."""
    @property
    def target(self) -> TargetId: ...

    """The runtime contract this target expects."""
    @property
    def runtime(self) -> Runtime: ...

    """The host environment this target expects."""
    @property
    def host(self) -> Host: ...

    """The platform this target expects."""
    @property
    def platform(self) -> str: ...

    """Whether this product target includes its toolchain build payload."""
    @property
    def includes_build(self) -> bool: ...

    """Whether this product target includes its linked bundle."""
    @property
    def includes_bundle(self) -> bool: ...

    """Whether this product target includes its executable program."""
    @property
    def includes_program(self) -> bool: ...

class Product:
    """One linked product crossing bridge boundaries."""

    """The configured product name."""
    @property
    def name(self) -> str: ...

    """The linked targets in deterministic order."""
    @property
    def targets(self) -> list[ProductTarget]: ...

class ModuleBuildKind:
    """Module build output family."""

    """Build the structured script artifact."""
    @staticmethod
    def script() -> ModuleBuildKind: ...

    """Build the compiled object artifact."""
    @staticmethod
    def object() -> ModuleBuildKind: ...

    """Build the opaque asset artifact."""
    @staticmethod
    def asset() -> ModuleBuildKind: ...

    @property
    def label(self) -> str: ...

class BuildRequest:
    """One language build request."""

    """Build one module artifact."""
    @staticmethod
    def module(module: Module, target: TargetId, output: ModuleBuildKind) -> BuildRequest: ...

    """Build one target build payload."""
    @staticmethod
    def build(target: TargetId) -> BuildRequest: ...

    """Build one package target."""
    @staticmethod
    def target(target: TargetId) -> BuildRequest: ...

    """Build one product."""
    @staticmethod
    def product(product: ProductId) -> BuildRequest: ...

    @property
    def kind(self) -> str: ...

class BuildOutput:
    """One language build output."""

    @property
    def kind(self) -> str: ...

    @property
    def asset(self) -> Asset | None: ...

    @property
    def build(self) -> Build | None: ...

    @property
    def bundle(self) -> Bundle | None: ...

    @property
    def object(self) -> Object | None: ...

    @property
    def product(self) -> Product | None: ...

    @property
    def program(self) -> Program | None: ...

    @property
    def script(self) -> Script | None: ...

    @property
    def version(self) -> ArtifactVersion | None: ...

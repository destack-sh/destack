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

    """Full Destack build."""
    @staticmethod
    def full() -> BuildProfile: ...

    """Smaller Destack build with optional services omitted."""
    @staticmethod
    def minimal() -> BuildProfile: ...

    """Freestanding output without the normal Destack runtime contract."""
    @staticmethod
    def freestanding() -> BuildProfile: ...

    @property
    def label(self) -> str: ...

class BuildLinkage:
    """Build payload linkage crossing bridge boundaries."""

    """Ship a portable Destack payload consumed by a runtime."""
    @staticmethod
    def portable() -> BuildLinkage: ...

    """Link the build payload into the produced platform binary."""
    @staticmethod
    def static() -> BuildLinkage: ...

    """Ship the build payload as a dynamic library."""
    @staticmethod
    def dynamic() -> BuildLinkage: ...

    @property
    def label(self) -> str: ...

class EmitFormat:
    """Emitted artifact family crossing bridge boundaries."""

    """JavaScript output."""
    @staticmethod
    def js() -> EmitFormat: ...

    """TypeScript output."""
    @staticmethod
    def ts() -> EmitFormat: ...

    """WebAssembly output."""
    @staticmethod
    def wasm() -> EmitFormat: ...

    """Native binary output."""
    @staticmethod
    def native() -> EmitFormat: ...

    @property
    def label(self) -> str: ...

class FileType:
    """Source file type crossing bridge boundaries."""

    """`.ds`."""
    @staticmethod
    def destack() -> FileType: ...

    """`.d.ds`."""
    @staticmethod
    def destack_declaration() -> FileType: ...

    """`.js`."""
    @staticmethod
    def java_script() -> FileType: ...

    """`.jsx`."""
    @staticmethod
    def java_script_xml() -> FileType: ...

    """`.ts`."""
    @staticmethod
    def type_script() -> FileType: ...

    """`.tsx`."""
    @staticmethod
    def type_script_xml() -> FileType: ...

    """`.d.ts`."""
    @staticmethod
    def type_script_declaration() -> FileType: ...

    """Text file."""
    @staticmethod
    def text() -> FileType: ...

    """TOML file."""
    @staticmethod
    def toml() -> FileType: ...

    """YAML file."""
    @staticmethod
    def yaml() -> FileType: ...

    """JSON file."""
    @staticmethod
    def json() -> FileType: ...

    """Environment file."""
    @staticmethod
    def env() -> FileType: ...

    """HTML file."""
    @staticmethod
    def html() -> FileType: ...

    """Markdown file."""
    @staticmethod
    def markdown() -> FileType: ...

    """CSS file."""
    @staticmethod
    def css() -> FileType: ...

    """SVG file."""
    @staticmethod
    def svg() -> FileType: ...

    """WebAssembly payload."""
    @staticmethod
    def wasm() -> FileType: ...

    """Node native module."""
    @staticmethod
    def node() -> FileType: ...

    """Source map file."""
    @staticmethod
    def source_map() -> FileType: ...

    """Native object file."""
    @staticmethod
    def object() -> FileType: ...

    """Image asset."""
    @staticmethod
    def image() -> FileType: ...

    """Font asset."""
    @staticmethod
    def font() -> FileType: ...

    """Audio asset."""
    @staticmethod
    def audio() -> FileType: ...

    """Video asset."""
    @staticmethod
    def video() -> FileType: ...

    """3D model asset."""
    @staticmethod
    def model() -> FileType: ...

    """AI model asset."""
    @staticmethod
    def neural() -> FileType: ...

    """Document asset."""
    @staticmethod
    def document() -> FileType: ...

    """Unknown binary file."""
    @staticmethod
    def binary() -> FileType: ...

    """Unknown file type."""
    @staticmethod
    def unknown() -> FileType: ...

    @property
    def label(self) -> str: ...

class SourceMapSource:
    """One emitted or linked source map crossing bridge boundaries."""

    def __init__(self, name: str, content: str | None) -> None: ...

    """The mapped source names."""
    @property
    def name(self) -> str: ...

    """The embedded source contents when they exist."""
    @property
    def content(self) -> str | None: ...

class SourceMap:
    """One emitted or linked source map crossing bridge boundaries."""

    def __init__(self, version: int, file: str | None, source_root: str | None, sources: Sequence[SourceMapSource], names: Sequence[str], mappings: str, debug_id: str | None) -> None: ...

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

    def __init__(self, text: str) -> None: ...

    """The declaration text."""
    @property
    def text(self) -> str: ...

class ScriptLanguage:
    """Structured script language crossing bridge boundaries."""

    """JavaScript output."""
    @staticmethod
    def java_script() -> ScriptLanguage: ...

    """TypeScript output."""
    @staticmethod
    def type_script() -> ScriptLanguage: ...

    @property
    def label(self) -> str: ...

class Script:
    """One structured script artifact crossing bridge boundaries."""

    def __init__(self, language: ScriptLanguage, declaration: Declaration | None, map: SourceMap | None, has_top_level_side_effects: bool) -> None: ...

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

    """Native relocatable object file."""
    @staticmethod
    def object() -> ObjectFormat: ...

    """WebAssembly object or module payload."""
    @staticmethod
    def wasm() -> ObjectFormat: ...

    @property
    def label(self) -> str: ...

class Object:
    """One compiled-code object artifact crossing bridge boundaries."""

    def __init__(self, format: ObjectFormat, content: ContentId, map: SourceMap | None) -> None: ...

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

    def __init__(self, file_type: FileType, content: ContentId, source: str | None, map: SourceMap | None) -> None: ...

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

    def __init__(self, profile: BuildProfile, linkage: BuildLinkage, content: ContentId) -> None: ...

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

    """Per-module library output."""
    @staticmethod
    def module() -> BundleSection: ...

    """Primary runnable entry output."""
    @staticmethod
    def entry() -> BundleSection: ...

    """Declaration or type surface."""
    @staticmethod
    def declaration() -> BundleSection: ...

    """Asset collection emitted by this target."""
    @staticmethod
    def asset() -> BundleSection: ...

    """Build manifest or output index."""
    @staticmethod
    def manifest() -> BundleSection: ...

    """Source maps or debug maps."""
    @staticmethod
    def source_map() -> BundleSection: ...

    """Native object or wasm payload."""
    @staticmethod
    def native() -> BundleSection: ...

    @property
    def label(self) -> str: ...

class BundleMode:
    """Bundle assembly mode crossing bridge boundaries."""

    """Per-module assets without target-level assembly."""
    @staticmethod
    def preserve_modules() -> BundleMode: ...

    """One assembled output file."""
    @staticmethod
    def single_file() -> BundleMode: ...

    """Multiple assembled output files."""
    @staticmethod
    def chunked() -> BundleMode: ...

    @property
    def label(self) -> str: ...

class BundleFile:
    """One derived bundle file crossing bridge boundaries."""

    def __init__(self, section: BundleSection, uri: str, file_type: FileType, content: ContentId, source: str | None) -> None: ...

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

    def __init__(self, emit: EmitFormat, mode: BundleMode, files: Sequence[BundleFile]) -> None: ...

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

    """VM executable program."""
    @staticmethod
    def vm() -> ProgramFormat: ...

    """Native executable program."""
    @staticmethod
    def native() -> ProgramFormat: ...

    @property
    def label(self) -> str: ...

class ProgramHeader:
    """Durable program header crossing bridge boundaries."""

    def __init__(self, name: str | None, fingerprint: str | None, target: str | None) -> None: ...

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

    def __init__(self, header: ProgramHeader, format: ProgramFormat, contents: Sequence[ContentId]) -> None: ...

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

    """Destack native runtime."""
    @staticmethod
    def destack() -> Runtime: ...

    """JavaScript host runtime."""
    @staticmethod
    def js() -> Runtime: ...

    @property
    def label(self) -> str: ...

class Host:
    """Host environment crossing bridge boundaries."""

    """Native host environment."""
    @staticmethod
    def native() -> Host: ...

    """Browser host environment."""
    @staticmethod
    def browser() -> Host: ...

    """WASI host environment."""
    @staticmethod
    def wasi() -> Host: ...

    """Emscripten host environment."""
    @staticmethod
    def emscripten() -> Host: ...

    """Freestanding target without host imports."""
    @staticmethod
    def freestanding() -> Host: ...

    @property
    def label(self) -> str: ...

class ProductTarget:
    """One linked product target crossing bridge boundaries."""

    def __init__(self, name: str, target: TargetId, runtime: Runtime, host: Host, platform: str, includes_build: bool, includes_bundle: bool, includes_program: bool) -> None: ...

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

    def __init__(self, name: str, targets: Sequence[ProductTarget]) -> None: ...

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

    @property
    def module_module(self) -> Module | None: ...

    @property
    def output(self) -> ModuleBuildKind | None: ...

    @property
    def product_product(self) -> ProductId | None: ...

    @property
    def target(self) -> TargetId | None: ...

    @property
    def target_target(self) -> TargetId | None: ...

class BuildOutput:
    """One language build output."""

    """Built script artifact."""
    @staticmethod
    def script(version: ArtifactVersion, script: Script) -> BuildOutput: ...

    """Built object artifact."""
    @staticmethod
    def object(version: ArtifactVersion, object: Object) -> BuildOutput: ...

    """Built asset artifact."""
    @staticmethod
    def asset(version: ArtifactVersion, asset: Asset) -> BuildOutput: ...

    """Built toolchain payload artifact."""
    @staticmethod
    def build(version: ArtifactVersion, build: Build) -> BuildOutput: ...

    """Built bundle artifact."""
    @staticmethod
    def bundle(version: ArtifactVersion, bundle: Bundle) -> BuildOutput: ...

    """Built program artifact."""
    @staticmethod
    def program(version: ArtifactVersion, program: Program) -> BuildOutput: ...

    """Built product artifact."""
    @staticmethod
    def product(version: ArtifactVersion, product: Product) -> BuildOutput: ...

    @property
    def kind(self) -> str: ...

    @property
    def asset_asset(self) -> Asset | None: ...

    @property
    def build_build(self) -> Build | None: ...

    @property
    def bundle_bundle(self) -> Bundle | None: ...

    @property
    def object_object(self) -> Object | None: ...

    @property
    def product_product(self) -> Product | None: ...

    @property
    def program_program(self) -> Program | None: ...

    @property
    def script_script(self) -> Script | None: ...

    @property
    def version(self) -> ArtifactVersion | None: ...

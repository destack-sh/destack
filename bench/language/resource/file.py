import base64
import hashlib
import io
import tempfile
from datetime import datetime, timedelta
from typing import (
    TYPE_CHECKING,
    Collection,
    Literal,
    Optional,
    Sequence,
    Union,
    assert_never,
    cast,
    overload,
)
from uuid import UUID

import aiohttp
import structlog
from opentelemetry import trace
from PIL import Image

from bench.language.core import (
    TITLE_CONSTRAINT,
    BuiltinObject,
    EnumType,
    LogLevel,
    NodeReference,
    NodeType,
    PrimitiveType,
    RunSpanType,
    Struct,
    StructType,
    active_session,
    constraint,
    enum_,
    node_,
    object_,
    p_internal,
    p_node_parent,
    p_regular,
    p_runtime,
    p_system,
    run_span,
    struct_,
)
from bench.pb2 import (
    DownloadFilesRequest,
    FileData,
    FileInfoData,
    UploadFilesRequest,
)
from bench.utils.func import IdEnum, group_by
from bench.utils.string import humanize_bytes
from bench.utils.utils import get_from_env

from .resource import DynamicResource

if TYPE_CHECKING:
    from magika import Magika

    from bench.language import Bench, File, Node, Session

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

FILE_HASH_LENGTH = 64  # 256 bits
MAX_FILE_SIZE = get_from_env(
    "MAX_FILE_SIZE", typ=int, default=1024 * 1024 * 128, description="Max file size (in bytes)"
)

MIME_TYPE_CONSTRAINT = constraint(min_length=1, max_length=255)
SHA256_CONSTRAINT = constraint(min_length=FILE_HASH_LENGTH, max_length=FILE_HASH_LENGTH)

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.FILE_KIND)
class FileKind(IdEnum):
    DRIVE = 1
    DRIVE_INLINE = 2
    INLINE = 3
    EXTERNAL = 10


@enum_(EnumType.FILE_RETENTION_MODE)
class FileRetentionMode(IdEnum):
    AUTOMATIC = 1  # garbage collected if no references
    MANUAL = 2  # never garbage collected
    TIMED = 3  # delete after a certain time


@enum_(EnumType.FILE_TYPE)
class FileType(IdEnum):
    TEXT = 1
    CODE = 2
    IMAGE = 3
    AUDIO = 4
    VIDEO = 5
    DOCUMENT = 6
    DATA = 7
    ARCHIVE = 8
    EXECUTABLE = 9
    GENERIC = 99


@enum_(EnumType.FILE_FORMAT)
class FileFormat(IdEnum):  # :FileFormats
    """The format of a file (roughly an extension)."""

    # text
    TXT = 10000
    MARKDOWN = 10001
    RTF = 10002
    INI = 10003
    LOG = 10004

    # code
    PYTHON = 20000
    JAVASCRIPT = 20001
    TYPESCRIPT = 20002
    GO = 20003
    C_LANG = 20004
    CPP = 20005
    OBJECTIVE_C = 20006
    SWIFT = 20007
    RUBY = 20008
    PHP = 20009
    CSS = 20011
    JAVA = 20012
    KOTLIN = 20013
    RUST = 20014
    SCALA = 20015
    SHELL = 20016
    SQL = 20017
    POWERSHELL = 20018
    ASSEMBLY = 20019
    LATEX = 20020

    # image
    JPEG = 30000
    PNG = 30001
    GIF = 30002
    BMP = 30003
    TIFF = 30004
    WEBP = 30005
    SVG = 30006
    ICO = 30007
    RAW = 30008
    HEIC = 30009
    HEIF = 30010

    # audio
    MP3 = 40000
    WAV = 40001
    FLAC = 40002
    AAC = 40003
    OGG = 40004
    M4A = 40005
    WMA = 40006

    # video
    MP4 = 50000
    WEBM = 50001
    AVI = 50002
    MOV = 50003
    WMV = 50004
    FLV = 50005
    MKV = 50006

    # document
    PDF = 60000
    DOCX = 60001
    PPTX = 60002
    ODT = 60003
    XLSX = 60004
    ODS = 60005
    EPUB = 60006
    MOBI = 60007
    CHM = 60008
    DOC = 60009
    XLS = 60100
    PPT = 60101
    HTML = 60102

    # data
    JSON = 70000
    YAML = 70001
    CSV = 70002
    XML = 70003
    TOML = 70004
    SQLITE = 70100
    PARQUET = 70101

    # archive
    ZIP = 80000
    RAR = 80001
    TAR = 80002
    SEVENZIP = 80003
    CAB = 80004
    GZIP = 80005
    BZIP2 = 80006
    XZ = 80007

    # executable
    EXE = 90000
    APP_IMAGE = 90001
    APK = 90002
    DMG = 90003
    JAR = 90004
    MSI = 90005
    DEB = 90006
    RPM = 90007

    @property
    def type(self) -> FileType:
        return FileType(self.value // 10000)

    @property
    def extension(self) -> str | None:
        return EXTENSION_BY_FILE_FORMAT.get(self)

    @property
    def extensions(self) -> Sequence[str]:
        return EXTENSIONS_BY_FILE_FORMAT.get(self, ())

    @property
    def mime_type(self) -> str | None:
        return MIME_TYPE_BY_FORMAT.get(self)

    @property
    def mime_types(self) -> Sequence[str]:
        return MIME_TYPES_BY_FILE_FORMAT.get(self, ())


# :FileFormats
FILE_FORMAT_BY_EXTENSION = {
    # text
    "txt": FileFormat.TXT,
    "md": FileFormat.MARKDOWN,
    "markdown": FileFormat.MARKDOWN,
    "rtf": FileFormat.RTF,
    # code
    "py": FileFormat.PYTHON,
    "js": FileFormat.JAVASCRIPT,
    "ts": FileFormat.TYPESCRIPT,
    "go": FileFormat.GO,
    "c": FileFormat.C_LANG,
    "cpp": FileFormat.CPP,
    "cxx": FileFormat.CPP,
    "h": FileFormat.C_LANG,
    "hpp": FileFormat.CPP,
    "m": FileFormat.OBJECTIVE_C,
    "swift": FileFormat.SWIFT,
    "rb": FileFormat.RUBY,
    "php": FileFormat.PHP,
    "css": FileFormat.CSS,
    "java": FileFormat.JAVA,
    "kt": FileFormat.KOTLIN,
    "rs": FileFormat.RUST,
    "scala": FileFormat.SCALA,
    "sh": FileFormat.SHELL,
    "bash": FileFormat.SHELL,
    "sql": FileFormat.SQL,
    "ps1": FileFormat.POWERSHELL,
    "asm": FileFormat.ASSEMBLY,
    "tex": FileFormat.LATEX,
    # image
    "jpg": FileFormat.JPEG,
    "jpeg": FileFormat.JPEG,
    "png": FileFormat.PNG,
    "gif": FileFormat.GIF,
    "bmp": FileFormat.BMP,
    "tiff": FileFormat.TIFF,
    "tif": FileFormat.TIFF,
    "webp": FileFormat.WEBP,
    "svg": FileFormat.SVG,
    "ico": FileFormat.ICO,
    "raw": FileFormat.RAW,
    "cr2": FileFormat.RAW,
    "nef": FileFormat.RAW,
    "arw": FileFormat.RAW,
    "heic": FileFormat.HEIC,
    "heif": FileFormat.HEIF,
    # audio
    "mp3": FileFormat.MP3,
    "wav": FileFormat.WAV,
    "flac": FileFormat.FLAC,
    "aac": FileFormat.AAC,
    "ogg": FileFormat.OGG,
    "m4a": FileFormat.M4A,
    "wma": FileFormat.WMA,
    # video
    "mp4": FileFormat.MP4,
    "webm": FileFormat.WEBM,
    "avi": FileFormat.AVI,
    "mov": FileFormat.MOV,
    "wmv": FileFormat.WMV,
    "flv": FileFormat.FLV,
    "mkv": FileFormat.MKV,
    # document
    "pdf": FileFormat.PDF,
    "docx": FileFormat.DOCX,
    "pptx": FileFormat.PPTX,
    "odt": FileFormat.ODT,
    "xlsx": FileFormat.XLSX,
    "ods": FileFormat.ODS,
    "epub": FileFormat.EPUB,
    "mobi": FileFormat.MOBI,
    "chm": FileFormat.CHM,
    "doc": FileFormat.DOC,
    "xls": FileFormat.XLS,
    "ppt": FileFormat.PPT,
    "html": FileFormat.HTML,
    "htm": FileFormat.HTML,
    # data
    "json": FileFormat.JSON,
    "yaml": FileFormat.YAML,
    "yml": FileFormat.YAML,
    "csv": FileFormat.CSV,
    "sqlite": FileFormat.SQLITE,
    "db": FileFormat.SQLITE,
    "parquet": FileFormat.PARQUET,
    # archive
    "zip": FileFormat.ZIP,
    "rar": FileFormat.RAR,
    "tar": FileFormat.TAR,
    "7z": FileFormat.SEVENZIP,
    "cab": FileFormat.CAB,
    "gz": FileFormat.GZIP,
    "bz2": FileFormat.BZIP2,
    "xz": FileFormat.XZ,
    # executable
    "exe": FileFormat.EXE,
    "appimage": FileFormat.APP_IMAGE,
    "apk": FileFormat.APK,
    "dmg": FileFormat.DMG,
    "jar": FileFormat.JAR,
    "msi": FileFormat.MSI,
    "deb": FileFormat.DEB,
    "rpm": FileFormat.RPM,
}
EXTENSIONS_BY_FILE_FORMAT = group_by(
    FILE_FORMAT_BY_EXTENSION.keys(), lambda ext: FILE_FORMAT_BY_EXTENSION[ext]
)
EXTENSION_BY_FILE_FORMAT: dict[FileFormat, str] = {
    v: k for k, v in FILE_FORMAT_BY_EXTENSION.items()
}
FILE_FORMAT_BY_MIME_TYPE = {
    # text
    "text/plain": FileFormat.TXT,
    "text/markdown": FileFormat.MARKDOWN,
    "text/x-markdown": FileFormat.MARKDOWN,
    "application/rtf": FileFormat.RTF,
    "text/rtf": FileFormat.RTF,
    # code
    "text/x-python": FileFormat.PYTHON,
    "application/x-python-code": FileFormat.PYTHON,
    "text/x-python-script": FileFormat.PYTHON,
    "text/javascript": FileFormat.JAVASCRIPT,
    "application/javascript": FileFormat.JAVASCRIPT,
    "application/x-javascript": FileFormat.JAVASCRIPT,
    "text/typescript": FileFormat.TYPESCRIPT,
    "application/typescript": FileFormat.TYPESCRIPT,
    "text/x-go": FileFormat.GO,
    "application/x-go": FileFormat.GO,
    "text/x-c": FileFormat.C_LANG,
    "text/x-csrc": FileFormat.C_LANG,
    "text/x-c++": FileFormat.CPP,
    "text/x-c++src": FileFormat.CPP,
    "text/x-objective-c": FileFormat.OBJECTIVE_C,
    "text/x-swift": FileFormat.SWIFT,
    "text/x-ruby": FileFormat.RUBY,
    "application/x-ruby": FileFormat.RUBY,
    "text/x-php": FileFormat.PHP,
    "application/x-httpd-php": FileFormat.PHP,
    "text/css": FileFormat.CSS,
    "text/x-java-source": FileFormat.JAVA,
    # image
    "image/jpeg": FileFormat.JPEG,
    "image/pjpeg": FileFormat.JPEG,
    "image/png": FileFormat.PNG,
    "image/gif": FileFormat.GIF,
    "image/bmp": FileFormat.BMP,
    "image/x-windows-bmp": FileFormat.BMP,
    "image/tiff": FileFormat.TIFF,
    "image/webp": FileFormat.WEBP,
    "image/svg+xml": FileFormat.SVG,
    "image/x-icon": FileFormat.ICO,
    "image/x-adobe-dng": FileFormat.RAW,
    "image/x-canon-cr2": FileFormat.RAW,
    "image/x-nikon-nef": FileFormat.RAW,
    "image/heic": FileFormat.HEIC,
    "image/heif": FileFormat.HEIF,
    # audio
    "audio/mpeg": FileFormat.MP3,
    "audio/mp3": FileFormat.MP3,
    "audio/x-wav": FileFormat.WAV,
    "audio/wav": FileFormat.WAV,
    "audio/flac": FileFormat.FLAC,
    "audio/x-flac": FileFormat.FLAC,
    "audio/aac": FileFormat.AAC,
    "audio/aacp": FileFormat.AAC,
    "audio/ogg": FileFormat.OGG,
    "application/ogg": FileFormat.OGG,
    "audio/x-m4a": FileFormat.M4A,
    "audio/mp4": FileFormat.M4A,
    "audio/x-ms-wma": FileFormat.WMA,
    # video
    "video/mp4": FileFormat.MP4,
    "video/webm": FileFormat.WEBM,
    "video/x-msvideo": FileFormat.AVI,
    "video/avi": FileFormat.AVI,
    "video/quicktime": FileFormat.MOV,
    "video/x-ms-wmv": FileFormat.WMV,
    "video/x-flv": FileFormat.FLV,
    "video/x-matroska": FileFormat.MKV,
    "video/x-m4v": FileFormat.MP4,
    # document
    "application/pdf": FileFormat.PDF,
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document": FileFormat.DOCX,
    "application/vnd.openxmlformats-officedocument.presentationml.presentation": FileFormat.PPTX,
    "application/vnd.oasis.opendocument.text": FileFormat.ODT,
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet": FileFormat.XLSX,
    "application/vnd.oasis.opendocument.spreadsheet": FileFormat.ODS,
    "application/epub+zip": FileFormat.EPUB,
    "application/x-mobipocket-ebook": FileFormat.MOBI,
    "application/vnd.ms-htmlhelp": FileFormat.CHM,
    "application/msword": FileFormat.DOC,
    "application/vnd.ms-excel": FileFormat.XLS,
    "application/vnd.ms-powerpoint": FileFormat.PPT,
    "text/html": FileFormat.HTML,
    "application/xhtml+xml": FileFormat.HTML,
    # data
    "application/json": FileFormat.JSON,
    "application/yaml": FileFormat.YAML,
    "text/yaml": FileFormat.YAML,
    "text/csv": FileFormat.CSV,
    "text/x-csv": FileFormat.CSV,
    "application/x-sqlite3": FileFormat.SQLITE,
    "application/vnd.sqlite3": FileFormat.SQLITE,
    "application/vnd.apache.parquet": FileFormat.PARQUET,
    # archive
    "application/zip": FileFormat.ZIP,
    "application/x-zip-compressed": FileFormat.ZIP,
    "application/x-rar-compressed": FileFormat.RAR,
    "application/vnd.rar": FileFormat.RAR,
    "application/x-tar": FileFormat.TAR,
    "application/x-7z-compressed": FileFormat.SEVENZIP,
    "application/vnd.ms-cab-compressed": FileFormat.CAB,
    "application/gzip": FileFormat.GZIP,
    "application/x-gzip": FileFormat.GZIP,
    "application/x-bzip2": FileFormat.BZIP2,
    "application/x-xz": FileFormat.XZ,
    # executable
    "application/x-msdownload": FileFormat.EXE,
    "application/x-executable": FileFormat.EXE,
    "application/vnd.microsoft.portable-executable": FileFormat.EXE,
    "application/vnd.appimage": FileFormat.APP_IMAGE,
    "application/vnd.android.package-archive": FileFormat.APK,
    "application/x-apple-diskimage": FileFormat.DMG,
    "application/java-archive": FileFormat.JAR,
    "application/x-java-archive": FileFormat.JAR,
    "application/x-msi": FileFormat.MSI,
    "application/vnd.debian.binary-package": FileFormat.DEB,
    "application/x-debian-package": FileFormat.DEB,
    "application/x-rpm": FileFormat.RPM,
    "application/x-redhat-package-manager": FileFormat.RPM,
}
MIME_TYPES_BY_FILE_FORMAT = group_by(
    FILE_FORMAT_BY_MIME_TYPE.keys(), lambda mime_type: FILE_FORMAT_BY_MIME_TYPE[mime_type]
)
MIME_TYPE_BY_FORMAT: dict[FileFormat, str] = {v: k for k, v in FILE_FORMAT_BY_MIME_TYPE.items()}


@object_()
class FileBase(BuiltinObject):
    """
    Core File info.
    """

    # common
    type: FileType = p_internal(30)
    kind: FileKind = p_internal(31)
    name: str = p_regular(32, constraint=TITLE_CONSTRAINT)

    # meta
    mime_type: str | None = p_internal(50, constraint=MIME_TYPE_CONSTRAINT)
    format: FileFormat | None = p_internal(51, default=None)
    size: int = p_internal(
        52, primitive_type=PrimitiveType.INT64, constraint=constraint(min_value=0)
    )
    sha256: str | None = p_internal(53, constraint=SHA256_CONSTRAINT)

    # content
    external_url: Optional[str] = p_regular(60, default=None)  # if external
    inline_content: Optional[bytes] = p_regular(
        61, default=None, constraint=constraint(min_length=1)
    )
    ...  # thumbnail/preview/...?

    # multimedia
    width: Optional[int] = p_internal(70, default=None)
    height: Optional[int] = p_internal(71, default=None)
    aspect_ratio: Optional[float] = p_internal(72, default=None)
    codec: Optional[str] = p_internal(73, default=None)
    duration: Optional[timedelta] = p_internal(74, default=None)
    bitrate: Optional[int] = p_internal(75, default=None)
    channels: Optional[int] = p_internal(76, default=None)
    sample_rate: Optional[int] = p_internal(77, default=None)

    # cached content
    _original: Optional["File"] = p_runtime(default=None)  # if converted
    _cached_get_url: Optional[str] = p_runtime(default=None)
    _cached_tmp_path: Optional[str] = p_runtime(default=None)
    _cached_content: Optional[bytes] = p_runtime(default=None)
    _cached_image: Optional[Image.Image] = p_runtime(default=None)

    def __content_str__(self) -> str:
        content_parts = [f"'{self.name}'", humanize_bytes(self.size)]
        if self.format:
            content_parts.append(f"{self.type.bench_name}/{self.format.bench_name}")
        else:
            content_parts.append(self.type.bench_name)
        if self.mime_type:
            content_parts.append(f"'{self.mime_type}'")
        if self.width and self.height:
            content_parts.append(f"{self.width}x{self.height}")
        if self.duration:
            duration_str = f"{self.duration.total_seconds():.3f}s"
            content_parts.append(f"{duration_str}s")
        return ", ".join(content_parts)

    def get_original(self) -> "File | None":
        """The original file (if converted or self)."""
        if self._original is not None:
            return self._original
        elif isinstance(self, File):
            return self
        else:
            return None

    @property
    def original(self) -> "File":
        """The original file (if converted or self)."""
        original = self.get_original()
        assert original is not None, f"no original for {self!r}"
        return original

    #
    # Generic content
    #

    async def upload(self) -> "File":
        """Uploads the file to the host (if not already uploaded)."""
        raise NotImplementedError

    @overload
    async def download(self, *, include_content: Literal[True] = True) -> bytes: ...
    @overload
    async def download(self, *, include_content: Literal[False] = False) -> str: ...
    async def download(self, *, include_content: bool = True) -> Union[bytes, str]:
        """Downloads the file from the source."""
        assert isinstance(self, File), f"cannot download {self!r}"
        if include_content:
            if self._cached_content is not None:
                return self._cached_content
        elif self._cached_get_url is not None:
            return self._cached_get_url
        await download_file_batch(
            [self], include_content=include_content, session=self.active_session
        )
        if include_content:
            assert self._cached_content is not None, f"content not ready for {self!r}"
            return self._cached_content
        else:
            assert self._cached_get_url is not None, f"content not ready for {self!r}"
            return self._cached_get_url

    def to_tmp_file(self) -> str:
        """Downloads the file to a temporary file."""
        if self._cached_tmp_path is None:
            with tempfile.NamedTemporaryFile(delete=False) as tmp_file:
                tmp_file.write(self.content)
            assert isinstance(tmp_file.name, str), f"no path in tmp file {tmp_file!r} for {self!r}"
            self._cached_tmp_path = tmp_file.name
        return self._cached_tmp_path

    @property
    def content(self) -> bytes:
        """The file content."""
        if self.inline_content is not None:
            return self.inline_content
        elif self._cached_content is not None:
            return self._cached_content
        else:
            raise ValueError(f"content not ready for {self!r}")

    @property
    def url(self) -> str:
        """The URL to GET the file from."""
        if self.external_url is not None:
            return self.external_url
        elif self._cached_get_url is not None:
            return self._cached_get_url
        else:
            raise ValueError(f"url not ready for {self!r}")

    def _clear_cache(self):
        """Clears the cached content and URL."""
        self._cached_content = None
        self._cached_get_url = None

    def b64encode(self) -> str:
        """Encodes the file content as base64."""
        return base64.b64encode(self.content).decode()

    @tracer.start_as_current_span("file.convert")
    async def convert(self, target_format: FileFormat) -> "FileBase":
        """Converts the file to the given type/format."""
        if target_format == self.format:
            return self
        elif self.type == FileType.IMAGE:
            assert target_format.type == self.type, f"cannot convert {self!r} to {target_format!r}"
            buffer = io.BytesIO()
            self.image.save(buffer, format=target_format.name)
            text = buffer.getvalue()
            return FileInfo(
                kind=FileKind.INLINE,
                name=self.name,
                mime_type=target_format.mime_type,
                type=target_format.type,
                format=target_format,
                size=len(text),
                width=self.width,
                height=self.height,
                aspect_ratio=self.aspect_ratio,
                inline_content=text,
                _original=self.original,
            )
        elif self.type == FileType.DOCUMENT:
            # NOTE :Incomplete: handle images when converting documents
            if target_format == FileFormat.MARKDOWN and self.format in (
                FileFormat.DOC,
                FileFormat.DOCX,
                FileFormat.ODT,
            ):
                # convert with pandoc
                import pypandoc

                tmp_file_path = self.to_tmp_file()
                text = pypandoc.convert_file(
                    tmp_file_path, target_format.name.lower(), format=self.format.name.lower()
                )
                content = text.encode()
                return FileInfo(
                    kind=FileKind.INLINE,
                    name=self.name,
                    mime_type="text/markdown",
                    type=target_format.type,
                    format=target_format,
                    size=len(content),
                    inline_content=content,
                    _original=self.original,
                )
            elif target_format == FileFormat.MARKDOWN and self.format == FileFormat.PDF:
                # convert with pypdf
                import pypdf

                reader = pypdf.PdfReader(io.BytesIO(self.content))
                pages_text: list[str] = []
                for page in reader.pages:
                    page_text = page.extract_text()
                    pages_text.append(page_text)
                text = "\n\n".join(pages_text)
                content = text.encode()
                return FileInfo(
                    kind=FileKind.INLINE,
                    name=self.name,
                    mime_type="text/markdown",
                    type=target_format.type,
                    format=target_format,
                    size=len(content),
                    inline_content=content,
                    _original=self.original,
                )

        raise ValueError(f"cannot convert {self!r} to {target_format!r}")

    #
    # Text content
    #

    @property
    def text(self) -> str:
        """Gets the text content of the file."""
        if self.type == FileType.TEXT or self.type == FileType.CODE:
            return self.content.decode()
        else:
            raise ValueError(f"cannot get text content of {self!r}")

    @property
    def lines(self) -> list[str]:
        """Gets the lines of the file."""
        if self.type == FileType.TEXT or self.type == FileType.CODE:
            return self.text.splitlines()
        else:
            raise ValueError(f"cannot get lines of {self!r}")

    #
    # Image content
    #

    @property
    def image(self) -> Image.Image:
        """Gets the image content of the file."""
        if self.type == FileType.IMAGE:
            if self._cached_image is None:
                self._cached_image = Image.open(io.BytesIO(self.content))
            return self._cached_image
        else:
            raise ValueError(f"cannot get image content of {self!r}")

    @tracer.start_as_current_span("file.downscale")
    async def downscale(self, max_pixels: int, max_size: int, quality_step: int = 20) -> "FileBase":
        """Downscales the image to the given max size and max pixels."""

        # scale down size
        width, height = self.image.size
        scale = min(1.0, max_pixels / max(width, height))

        # resize the image if needed
        if scale < 1.0:
            new_width = int(width * scale)
            new_height = int(height * scale)
            optimized_image = self.image.copy().resize(
                (new_width, new_height), Image.Resampling.LANCZOS
            )
            buffer = io.BytesIO()
            if optimized_image.mode in ("RGBA", "P"):
                optimized_image = optimized_image.convert("RGB")
            optimized_image.save(buffer, format="JPEG", subsampling=0, quality=100)
            content = buffer.getvalue()
        else:
            new_width = width
            new_height = height
            optimized_image = self.image
            content = self.content

        # reduce quality until it fits
        quality = 100 - quality_step
        while len(content) > max_size and quality > quality_step:
            quality -= quality_step
            buffer = io.BytesIO()
            optimized_image.save(buffer, format="JPEG", subsampling=0, quality=quality)
            content = buffer.getvalue()

        return FileInfo(
            kind=FileKind.INLINE,
            name=self.name,
            inline_content=content,
            type=self.type,
            format=FileFormat.JPEG,
            size=len(content),
            width=new_width,
            height=new_height,
            aspect_ratio=new_width / new_height,
            mime_type="image/jpeg",
            _original=self.original,
        )


@struct_(StructType.FILE_INFO)
class FileInfo(Struct[FileInfoData], FileBase):
    """
    File metadata.
    """

    __content_str__ = FileBase.__content_str__  # type: ignore


@node_(NodeType.FILE)
class File(DynamicResource[FileData], FileBase):
    """
    A File stored somewhere (like in a Drive, or externally).
    """

    parent: Union["Bench", None] = p_node_parent(4, NodeType.BENCH, is_system=True)

    # content/info
    # ...FileInfoBase[50-79]

    # meta
    retention: FileRetentionMode = p_system(
        80, default=FileRetentionMode.AUTOMATIC, default_sql=None
    )
    expires_at: Optional[datetime] = p_system(81)

    __content_str__ = FileBase.__content_str__  # type: ignore


@tracer.start_as_current_span("file.upload_batch")
async def upload_file_batch(
    files: list[File], file_contents: list[bytes], session: "Session | None" = None
):
    """Uploads the given Files to their Host."""
    from bench.proto import MACHINE_ENVIRONMENT

    assert len(files) == len(
        file_contents
    ), f"unexpected files: {len(files)} != {len(file_contents)}"
    if not files:
        return
    if session is None:
        session = active_session()

    # get upload URLs
    with run_span(
        tracer, "file.prepare_upload", RunSpanType.FILE_PREPARE_UPLOAD, level=LogLevel.DEBUG
    ):
        upload_req = UploadFilesRequest(
            scope=session._get_scope_for_node(files[0]),
            files=[f._to_data() for f in files],
            environment=MACHINE_ENVIRONMENT,
        )
        upload_rep = await session.host.upload_files(upload_req, metadata=session._rpc_headers)
        assert len(upload_rep.handles) == len(
            files
        ), f"unexpected handles: {len(upload_rep.handles)} != {len(files)}"
        handles_by_id = {h.file.id: h for h in upload_rep.handles}
        del upload_rep
        logger.trace("file.upload_batch.prepare", files=files, span="current")

    # upload files
    async with aiohttp.ClientSession() as http_session:
        for file, file_content in zip(files, file_contents):
            handle = handles_by_id[str(file.id)]
            with tracer.start_as_current_span("file.upload", attributes={"file": repr(file)}):
                assert file.kind in (
                    FileKind.DRIVE,
                    FileKind.DRIVE_INLINE,
                ), f"unexpected file: {file!r}"

                # POST file to url
                form_data = aiohttp.FormData()
                for key, value in handle.fields.items():
                    form_data.add_field(key, value)
                form_data.add_field(
                    "file", file_content, filename=file.name, content_type=file.mime_type
                )
                async with http_session.post(handle.post_url, data=form_data) as resp:
                    resp.raise_for_status()
                file._cached_content = file_content
                file._cached_get_url = handle.get_url
                logger.debug("file.upload", file=file, url=file._cached_get_url, span="current")
    logger.debug("file.upload_batch", files=files, span="current")


@run_span(tracer, "file.download_batch", RunSpanType.FILE_DOWNLOAD)
async def download_file_batch(
    file_refs: Sequence[NodeReference | File],
    *,
    include_content: bool | Collection[NodeReference | File],
    session: "Session | None" = None,
) -> list[File]:
    """Downloads the given Files from their Host."""
    from bench.proto import MACHINE_ENVIRONMENT, unpack_builtin_object

    if not file_refs:
        return []
    if session is None:
        session = active_session()

    # get download URLs
    with run_span(tracer, "file.prepare_download", RunSpanType.FILE_PREPARE_DOWNLOAD) as span:
        download_req = DownloadFilesRequest(
            scope=session._get_scope_for_node(session),
            files=[(f.to_ref() if isinstance(f, File) else f)._to_data() for f in file_refs],
            environment=MACHINE_ENVIRONMENT,
        )
        download_rep = await session.host.download_files(
            download_req, metadata=session._rpc_headers
        )
        handles_by_id = {h.file.id: h for h in download_rep.handles}
        del download_rep
        if span is not None:
            span.nodes = cast(list["Node"], [h.file for h in handles_by_id.values()])
        file_refs_by_id = {f.id: f for f in file_refs}
        files_by_id: dict[UUID, File] = {}
        for file_ref in file_refs:
            handle = handles_by_id.get(str(file_ref.id))
            if handle is None:
                raise RuntimeError(f"missing download handle for {file_ref!r}")
            if isinstance(file_ref, File):
                file = file_ref
            else:
                file = unpack_builtin_object(
                    handle.file, supergraph=session._supergraph, expect=File
                )
            files_by_id[file.id] = file
            file._cached_get_url = handle.get_url

    # download files
    if include_content is True:
        files_to_download = files_by_id.values()
    elif include_content is False:
        files_to_download = []
    else:
        files_to_download = [files_by_id[cast(UUID, f.id)] for f in include_content]
    if files_to_download:
        file_contents: list[bytes] = []
        async with aiohttp.ClientSession() as http_session:
            for file in files_to_download:
                handle = handles_by_id[str(file.id)]
                # GET file from url
                with tracer.start_as_current_span("file.download", attributes={"file": repr(file)}):
                    async with http_session.get(handle.get_url) as resp:
                        resp.raise_for_status()
                        file_content = await resp.read()
                    file_contents.append(file_content)
                    file._cached_content = file_content
                    file_ref = file_refs_by_id[file.id]

    return list(files_by_id.values())


FileIn = Union[str, bytes, Image.Image]


@tracer.start_as_current_span("file.extract_info")
async def extract_file_info(  # noqa: RUF029
    file_in: FileIn,
    name: str,
    *,
    mime_type: str | None = None,
    type: FileType | None = None,
    format: FileFormat | str | None = None,
) -> tuple["File", bytes]:  # :ExtractFileInfo
    """Extracts the metadata from a file."""
    # content
    content: bytes
    if isinstance(file_in, str):
        content = file_in.encode()
    elif isinstance(file_in, (bytes, bytearray, memoryview)):
        content = file_in
    elif isinstance(file_in, Image.Image):
        content_io = io.BytesIO()
        file_in.save(content_io, format="PNG")
        content = content_io.getvalue()
    else:
        assert_never(file_in)

    # guess file type
    if isinstance(format, str):
        format = format.lower()
        assert format in FILE_FORMAT_BY_EXTENSION, f"unknown format: {format}"
        format = FILE_FORMAT_BY_EXTENSION.get(format)
    if format is None and name is not None and "." in name:
        format = FILE_FORMAT_BY_EXTENSION.get(name.split(".")[-1])
    if format is not None:
        type = format.type
    elif type is None:
        type = FileType.GENERIC
    if mime_type is None and format is not None:
        mime_type = format.mime_type

    # guess with magika if needed
    if format is None:
        mime_type, format = detect_file_format(content)
        if format is not None:
            type = format.type

    # add extension if needed
    if format is not None and "." not in name and format.extension is not None:
        name = f"{name}.{format.extension}"

    size = len(content)
    sha256 = hashlib.sha256(content).hexdigest()
    file = File(
        kind=FileKind.DRIVE,
        name=name,
        type=type,
        mime_type=mime_type,
        format=format,
        size=size,
        sha256=sha256,
    )

    # TODO :Incomplete: extract more file metadata :ExtractFileInfo

    # image metadata
    if type == FileType.IMAGE:
        image = file_in if isinstance(file_in, Image.Image) else Image.open(io.BytesIO(content))
        file.width, file.height = image.size
        file.aspect_ratio = file.width / file.height

    return file, content


@run_span(tracer, "file.upload", RunSpanType.FILE_UPLOAD)
async def upload_file(
    file_in: FileIn,
    name: str,
    *,
    mime_type: str | None = None,
    type: FileType | None = None,
    format: FileFormat | str | None = None,
    bench: "Bench | None" = None,
    session: "Session | None" = None,
) -> "File":
    """Uploads the given file to the given (or current) session."""

    # extract file info
    file, content = await extract_file_info(
        file_in, name, mime_type=mime_type, type=type, format=format
    )

    # bench
    if session is None:
        session = active_session()
    if bench is None:
        bench = session.bench
        if bench is None:
            raise ValueError(f"no Bench to upload file {name!r} to in {session!r}")
    file.parent = bench

    # upload file, then create in session
    await upload_file_batch(files=[file], file_contents=[content], session=session)
    session._create(file)

    return file


_magika: "Magika | None" = None


def _get_magika() -> "Magika":
    from magika import Magika

    global _magika
    if _magika is None:
        _magika = Magika()
    return _magika


@tracer.start_as_current_span("file.detect_format")
def detect_file_format(content: bytes) -> tuple[str | None, FileFormat | None]:
    """Detects the file format from the given file content."""
    magika = _get_magika()
    magika_result = magika.identify_bytes(content)
    if magika_result:
        mime_type = magika_result.output.mime_type
        format = FILE_FORMAT_BY_MIME_TYPE.get(mime_type)
        return mime_type, format
    else:
        return None, None

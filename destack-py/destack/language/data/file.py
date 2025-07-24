import base64
import io
import tempfile
from collections.abc import Collection, Sequence
from datetime import timedelta
from typing import (
    TYPE_CHECKING,
    Literal,
    Optional,
    Union,
    overload,
)

import structlog
from opentelemetry import trace
from PIL import Image

from destack.language.core import (
    Enum,
    EnumType,
    NodeType,
    PrimitiveType,
    Resource,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
    builtin_property_runtime,
)
from destack.utils.env import get_from_env
from destack.utils.func import group_by

if TYPE_CHECKING:
    from destack.language import File, Session, Space

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

FILE_HASH_LENGTH = 64  # 256 bits
MAX_FILE_SIZE = get_from_env(
    "MAX_FILE_SIZE", typ=int, default=1024 * 1024 * 128, description="Max file size (in bytes)"
)


# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.FILE_RETENTION_MODE)
class FileRetentionMode(Enum):
    AUTOMATIC = 1  # garbage collected if no references
    MANUAL = 2  # never garbage collected
    TIMED = 3  # delete after a certain time


@builtin_enum(EnumType.FILE_TYPE)
class FileType(Enum):
    TEXT = 1, None, None, "fas fa-file-lines"
    CODE = 2, None, None, "fas fa-file-code"
    IMAGE = 3, None, None, "fas fa-image"
    AUDIO = 4, None, None, "fas fa-volume"
    VIDEO = 5, None, None, "fas fa-video"
    DOCUMENT = 6, None, None, "fas fa-file-invoice"
    DATA = 7, None, None, "fas fa-database"
    ARCHIVE = 8, None, None, "fas fa-file-zipper"
    EXECUTABLE = 9, None, None, "fas fa-file-binary"
    GENERIC = 99, None, None, "fas fa-file"


@builtin_enum(EnumType.FILE_FORMAT)
class FileFormat(Enum):  # :FileFormats
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


@builtin_node(NodeType.FILE)
class File(Resource):
    """
    A File stored somewhere.
    """

    parent: Optional["Space"] = builtin_property_parent()
    type: FileType = builtin_property(100, is_repr=True)

    # meta
    mime_type: str | None = builtin_property(120, is_repr=True)
    format: FileFormat | None = builtin_property(121, is_repr=True)
    size: int | None = builtin_property(122, primitive_type=PrimitiveType.SINT64, is_repr=True)
    sha256: str | None = builtin_property(123)
    width: int | None = builtin_property(124)
    height: int | None = builtin_property(125)
    aspect_ratio: float | None = builtin_property(126)
    codec: str | None = builtin_property(127)
    duration: Optional[timedelta] = builtin_property(128)

    # content
    url: str | None = builtin_property(130, is_repr=True)  # if external
    content_url: str | None = builtin_property(131)  # if external
    thumbnail_url: str | None = builtin_property(132)  # if external
    favicon_url: str | None = builtin_property(133)
    thumbnail_width: int | None = builtin_property(134)
    thumbnail_height: int | None = builtin_property(135)
    content: bytes | None = builtin_property(136)

    # cached content
    _original: Optional["File"] = builtin_property_runtime(default=None)  # if converted
    _cached_get_url: Optional[str] = builtin_property_runtime(default=None)
    _cached_tmp_path: Optional[str] = builtin_property_runtime(default=None)
    _cached_content: Optional[bytes] = builtin_property_runtime(default=None)
    _cached_image: Optional[Image.Image] = builtin_property_runtime(default=None)

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
        """Uploads the file to the space (if not already uploaded)."""
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
        await download_file_batch([self], include_content=include_content, session=self._session)
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
                tmp_file.write(self.read_content())
            assert isinstance(tmp_file.name, str), f"no path in tmp file {tmp_file!r} for {self!r}"
            self._cached_tmp_path = tmp_file.name
        return self._cached_tmp_path

    def read_content(self) -> bytes:
        """The full file content."""
        if self.content is not None:
            return self.content
        elif self._cached_content is not None:
            return self._cached_content
        else:
            raise ValueError(f"content not ready for {self!r}")

    def read_content_b64(self) -> str:
        """The full file content as base64."""
        return base64.b64encode(self.read_content()).decode("utf-8")

    def read_url(self) -> str:
        """The URL to read the file from."""
        if self.url is not None:
            return self.url
        elif self._cached_get_url is not None:
            return self._cached_get_url
        elif self.content is not None:
            return f"data:{self.mime_type};base64,{self.read_content_b64()}"
        else:
            raise ValueError(f"read URL not available for {self!r}")

    def _clear_cache(self):
        """Clears the cached content and URL."""
        self._cached_content = None
        self._cached_get_url = None

    #
    # Text content
    #

    @property
    def text(self) -> str:
        """Gets the text content of the file."""
        if self.type == FileType.TEXT or self.type == FileType.CODE:
            return self.read_content().decode()
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
                self._cached_image = Image.open(io.BytesIO(self.read_content()))
            return self._cached_image
        else:
            raise ValueError(f"cannot get image content of {self!r}")


@tracer.start_as_current_span("file.upload_batch")
async def upload_file_batch(
    files: list[File], file_contents: list[bytes], session: "Session | None" = None
):
    """Uploads the given Files to their Space."""
    raise NotImplementedError


@tracer.start_as_current_span("file.download_batch")
async def download_file_batch(
    file_refs: Sequence[File],
    *,
    include_content: bool | Collection[File],
    session: "Session | None" = None,
) -> list[File]:
    """Downloads the given Files from their Space."""
    raise NotImplementedError


FileIn = Union[bytes, Image.Image]

import hashlib
from datetime import datetime
from typing import TYPE_CHECKING, Literal, Optional, Union, assert_never, overload, override

import aiohttp
import structlog
from opentelemetry import trace

from bench.language.bench import Drive
from bench.language.const import (
    EnumType,
    NodeType,
    PrimitiveType,
    StructType,
    active_session,
    enum_,
)
from bench.language.node import (
    BuiltinObject,
    NodeReference,
    NodeReferenceBase,
    RemoteNode,
    Struct,
    local_node_,
    object_component,
    struct_,
)
from bench.language.property import (
    Property,
    p_internal,
    p_node_parent,
    p_regular,
    p_runtime,
    p_system,
)
from bench.language.validation import TITLE_CONSTRAINT, ValidationHandler, constraint
from bench.proto.wire import (
    DownloadFilesRequest,
    FileData,
    FileInfoData,
    FileReferenceData,
    UploadFilesRequest,
)
from bench.utils.func import IdEnum
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from magika import Magika

    from bench.language import Block, Package, Session

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
    EXTERNAL = 3


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
    TXT = 1000
    MARKDOWN = 1001
    RTF = 1002
    INI = 1003
    LOG = 1004

    # code
    PYTHON = 2000
    JAVASCRIPT = 2001
    TYPESCRIPT = 2002
    GO = 2003
    C = 2004
    CPP = 2005
    OBJECTIVE_C = 2006
    SWIFT = 2007
    RUBY = 2008
    PHP = 2009
    HTML = 2010
    CSS = 2011
    JAVA = 2012
    KOTLIN = 2013
    RUST = 2014
    SCALA = 2015
    SHELL = 2016
    SQL = 2017
    POWERSHELL = 2018
    ASSEMBLY = 2019
    LATEX = 2020

    # image
    JPEG = 3000
    PNG = 3001
    GIF = 3002
    BMP = 3003
    TIFF = 3004
    WEBP = 3005
    SVG = 3006
    ICO = 3007
    PSD = 3008
    AI = 3009
    EPS = 3010
    RAW = 3011

    # audio
    MP3 = 4000
    WAV = 4001
    FLAC = 4002
    AAC = 4003
    OGG = 4004
    M4A = 4005
    WMA = 4006

    # video
    MP4 = 5000
    WEBM = 5001
    AVI = 5002
    MOV = 5003
    WMV = 5004
    FLV = 5005
    MKV = 5006

    # document
    PDF = 6000
    DOCX = 6001
    PPTX = 6002
    ODT = 6003
    XLSX = 6004
    ODS = 6005
    EPUB = 6006
    MOBI = 6007
    CHM = 6008
    DOC = 6009
    XLS = 6010
    PPT = 6011

    # data
    JSON = 7000
    YAML = 7001
    CSV = 7002
    XML = 7003
    TOML = 7004
    SQLITE = 7100
    PARQUET = 7101
    AVRO = 7102
    PROTOBUF = 7103

    # archive
    ZIP = 8000
    RAR = 8001
    TAR = 8002
    SEVENZIP = 8003
    CAB = 8004
    GZIP = 8005
    BZIP2 = 8006
    XZ = 8007

    # executable
    EXE = 9000
    APP_IMAGE = 9001
    APK = 9002
    DMG = 9003
    JAR = 9004
    MSI = 9005
    DEB = 9006
    RPM = 9007

    @property
    def coarse_type(self) -> FileType:
        return FileType(self.value // 1000)

    @property
    def extension(self) -> str | None:
        return EXTENSION_BY_FORMAT.get(self)

    @property
    def mime_type(self) -> str | None:
        return MIME_TYPE_BY_FORMAT.get(self)


# NOTE :Cleanup: auto-copy file format mappings into bench-web?
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
    "c": FileFormat.C,
    "cpp": FileFormat.CPP,
    "cxx": FileFormat.CPP,
    "h": FileFormat.C,
    "hpp": FileFormat.CPP,
    "m": FileFormat.OBJECTIVE_C,
    "swift": FileFormat.SWIFT,
    "rb": FileFormat.RUBY,
    "php": FileFormat.PHP,
    "html": FileFormat.HTML,
    "htm": FileFormat.HTML,
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
    "psd": FileFormat.PSD,
    "ai": FileFormat.AI,
    "eps": FileFormat.EPS,
    "raw": FileFormat.RAW,
    "cr2": FileFormat.RAW,
    "nef": FileFormat.RAW,
    "arw": FileFormat.RAW,
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
    # data
    "json": FileFormat.JSON,
    "yaml": FileFormat.YAML,
    "yml": FileFormat.YAML,
    "csv": FileFormat.CSV,
    "sqlite": FileFormat.SQLITE,
    "db": FileFormat.SQLITE,
    "parquet": FileFormat.PARQUET,
    "avro": FileFormat.AVRO,
    "proto": FileFormat.PROTOBUF,
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
EXTENSION_BY_FORMAT: dict[FileFormat, str] = {v: k for k, v in FILE_FORMAT_BY_EXTENSION.items()}
FORMAT_BY_MIME_TYPE = {
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
    "text/x-c": FileFormat.C,
    "text/x-csrc": FileFormat.C,
    "text/x-c++": FileFormat.CPP,
    "text/x-c++src": FileFormat.CPP,
    "text/x-objective-c": FileFormat.OBJECTIVE_C,
    "text/x-swift": FileFormat.SWIFT,
    "text/x-ruby": FileFormat.RUBY,
    "application/x-ruby": FileFormat.RUBY,
    "text/x-php": FileFormat.PHP,
    "application/x-httpd-php": FileFormat.PHP,
    "text/html": FileFormat.HTML,
    "application/xhtml+xml": FileFormat.HTML,
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
    "image/vnd.adobe.photoshop": FileFormat.PSD,
    "application/x-photoshop": FileFormat.PSD,
    "image/x-photoshop": FileFormat.PSD,
    "application/postscript": FileFormat.EPS,
    "image/x-eps": FileFormat.EPS,
    "image/x-adobe-dng": FileFormat.RAW,
    "image/x-canon-cr2": FileFormat.RAW,
    "image/x-nikon-nef": FileFormat.RAW,
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
    # data
    "application/json": FileFormat.JSON,
    "application/yaml": FileFormat.YAML,
    "text/yaml": FileFormat.YAML,
    "text/csv": FileFormat.CSV,
    "text/x-csv": FileFormat.CSV,
    "application/x-sqlite3": FileFormat.SQLITE,
    "application/vnd.sqlite3": FileFormat.SQLITE,
    "application/vnd.apache.parquet": FileFormat.PARQUET,
    "avro/binary": FileFormat.AVRO,
    "application/x-protobuf": FileFormat.PROTOBUF,
    "application/protobuf": FileFormat.PROTOBUF,
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
MIME_TYPE_BY_FORMAT: dict[FileFormat, str] = {v: k for k, v in FORMAT_BY_MIME_TYPE.items()}


@object_component()
class FileInfoBase(BuiltinObject):
    """
    Base class for file info.
    """

    # content
    kind: FileKind = p_internal(40)
    title: str = p_regular(41, constraint=TITLE_CONSTRAINT)
    drive: Drive | None = p_regular(
        42, references=NodeType.DRIVE, require=False, array=False, same_bench=True
    )
    if TYPE_CHECKING:
        drive_ptr: Optional[NodeReference] = None
    url: Optional[str] = p_regular(43, default=None)  # if external
    inline_content: Optional[bytes] = p_regular(
        44, default=None, constraint=constraint(min_length=1)
    )
    ...  # thumbnail/preview/...?

    # common meta
    coarse_type: FileType = p_internal(50)
    mime_type: str | None = p_internal(51, constraint=MIME_TYPE_CONSTRAINT)
    format: FileFormat | None = p_internal(52, default=None)
    size: int = p_internal(
        53, primitive_type=PrimitiveType.INT64, constraint=constraint(min_value=0)
    )
    sha256: str | None = p_internal(54, constraint=SHA256_CONSTRAINT)

    # multimedia
    width: Optional[int] = p_internal(55, default=None)
    height: Optional[int] = p_internal(56, default=None)
    aspect_ratio: Optional[float] = p_internal(57, default=None)
    codec: Optional[str] = p_internal(58, default=None)
    duration: Optional[float] = p_internal(60, default=None)
    bitrate: Optional[int] = p_internal(61, default=None)
    channels: Optional[int] = p_internal(62, default=None)
    sample_rate: Optional[int] = p_internal(63, default=None)


@struct_(StructType.FILE_INFO)
class FileInfo(Struct[FileInfoData], FileInfoBase):
    """
    File metadata.
    """

    ...


@local_node_(NodeType.FILE, indexes=(("drive_id", "sha256"),))
class File(RemoteNode[FileData], FileInfoBase):
    """
    A file stored somewhere (like a Drive, orexternally).
    De-duplicated so that there's only one File per unique file content for our own files.
    """

    parent: Union["Package", "Block", None] = p_node_parent(
        4, NodeType.PACKAGE, NodeType.BLOCK, is_system=True
    )

    # meta
    retention: FileRetentionMode | None = p_system(
        30, default=FileRetentionMode.AUTOMATIC, default_sql=None
    )
    expires_at: Optional[datetime] = p_system(31)

    # content/info
    # ...FileInfoBase[40-69]

    # cached content
    _cached_get_url: Optional[str] = p_runtime(default=None)
    _cached_content: Optional[bytes] = p_runtime(default=None)

    def to_ref(self) -> "FileReference":
        """Gets a reference to this file."""
        return FileReference._ref_from_node(self)

    def _to_ref_data(self) -> FileReferenceData:
        """Gets a data reference to this file."""
        return FileReference._ref_from_node(self)._to_data()

    #
    # Generic content
    #

    @property
    def content(self) -> bytes:
        """The file content."""
        if self.inline_content is not None:
            return self.inline_content
        elif self._cached_content is not None:
            return self._cached_content
        else:
            raise ValueError(f"content not ready for {self!r}")

    def clear_cache(self):
        """Clears the cached content and URL."""
        self._cached_content = None
        self._cached_get_url = None

    @overload
    async def download(self, *, include_content: Literal[True] = True) -> bytes: ...
    @overload
    async def download(self, *, include_content: Literal[False] = False) -> str: ...
    async def download(self, *, include_content: bool = True) -> Union[bytes, str]:
        """Downloads the file from the host."""
        await _do_download_files(self.active_session, [self], include_content=include_content)
        if include_content:
            assert self._cached_content is not None, f"content not ready for {self!r}"
            return self._cached_content
        else:
            assert self._cached_get_url is not None, f"content not ready for {self!r}"
            return self._cached_get_url

    @staticmethod
    async def upload(
        file: "FileIn",
        title: str,
        *,
        mime_type: str | None = None,
        coarse_type: FileType | None = None,
        format: FileFormat | None = None,
        parent: "Block | Package | None" = None,
        session: "Session | None" = None,
    ) -> "File":
        return await upload(
            file,
            title,
            mime_type=mime_type,
            coarse_type=coarse_type,
            format=format,
            parent=parent,
            session=session,
        )


@struct_(StructType.FILE_REFERENCE)
class FileReference(
    Struct[FileReferenceData],
    FileInfoBase,
    NodeReferenceBase[File, FileData, "FileReference", FileReferenceData],
):
    """
    A reference to a File. Extends NodeReference with file-specific metadata.
    """  # :RichReferences

    # ...NodeReferenceBase[30-39]

    # content/info
    # ...FileInfoBase[40-69]

    def _validate_component(self, properties: tuple[Property, ...], invalid: ValidationHandler):
        if self.type != NodeType.FILE:
            invalid("type", f"referenced node must be File, got {self.type}", (FileReference.type,))

    @override
    @staticmethod
    def _ref_from_node(node: File) -> "FileReference":
        node_ref = NodeReference._ref_from_node(node)
        kwargs = {}
        for prop in FileInfoBase.__declared_properties__.values():
            if hasattr(node, prop.name):
                kwargs[prop.name] = getattr(node, prop.name)
        return FileReference._clone_ref(FileReference, node_ref, **kwargs)

    @override
    @staticmethod
    def _ref_data_from_node_data(node_data: FileData) -> FileReferenceData:
        node_ref = NodeReference._ref_data_from_node_data(node_data)
        kwargs = {}
        for prop in FileInfoBase.__declared_properties__.values():
            if hasattr(node_data, prop.name):
                kwargs[prop.name] = getattr(node_data, prop.name)
        return FileReference._clone_ref(FileReferenceData, node_ref, **kwargs)


@tracer.start_as_current_span("file.upload_batch")
async def _do_upload_files(session: "Session", files: list[File], file_contents: list[bytes]):
    """Uploads the given Files to their Host."""
    assert len(files) == len(
        file_contents
    ), f"unexpected files: {len(files)} != {len(file_contents)}"
    if not files:
        return

    # get upload URLs
    upload_req = UploadFilesRequest(
        scope=session._get_scope_for_node(files[0]), files=[f._to_data() for f in files]
    )
    upload_rep = await session.host.upload_files(upload_req, metadata=session._rpc_headers)
    assert len(upload_rep.handles) == len(
        files
    ), f"unexpected handles: {len(upload_rep.handles)} != {len(files)}"

    # upload files
    async with aiohttp.ClientSession() as http_session:
        for file, file_content, handle in zip(files, file_contents, upload_rep.handles):
            with tracer.start_as_current_span("file.upload", attributes={"file": repr(file)}):
                assert file.kind in (
                    FileKind.DRIVE,
                    FileKind.DRIVE_INLINE,
                ), f"unexpected file: {file!r}"

                # post file to url
                fields = handle.fields.to_dict()
                form_data = aiohttp.FormData()
                for key, value in fields.items():
                    form_data.add_field(key, value)
                form_data.add_field(
                    "file", file_content, filename=file.title, content_type=file.mime_type
                )
                async with http_session.post(handle.post_url, data=form_data) as resp:
                    resp.raise_for_status()
                file._cached_content = file_content
                file._cached_get_url = handle.get_url


@tracer.start_as_current_span("file.download_batch")
async def _do_download_files(
    session: "Session", file_refs: list[FileReference | File], *, include_content: bool
) -> list[File]:
    """Downloads the given Files from their Host."""
    if not file_refs:
        return []

    from bench.proto.wiring import unpack_object

    # get download URLs
    download_req = DownloadFilesRequest(
        scope=session._get_scope_for_node(session),
        files=[(f.to_ref() if isinstance(f, File) else f)._to_data() for f in file_refs],
    )
    download_rep = await session.host.download_files(download_req, metadata=session._rpc_headers)
    files: list[File] = []
    for file_ref, handle in zip(file_refs, download_rep.handles):
        if isinstance(file_ref, File):
            file = file_ref
        else:
            file = unpack_object(handle.file, supergraph=session._supergraph, expect=File)
        files.append(file)
        file._cached_get_url = handle.get_url

    # download files
    if include_content:
        file_contents: list[bytes] = []
        async with aiohttp.ClientSession() as http_session:
            for file, handle in zip(files, download_rep.handles):
                # get file from url
                with tracer.start_as_current_span("file.download", attributes={"file": repr(file)}):
                    async with http_session.get(handle.get_url) as resp:
                        resp.raise_for_status()
                        file_content = await resp.read()
                    file_contents.append(file_content)
                    file._cached_content = file_content

    return files


FileIn = Union[str, bytes]


async def upload(
    file_in: FileIn,
    title: str,
    *,
    mime_type: str | None = None,
    coarse_type: FileType | None = None,
    format: FileFormat | str | None = None,
    parent: "Block | Package | None" = None,
    drive: "Drive | None" = None,
    session: "Session | None" = None,
) -> "File":
    """Uploads the given file to the given (or current) session."""

    # context
    if session is None:
        session = active_session()
    if parent is None:
        parent = session.package
    if drive is None:
        drive = session.bench.main_drive
        if drive is None:
            raise ValueError(f"no drive to upload file {title!r} to in {session!r}")

    # content
    content: bytes
    if isinstance(file_in, str):
        content = file_in.encode()
    elif isinstance(file_in, (bytes, bytearray, memoryview)):
        content = file_in
    else:
        assert_never(file_in)

    # guess file type
    if isinstance(format, str):
        format = format.lower()
        assert format in FILE_FORMAT_BY_EXTENSION, f"unknown format: {format}"
        format = FILE_FORMAT_BY_EXTENSION.get(format)
    if format is None and title is not None and "." in title:
        format = FILE_FORMAT_BY_EXTENSION.get(title.split(".")[-1])
    if format is not None:
        coarse_type = format.coarse_type
    elif coarse_type is None:
        coarse_type = FileType.GENERIC
    if mime_type is None and format is not None:
        mime_type = format.mime_type

    # guess with magika if needed
    if format is None:
        mime_type, format = detect_file_format(content)
        if format is not None:
            coarse_type = format.coarse_type

    # add extension if needed
    if format is not None and "." not in title and format.extension is not None:
        title = f"{title}.{format.extension}"

    # nocheckin: extract file metadata
    size = len(content)
    sha256 = hashlib.sha256(content).hexdigest()
    file = File(
        parent=parent,
        kind=FileKind.DRIVE,
        drive=drive,
        title=title,
        coarse_type=coarse_type,
        mime_type=mime_type,
        format=format,
        size=size,
        sha256=sha256,
    )

    # upload file, then create in session
    await _do_upload_files(session, [file], [content])
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
        format = FORMAT_BY_MIME_TYPE.get(mime_type)
        return mime_type, format
    else:
        return None, None

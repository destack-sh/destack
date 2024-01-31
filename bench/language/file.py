import hashlib
import io
import mimetypes
from typing import TYPE_CHECKING, BinaryIO, Collection, Optional
from urllib.parse import parse_qs, urlparse, urlunparse

import aiohttp
import requests
import structlog

from bench.language.const import FileStatus, NodeType, StructType, active_session, PrimitiveType
from bench.language.node import (
    Bench,
    Node,
    Struct,
    node,
    node_parent,
    struct,
    struct_internal,
    struct_property,
    struct_runtime,
)
from bench.language.validation import ValidationHandler, on_invalid_raise
from bench.utils.func import _auto_async_to_sync

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)

FILE_HASH_LENGTH = 128  # 512 bits
FILE_MAX_SIZE = 1024 * 1024 * 1024  # 1GB
FILE_MAX_NAME_LENGTH = 256


@node(NodeType.BUCKET_OBJECT)
class BucketObject(Node):
    """The actual file resource ('object') stored in a bucket somewhere. De-duped to 1 per sha512."""

    parent: Bench = node_parent(4, NodeType.BENCH)
    sha512: str = struct_internal(30)
    content_length: int = struct_internal(31, primitive_type=PrimitiveType.INT64)
    content_type: str = struct_internal(32)
    status: FileStatus = struct_internal(33)


@struct(StructType.FILE)
class File(Struct):
    """A reference to a file stored somewhere."""

    sha512: Optional[str] = struct_internal(30)
    content_length: Optional[int] = struct_internal(31)
    content_type: Optional[str] = struct_internal(32)
    name: str = struct_property(33)
    object: Optional[BucketObject] = struct_internal(
        34, require=False, array=False, references=NodeType.BUCKET_OBJECT
    )
    status: FileStatus = struct_internal(35, default=FileStatus.PENDING)

    _cached_bytes: Optional[bytes] = struct_runtime(default=None)

    def __content_str__(self):
        return f"{self.name} {self.status}, {self.content_type}, {self.content_length} bytes"

    def _validate_inner(self, properties: Collection[str], on_invalid: ValidationHandler) -> None:
        if len(self.name) > FILE_MAX_NAME_LENGTH:
            on_invalid(
                self,
                f"{self} name is too long ({len(self.name)} > {FILE_MAX_NAME_LENGTH})",
            )
        if self.content_length > FILE_MAX_SIZE:
            on_invalid(
                self,
                f"{self} is too big ({self.content_length} > {FILE_MAX_SIZE} bytes)",
            )

    @_auto_async_to_sync
    async def download(self) -> bytes:
        """Read the object from the remote storage."""
        get_url = await self.get_url()
        # download file from url
        async with aiohttp.ClientSession() as session:
            async with session.get(get_url) as response:
                logger.debug("file.read", file=self, status=response.status, url=get_url)
                if response.status != 200:
                    raise ValueError(
                        f"unable to download {self}: {response.status} {response.reason}"
                    )
                content = await response.read()
                self._cached_bytes = content
                return content

    @_auto_async_to_sync
    async def get_url(self):
        if self.status != FileStatus.AVAILABLE:
            raise ValueError(f"unable to read {self}")
        return await self.session.host.download_file(self)

    @_auto_async_to_sync
    async def text(self) -> str:
        content = self._cached_bytes or await self.download()
        return content.decode()

    @_auto_async_to_sync
    async def lines(self) -> list[str]:
        content = self._cached_bytes or await self.download()
        return content.decode().splitlines()

    @_auto_async_to_sync
    async def io(self) -> BinaryIO:
        """Get a file-like object for the file."""
        return io.BytesIO(await self.download())

    async def _prep_upload(self) -> Optional[str]:
        """
        Prepare to upload the object to the remote storage (or not if already exists).
        Note that we perform a sleight of hand here: we change the id and status if the object
        already exists under a different id in the object store.
        """
        file, post_url = await self.session.host.prepare_upload_file(self)
        self._set_untracked("id", file.id)
        self._set_untracked("ck", file.ck)
        self.status = file.status
        return post_url

    async def _mark_uploaded(self) -> None:
        """Mark the object as uploaded to the remote storage."""
        await self.session.host.mark_uploaded_file(self)
        self.status = FileStatus.AVAILABLE

    async def _do_upload(self, content: bytes) -> None:
        logger.debug("file.do_upload", file=self)

        # prepare upload (skip if already uploaded)
        post_url = await File._prep_upload(self)
        if post_url is None:
            logger.debug("file.do_upload.skip", file=self)
            return  # already uploaded
        url_parts = urlparse(post_url)
        query_params = parse_qs(url_parts.query)
        form_data = {k: v[0] for k, v in query_params.items()}
        url_main = urlunparse((url_parts.scheme, url_parts.netloc, url_parts.path, "", "", ""))

        # upload (and mark as uploaded in DB)
        response = requests.post(url_main, data=form_data, files={"file": content})
        response.raise_for_status()
        await File._mark_uploaded(self)
        logger.debug("file.do_upload.done", file=self)

    @staticmethod
    @_auto_async_to_sync
    async def from_url(url: str, name: str = None, timeout: int = None) -> "File":
        """Upload a file to file storage."""
        response = requests.get(url, timeout=timeout)
        return await File.from_requests(response, name=name)

    @staticmethod
    @_auto_async_to_sync
    async def from_requests(response: requests.Response, name: str = None) -> "File":
        """Upload a file to file storage."""
        session = active_session()
        response.raise_for_status()
        obj = File(
            sha512=hashlib.sha512(response.content).hexdigest(),
            content_length=int(response.headers["Content-Length"]),
            content_type=response.headers["Content-Type"],
            name=name or response.url.split("/")[-1],
        )
        obj._assign_id_and_ck(session.package.ck)
        obj._validate_self(["name", "content_type", "content_length"], on_invalid=on_invalid_raise)
        await obj._do_upload(response.content)
        return obj

    @staticmethod
    @_auto_async_to_sync
    async def from_file(file: BinaryIO, name: str = None, content_type: str = None) -> "File":
        """Upload a file to file storage."""
        content = file.read()
        content_type = content_type or mimetypes.guess_type(file.name)[0]
        return await File.from_content(name or file.name, content_type, content)

    @staticmethod
    @_auto_async_to_sync
    async def from_content(name: str, content_type: str, content: bytes | BinaryIO) -> "File":
        """Upload a file to file storage."""
        session = active_session()
        if isinstance(content, BinaryIO):
            content = content.read()
        obj = File(
            sha512=hashlib.sha512(content).hexdigest(),
            content_length=len(content),
            content_type=content_type,
            name=name,
            _session=None,
        )
        obj._assign_id_and_ck(session.package.ck)
        obj._validate_self(["name", "content_type", "content_length"], on_invalid=on_invalid_raise)
        await obj._do_upload(content)
        return obj

    @staticmethod
    @_auto_async_to_sync
    async def from_text(name: str, content: str) -> "File":
        """Upload a file to file storage."""
        # append .txt if no extension
        suffix = name.split(".")[-1]
        if suffix not in ("txt", "md", "csv", "rst", "log", "json", "yaml", "yml", "toml"):
            name += ".txt"
        return await File.from_content(name, "text/plain", content.encode())


@struct(StructType.ICON)
class Icon(Struct):
    builtin_name: str = struct_property(30)
    custom_file: Optional[File] = struct_internal(
        31, require=False, array=False, struct=StructType.FILE
    )

import hashlib
import io
import mimetypes
from typing import TYPE_CHECKING, BinaryIO, Collection, Optional
from urllib.parse import parse_qs, urlparse, urlunparse
from uuid import UUID, uuid5

import aiohttp
import requests
import structlog

from bench.language.const import BlobStatus, NodeType, StructType, active_session
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

BLOB_HASH_LENGTH = 128  # 512 bits
BLOB_MAX_SIZE = 1024 * 1024 * 1024  # 1GB
BLOB_MAX_NAME_LENGTH = 256


@node(NodeType.BUCKET_OBJECT, in_package=False)
class BucketObject(Node):
    """The actual file resource ('object') stored in a bucket somewhere. De-duped to 1 per sha512."""

    parent: Bench = node_parent(4, NodeType.BENCH)
    sha512: str = struct_internal(30)
    content_length: int = struct_internal(31)
    content_type: str = struct_internal(32)
    status: BlobStatus = struct_internal(33)


@struct(StructType.BLOB)
class Blob(Struct):
    """A reference to a file stored somewhere."""

    sha512: Optional[str] = struct_internal(30)
    content_length: Optional[int] = struct_internal(31)
    content_type: Optional[str] = struct_internal(32)
    name: str = struct_property(33)
    object: Optional[BucketObject] = struct_internal(
        34, require=False, array=False, references=NodeType.BUCKET_OBJECT
    )
    status: BlobStatus = struct_internal(35, default=BlobStatus.PENDING)

    _cached_bytes: Optional[bytes] = struct_runtime(default=None)

    def __str__(self):
        return f"{self.name} ({self.status}, {self.content_type}, {self.content_length} bytes)"

    @property
    def is_external(self) -> None:
        return self.object_ptr is None

    def _validate_inner(self, properties: Collection[str], on_invalid: ValidationHandler) -> None:
        if len(self.name) > BLOB_MAX_NAME_LENGTH:
            on_invalid(
                self,
                f"{self} name is too long ({len(self.name)} > {BLOB_MAX_NAME_LENGTH})",
            )
        if self.content_length > BLOB_MAX_SIZE:
            on_invalid(
                self,
                f"{self} is too big ({self.content_length} > {BLOB_MAX_SIZE} bytes)",
            )

    @_auto_async_to_sync
    async def download(self) -> bytes:
        """Read the object from the remote storage."""
        get_url = await self.get_url()
        # download file from url
        async with aiohttp.ClientSession() as session:
            async with session.get(get_url) as response:
                logger.debug("blob.read", blob=self, status=response.status, url=get_url)
                if response.status != 200:
                    raise ValueError(
                        f"unable to download {self}: {response.status} {response.reason}"
                    )
                content = await response.read()
                self._cached_bytes = content
                return content

    @_auto_async_to_sync
    async def get_url(self):
        if self.status != BlobStatus.AVAILABLE:
            raise ValueError(f"unable to read {self}")
        return await self.session.host.download_blob(self)

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
        """Get a file-like object for the blob."""
        return io.BytesIO(await self.download())

    async def _prep_upload(self) -> Optional[str]:
        """
        Prepare to upload the object to the remote storage (or not if already exists).
        Note that we perform a sleight of hand here: we change the id and status if the object
        already exists under a different id in the object store.
        """
        blob, post_url = await self.session.host.prepare_upload_blob(self)
        self._set_untracked("id", blob.id)
        self._set_untracked("ck", blob.ck)
        self.status = blob.status
        return post_url

    async def _mark_uploaded(self) -> None:
        """Mark the object as uploaded to the remote storage."""
        await self.session.host.mark_uploaded_blob(self)
        self.status = BlobStatus.AVAILABLE

    async def _do_upload(self, content: bytes) -> None:
        logger.debug("blob.do_upload", blob=self)

        # prepare upload (skip if already uploaded)
        post_url = await Blob._prep_upload(self)
        if post_url is None:
            logger.debug("blob.do_upload.skip", blob=self)
            return  # already uploaded
        url_parts = urlparse(post_url)
        query_params = parse_qs(url_parts.query)
        form_data = {k: v[0] for k, v in query_params.items()}
        url_main = urlunparse((url_parts.scheme, url_parts.netloc, url_parts.path, "", "", ""))

        # upload (and mark as uploaded in DB)
        response = requests.post(url_main, data=form_data, files={"file": content})
        response.raise_for_status()
        await Blob._mark_uploaded(self)
        logger.debug("blob.do_upload.done", blob=self)

    @staticmethod
    @_auto_async_to_sync
    async def from_url(url: str, name: str = None, timeout: int = None) -> "Blob":
        """Upload a file to blob storage."""
        response = requests.get(url, timeout=timeout)
        return await Blob.from_requests(response, name=name)

    @staticmethod
    @_auto_async_to_sync
    async def from_requests(response: requests.Response, name: str = None) -> "Blob":
        """Upload a file to blob storage."""
        session = active_session()
        response.raise_for_status()
        obj = Blob(
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
    async def from_file(file: BinaryIO, name: str = None, content_type: str = None) -> "Blob":
        """Upload a file to blob storage."""
        content = file.read()
        content_type = content_type or mimetypes.guess_type(file.name)[0]
        return await Blob.from_content(name or file.name, content_type, content)

    @staticmethod
    @_auto_async_to_sync
    async def from_content(name: str, content_type: str, content: bytes | BinaryIO) -> "Blob":
        """Upload a file to blob storage."""
        session = active_session()
        if isinstance(content, BinaryIO):
            content = content.read()
        obj = Blob(
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
    async def from_text(name: str, content: str) -> "Blob":
        """Upload a file to blob storage."""
        # append .txt if no extension
        suffix = name.split(".")[-1]
        if suffix not in ("txt", "md", "csv", "rst", "log", "json", "yaml", "yml", "toml"):
            name += ".txt"
        return await Blob.from_content(name, "text/plain", content.encode())

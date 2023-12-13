import hashlib
import io
import mimetypes
import typing
from typing import Optional
from urllib.parse import parse_qs, urlparse, urlunparse
from uuid import UUID, uuid5

import aiohttp
import requests
import structlog

from bench.language.builtin import _auto_async_to_sync, active_session
from bench.language.const import BlobStatus, NodeType
from bench.language.module import Module, Node, node, struct_internal, struct_runtime
from bench.language.validation import ValidationHandler, on_invalid_raise

logger = structlog.get_logger(__name__)

BLOB_HASH_LENGTH = 128  # 512 bits
BLOB_MAX_SIZE = 1024 * 1024 * 1024  # 1GB
BLOB_MAX_NAME_LENGTH = 256


@node(NodeType.BLOB)
class Blob(Node):
    """
    A proxy to a remotely stored object behaving like a Python file on demand.
    :BlobType
    """

    sha512: str = struct_internal()
    content_length: int = struct_internal()
    content_type: str = struct_internal()
    name: str = struct_internal()
    status: BlobStatus = struct_internal(default=BlobStatus.PREPARED)

    _cached_bytes: Optional[bytes] = struct_runtime(default=None)

    def __str__(self):
        return f"{self.id} {self.name} ({self.status}, {self.content_type}, {self.content_length} bytes)"

    def __repr__(self):
        return f"<Blob {self}>"

    def __getitem__(self, item):
        return self.__dict__[item]

    def _validate_inner(
        self, properties: typing.Collection[str], on_invalid: ValidationHandler
    ) -> None:
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
        return await self.session._runtime.download_blob(self)

    @_auto_async_to_sync
    async def text(self) -> str:
        content = self._cached_bytes or await self.download()
        return content.decode()

    @_auto_async_to_sync
    async def lines(self) -> list[str]:
        content = self._cached_bytes or await self.download()
        return content.decode().splitlines()

    @_auto_async_to_sync
    async def io(self) -> typing.BinaryIO:
        """Get a file-like object for the blob."""
        return io.BytesIO(await self.download())

    async def _prep_upload(self) -> Optional[str]:
        """
        Prepare to upload the object to the remote storage (or not if already exists).
        Note that we perform a sleight of hand here: we change the id and status if the object
        already exists under a different id in the object store.
        """
        blob, post_url = await self.session._runtime.prepare_upload_blob(self)
        self._set_untracked("id", blob.id)
        self._set_untracked("ck", blob.ck)
        self.status = blob.status
        return post_url

    async def _mark_uploaded(self) -> None:
        """Mark the object as uploaded to the remote storage."""
        await self.session._runtime.mark_uploaded_blob(self)
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

    def _assign_id_and_ck(self, module_ck: UUID):
        # derive ck from module ck and sha512 (and id==ck because detached)
        self._set_untracked("ck", uuid5(module_ck, self.sha512))
        self._set_untracked("id", self.ck)

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
        obj._assign_id_and_ck(session.module.ck)
        obj._validate_self(["name", "content_type", "content_length"], on_invalid=on_invalid_raise)
        await obj._do_upload(response.content)
        return obj

    @staticmethod
    @_auto_async_to_sync
    async def from_file(
        file: typing.BinaryIO, name: str = None, content_type: str = None
    ) -> "Blob":
        """Upload a file to blob storage."""
        content = file.read()
        content_type = content_type or mimetypes.guess_type(file.name)[0]
        return await Blob.from_content(name or file.name, content_type, content)

    @staticmethod
    @_auto_async_to_sync
    async def from_content(
        name: str, content_type: str, content: bytes | typing.BinaryIO
    ) -> "Blob":
        """Upload a file to blob storage."""
        session = active_session()
        if isinstance(content, typing.BinaryIO):
            content = content.read()
        obj = Blob(
            sha512=hashlib.sha512(content).hexdigest(),
            content_length=len(content),
            content_type=content_type,
            name=name,
            _session=None,
        )
        obj._assign_id_and_ck(session.module.ck)
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


class Blobs:
    """Convenience wrapper around a module's blob storage."""

    def __init__(self, module: Module):
        self.module = module

    def __str__(self):
        return f"{self.module} storage"

    def __repr__(self):
        return f"<Storage {self}>"

    @_auto_async_to_sync
    async def upload(
        self, file: typing.BinaryIO, name: str = None, content_type: str = None
    ) -> Blob:
        """Upload a file to blob storage."""
        return await Blob.from_file(file, name=name, content_type=content_type)

    @_auto_async_to_sync
    async def upload_content(
        self, name: str, content_type: str, content: bytes | typing.BinaryIO
    ) -> Blob:
        """Upload a file to blob storage."""
        return await Blob.from_content(name, content_type, content)

    @_auto_async_to_sync
    async def upload_text(self, name: str, content: str) -> Blob:
        """Upload a file to blob storage."""
        return await Blob.from_text(name, content)

    @_auto_async_to_sync
    async def upload_from_url(self, url: str, name: str = None, timeout: int = None) -> Blob:
        """Upload a file to blob storage."""
        return await Blob.from_url(url, self.module.session, name=name, timeout=timeout)

    @_auto_async_to_sync
    async def upload_from_requests(self, response: requests.Response, name: str = None) -> Blob:
        """Upload a file to blob storage."""
        return await Blob.from_requests(response, self.module.session, name=name)

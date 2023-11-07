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
from asgiref.sync import async_to_sync

from bench.language.builtin import active_session
from bench.language.const import MNT, BlobStatus
from bench.language.module import Module, Node, ninternal, node, nproperty, nruntime
from bench.language.validation import ValidationHandler, on_issue_raise

if typing.TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)

BLOB_HASH_LENGTH = 128  # 512 bits
BLOB_MAX_SIZE = 1024 * 1024 * 1024  # 1GB


@node(MNT.BLOB)
class Blob(Node):
    """
    A proxy to a remotely stored object behaving like a Python file on demand.
    :BlobType
    """

    sha512: str = ninternal()
    content_length: int = ninternal()
    content_type: str = ninternal()
    name: str = ninternal()
    status: BlobStatus = ninternal(default=BlobStatus.PREPARED)

    _cached_bytes: Optional[bytes] = nruntime(default=None)

    def __str__(self):
        return f"{self.id} {self.name} ({self.status}, {self.content_type}, {self.content_length} bytes)"

    def __repr__(self):
        return f"<Blob {self}>"

    def __getitem__(self, item):
        return self.__dict__[item]

    def _validate_inner(self, properties: set[str], on_issue: ValidationHandler) -> None:
        if self.content_length > BLOB_MAX_SIZE:
            on_issue(self, f"{self} is too big ({self.content_length} > {BLOB_MAX_SIZE} bytes)")

    async def aread(self, timeout: float = 1) -> bytes:
        """Read the object from the remote storage."""
        get_url = await self.aget_url(timeout)
        # download file from url
        async with aiohttp.ClientSession() as session:
            async with session.get(get_url) as response:
                logger.debug("blob.read", object=self, status=response.status, url=get_url)
                if response.status != 200:
                    raise ValueError(
                        f"unable to download {self}: {response.status} {response.reason}"
                    )
                content = await response.read()
                self._cached_bytes = content
                return content

    async def aget_url(self, timeout: float = 10):
        from bench.language import wire
        from bench.msg.core import NMessage, request
        from bench.msg.messages import NMessageType, RepReadBlobPayload, ReqReadBlobPayload

        if self.status != BlobStatus.AVAILABLE:
            raise ValueError(f"unable to read {self}")
        rep: NMessage[RepReadBlobPayload] = await request(
            NMessageType.READ_BLOB,
            ReqReadBlobPayload(blobs=[wire.pack_data(self)]),
            reply_t=RepReadBlobPayload,
            timeout=timeout,
        )
        get_url = rep.p.get_urls[0] if rep.p.get_urls else None
        if get_url is None:
            raise ValueError(f"unable to GET {self}")
        return get_url

    async def areadtext(self) -> str:
        content = self._cached_bytes or await self.aread()
        return content.decode()

    async def areadlines(self) -> list[str]:
        content = self._cached_bytes or await self.aread()
        return content.decode().splitlines()

    def get_url(self, timeout: float = 1) -> str:
        return async_to_sync(self.aread_url)(timeout=timeout)

    def read(self, timeout: float = 1) -> bytes:
        """Read the object from the remote storage."""
        content = self._cached_bytes or async_to_sync(self.aread)(timeout=timeout)
        return content

    def readtext(self) -> str:
        return self.read().decode()

    def readlines(self) -> list[str]:
        return self.read().decode().splitlines()

    def io(self) -> typing.BinaryIO:
        """Get a file-like object for the blob."""
        return io.BytesIO(self.read())

    async def aio(self) -> typing.BinaryIO:
        """Get a file-like object for the blob."""
        return io.BytesIO(await self.aread())

    async def _prep_upload(self) -> Optional[str]:
        """
        Prepare to upload the object to the remote storage (or not if already exists).
        Note that we perform a sleight of hand here: we change the id and status if the object
        already exists under a different id in the object store.
        """
        from bench.language import wire
        from bench.msg.core import NMessage, request
        from bench.msg.messages import NMessageType, RepWriteObjectPayload, ReqWriteBlobPayload

        if not self.id:
            self._assign_id_and_ck(self.session.module.ck)

        logger.debug("blob.prepare_upload", object=self)
        # first get POST url to upload the object
        rep: NMessage[RepWriteObjectPayload] = await request(
            NMessageType.WRITE_BLOB,
            ReqWriteBlobPayload(module_id=self.session.module.id, blobs=[wire.pack_data(self)]),
            reply_t=RepWriteObjectPayload,
        )
        blob = rep.p.blobs[0]
        self._set_untracked("id", blob.id)
        if blob.status == BlobStatus.AVAILABLE:
            # already uploaded
            self.status = BlobStatus.AVAILABLE
            return None
        else:
            post_url = rep.p.post_urls[0]
            self.status = BlobStatus.UPLOADING
            return post_url

    async def _mark_uploaded(self) -> None:
        """Mark the object as uploaded to the remote storage."""
        from bench.language import wire
        from bench.msg.core import NMessage, request
        from bench.msg.messages import (
            NMessageType,
            RepMarkUploadedBlobPayload,
            ReqMarkUploadedBlobPayload,
        )

        logger.debug("blob.mark_uploaded", object=self)
        rep: NMessage[RepMarkUploadedBlobPayload] = await request(
            NMessageType.MARK_UPLOADED_BLOB,
            ReqMarkUploadedBlobPayload(blobs=[wire.pack_data(self)]),
            reply_t=RepMarkUploadedBlobPayload,
        )
        if not rep.p.success:
            raise ValueError(f"unable to mark uploaded {self}")
        self.status = BlobStatus.AVAILABLE

    async def _do_upload(self, content: bytes) -> None:
        logger.debug("blob.do_upload", object=self)

        # prepare upload (skip if already uploaded)
        post_url = await Blob._prep_upload(self)
        if post_url is None:
            logger.debug("blob.do_upload.skip", object=self)
            return  # already uploaded
        url_parts = urlparse(post_url)
        query_params = parse_qs(url_parts.query)
        form_data = {k: v[0] for k, v in query_params.items()}
        url_main = urlunparse((url_parts.scheme, url_parts.netloc, url_parts.path, "", "", ""))

        # upload (and mark as uploaded in DB)
        response = requests.post(url_main, data=form_data, files={"file": content})
        response.raise_for_status()
        await Blob._mark_uploaded(self)
        logger.debug("blob.do_upload.done", object=self)

    def _assign_id_and_ck(self, module_ck: UUID):
        # derive ck from module ck and sha512 (and id==ck because detached)
        self._set_untracked("ck", uuid5(module_ck, self.sha512))
        self._set_untracked("id", self.ck)

    @staticmethod
    def from_url(url: str, name: str = None, timeout: int = None) -> "Blob":
        """Upload a file to object storage."""
        response = requests.get(url, timeout=timeout)
        return Blob.from_requests(response, name=name)

    @staticmethod
    def from_requests(response: requests.Response, name: str = None) -> "Blob":
        """Upload a file to object storage."""
        session = active_session()
        response.raise_for_status()
        obj = Blob(
            sha512=hashlib.sha512(response.content).hexdigest(),
            content_length=int(response.headers["Content-Length"]),
            content_type=response.headers["Content-Type"],
            name=name or response.url.split("/")[-1],
        )
        obj._assign_id_and_ck(session.module.ck)
        obj._validate_self(["name", "content_type", "content_length"], on_issue=on_issue_raise)
        session.async_to_sync(obj._do_upload)(response.content)
        return obj

    @staticmethod
    def from_file(file: typing.BinaryIO, name: str = None, content_type: str = None) -> "Blob":
        """Upload a file to object storage."""
        content = file.read()
        content_type = content_type or mimetypes.guess_type(file.name)[0]
        return Blob.from_content(name or file.name, content_type, content)

    @staticmethod
    def from_content(name: str, content_type: str, content: bytes | typing.BinaryIO) -> "Blob":
        """Upload a file to object storage."""
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
        obj._validate_self(["name", "content_type", "content_length"], on_issue=on_issue_raise)
        session.async_to_sync(obj._do_upload)(content)
        return obj


class Storage:
    """Convenience wrapper around a module's object storage."""

    def __init__(self, module: Module):
        self.module = module

    def __str__(self):
        return f"{self.module} storage"

    def __repr__(self):
        return f"<Storage {self}>"

    def upload(self, file: typing.BinaryIO, name: str = None, content_type: str = None) -> Blob:
        """Upload a file to object storage."""
        return Blob.from_file(file, name=name, content_type=content_type)

    def upload_from_url(self, url: str, name: str = None, timeout: int = None) -> Blob:
        """Upload a file to object storage."""
        return Blob.from_url(url, self.module.session, name=name, timeout=timeout)

    def upload_from_requests(self, response: requests.Response, name: str = None) -> Blob:
        """Upload a file to object storage."""
        return Blob.from_requests(response, self.module.session, name=name)


SecretValueT = typing.TypeVar("SecretValueT")


@node(MNT.SECRET)
class Secret(Node, typing.Generic[SecretValueT]):
    """A proxy to a remotely stored secret."""

    sha512: str = nproperty()
    value: Optional[SecretValueT] = nruntime(default=None)

    def __str__(self):
        return f"{self.id} ({self.sha512[:8]})"

    def __repr__(self):
        return f"<Secret {self}>"

    async def areveal(self) -> SecretValueT:
        if self.value is not None:
            return self.value

        from bench.language import wire
        from bench.msg import messages
        from bench.msg.core import NMessage, NMessageType, request

        rep: NMessage[messages.RepReadSecretPayload] = await request(
            NMessageType.READ_SECRET,
            messages.ReqReadSecretPayload(secrets=[wire.pack_data(self)]),
            reply_t=messages.RepReadSecretPayload,
            timeout=10,
        )
        if not rep.p.secrets:
            raise ValueError(f"could not reveal {self!r}")
        self.value = rep.p.secrets[0].value
        return self.value

    def reveal(self) -> SecretValueT:
        return async_to_sync(self.areveal)()

import hashlib
import io
import mimetypes
import typing
from typing import Optional
from urllib.parse import parse_qs, urlparse, urlunparse

import aiohttp
import requests
import structlog
from asgiref.sync import async_to_sync

from bench.language.const import RemoteObjectStatus, MNT
from bench.language.issue import ValidationHandler
from bench.language.module import Module, ModuleNode, node, nproperty, nruntime, ninternal
from bench.utils.func import did_you_mean_str

if typing.TYPE_CHECKING:
    from bench.language.session import Session

logger = structlog.get_logger(__name__)

REMOTE_OBJECT_HASH_LENGTH = 128  # 512 bits
REMOTE_OBJECT_MAX_SIZE = 1024 * 1024 * 100  # 100 MB


@node(MNT.RemoteObject)
class RemoteObject(ModuleNode):
    """
    A proxy to a remotely stored object behaving like a Python file on demand.
    :RemoteObjectType
    """

    sha512: str = ninternal()
    content_length: int = ninternal()
    content_type: str = ninternal()
    name: str = ninternal()
    status: RemoteObjectStatus = ninternal(default=RemoteObjectStatus.PREPARED)

    _cached_bytes: Optional[bytes] = nruntime(default=None)

    def __str__(self):
        return f"{self.id} {self.name} ({self.status}, {self.content_type}, {self.content_length} bytes)"

    def __repr__(self):
        return f"<RemoteObject {self}>"

    def __getitem__(self, item):
        return self.__dict__[item]

    def _validate_inner(self, properties: set[str], on_issue: ValidationHandler) -> None:
        if self.content_length > REMOTE_OBJECT_MAX_SIZE:
            on_issue(
                self, f"{self} is too big ({self.content_length} > {REMOTE_OBJECT_MAX_SIZE} bytes)"
            )

    async def aread(self, timeout: float = 1) -> bytes:
        """Read the object from the remote storage."""
        from bench.language import wire
        from bench.msg.core import NMessage, request
        from bench.msg.messages import NMessageType, RepReadObjectPayload, ReqReadObjectPayload

        if self.status != RemoteObjectStatus.AVAILABLE:
            raise ValueError(f"unable to read {self}")
        rep: NMessage[RepReadObjectPayload] = await request(
            NMessageType.READ_OBJECT,
            ReqReadObjectPayload(objects=[wire.pack_data(self)]),
            reply_t=RepReadObjectPayload,
            timeout=timeout,
        )
        get_url = rep.p.get_urls[0]
        if get_url is None:
            raise ValueError(f"unable to GET {self}")
        # download file from url
        async with aiohttp.ClientSession() as session:
            async with session.get(get_url) as response:
                if response.status != 200:
                    raise ValueError(
                        f"unable to download {self}: {response.status} {response.reason}"
                    )
                content = await response.read()
                self._cached_bytes = content
                return content

    async def areadtext(self) -> str:
        content = self._cached_bytes or await self.aread()
        return content.decode()

    async def areadlines(self) -> list[str]:
        content = self._cached_bytes or await self.aread()
        return content.decode().splitlines()

    def read(self, timeout: float = 1) -> bytes:
        """Read the object from the remote storage."""
        content = self._cached_bytes or async_to_sync(self.aread)(timeout=timeout)
        return content

    def readtext(self) -> str:
        return self.read().decode()

    def readlines(self) -> list[str]:
        return self.read().decode().splitlines()

    def io(self) -> typing.BinaryIO:
        """Get a file-like object for the object."""
        return io.BytesIO(self.read())

    async def aio(self) -> typing.BinaryIO:
        """Get a file-like object for the object."""
        return io.BytesIO(await self.aread())

    async def _prep_upload(self) -> Optional[str]:
        """
        Prepare to upload the object to the remote storage (or not if already exists).
        Note that we perform a sleight of hand here: we change the id and status if the object
        already exists under a different id in the object store.
        """
        from bench.language import wire
        from bench.msg.core import NMessage, request
        from bench.msg.messages import NMessageType, RepWriteObjectPayload, ReqWriteObjectPayload

        logger.debug("object.prepare_upload", object=self)
        # first get POST url to upload the object
        rep: NMessage[RepWriteObjectPayload] = await request(
            NMessageType.WRITE_OBJECT,
            ReqWriteObjectPayload(module_id=self.session.module.id, objects=[wire.pack_data(self)]),
            reply_t=RepWriteObjectPayload,
        )
        remote_obj = rep.p.objects[0]
        self.id = remote_obj.id
        if remote_obj.status == RemoteObjectStatus.AVAILABLE:
            # already uploaded
            self.status = RemoteObjectStatus.AVAILABLE
            return None
        else:
            post_url = rep.p.post_urls[0]
            self.status = RemoteObjectStatus.UPLOADING
            return post_url

    async def _mark_uploaded(self) -> None:
        """Mark the object as uploaded to the remote storage."""
        from bench.language import wire
        from bench.msg.core import NMessage, request
        from bench.msg.messages import (
            NMessageType,
            RepMarkUploadedObjectPayload,
            ReqMarkUploadedObjectPayload,
        )

        logger.debug("object.mark_uploaded", object=self)
        rep: NMessage[RepMarkUploadedObjectPayload] = await request(
            NMessageType.MARK_UPLOADED_OBJECT,
            ReqMarkUploadedObjectPayload(objects=[wire.pack_data(self)]),
            reply_t=RepMarkUploadedObjectPayload,
        )
        if not rep.p.success:
            raise ValueError(f"unable to mark uploaded {self}")
        self.status = RemoteObjectStatus.AVAILABLE

    def _do_upload(self, content: bytes) -> None:
        logger.debug("object.do_upload", object=self)

        # prepare upload (skip if already uploaded)
        post_url = self.session.async_to_sync(RemoteObject._prep_upload)(self)
        if post_url is None:
            logger.debug("object.do_upload.skip", object=self)
            return  # already uploaded
        url_parts = urlparse(post_url)
        query_params = parse_qs(url_parts.query)
        form_data = {k: v[0] for k, v in query_params.items()}
        url_main = urlunparse((url_parts.scheme, url_parts.netloc, url_parts.path, "", "", ""))

        # upload (and mark as uploaded in DB)
        response = requests.post(url_main, data=form_data, files={"file": content})
        response.raise_for_status()
        self.session.async_to_sync(RemoteObject._mark_uploaded)(self)
        logger.debug("object.do_upload.done", object=self)

    @staticmethod
    def from_url(
        url: str, session: "Session", name: str = None, timeout: int = None
    ) -> "RemoteObject":
        """Upload a file to object storage."""
        response = requests.get(url, timeout=timeout)
        return RemoteObject.from_requests(response, session, name=name)

    @staticmethod
    def from_requests(
        response: requests.Response, session: "Session", name: str = None
    ) -> "RemoteObject":
        """Upload a file to object storage."""
        response.raise_for_status()
        obj = RemoteObject(
            sha512=hashlib.sha512(response.content).hexdigest(),
            content_length=int(response.headers["Content-Length"]),
            content_type=response.headers["Content-Type"],
            name=name or response.url,
            _session=session,
        )
        obj._validate()
        obj._do_upload(response.content)
        return obj

    @staticmethod
    def from_file(
        file: typing.BinaryIO, name: str = None, content_type: str = None
    ) -> "RemoteObject":
        """Upload a file to object storage."""
        content = file.read()
        content_type = content_type or mimetypes.guess_type(file.name)[0]
        return RemoteObject.from_content(name or file.name, content_type, content)

    @staticmethod
    def from_content(
        name: str, content_type: str, content: bytes | typing.BinaryIO
    ) -> "RemoteObject":
        """Upload a file to object storage."""
        if isinstance(content, typing.BinaryIO):
            content = content.read()
        obj = RemoteObject(
            sha512=hashlib.sha512(content).hexdigest(),
            content_length=len(content),
            content_type=content_type,
            name=name,
            _session=None,
        )
        obj._validate()
        obj._do_upload(content)
        return obj


class Storage:
    """Convenience wrapper around a module's object storage."""

    def __init__(self, module: Module):
        self.module = module

    def __str__(self):
        return f"{self.module} storage"

    def __repr__(self):
        return f"<Storage {self}>"

    def upload(
        self, file: typing.BinaryIO, name: str = None, content_type: str = None
    ) -> RemoteObject:
        """Upload a file to object storage."""
        return RemoteObject.from_file(file, name=name, content_type=content_type)

    def upload_from_url(self, url: str, name: str = None, timeout: int = None) -> RemoteObject:
        """Upload a file to object storage."""
        return RemoteObject.from_url(url, self.module.session, name=name, timeout=timeout)

    def upload_from_requests(self, response: requests.Response, name: str = None) -> RemoteObject:
        """Upload a file to object storage."""
        return RemoteObject.from_requests(response, self.module.session, name=name)


SecretValueT = typing.TypeVar("SecretValueT")


@node(MNT.Secret)
class Secret(ModuleNode, typing.Generic[SecretValueT]):
    """A proxy to a remotely stored secret."""

    sha512: str = nproperty()
    value: Optional[SecretValueT] = nruntime(default=None)

    def __str__(self):
        return f"{self.id} ({self.sha512[:8]})"

    def __repr__(self):
        return f"<Secret {self}>"

    def __getattr__(self, item):
        try:
            return self.__getattribute__(item)
        except AttributeError:
            did_you_mean = did_you_mean_str({"reveal": self.reveal, "areveal": self.areveal}, item)
            raise AttributeError(f"{self} has no attribute {item} ({did_you_mean})")

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
        self.value = rep.p.secrets[0].value
        return self.value

    def reveal(self) -> SecretValueT:
        return async_to_sync(self.areveal)()

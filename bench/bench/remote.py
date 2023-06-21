from __future__ import annotations

import enum
import typing
import uuid
from dataclasses import field
from typing import Optional
from uuid import UUID

import aiohttp
from asgiref.sync import async_to_sync

from bench.bench.core import HasSession, node
from bench.msg.core import NMessage, NMessageType, request
from bench.msg.messages import (
    RepReadObjectPayload,
    RepReadSecretPayload,
    ReqReadObjectPayload,
    ReqReadSecretPayload,
)
from bench.utils.utils import required_field


@node
class RemoteObject(HasSession):
    """
    A proxy to a remotely stored object behaving like a Python file on demand.
    :RemoteObjectType
    """

    id: UUID = field(default_factory=uuid.uuid4)
    sha512: str = required_field()
    content_length: int = required_field()
    content_type: str = required_field()
    name: str = required_field()
    status: RemoteObjectStatus = required_field()

    def __str__(self):
        return f"{self.id} {self.name} ({self.status}, {self.content_type}, {self.content_length} bytes)"

    def __repr__(self):
        return f"<RemoteObject {self}>"

    def __getitem__(self, item):
        return self.__dict__[item]

    async def aread(self, timeout: float = 1) -> bytes:
        """Read the object from the remote storage."""
        if self.status != RemoteObjectStatus.AVAILABLE:
            raise ValueError(f"unable to read {self}")
        rep: NMessage[RepReadObjectPayload] = await request(
            NMessageType.REQUEST_READ_OBJECT,
            ReqReadObjectPayload(objects=[wire.pack_data(self)]),
            reply_t=RepReadObjectPayload,
            timeout=timeout,
        )
        get_url = rep.p.get_urls[0]
        if get_url is None:
            raise ValueError(f"unable to get {self}")
        # download file from url
        async with aiohttp.ClientSession() as session:
            async with session.get(get_url) as response:
                if response.status != 200:
                    raise ValueError(f"unable to download {self}")
                return await response.read()

    async def areadtext(self) -> str:
        return (await self.aread()).decode()

    async def areadlines(self) -> list[str]:
        return (await self.aread()).decode().splitlines()

    def read(self, timeout: float = 1) -> bytes:
        """Read the object from the remote storage."""
        return async_to_sync(self.aread)(timeout=timeout)

    def readtext(self) -> str:
        return self.read().decode()

    def readlines(self) -> list[str]:
        return self.read().decode().splitlines()


SecretValueT = typing.TypeVar("SecretValueT")


@node
class Secret(HasSession, typing.Generic[SecretValueT]):
    """A proxy to a remotely stored secret."""

    id: UUID = field(default_factory=uuid.uuid4)
    sha512: str = required_field()
    value: Optional[SecretValueT] = None

    def __str__(self):
        return f"{self.id} ({self.sha512})"

    def __repr__(self):
        return f"<Secret {self}>"

    async def areveal(self) -> SecretValueT:
        if self.value is not None:
            return self.value
        from bench.bench import wire

        rep: NMessage[RepReadSecretPayload] = await request(
            NMessageType.REQUEST_READ_SECRET,
            ReqReadSecretPayload(secrets=[wire.pack_data(self)]),
            reply_t=RepReadSecretPayload,
            timeout=10,
        )
        self.value = rep.p.secrets[0].value
        return self.value

    def reveal(self) -> SecretValueT:
        return async_to_sync(self.areveal)()


class RemoteObjectStatus(enum.StrEnum):
    PREPARED = "prepared"
    UPLOADING = "uploading"
    AVAILABLE = "available"

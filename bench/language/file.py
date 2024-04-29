import io
from typing import TYPE_CHECKING, BinaryIO, Optional

import aiohttp
import structlog

from bench.language.const import EnumType, NodeType, StructType, enum_
from bench.language.node import Struct, struct
from bench.language.property import Property, p_internal, p_regular, p_runtime
from bench.language.validation import ValidationHandler, validate_name
from bench.utils.func import IdEnum, _auto_async_to_sync
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from bench.language import Color, FileContent

logger = structlog.get_logger(__name__)

FILE_HASH_LENGTH = 128  # 512 bits
FILE_MAX_SIZE = 1024 * 1024 * 1024  # 1GB
FILE_MAX_NAME_LENGTH = 256
GLOBAL_PROJECT_BUCKET_NAME = get_from_env("GLOBAL_PROJECT_BUCKET_NAME", optional=True)

# pyright: reportIncompatibleVariableOverride=false,reportIncompatibleMethodOverride=false


@struct(StructType.FILE, inline=True)
class File(Struct):
    """A reference to a file stored somewhere."""

    type: Optional[str] = p_internal(31)
    name: str = p_regular(33, validate=validate_name)
    size: Optional[int] = p_internal(34)
    sha512: Optional[str] = p_internal(35)
    content: Optional["FileContent"] = p_internal(
        36, require=False, array=False, references=NodeType.FILE_CONTENT
    )
    external_url: Optional[str] = p_internal(37)

    _cached_bytes: Optional[bytes] = p_runtime(default=None)

    def __content_str__(self):
        return f"{self.name} {self.type}, {self.size} bytes"

    def _validate_inner(
        self, properties: tuple[Property, ...], on_invalid: ValidationHandler
    ) -> None:
        if len(self.name) > FILE_MAX_NAME_LENGTH:
            on_invalid(
                self,
                f"{self} name is too long ({len(self.name)} > {FILE_MAX_NAME_LENGTH})",
                (File.name,),
                None,
            )
        if self.size and self.size > FILE_MAX_SIZE:
            on_invalid(
                self, f"{self} is too big ({self.size} > {FILE_MAX_SIZE} bytes)", (File.size,), None
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
        raise NotImplementedError

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


@enum_(EnumType.ICON_KIND)
class IconKind(IdEnum):
    EMOJI = 1
    FILE = 2
    FONT_AWESOME = 3


@struct(StructType.ICON)
class Icon(Struct):
    kind: IconKind = p_internal(30, default=False)
    # content
    emoji: Optional[str] = p_internal(31, require=False)
    file: Optional["File"] = p_internal(32, require=False, array=False, struct=StructType.FILE)
    fa_name: Optional[str] = p_internal(33, require=False)
    # style
    color: Optional["Color"] = p_internal(40, require=False, array=False, struct=StructType.COLOR)

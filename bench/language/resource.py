from datetime import datetime
from typing import Optional
from uuid import UUID

from bench.language import RichText
from bench.language.const import (
    NodeType,
    StructType,
    ServerProfile,
    ServerStatus,
    StoreKind,
    FileStatus,
    PrimitiveType,
    StoreEngine,
)
from bench.language.node import (
    Bench,
    Node,
    Struct,
    node,
    p_parent,
    struct,
    p_internal,
    p_regular,
    p_system,
)
from bench.utils.cache import redis
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import _auto_async_to_sync


@node(NodeType.SERVER)
class Server(Node):
    """A server providing the compute runtime for a Bench."""

    parent: "Bench" = p_parent(4, NodeType.BENCH, is_system=True)

    profile: ServerProfile = p_regular(30)
    image: Optional["ServerImage"] = p_regular(31, struct=StructType.SERVER_IMAGE)
    version: Optional[str] = p_system(32)
    sleep: bool = p_regular(33, default=True)

    status: ServerStatus = p_system(40)
    current_profile: Optional[ServerProfile] = p_system(41)
    current_image: Optional["ServerImage"] = p_system(42, struct=StructType.SERVER_IMAGE)
    current_version: Optional[str] = p_system(43)
    last_active_at: Optional[datetime] = p_internal(44, default_factory=utcnow_with_tz)
    last_bumped_at: Optional[datetime] = p_internal(45, default_factory=utcnow_with_tz)


@struct(StructType.SERVER_IMAGE)
class ServerImage(Struct):
    language: str = p_regular(30)
    version: str = p_regular(31)
    platform: str = p_regular(32)
    requirements: list["ServerImageRequirement"] = p_regular(
        33, struct=StructType.SERVER_IMAGE_REQUIREMENT
    )


@struct(StructType.SERVER_IMAGE_REQUIREMENT)
class ServerImageRequirement(Struct):
    name: str = p_regular(30)
    version: str = p_regular(31)


@node(NodeType.STORE)
class Store(Node):
    """
    A store for database-like storage in a Bench.
    Virtualizes a physical database of that kind/engine (may be a sub-database/schema or such).
    """

    parent: "Bench" = p_parent(4, NodeType.BENCH, is_system=True)
    kind: StoreKind = p_system(30)
    engine: StoreEngine = p_system(31)
    name: str = p_regular(32)
    text: Optional["RichText"] = p_regular(34, default=None, struct=StructType.RICH_TEXT)
    is_host_dedicated: bool = p_system(35, default=False)
    is_database_dedicated: bool = p_system(36, default=False)
    is_schema_dedicated: bool = p_system(37, default=False)

    # base: Optional[Store] ...if shared?
    host: Optional[str] = p_system(41, sensitive=True, defer=True)
    database: Optional[str] = p_system(42, sensitive=True, defer=True)
    schema: Optional[str] = p_system(43, sensitive=True, defer=True)
    username: Optional[str] = p_system(44, sensitive=True, encrypt=True, defer=True)
    password: Optional[str] = p_system(45, sensitive=True, encrypt=True, defer=True)


@node(NodeType.DRIVE)
class Drive(Node):
    """
    A drive for file-like storage in a Bench.
    Virtualizes simple bucket-style access to some S3-like storage.
    """

    parent: "Bench" = p_parent(4, NodeType.BENCH, is_system=True)
    # engine: ...?
    name: str = p_regular(32)
    text: Optional["RichText"] = p_regular(34, default=None, struct=StructType.RICH_TEXT)

    host: Optional[str] = p_system(40, unique=True)
    uri: Optional[str] = p_system(41, unique=True)


@node(NodeType.FILE_CONTENT)
class FileContent(Node):
    """The actual file resource ('object') stored in a bucket somewhere. De-duped to 1 per sha512."""

    parent: Drive = p_parent(4, NodeType.DRIVE, is_system=True)
    sha512: str = p_internal(30)
    content_length: int = p_internal(31, primitive_type=PrimitiveType.INT64)
    content_type: str = p_internal(32)
    status: FileStatus = p_internal(33)


CacheKey = str | bytes
CacheValue = str | bytes
CACHE_USAGE_KEY = "__used"


@node(NodeType.CACHE)
class Cache(Node):
    """Cache for ephemeral data."""

    parent: "Bench" = p_parent(4, NodeType.BENCH, is_system=True)

    @_auto_async_to_sync
    async def get(self, key: CacheKey) -> Optional[CacheValue]:
        return await redis.get(f"{self.scope_key}.{key}")

    @_auto_async_to_sync
    async def get_many(self, keys: list[CacheKey]) -> dict[CacheKey, CacheValue]:
        values = await redis.mget([f"{self.scope_key}.{key}" for key in keys])
        return {key: value for key, value in zip(keys, values) if value is not None}

    @_auto_async_to_sync
    async def set(self, key: CacheKey, value: CacheValue, expire: int = None):
        value_size = self.get_value_size(value)
        pipe = redis.pipeline()
        pipe.set(f"{self.scope_key}.{key}", value, ex=expire)
        pipe.incrby(self.usage_key, value_size)
        await pipe.execute()

    @_auto_async_to_sync
    async def set_many(self, values: dict[CacheKey, CacheValue], expire: int = None):
        total_size = sum(self.get_value_size(value) for value in values.values())
        pipe = redis.pipeline()
        for key, value in values.items():
            pipe.set(f"{self.scope_key}.{key}", value, ex=expire)
        pipe.incrby(self.usage_key, total_size)
        await pipe.execute()

    @_auto_async_to_sync
    async def delete(self, key: CacheKey) -> CacheValue:
        value = await redis.get(f"{self.scope_key}.{key}")
        if value is None:
            raise KeyError(key)
        pipe = redis.pipeline()
        pipe.delete(f"{self.scope_key}.{key}")
        pipe.decrby(self.usage_key, self.get_value_size(value))
        await pipe.execute()
        return value

    def get_value_size(self, value: CacheValue) -> int:
        if isinstance(value, bytes):
            return len(value)
        elif isinstance(value, str):
            return len(value.encode("utf-8"))
        else:
            raise ValueError(f"unexpected value type {type(value)}")

    def _get_scope_key(self, bench_id: UUID) -> str:
        return f"bench.{bench_id}"

    def _get_usage_key(self, bench_id: UUID) -> str:
        return f"{self._get_scope_key(bench_id)}.{CACHE_USAGE_KEY}"

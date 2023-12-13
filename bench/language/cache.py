from typing import Optional
from uuid import UUID

from bench.language.builtin import _auto_async_to_sync
from bench.language.module import Module
from bench.utils.cache import redis

Key = str | bytes
Value = str | bytes

CACHE_USAGE_KEY = "__used"


def get_value_size(value: Value) -> int:
    if isinstance(value, bytes):
        return len(value)
    elif isinstance(value, str):
        return len(value.encode("utf-8"))
    else:
        raise ValueError(f"unexpected value type {type(value)}")


def _get_scope_key(project_id: UUID) -> str:
    return f"bench.{project_id}"


def _get_usage_key(project_id: UUID) -> str:
    return f"{_get_scope_key(project_id)}.{CACHE_USAGE_KEY}"


class Cache:
    """Cache for a Bench (async)."""

    def __init__(self, module: Optional[Module], subkey: str = None, project_id: UUID = None):
        self.module = module
        if module is None and project_id is None:
            raise ValueError("project_id must be provided if module is None")
        self.project_id = project_id or module.project_id
        self.scope_key = _get_scope_key(self.project_id)
        if subkey is not None:
            self.scope_key = f"{self.scope_key}.{subkey}"
        self.usage_key = _get_usage_key(self.project_id)

    def __str__(self):
        return f"{self.module} cache"

    def __repr__(self):
        return f"<CacheAsync {self}>"

    @_auto_async_to_sync
    async def get(self, key: Key) -> Optional[Value]:
        return await redis.get(f"{self.scope_key}.{key}")

    @_auto_async_to_sync
    async def get_many(self, keys: list[Key]) -> dict[Key, Value]:
        values = await redis.mget([f"{self.scope_key}.{key}" for key in keys])
        return {key: value for key, value in zip(keys, values) if value is not None}

    @_auto_async_to_sync
    async def set(self, key: Key, value: Value, expire: int = None):
        value_size = get_value_size(value)
        pipe = redis.pipeline()
        pipe.set(f"{self.scope_key}.{key}", value, ex=expire)
        pipe.incrby(self.usage_key, value_size)
        await pipe.execute()

    @_auto_async_to_sync
    async def set_many(self, values: dict[Key, Value], expire: int = None):
        total_size = sum(get_value_size(value) for value in values.values())
        pipe = redis.pipeline()
        for key, value in values.items():
            pipe.set(f"{self.scope_key}.{key}", value, ex=expire)
        pipe.incrby(self.usage_key, total_size)
        await pipe.execute()

    @_auto_async_to_sync
    async def delete(self, key: Key) -> Value:
        value = await redis.get(f"{self.scope_key}.{key}")
        if value is None:
            raise KeyError(key)
        pipe = redis.pipeline()
        pipe.delete(f"{self.scope_key}.{key}")
        pipe.decrby(self.usage_key, get_value_size(value))
        await pipe.execute()
        return value

from typing import Optional
from uuid import UUID

from bench.language.node import Package
from bench.utils.cache import redis
from bench.utils.func import _auto_async_to_sync

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


def _get_scope_key(bench_id: UUID) -> str:
    return f"bench.{bench_id}"


def _get_usage_key(bench_id: UUID) -> str:
    return f"{_get_scope_key(bench_id)}.{CACHE_USAGE_KEY}"


class Cache:
    """Cache for a Bench (async)."""

    def __init__(self, package: Optional[Package], subkey: str = None, bench_id: UUID = None):
        self.package = package
        if package is None and bench_id is None:
            raise ValueError("bench_id must be provided if package is None")
        self.bench_id = bench_id or package.bench_id
        self.scope_key = _get_scope_key(self.bench_id)
        if subkey is not None:
            self.scope_key = f"{self.scope_key}.{subkey}"
        self.usage_key = _get_usage_key(self.bench_id)

    def __str__(self):
        return f"{self.package} cache"

    def __repr__(self):
        return f"<Cache {self}>"

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

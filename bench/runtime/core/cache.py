from abc import ABC, abstractmethod
from typing import override

import redis.asyncio as redis

from bench.language import Bench
from bench.utils.utils import get_from_env, get_from_env_maybe

LOCAL_CACHE_HOST = get_from_env("LOCAL_CACHE_HOST", description="Redis URL")
LOCAL_CACHE_USERNAME = get_from_env_maybe("LOCAL_CACHE_USERNAME", description="Redis username")
LOCAL_CACHE_PASSWORD = get_from_env_maybe("LOCAL_CACHE_PASSWORD", description="Redis password")

redis_client = redis.Redis(
    host=LOCAL_CACHE_HOST, username=LOCAL_CACHE_USERNAME, password=LOCAL_CACHE_PASSWORD
)


class Cache(ABC):
    """Cache interface for some Bench."""

    def __init__(self, bench: Bench) -> None:
        self.bench = bench

    @abstractmethod
    async def clear(self, prefix: str = "") -> None:
        """Clear all keys with the given prefix."""
        ...

    @abstractmethod
    async def get(self, key: str) -> bytes | None:
        """Get the value for the given key."""
        ...

    @abstractmethod
    async def set(self, key: str, value: bytes, ttl: float | None = None) -> None:
        """Set the value for the given key."""
        ...


class RedisCache(Cache):
    """Cache for a Bench backed by a Redis instance."""

    def __init__(self, bench: Bench) -> None:
        super().__init__(bench)
        self.prefix = f"{bench.id}_"

    @override
    async def clear(self, prefix: str = "") -> None:
        raise NotImplementedError

    @override
    async def get(self, key: str) -> bytes | None:
        return await redis_client.get(key)

    @override
    async def set(self, key: str, value: bytes, ttl: float | None = None) -> None:
        await redis_client.set(key, value, ex=ttl)


class MemoryCache(Cache):
    """Cache for a Bench backed by a Redis instance."""

    def __init__(self, bench: Bench) -> None:
        super().__init__(bench)
        self.prefix = f"{bench.id}_"
        self.cache = {}

    @override
    async def clear(self, prefix: str = "") -> None:
        prefix = f"{self.prefix}{prefix}"
        self.cache = {k: v for k, v in self.cache.items() if not k.startswith(prefix)}

    @override
    async def get(self, key: str) -> bytes | None:
        return self.cache.get(key)

    @override
    async def set(self, key: str, value: bytes, ttl: float | None = None) -> None:
        self.cache[key] = value

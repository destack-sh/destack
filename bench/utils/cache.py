from redis import Redis as RedisSync
from redis.asyncio.client import Redis as RedisAsync

from bench.utils.utils import get_from_env

REDIS_URL = get_from_env("REDIS_URL", "redis://localhost:6379")

redis = RedisAsync.from_url(REDIS_URL)
redis_sync = RedisSync.from_url(REDIS_URL)

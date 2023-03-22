from redis.asyncio.client import Redis

from bench.utils.utils import get_from_env

REDIS_URL = get_from_env("REDIS_URL", "redis://localhost:6379")

redis = Redis.from_url(REDIS_URL)

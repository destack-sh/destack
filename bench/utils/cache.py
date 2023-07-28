import asyncio

import structlog
from redis import Redis as RedisSync
from redis.asyncio.client import Redis as RedisAsync

from bench.utils.utils import get_from_env

REDIS_URL = get_from_env("REDIS_URL", "redis://localhost:6379")

logger = structlog.get_logger(__name__)

redis = RedisAsync.from_url(REDIS_URL)
redis_sync = RedisSync.from_url(REDIS_URL)


async def test_redis_connection():
    try:
        logger.info("redis.ping")
        await asyncio.wait_for(redis.ping(), 3)
        logger.info("redis.pong")
    except Exception as e:
        logger.error("redis.failed", exc_info=e)
        raise RuntimeError(f"redis connection failed: {e}") from e

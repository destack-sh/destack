from datetime import timedelta

import structlog

from bench.utils.utils import get_from_env

SERVER_RUN_TIMEOUT = get_from_env("SERVER_RUN_TIMEOUT", 3000, type_cast=int)
SERVER_ACTIVE_TIMEOUT = timedelta(seconds=30)
SERVER_ACTIVE_PUBLISH_INTERVAL = 10
SERVER_SCHEDULE_BLOCK_AHEAD = 1

logger = structlog.get_logger(__name__)


class ServerProcess:
    """Server process to run one 'thread' for a Bench package. *May* 1:1 with actual OS processes."""

    async def run(self):
        raise NotImplementedError("nocheckin: server_process.run")

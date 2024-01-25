from datetime import timedelta

import structlog

from bench.utils.utils import get_from_env

WORKER_RUN_TIMEOUT = get_from_env("WORKER_RUN_TIMEOUT", 3000, type_cast=int)
WORKER_ACTIVE_TIMEOUT = timedelta(seconds=30)
WORKER_ACTIVE_PUBLISH_INTERVAL = 10
WORKER_SCHEDULE_BLOCK_AHEAD = 1

logger = structlog.get_logger(__name__)


class WorkerProcess:
    """Worker process to run one 'thread' for a Bench package. *May* 1:1 with actual OS processes."""

    async def run(self):
        raise NotImplementedError("nocheckin: worker_process.run")

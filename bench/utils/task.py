import asyncio
from asyncio import CancelledError

import structlog

from bench.utils.utils import sentry_capture

logger = structlog.get_logger(__name__)


class TaskManager:
    """Asyncio task manager incl. error handling and logging"""

    def __init__(self):
        self._tasks = {}
        self._errors = []

    @property
    def healthy(self):
        return not self._errors

    async def _wrap_task(self, coro, task_id: str = None):
        task_id = task_id or coro.__name__
        try:
            return await coro
        except CancelledError as e:
            logger.exception("task.cancelled", task_id=task_id, exc_info=e)
            raise
        except Exception as e:
            logger.exception("task.error", task_id=task_id, exc_info=e, sentry=sentry_capture(e))
            self._errors.append(e)
            raise

    def start(self, coro, name: str = None) -> None:
        asyncio.create_task(self._wrap_task(coro, name))

    async def stop(self) -> None:
        for task in self._tasks.values():
            task.cancel()
        await asyncio.gather(*self._tasks.values(), return_exceptions=True)
        self._tasks.clear()

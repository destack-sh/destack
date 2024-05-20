import asyncio
from asyncio import CancelledError
from typing import Awaitable, Callable, Coroutine

import structlog

from bench.utils.utils import sentry_capture

logger = structlog.get_logger(__name__)


class TaskManager:
    """Simple async task manager incl. error handling and logging"""

    def __init__(self):
        self._tasks = {}
        self._errors = []

    @property
    def healthy(self):
        return not self._errors

    async def _wrap_task(self, coro, task_id: str | None = None):
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

    def start(self, coro, name: str | None = None) -> None:
        asyncio.create_task(self._wrap_task(coro, name))

    def start_queue[
        T
    ](
        self,
        queue: asyncio.Queue[T],
        process_item: Callable[[T], Awaitable[T]],
        name: str | None = None,
    ) -> None:
        async def _queue_wrapper():
            while True:
                item = await queue.get()
                await process_item(item)

        _queue_wrapper.__name__ = f"{process_item.__name__}_queue"

        self.start(_queue_wrapper(), name)

    def close(self):
        for task in self._tasks.values():
            task.cancel()

    async def wait_closed(self) -> None:
        await asyncio.gather(*self._tasks.values(), return_exceptions=True)
        self._tasks.clear()

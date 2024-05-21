import asyncio
from asyncio import CancelledError
from typing import Any, Awaitable, Callable, Coroutine

from bench.utils.utils import sentry_capture


class TaskManager:
    """Simple async task manager incl. error handling and logging"""

    def __init__(self, owner: Any, logger: Any):
        self._tasks = {}
        self._errors = []
        self._owner = owner
        self._logger = logger

    @property
    def healthy(self):
        return not self._errors

    async def _wrap_task(self, coro, task_id: str | None = None):
        task_id = task_id or coro.__name__
        try:
            ret = await coro
            self._logger.debug("task.done", owner=self._owner, task_id=task_id)
            return ret
        except CancelledError:
            self._logger.debug("task.cancelled", owner=self._owner, task_id=task_id)
        except Exception as e:
            self._logger.exception(
                "task.error",
                owner=self._owner,
                task_id=task_id,
                exc_info=e,
                sentry=sentry_capture(e),
            )
            self._errors.append(e)
            raise

    def start(self, coro: Awaitable[Any], name: str | None = None) -> None:
        asyncio.create_task(self._wrap_task(coro, name))

    def start_queue[
        T
    ](
        self,
        queue: asyncio.Queue[T],
        process_item: Callable[[T], Awaitable[T] | Coroutine[T, None, None]],
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

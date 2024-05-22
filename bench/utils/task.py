import asyncio
from asyncio import CancelledError
from typing import Any, Awaitable, Callable, Coroutine

from bench.utils.utils import sentry_capture


class TaskManager:
    """Simple async task manager incl. error handling and logging"""

    def __init__(
        self,
        *,
        owner: Any,
        logger: Any,
        on_error: Callable[[Exception], None] | None = None,
        task_id_prefix: str | None = None,
    ):
        self._active_tasks: list[asyncio.Task] = []
        self._errors = []
        self._owner = owner
        self._logger = logger
        self._on_error = on_error
        self._task_id_prefix = task_id_prefix

    @property
    def healthy(self):
        return not self._errors

    def check_no_errors(self):
        if self._errors:
            if len(self._errors) == 1:
                raise self._errors[0]
            else:
                raise RuntimeError(f"multiple errors occurred: {self._errors}")

    async def _run_task(self, coro, task_id: str | None, on_complete: Callable[[], None]):
        task_id = task_id or coro.__name__
        if self._task_id_prefix:
            task_id = f"{self._task_id_prefix}_{task_id}"
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
            if self._on_error is not None:
                self._on_error(e)
            self._errors.append(e)
            raise
        finally:
            on_complete()

    def start(self, coro: Awaitable[Any], name: str | None = None) -> None:
        task = asyncio.create_task(
            self._run_task(coro, name, lambda: self._active_tasks.remove(task))
        )
        self._active_tasks.append(task)

    async def _run_queue_task(
        self,
        queue: asyncio.Queue,
        process: Callable[[Any], Awaitable[Any] | Coroutine[Any, None, None]],
        task_id: str | None = None,
        skip_errors: bool = False,
    ) -> None:
        task_id = task_id or process.__name__
        if self._task_id_prefix:
            task_id = f"{self._task_id_prefix}_{task_id}"
        while True:
            item = await queue.get()
            try:
                await process(item)
            except Exception as e:
                self._logger.exception(
                    "task.error",
                    owner=self._owner,
                    task_id=task_id,
                    exc_info=e,
                    sentry=sentry_capture(e),
                )
                if not skip_errors:
                    if self._on_error is not None:
                        self._on_error(e)
                    self._errors.append(e)
                    raise
            finally:
                queue.task_done()

    def start_queue[
        T
    ](
        self,
        queue: asyncio.Queue[T],
        process: Callable[[T], Awaitable[T] | Coroutine[T, None, None]],
        name: str | None = None,
        *,
        skip_errors: bool,
    ) -> None:
        task = asyncio.create_task(self._run_queue_task(queue, process, name, skip_errors))
        self._active_tasks.append(task)

    def close(self):
        for task in self._active_tasks:
            task.cancel()

    async def wait_closed(self) -> None:
        await asyncio.gather(*self._active_tasks, return_exceptions=True)
        self._active_tasks.clear()

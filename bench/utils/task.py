import asyncio
import contextlib
from typing import Any, Awaitable, Callable, Coroutine

from bench.utils.oracle import Oracle
from bench.utils.utils import sentry_capture


async def wrap_task(coro: Coroutine, logger, task_id: str, owner: Any) -> None:
    try:
        return await coro
    except asyncio.CancelledError as e:
        logger.exception(f"{task_id}.cancel", task_id=task_id, owner=owner, exc_info=e)
        pass
    except BaseException as e:
        logger.exception(
            f"{task_id}.error", task_id=task_id, owner=owner, exc_info=e, sentry=sentry_capture(e)
        )
        raise


class TaskManager:
    """Simple async task manager incl. error handling and logging"""

    def __init__(
        self,
        *,
        owner: Any,
        logger: Any,
        oracle: Oracle,
        on_error: Callable[[Exception], None] | None = None,
        task_id_prefix: str | None = None,
    ):
        self._active_tasks: list[asyncio.Task] = []
        self._errors = []
        self._owner = owner
        self._logger = logger
        self._oracle = oracle
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

    def _make_task_id(self, task_id: str | None, default: str) -> str:
        task_id = task_id or default
        if self._task_id_prefix:
            if task_id.startswith("_"):
                task_id = task_id[1:]
            task_id = f"{self._task_id_prefix}_{task_id}"
        return task_id

    async def _run_queue_task(
        self,
        queue: asyncio.Queue,
        process: Callable[[Any], Awaitable[Any] | Coroutine[Any, None, None]],
        task_id: str,
        skip_errors: bool = False,
    ) -> None:
        while True:
            try:
                item = await queue.get()
            except (asyncio.CancelledError, RuntimeError):
                # queue throws RuntimeError if event loop is closed (happens when pytest shuts down)
                self._logger.trace("task.cancelled", owner=self._owner, task_id=task_id)
                break
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

    def start_queue[T](
        self,
        queue: asyncio.Queue[T],
        process: Callable[[T], Awaitable[T] | Coroutine[T, None, None]],
        task_id: str | None = None,
        *,
        skip_errors: bool,
    ) -> None:
        task_id = self._make_task_id(task_id, process.__name__)
        task = asyncio.create_task(
            coro=self._run_queue_task(
                queue=queue, process=process, task_id=task_id, skip_errors=skip_errors
            ),
            name=task_id,
        )
        self._active_tasks.append(task)

    async def _run_scheduled_tasks(
        self,
        run_every: float,
        process: Callable[[], None | Awaitable[None] | Coroutine[None, None, None]],
        task_id: str,
        skip_errors: bool = False,
    ) -> None:
        while True:
            try:
                await self._oracle.sleep(run_every)
            except (asyncio.CancelledError, RuntimeError):
                self._logger.trace("task.cancelled", owner=self._owner, task_id=task_id)
                break
            try:
                ret = process()
                if ret is not None:
                    await ret
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

    def start_scheduled(
        self,
        run_every: float,
        process: Callable[[], None | Awaitable[None] | Coroutine[None, None, None]],
        task_id: str | None = None,
        *,
        skip_errors: bool,
    ) -> None:
        task_id = self._make_task_id(task_id, process.__name__)
        task = asyncio.create_task(
            coro=self._run_scheduled_tasks(
                run_every=run_every, process=process, task_id=task_id, skip_errors=skip_errors
            ),
            name=task_id,
        )
        self._active_tasks.append(task)

    def close(self):
        for task in self._active_tasks:
            with contextlib.suppress(asyncio.CancelledError, RuntimeError):  # see above
                task.cancel()

    async def wait_closed(self) -> None:
        with contextlib.suppress(asyncio.CancelledError, RuntimeError):
            await asyncio.gather(*self._active_tasks, return_exceptions=True)
        self._active_tasks.clear()

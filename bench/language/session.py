import abc
import asyncio
import contextvars
from concurrent.futures import Executor, ThreadPoolExecutor
from dataclasses import dataclass
from datetime import datetime
from typing import TYPE_CHECKING, Any, Awaitable, Callable, Coroutine, Optional
from uuid import UUID, uuid4

import asgiref.sync
import structlog

from bench.language.const import ModuleOp, SessionMode, TriggerType
from bench.language.module import Module, ModuleNode, ScopedNode
from bench.language.query import Query, Sort, SortOrder
from bench.language.search import Search
from bench.utils.dt import utcnow_with_tz

if TYPE_CHECKING:
    from bench.language.mutate import MMT, ModuleMutation, NodeMutator
    from bench.language.run import Run
    from bench.language.statement import Statement
    from bench.language.tracing import _RunCapture
    from bench.language.wire import LogEntryData, RunData

logger = structlog.get_logger(__name__)
active_session: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)
SESSION_UNSET = object()


@dataclass(slots=True)
class SessionContext:
    module_id: UUID
    project_id: UUID
    worker_node_id: str
    worker_process_id: Optional[str]
    trigger_type: TriggerType
    trigger_id: Optional[UUID]
    first_run_id: Optional[UUID] = None


SESSION_MUTATION_FLUSH_WATERMARK = 512


class ModuleWriter(abc.ABC):
    """Base for writing module/session for type-checking."""

    async def write_module(self, mutations: list["ModuleMutation"], refresh_index: bool) -> bool:
        raise NotImplementedError

    async def write_session(
        self, session: "Session", runs: list["Run"], logs: list["LogEntry"]
    ) -> bool:
        raise NotImplementedError


class NoopModuleWriter(ModuleWriter):
    async def write_module(self, mutations: list["ModuleMutation"], refresh_index: bool) -> bool:
        return True

    async def write_session(
        self, session: "Session", runs: list["Run"], logs: list["LogEntry"]
    ) -> bool:
        return True


class Session:
    """A managed context for running code in a module (may mutate)."""

    def __init__(
        self,
        module: Module,
        writer: ModuleWriter,
        id: UUID = None,
        ctx: SessionContext | None = None,
        cache_inferences: bool = True,
        inference_timeout: int = 300,
        inference_retries: int = 5,
        mode: SessionMode = SessionMode.READ_ONLY,
        executor: Executor = None,
    ):
        from bench.language.cache import CacheAsync, CacheSync
        from bench.language.mutate import NodeMutator
        from bench.language.remote import Storage
        from bench.language.tracing import SessionTracer

        self.id = id or uuid4()
        self.ctx = ctx
        self.module = module
        self.cache_inferences = cache_inferences
        self.inference_timeout = inference_timeout
        self.inference_retries = inference_retries
        self.mode = mode
        self.writer = writer

        self.cache_sync = CacheSync(module, project_id=ctx.project_id)
        self.cache_async = CacheAsync(module, project_id=ctx.project_id)
        self.storage = Storage(module)

        self.anonymous_scope = ScopedNode(parent=self.module)
        self.executor = executor or ThreadPoolExecutor(max_workers=1)
        self.logger = logger.bind(session=self)
        self.mutator = NodeMutator(self.module, hooks=[self._on_mutated])
        self.tracer = SessionTracer(self, mutator=self.mutator, validate=True)
        self.opened_at: Optional[datetime] = None
        self.closed_at: Optional[datetime] = None
        self._past_flushes: list[tuple[int, set[MMT]]] = []
        self._pending_flushes: list[tuple[int, Awaitable[bool]]] = []

    def __str__(self):
        status = "open" if self.opened_at else ("closed" if self.closed_at else "pending")
        return (
            f"{self.module.name} {self.id} ({self.mode}, {status}, {len(self.mutator.mutations)})"
        )

    def __repr__(self):
        return f"<Session {self}>"

    def sync_to_async(self, fn: Callable) -> Callable[..., Awaitable]:
        return asgiref.sync.sync_to_async(fn, thread_sensitive=False, executor=self.executor)  # type: ignore

    def async_to_sync(self, fn: Awaitable | Callable | Coroutine) -> Callable:
        return asgiref.sync.async_to_sync(fn)  # type: ignore

    @property
    def current_run(self) -> "Run":
        return self.tracer.run.current_run

    def capture_runs(self) -> "_RunCapture":
        return self.tracer.run.start_capture()

    def run_value(self, **kwargs):
        return self.tracer.run.value(**kwargs)

    @property
    def is_open(self) -> bool:
        return self.opened_at is not None and self.closed_at is None

    def check_can(self, op: ModuleOp, thing: ModuleNode):
        if not self.can(op, thing):
            raise RuntimeError(f"cannot {op} {thing} in {self}")

    def can(self, op: ModuleOp, thing: ModuleNode) -> bool:
        if self.mode == SessionMode.READ_ONLY:
            return op in (ModuleOp.READ, ModuleOp.READ)
        elif self.mode == SessionMode.WRITE:
            return True
        else:
            raise RuntimeError(f"unknown session mode {self.mode}")

    async def aopen(self):
        """Opens the session for execution and modification."""
        if self.opened_at is not None:
            raise RuntimeError(f"session already opened {self}")
        self.opened_at = utcnow_with_tz()
        if active_session.get() is not None:
            raise RuntimeError(f"another session is active: {active_session.get()}")
        active_session.set(self)
        await self.tracer.open()
        logger.debug("session.open", session=self)

    async def _do_search_preflight(self, search: Search) -> None:
        """FLush any relevant mutations before searching."""
        from bench.language.database import RecordSearch

        if isinstance(search, RecordSearch):
            # force flush and index if there are any pending database mutations
            #  (or previous mutations that were already flushed but didn't refresh the index)
            # TODO @Performance: force flush module for record search only if needed
            if self.mutator.mutations or self._past_flushes:
                await self.aflush(optimistic=False, refresh_index=True)

    async def _do_flush(self, mutations: list["ModuleMutation"], refresh_index: bool) -> bool:
        # TODO @Robustness: auto-split mutations if not in atomic block and too large
        success = await self.writer.write_module(mutations, refresh_index)
        if not success:
            if len(mutations) > 10:
                mutations_str = f"{mutations[:5]} ... {mutations[-5:]}"
            else:
                mutations_str = str(mutations)
            raise RuntimeError(f"failed to write {len(mutations)} mutations {mutations_str}")
        logger.debug("session.flush.done", session=self, mutator=self.mutator)
        return success

    async def aflush(self, optimistic: bool = False, refresh_index: bool = False):
        """
        Flushes all module mutations.
        If optimistic, this will return before the flush is complete (but will wait on close).
        """
        if not self.mutator.mutations and not refresh_index:
            return  # skip if no mutations and no index refresh
        if self.mode == SessionMode.READ_ONLY:
            raise RuntimeError(f"cannot mutate read-only session {self}")
        logger.debug(
            "session.flush",
            session=self,
            mutator=self.mutator,
            optimistic=optimistic,
            refresh_index=refresh_index,
        )
        mutations = self.mutator.bundle().compact()
        self.mutator.reset()
        flush = self._do_flush(mutations, refresh_index)
        if optimistic:
            self._pending_flushes.append((len(mutations), asyncio.create_task(flush)))
        else:
            await flush
        self._past_flushes.append((len(mutations), set(m.type for m in mutations)))

    def flush(self, optimistic: bool = False):
        if not self.mutator.mutations:
            return
        asgiref.sync.async_to_sync(self.aflush)(optimistic=optimistic)

    async def aclose(self):
        """Closes the session, flushing any mutations and preventing further execution/mutation."""
        if self.closed_at is not None:
            raise RuntimeError(f"session already closed {self}")
        self.closed_at = utcnow_with_tz()
        await self.aflush(optimistic=True)
        # TODO @Robustness: flush pending mutations inside top level run (to report errors properly)
        # await all pending flushes
        pending_mutations_count = sum(count for count, _ in self._pending_flushes)
        logger.debug(
            "session.close.pending", session=self, pending_mutations_count=pending_mutations_count
        )
        await asyncio.gather(*(task for _, task in self._pending_flushes))
        active_session.set(None)
        await self.tracer.close()
        logger.debug("session.close", session=self)

    def close(self):
        asgiref.sync.async_to_sync(self.aclose)()

    def _on_mutated(self, mutator: "NodeMutator", mutation: "ModuleMutation"):
        if len(self.mutator.mutations) > SESSION_MUTATION_FLUSH_WATERMARK:
            self.flush(optimistic=True)

    async def __aenter__(self):
        await self.aopen()
        return self

    async def __aexit__(self, exc_type, exc_value, traceback):
        await self.aclose()

    def sync(self):
        """A sync context manager for this session."""
        session = self

        class SyncSession:
            def __enter__(self):
                asgiref.sync.async_to_sync(session.aopen)()
                return session

            def __exit__(self, exc_type, exc_value, traceback):
                asgiref.sync.async_to_sync(session.aclose)()

        return SyncSession()


@dataclass
class LazyRun:
    """Run that's not loaded."""

    id: UUID

    def load(self) -> "Run":
        raise NotImplementedError


# @reflect_struct("LogEntry", "A single log entry from a run")
@dataclass
class LogEntry:
    id: UUID
    module: Module
    created_at: datetime
    stream: str
    session: Session
    level: Optional[str] = None
    logger: Optional[str] = None
    runnable: Optional["Statement"] = None
    run: Optional["Run"] = None
    message: Optional[str] = None
    value: dict[str, Any] = None

    def __str__(self):
        return f"'{self.message}' ({self.created_at})"

    def __repr__(self):
        return f"<LogEntry {self}>"


class RunSearch(Search["RunData", "Run"]):
    """Search over runs."""

    def __init__(
        self,
        module: Module,
        runnables: list["Statement"] | None,
        query: Query | None,
        sort: list[Sort] | None,
        limit: Optional[int],
    ):
        super().__init__(module, query, sort, limit)
        self.runnables = runnables

    async def _do_search(
        self, after: list[Any] = None, limit: Optional[int] = None, count: bool = False
    ):
        from bench.msg import NMessage
        from bench.msg.core import request
        from bench.msg.messages import NMessageType, RepSearchRunPayload, ReqSearchRunsPayload

        await self.module.session._do_search_preflight(self)
        batch_limit = min(self.RESULT_BATCH_SIZE, limit or self._limit or self.RESULT_BATCH_SIZE)
        runnables_cks = [runnable.ck for runnable in self.runnables] if self.runnables else None
        rep: NMessage[RepSearchRunPayload] = await request(
            NMessageType.SEARCH_RUNS,
            ReqSearchRunsPayload(
                module_id=self.module.id,
                runnables_cks=runnables_cks,
                query=self._query,
                sort=self._sort,
                after=after,
                limit=batch_limit,
                count=count,
            ),
            reply_t=RepSearchRunPayload,
        )
        if rep.p.error:
            raise RuntimeError(f"{self} failed (after={after}, limit={limit}): {rep.p.error}")
        return rep

    def _unpack_element_data(self, element_data: "RunData") -> "Run":
        from bench.language import wire

        return wire.unpack_data(element_data, module=self.module)

    def filter(self, query: Query) -> "RunSearch":
        combined_query = Query.and_if_set(self._query, query)
        return RunSearch(self.module, self.runnables, combined_query, self._sort, self._limit)

    def sort(self, sort: list[Sort] | Sort) -> "RunSearch":
        sort = [sort] if isinstance(sort, Sort) else sort
        return RunSearch(self.module, self.runnables, self._query, sort, self._limit)

    def limit(self, limit: int) -> "RunSearch":
        return RunSearch(self.module, self.runnables, self._query, self._sort, limit)

    @staticmethod
    def from_runnable(runnable: "Statement") -> "RunSearch":
        return RunSearch(
            module=runnable.module,
            runnables=[runnable],
            query=None,
            sort=[Sort("created_at", SortOrder.DESCENDING)],
            limit=None,
        )


class LogSearch(Search["LogEntryData", LogEntry]):
    """Search over logs."""

    def __init__(
        self,
        module: Module,
        runnables: list["Statement"] | None,
        query: Query | None,
        sort: list[Sort] | None,
        limit: Optional[int],
    ):
        super().__init__(module, query, sort, limit)
        self.runnables = runnables

    async def _do_search(
        self, after: list[Any] = None, limit: Optional[int] = None, count: bool = False
    ):
        from bench.msg import NMessage
        from bench.msg.core import request
        from bench.msg.messages import NMessageType, RepSearchLogPayload, ReqSearchLogPayload

        await self.module.session._do_search_preflight(self)
        batch_limit = min(self.RESULT_BATCH_SIZE, limit or self._limit or self.RESULT_BATCH_SIZE)
        runnables_ids = [runnable.id for runnable in self.runnables] if self.runnables else None
        rep: NMessage[RepSearchLogPayload] = await request(
            NMessageType.SEARCH_LOGS,
            ReqSearchLogPayload(
                module_id=self.module.id,
                runnables_ids=runnables_ids,
                query=self._query,
                sort=self._sort,
                after=after,
                limit=batch_limit,
                count=count,
            ),
            reply_t=RepSearchLogPayload,
        )
        if rep.p.error:
            raise RuntimeError(f"{self} failed (after={after}, limit={limit}): {rep.p.error}")
        return rep

    def _unpack_element_data(self, element_data: "LogEntryData") -> LogEntry:
        from bench.language import wire

        return wire.unpack_data(element_data, module=self.module)

    def filter(self, query: Query) -> "LogSearch":
        combined_query = Query.and_if_set(self._query, query)
        return LogSearch(self.module, self.runnables, combined_query, self._sort, self._limit)

    def sort(self, sort: list[Sort] | Sort) -> "LogSearch":
        sort = [sort] if isinstance(sort, Sort) else sort
        return LogSearch(self.module, self.runnables, self._query, sort, self._limit)

    def limit(self, limit: int) -> "LogSearch":
        return LogSearch(self.module, self.runnables, self._query, self._sort, limit)

    @staticmethod
    def from_runnable(runnable: "Statement") -> "LogSearch":
        return LogSearch(
            module=runnable.module,
            runnables=[runnable],
            query=None,
            sort=[Sort("created_at", SortOrder.DESCENDING)],
            limit=None,
        )

import abc
import asyncio
import contextvars
import enum
import sys
import traceback
from concurrent.futures import Executor, ThreadPoolExecutor
from dataclasses import dataclass, field
from datetime import datetime
from typing import TYPE_CHECKING, Any, Awaitable, Callable, Coroutine, Optional, Union
from uuid import UUID, uuid4

import structlog
from asgiref.sync import async_to_sync

from bench.language.const import ModuleOp, SessionMode, StatementType, TriggerType
from bench.language.module import Module, ModuleNode, Scope
from bench.language.query import Query, Sort, SortOrder
from bench.language.reflect import reflect_enum, reflect_struct
from bench.language.search import Search
from bench.utils.dt import utcnow_with_tz
from bench.utils.utils import IdentifierType, to_pyidentifier_multi

if TYPE_CHECKING:
    from bench.language.code_ import Code
    from bench.language.field import Field
    from bench.language.model import Model
    from bench.language.task import Task
    from bench.language.wire import LogEntryData, RunData

logger = structlog.get_logger(__name__)
active_session: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)


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

    async def write_module(self, mutations: list["ModuleMutation"]) -> bool:
        raise NotImplementedError

    async def write_session(
        self, session: "Session", runs: list["Run"], logs: list["LogEntry"]
    ) -> bool:
        raise NotImplementedError


class NoopModuleWriter(ModuleWriter):
    async def write_module(self, mutations: list["ModuleMutation"]) -> bool:
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
        from bench.language.mutate import ModuleMutator
        from bench.language.remote import Storage
        from bench.language.tracing import SessionTracer

        self.id = id or uuid4()
        self.ctx = ctx
        self.module = module
        self.instances_by_id: dict[UUID, "ModuleNode"] = {}
        self.cache_inferences = cache_inferences
        self.inference_timeout = inference_timeout
        self.inference_retries = inference_retries
        self.mode = mode
        self.writer = writer

        self.cache_sync = CacheSync(module, project_id=ctx.project_id)
        self.cache_async = CacheAsync(module, project_id=ctx.project_id)
        self.storage = Storage(module)

        self.anonymous_scope = Scope(parent=self.module)
        self.executor = executor or ThreadPoolExecutor(max_workers=1)
        self.logger = logger.bind(session=self)
        self.mutator = ModuleMutator(self.module, hooks=[self._on_mutated])
        self.tracer = SessionTracer(self, mutator=self.mutator, validate=True)
        self.opened_at: Optional[datetime] = None
        self.closed_at: Optional[datetime] = None
        self.metadata: dict[str, Any] = {}
        self._pending_flushes: list[tuple[int, Awaitable[bool]]] = []

    def __str__(self):
        status = "open" if self.opened_at else ("closed" if self.closed_at else "pending")
        return (
            f"{self.module.name} {self.id} ({self.mode}, {status}, {len(self.mutator.mutations)})"
        )

    def __repr__(self):
        return f"<Session {self}>"

    def sync_to_async(self, fn: Callable) -> Callable[..., Awaitable]:
        return sync_to_async(fn, thread_sensitive=False, executor=self.executor)  # type: ignore

    def async_to_sync(self, fn: Awaitable | Callable | Coroutine) -> Callable:
        return async_to_sync(fn)  # type: ignore

    @property
    def current_run(self) -> "Run":
        return self.tracer.run.current_run

    @property
    def is_open(self) -> bool:
        return self.opened_at is not None and self.closed_at is None

    def add(self, *objs: "ModuleNode", new: bool = False) -> None:
        from bench.language import File, Statement

        if new:
            for obj in objs:
                # permissions are checked in tracer
                if isinstance(obj, File):
                    self.tracer.file_create(obj)
                elif isinstance(obj, Statement):
                    self.tracer.statement_create(obj)
                    if isinstance(obj, ModuleNode):
                        # it feels like this should be done in some tracer? also (re?)-index?
                        self.module._nodes_by_id[obj.id] = obj
                        self.module._nodes_by_ck[obj.ck] = obj
        for obj in objs:
            self.instances_by_id[obj.id] = obj

    def remove(self, *objs: "ModuleNode") -> None:
        for obj in objs:
            if obj.id in self.instances_by_id:
                del self.instances_by_id[obj.id]
                # not doing anything yet?

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

    async def _do_flush(self, mutations: list["ModuleMutation"]) -> bool:
        # TODO @Robustness: auto-split mutations if not in atomic block and too large
        success = await self.writer.write_module(mutations)
        if not success:
            if len(mutations) > 20:
                mutations_str = f"{mutations[:10]} ... {mutations[-10:]}"
            else:
                mutations_str = str(mutations)
            raise RuntimeError(f"failed to write {len(mutations)} mutations {mutations_str}")
        logger.debug("session.flush.done", session=self, mutator=self.mutator)
        return success

    async def aflush(self, optimistic: bool = False):
        """
        Flushes all module mutations.
        If optimistic, this will return before the flush is complete (but will wait on close).
        """
        if not self.mutator.mutations:
            return
        if self.mode == SessionMode.READ_ONLY:
            raise RuntimeError(f"cannot mutate read-only session {self}")
        logger.debug("session.flush", session=self, mutator=self.mutator, optimistic=optimistic)
        mutations = self.mutator.bundle().compact()
        self.mutator.reset()
        flush = self._do_flush(mutations)
        if optimistic:
            self._pending_flushes.append((len(mutations), asyncio.create_task(flush)))
        else:
            await flush

    def flush(self, optimistic: bool = False):
        async_to_sync(self.aflush)(optimistic=optimistic)

    async def aclose(self):
        """Closes the session, flushing any mutations and preventing further execution/mutation."""
        if self.closed_at is not None:
            raise RuntimeError(f"session already closed {self}")
        self.closed_at = utcnow_with_tz()
        await self.aflush(optimistic=True)
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
        async_to_sync(self.aclose)()

    def _on_mutated(self, mutator: "ModuleMutator", mutation: "ModuleMutation"):
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
                async_to_sync(session.aopen)()
                return session

            def __exit__(self, exc_type, exc_value, traceback):
                async_to_sync(session.aclose)()

        return SyncSession()


class WorkerProfile(enum.StrEnum):
    TINY = "TINY"
    SMALL = "SMALL"
    MEDIUM = "MEDIUM"
    LARGE = "LARGE"
    XLARGE_CPU = "XLARGE_CPU"
    XLARGE_MEM = "XLARGE_MEM"


class WorkerRegion(enum.StrEnum):
    US_CENTRAL = "US_CENTRAL"
    EU_CENTRAL = "EU_CENTRAL"


class WorkerSetStatus(enum.StrEnum):
    SLEEPING = "SLEEPING"
    PENDING = "PENDING"
    UPDATING = "UPDATING"
    HEALTHY = "HEALTHY"
    UNHEALTHY = "UNHEALTHY"
    UNAVAILABLE = "UNAVAILABLE"
    UNKNOWN = "UNKNOWN"


@reflect_enum("RunStatus", "The status of a run")
class RunStatus(enum.StrEnum):
    Scheduled = "Scheduled"
    Queued = "Queued"
    Running = "Running"
    Suspended = "Suspended"
    Aborting = "Aborting"
    # terminal statuses
    Cancelled = "Cancelled"
    Aborted = "Aborted"
    Failed = "Failed"
    Completed = "Completed"


TERMINAL_RUN_STATUSES = {
    RunStatus.Cancelled,
    RunStatus.Aborted,
    RunStatus.Failed,
    RunStatus.Completed,
}
PENDING_RUN_STATUSES = set(RunStatus) - TERMINAL_RUN_STATUSES


@dataclass
class LazyRun:
    """Run that's not loaded."""

    id: UUID

    def load(self) -> "Run":
        raise NotImplementedError


@reflect_struct("RunMetadata", "Default metadata of a run", return_type=True)
class RunMetadata:
    name: Optional[str]
    test: Optional[bool]
    queue_position: Optional[int]
    cached_at: Optional[datetime]
    cached_in: Optional[UUID]
    cached_duration: Optional[float]
    progress: Optional[float]


@dataclass
class Run:
    id: UUID
    runnable: Union["Code", "Model", "Task"]
    module: Module
    session: Optional[Session]
    root: Optional["Run"]
    parent: Optional["Run"]
    scheduled_at: Optional[datetime]
    started_at: Optional[datetime]
    terminated_at: Optional[datetime]
    trigger_type: Optional[TriggerType]
    trigger: Union["Trigger", UUID]
    status: RunStatus = field(init=False)
    inputs: Optional[dict[str, Any]]
    outputs: Optional[dict[str, Any]]
    error: Optional["RunError"]
    metadata: Optional[dict[str, Any]]
    created_at: datetime = field(default_factory=utcnow_with_tz)
    updated_at: datetime = field(default_factory=utcnow_with_tz)
    children: list["Run"] = field(default_factory=list)

    def __post_init__(self):
        self._update_status()
        self.updated_at = utcnow_with_tz()

    def __str__(self):
        metadata_keys_str = ", ".join(self.metadata.keys()) if self.metadata else ""
        return f"{self.runnable} ({self.status}, metadata={metadata_keys_str or '<none>'})"

    def __repr__(self):
        return f"<Run {self}>"

    def _update_status(self):
        if self.error:
            self.status = RunStatus.Failed
        elif self.terminated_at:
            self.status = RunStatus.Completed
        elif self.queue_position:
            self.status = RunStatus.Queued
        else:
            self.status = RunStatus.Running

    @property
    def duration(self) -> float:
        if self.terminated_at is None:
            return 0
        return (self.terminated_at - self.started_at).total_seconds()

    @property
    def duration_with_cache(self) -> float:
        if self.terminated_at is None:
            return 0
        return self.duration + (self.cached_duration or 0)

    def walk_descendants(self):
        yield self
        for child in self.children:
            yield from child.walk_descendants()

    def get_metadata(self, key: Union[str, "Field"], default: Any = None) -> Any:
        if not isinstance(key, str):
            key = key.typed_key
        if self.metadata is None:
            return default
        return self.metadata.get(key, default)

    def set_metadata(self, key: Union[str, "Field"], value: Any):
        if not isinstance(key, str):
            key = key.typed_key
        if self.metadata is None:
            self.metadata = {}
        self.metadata[key] = value

    # direct accessors for default metadata (not great but good enough for now)
    # TODO @Cleanup: wrap all default run metadata and task metadata here

    @property
    def cached_duration(self) -> Optional[float]:
        return self.get_metadata(RunMetadata.cached_duration)

    @cached_duration.setter
    def cached_duration(self, value: Optional[float]):
        self.set_metadata(RunMetadata.cached_duration, value)

    @property
    def queue_position(self) -> Optional[int]:
        return self.get_metadata(RunMetadata.queue_position)

    @queue_position.setter
    def queue_position(self, value: Optional[int]):
        self.set_metadata(RunMetadata.queue_position, value)

    @property
    def cached_at(self) -> Optional[datetime]:
        cached_at = self.get_metadata(RunMetadata.cached_at)
        return datetime.fromisoformat(cached_at) if cached_at else None

    @cached_at.setter
    def cached_at(self, value: Optional[datetime]):
        self.set_metadata(RunMetadata.cached_at, value.isoformat() if value else None)

    @property
    def cached_in(self) -> Optional[UUID]:
        return self.get_metadata(RunMetadata.cached_in)

    @cached_in.setter
    def cached_in(self, value: Optional[UUID]):
        self.set_metadata(RunMetadata.cached_in, value)

    @property
    def test(self) -> Optional[bool]:
        return self.get_metadata(RunMetadata.test)

    @test.setter
    def test(self, value: Optional[bool]):
        self.set_metadata(RunMetadata.test, value)


_IGNORED_PACKAGE_PREFIXES = [
    "bench.runtime",
    "bench.bench",
    "asgiref",
    "concurrent",
]
_IGNORED_PACKAGE_PATHS = [package.replace(".", "/") for package in _IGNORED_PACKAGE_PREFIXES]


@reflect_struct("RunCodeFrame", "The frame of code that was executed for a traceback")
class RunCodeFrame:
    filename: str
    lineno: int
    name: str
    locals: dict[str, Any] = None  # locals should be richer for deep linking (with ids)
    line: str = None

    @staticmethod
    def from_dict(data: dict[str, Any]) -> "RunCodeFrame":
        return RunCodeFrame(
            filename=data["filename"],
            lineno=data["lineno"],
            name=data["name"],
            locals=data["locals"],
            line=data["line"],
        )

    @staticmethod
    def from_stack(stack: traceback.StackSummary) -> list["RunCodeFrame"]:
        return [
            RunCodeFrame(
                filename=frame.filename,
                lineno=frame.lineno,
                name=frame.name,
                locals=frame.locals,
                line=frame.line,
            )
            for frame in stack
        ]

    @staticmethod
    def clean(
        stack: list["RunCodeFrame"], from_statement: "Statement", session: "Session"
    ) -> list["RunCodeFrame"]:
        code_by_method: dict[str, Code] = {
            symbol._transform.method_name: symbol
            for symbol in session.instances_by_id.values()
            if isinstance(symbol, Code) and symbol._transform is not None
        }

        found_start = False
        cleaned_stack = []
        for frame in stack:
            if any(prefix in frame.filename for prefix in _IGNORED_PACKAGE_PATHS):
                continue  # skip support code
            if not found_start:
                # impute bench source info into instantiated code callables
                code = code_by_method.get(frame.name)
                if code is not None:
                    if code == from_statement:
                        found_start = True
                    elif not found_start:
                        continue  # ignore
                    frame.filename = to_pyidentifier_multi(
                        from_statement.file.name, from_statement.name, type=IdentifierType.PATH
                    )
                    frame.name = from_statement.name
                    frame.lineno = frame.lineno - code._transform.start_offset
                    frame.line = code.code.splitlines()[frame.lineno - 1]
                    frame.locals = frame.locals or {}
                    for ident, var in code._statement_references.items():
                        if ident not in frame.locals and var.id in session.instances_by_id:
                            frame.locals[ident] = repr(session.instances_by_id[var.id])
            if found_start:
                # trim file path for python modules
                python_version = f"{sys.version_info.major}.{sys.version_info.minor}"
                if python_version in frame.filename:
                    frame.filename = frame.filename.split(python_version)[-1][1:]  # skip slash
                cleaned_stack.append(frame)
        return cleaned_stack


@reflect_enum("RunErrorKind", "Error type of a run")
class RunErrorKind(enum.StrEnum):
    Internal = "Internal"
    Parse = "Parse"
    Validation = "Validation"
    Runtime = "Runtime"
    Untrusted = "Untrusted"


# RunError/LogEntry and many others should be reflect types as well, but missing Statement and such
# @reflect_struct("RunError", "An error while running a statement")
@dataclass
class RunError(Exception):  # can this really be a subclass of Exception?
    """Wire-able representation of an exception."""

    kind: RunErrorKind
    type: str
    message: Optional[str] = None
    runnable: Optional["Statement"] = None
    traceback: list[RunCodeFrame] = None

    @staticmethod
    def from_exception(e: Exception, runnable: Optional["Statement"]) -> "RunError":
        if isinstance(e, RunError):
            return e
        stack = RunCodeFrame.from_stack(traceback.extract_tb(e.__traceback__))
        stack = RunCodeFrame.clean(stack, runnable, runnable.session)
        return RunError(
            kind=RunErrorKind.Runtime,
            type=type(e).__name__,
            message=str(e),
            runnable=runnable,
            traceback=stack,
        )


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
    run: Optional[Run] = None
    message: Optional[str] = None
    metadata: dict[str, Any] = None

    def __str__(self):
        return f"'{self.message}' ({self.created_at})"

    def __repr__(self):
        return f"<LogEntry {self}>"


class RunSearch(Search["RunData", Run]):
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

    def _unpack_element_data(self, element_data: "RunData") -> Run:
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


@dataclass
class MissingStatement:
    id: UUID
    type: Optional[StatementType] = None

    def __str__(self):
        return str(self.id)

    def __repr__(self):
        return f"<MissingStatement {self.id} {self.type or '<unknown type>'}>"

    def __getattr__(self, item):
        if item == "id":
            return self.id
        raise AttributeError(f"statement {self.id} not found, so {item} cannot be accessed")

import abc
import asyncio
import contextlib
import sys
from collections import deque
from concurrent.futures import Executor, ThreadPoolExecutor
from contextvars import ContextVar
from dataclasses import dataclass
from datetime import datetime
from itertools import chain
from typing import TYPE_CHECKING, Any, Awaitable, Callable, Coroutine, Optional, Union
from uuid import UUID, uuid4

import asgiref.sync
import structlog

from bench.language.builtin import active_session
from bench.language.const import (
    MNT,
    ModuleOp,
    RunStatus,
    SessionMode,
    TriggerType,
    TypeFlag,
    TypeTag,
)
from bench.language.mapping import check_type, map_value, pack_value, pack_value_flat
from bench.language.module import Module, ModuleNode, ScopeNode
from bench.language.query import Query, Sort, SortOrder
from bench.language.run import HasRun, LogEntry, Run, RunError
from bench.language.search import Search
from bench.language.statement import Statement
from bench.utils.dt import utcnow_with_tz
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import HasFields, Trigger
    from bench.language.mutate import MMT, ModuleMutation, ModuleMutator
    from bench.language.wire import LogEntryData, RunData

logger = structlog.get_logger(__name__)


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
        from bench.language.mutate import ModuleMutator
        from bench.language.remote import Storage

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

        self.anonymous_scope = ScopeNode(parent=self.module)
        self.executor = executor or ThreadPoolExecutor(max_workers=1)
        self.logger = logger.bind(session=self)
        self.mutator = ModuleMutator(self.module._source, ctx.project_id, module.id)
        self.tracer = SessionTracer(self, mutator=self.mutator)
        self.opened_at: Optional[datetime] = None
        self.closed_at: Optional[datetime] = None
        self._past_flushes: list[tuple[int, set[MMT]]] = []
        self._pending_flushes: list[tuple[int, Awaitable[bool]]] = []
        self._failed_flush: bool = False

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
        return self.tracer.current_run

    def capture_runs(self) -> "_RunCapture":
        return self.tracer.start_capture()

    def run_value(self, **kwargs):
        return self.tracer.value(**kwargs)

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
            self._failed_flush = True
            self.module._reset_from_source()
            if len(mutations) > 10:
                mutations_str = f"{mutations[:5]} ... {mutations[-5:]}"
            else:
                mutations_str = str(mutations)
            raise RuntimeError(f"failed to write {len(mutations)} mutations {mutations_str}")
        else:
            self.module._apply_source_mutations(mutations)
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
        assert not self._failed_flush, f"session {self!r} is broken after failed flush"
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
        if not self._failed_flush:
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
                asgiref.sync.async_to_sync(session.aopen)()
                return session

            def __exit__(self, exc_type, exc_value, traceback):
                asgiref.sync.async_to_sync(session.aclose)()

        return SyncSession()


class Tracer(abc.ABC):
    """
    Trace and track everything in a module/session (runs, mutations, etc.).
    """

    # module

    def node_create(self, *nodes: ModuleNode):
        raise NotImplementedError

    def node_update(self, node: ModuleNode, properties: list[str]):
        raise NotImplementedError

    def node_delete(self, *node: ModuleNode):
        raise NotImplementedError

    def node_truncate(self, node: ModuleNode, mnt: MNT):
        raise NotImplementedError

    # session

    def run_enter(self, statement: HasRun, inputs: dict):
        raise NotImplementedError

    def run_exit(self, statement: HasRun, outputs: dict):
        raise NotImplementedError

    def run_cached(
        self,
        statement: HasRun,
        inputs: dict,
        outputs: dict,
        generated_at: datetime,
        generated_in: UUID,
        duration: float,
    ):
        raise NotImplementedError

    def run_exception(self, statement: HasRun, exception: Exception):
        raise NotImplementedError


# TODO @Performance: improve performance of contextual stdout/stderr capture

stderr_track: ContextVar[Callable[[str], None] | None] = ContextVar("stderr_track", default=None)
stdout_track: ContextVar[Callable[[str], None] | None] = ContextVar("stdout_track", default=None)


class _ContextRedirectedStream:
    """Redirect stdout/stderr for dual-writing to context-specific track functions."""

    def __init__(self, native, contextvar: ContextVar[Callable[[str], None]]):
        self.native = native
        self.contextvar = contextvar
        self._just_saw_newline = False

    def write(self, data: str) -> int:
        ret = self.native.write(data)
        track = self.contextvar.get()
        if track and (data != "\n" or self._just_saw_newline):
            # TODO @Robustness: figure out better way of collecting stdout/stderr
            #  This is very hacky because we don't know who called print and want to skip
            #  some of our own log messages. Unfortunately we can't just trivially
            #  provide a custom 'print' since many libraries use the real 'print' internally.
            if not ("[debug    ]" in data or "[info     ]" in data):
                track(data)
        self._just_saw_newline = data == "\n"
        return ret

    def flush(self) -> None:
        self.native.flush()


def redirect_streams_if_needed():
    """Redirect stdout/stderr to the current context's track functions if they are set."""
    if not isinstance(sys.stdout, _ContextRedirectedStream):
        sys.stdout = _ContextRedirectedStream(sys.stdout, stdout_track)
    if not isinstance(sys.stderr, _ContextRedirectedStream):
        sys.stderr = _ContextRedirectedStream(sys.stderr, stderr_track)


class LogCollector:
    def __init__(self, track: Callable[[LogEntry], None], stream: str, session: "Session"):
        self.track = track
        self.session = session
        self.stream = stream
        self.module_id = session.module.id

    def _track(self, message: str) -> None:
        active_run = _get_active_run()
        if active_run:
            runnable = active_run.runnable
            run = active_run
        else:
            runnable = None
            run = None
        log_entry = LogEntry(
            id=UUIDT(),
            module=self.session.module,
            created_at=utcnow_with_tz(),
            stream=self.stream,
            session=self.session,
            runnable=runnable,
            run=run,
            message=message,
        )
        self.track(log_entry)

    def start(self):
        redirect_streams_if_needed()
        if self.stream == "stderr":
            stderr_track.set(self._track)
        elif self.stream == "stdout":
            stdout_track.set(self._track)
        else:
            raise ValueError(f"invalid stream: {self.stream}")

    def stop(self):
        if self.stream == "stderr":
            stderr_track.set(None)
        elif self.stream == "stdout":
            stdout_track.set(None)
        else:
            raise ValueError(f"invalid stream: {self.stream}")


SESSION_FLUSH_INTERVAL = 0.1
LOG_CACHE_SIZE = 1000
MAX_STACK_DEPTH = 16


class SessionTracer(Tracer):
    def __init__(self, session: Session, mutator: "ModuleMutator"):
        self.session = session
        self._cached_logs: deque[LogEntry] = deque(maxlen=LOG_CACHE_SIZE)
        self._pending_logs: list[LogEntry] = []
        self._pending_runs: dict[UUID, Run] = {}
        self._flush_cancel: asyncio.Event | None = None
        self._flush_task: asyncio.Task | None = None

        self.stdout_collector = LogCollector(self._track_log, "stdout", session)
        self.stderr_collector = LogCollector(self._track_log, "stderr", session)
        self.session = session
        self.mutator = mutator
        self.stacktrace = []
        self.runs = {}

    def __str__(self):
        return f"{len(self.stacktrace)} stack, {len(self.runs)} runs"

    def __repr__(self):
        return f"<RunTracer {self}>"

    #
    # Module
    # Mutations are actually written to local source in Session._do_flush.
    #

    def node_create(self, *nodes: ModuleNode):
        self.mutator.create(*nodes, apply=False)

    def node_update(self, node: ModuleNode, properties: list[str]):
        self.mutator.update(node, properties=properties, apply=False)

    def node_delete(self, *node: ModuleNode):
        self.mutator.delete(*node, apply=False)

    def node_truncate(self, node: ModuleNode, mnt: MNT):
        self.mutator.truncate(node, mnt, apply=False)

    #
    # Session
    #

    def _track_run(self, run: Run):
        # replace if already exists by id (runs are updated)
        self.runs[run.id] = run
        self._pending_runs[run.id] = run

    @property
    def current_run(self) -> Optional[Run]:
        if self.stacktrace:
            return self.stacktrace[-1]
        return None

    @property
    def cached_logs(self) -> list[LogEntry]:
        return list(self._cached_logs)

    @property
    def pending_logs(self) -> list[LogEntry]:
        return self._pending_logs

    def _track_log(self, log: LogEntry):
        self._pending_logs.append(log)
        self._cached_logs.append(log)

    def pop_stacktrace(self) -> Run:
        run = self.stacktrace.pop()
        # update cached info in parent(s)
        if run.value.cached_at is not None:
            self._update_cached_info()
        return run

    def run_enter(self, statement: "Statement", inputs):
        # we set invalid values to none here unlike in other packing places because
        #  these values may be written even if invalid
        run = self._create_run(
            runnable=statement,
            inputs=pack_value(inputs, statement, is_output=False, none_if_invalid=True),
        )
        self.stacktrace.append(run)
        _set_active_run(run)
        self._track_run(run)  # tracker may mutate/do other things, so log after it's run
        logger.debug("trace.run.enter", run=run, stackdepth=len(self.stacktrace))

        # pre-run validation
        try:
            if len(self.stacktrace) >= MAX_STACK_DEPTH:
                raise RunError(f"stack depth exceeded: {MAX_STACK_DEPTH}")
            check_type(inputs, statement, is_output=False)
        except BaseException as e:
            e = RunError.from_exception(e, statement)
            self.run_exception(statement, e)
            raise e

    def run_exit(self, statement: "Statement", outputs):
        run = self.pop_stacktrace()
        run.terminated_at = utcnow_with_tz()
        run.outputs = _pack_and_truncate_value(
            outputs, statement, is_output=True, none_if_invalid=True
        )
        run.status = RunStatus.Completed
        self._track_run(run)
        _clear_active_run(run)
        logger.debug("trace.run.exit", run=run, stackdepth=len(self.stacktrace))

        # post-run validation
        try:
            check_type(outputs, statement, is_output=False)
        except BaseException as e:
            e = RunError.from_exception(e, statement)
            self.run_exception(statement, e)
            raise e

    def run_exception(self, statement: "Statement", exception: Exception):
        run = self.pop_stacktrace()
        run.terminated_at = utcnow_with_tz()
        if isinstance(exception, asyncio.CancelledError):
            run.status = RunStatus.Aborted
        else:
            run.status = RunStatus.Failed
            run.error = RunError.from_exception(exception, statement)
        self._track_run(run)
        _clear_active_run(run)
        logger.debug("trace.run.exception", run=run, stackdepth=len(self.stacktrace))

    def run_cached(
        self,
        statement: "Statement",
        inputs,
        outputs,
        generated_at: datetime,
        generated_in: UUID,
        duration: float,
    ):
        run = self._create_run(runnable=statement, trace=True)
        run.terminated_at = utcnow_with_tz()
        run.inputs = _pack_and_truncate_value(
            inputs, statement, is_output=False, none_if_invalid=True
        )
        run.outputs = _pack_and_truncate_value(
            outputs, statement, is_output=True, none_if_invalid=True
        )
        run.status = RunStatus.Completed
        run.value.cached_at = generated_at
        run.value.cached_in = generated_in
        run.value.cached_duration = duration
        custom_value = _custom_value.get()
        for k, v in (custom_value or {}).items():
            run.value[k] = v
        self._track_run(run)
        self._update_cached_info()
        logger.debug("trace.run.cached", run=run, stackdepth=len(self.stacktrace))

    def _update_cached_info(self):
        for run in self.stacktrace:
            run.value.cached_at = min(
                r.value.cached_at for r in run.walk_descendants() if r.value.cached_at
            )
            run.value.cached_duration = sum(
                r.value.cached_duration for r in run.walk_descendants() if r.value.cached_duration
            )

    def _create_run(
        self,
        runnable: Optional[Statement] = None,
        inputs: dict[str, Any] | None = None,
        queue_position: int | None = None,
        trace: bool = True,
        trigger_type: TriggerType | None = None,
        trigger: Union["Trigger", UUID, None] = None,
    ):
        active_run = _get_active_run()
        if trace and active_run is not None:
            root = active_run.root or active_run
            parent = active_run
        else:
            root = None
            parent = None
        if parent:
            trigger_type = trigger_type or TriggerType.INVOKE
        if not trigger_type and root is None:
            # inherit trigger type from session if we're not nested
            # (this will be wrong once we process other triggers within a session)
            trigger_type = self.session.ctx.trigger_type
            trigger = self.session.ctx.trigger_id
        run = Run(
            id=self.session.ctx.first_run_id if root is None else UUIDT(),
            module=self.session.module,
            runnable=runnable,
            session=self.session,
            trigger_type=trigger_type,
            trigger=trigger,
            root=root,
            parent=parent,
            scheduled_at=None,
            started_at=utcnow_with_tz(),
            terminated_at=None,
            inputs=inputs,
            outputs=None,
            error=None,
            status=RunStatus.Queued if queue_position is not None else RunStatus.Running,
            value={},
        )
        run._activate_inner(self.session, queue_position=queue_position)
        if parent is not None:
            parent.children.append(run)
        custom_value = _custom_value.get()
        for k, v in (custom_value or {}).items():
            run.value[k] = v
        return run

    @contextlib.contextmanager
    def capture(self) -> "_CapturedRuns":
        """Get all runs that are created within the context."""
        start_ids = set(self.runs.keys())
        capture = _CapturedRuns()
        try:
            yield
        finally:
            capture.runs = [run for run in self.runs.values() if run.id not in start_ids]

    @contextlib.contextmanager
    def value(self, **kwargs):
        """Set custom value for all runs created within the context."""
        old = _custom_value.get() or {}
        _custom_value.set({**old, **kwargs})
        try:
            yield
        finally:
            _custom_value.set(old or None)

    def start_capture(self) -> "_RunCapture":
        """Start capturing runs."""
        capture = _RunCapture(self)
        capture.start()
        return capture

    async def _flush(self, force: bool = False, kill_pending: bool = False) -> None:
        """Flushes session data."""
        if not force and not self._pending_logs and not self._pending_runs:
            return  # skip if nothing to flush

        logs = self._pending_logs[:]
        runs = list(self._pending_runs.values())
        self._pending_logs.clear()
        self._pending_runs.clear()

        if kill_pending:
            # abort any remaining active runs
            for run in chain(runs, self.runs.values()):
                run.mark_dead_if_active()

        success = await self.session.writer.write_session(self.session, runs, logs)
        if not success:
            raise RuntimeError(f"failed to write session {self.session}")

    async def open(self, flush_interval: float = SESSION_FLUSH_INTERVAL):
        self.stdout_collector.start()
        self.stderr_collector.start()

        _cancel = asyncio.Event()

        async def _flush_loop():
            while not _cancel.is_set():
                await self._flush()
                await asyncio.sleep(flush_interval)

        self._flush_cancel = _cancel
        self._flush_task = asyncio.create_task(_flush_loop())

    async def close(self):
        self.stdout_collector.stop()
        self.stderr_collector.stop()

        self._flush_cancel.set()
        await self._flush(force=True, kill_pending=True)  # flush pending data


# We track the active root in a contextvar but not children
#  because they may be in different contexts, and we cannot reset across contexts.
# This will need to be expanded when we get to parallel runs.
_active_root_run: ContextVar[Run | None] = ContextVar("active_root_run", default=None)
_active_run_by_root: dict[UUID, Run] = {}
_custom_value: ContextVar[dict[str, Any] | None] = ContextVar("custom_value", default=None)


def _get_active_run() -> Run | None:
    root = _active_root_run.get()
    if root is not None:
        return _active_run_by_root[root.id]
    return None


def _clear_active_run(run: Run):
    root = run.root or run
    if root.id in _active_run_by_root:
        if run.parent is None:
            del _active_run_by_root[root.id]
        else:
            _active_run_by_root[root.id] = run.parent
    if _active_root_run.get() == run:
        _active_root_run.set(None)


def _set_active_run(run: Run):
    root = run.root or run
    _active_run_by_root[root.id] = run
    if _active_root_run.get() is None:
        _active_root_run.set(root)


def _pack_and_truncate_value(
    value: Any,
    type: Statement,
    ignore_array: bool = False,
    ignore_outer_map: bool = False,
    none_if_invalid: bool = False,
    is_output: bool = None,
) -> Any:
    def _is_type_truncated(type: "HasFields") -> bool:
        return type.tag in (TypeTag.VECTOR,)

    def _truncate_value(value: Any, type: "HasFields", *args, **kwargs) -> Any:
        if _is_type_truncated(type):
            if type.flags & TypeFlag.IsArrayable or type.flags & TypeFlag.IsArray:
                return []
            return None
        return value

    return map_value(
        value=value,
        type=type,
        map_k=lambda f: (f.py_ident, f.typed_key),
        map_v=pack_value_flat,
        premap_v=_truncate_value,
        ignore_array=ignore_array,
        ignore_outer_map=ignore_outer_map,
        none_if_invalid=none_if_invalid,
        is_output=is_output,
    )


@dataclass
class _CapturedRuns:
    runs: list[Run] = None


class _RunCapture:
    def __init__(self, tracer: SessionTracer):
        self.tracer = tracer
        self._start_ids: set[UUID] | None = None

    def start(self):
        self._start_ids = set(self.tracer.runs.keys())

    def stop(self) -> list[Run]:
        runs = [run for run in self.tracer.runs.values() if run.id not in self._start_ids]
        self._start_ids = None
        return runs

    def stop_one_or_none(self) -> Run | None:
        runs = self.stop()
        if len(runs) == 0:
            return None
        if len(runs) > 1:
            raise ValueError(f"expected 1 run, got {len(runs)}")
        return runs[0]


@dataclass
class LazyRun:
    """Run that's not loaded."""

    id: UUID

    def load(self) -> "Run":
        raise NotImplementedError


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

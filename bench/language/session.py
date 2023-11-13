import abc
import asyncio
import contextlib
import sys
import threading
from collections import deque
from concurrent.futures import ThreadPoolExecutor
from contextvars import ContextVar
from dataclasses import dataclass
from datetime import datetime
from itertools import chain
from typing import TYPE_CHECKING, Any, Awaitable, Callable, Coroutine, Optional, Union
from uuid import UUID, uuid4

import asgiref.sync
import structlog

from bench.language.builtin import _active_session, symbolx_lib
from bench.language.const import (
    INTERP_NODE_TYPES,
    MNT,
    NTL,
    RunStatus,
    SessionAccessLevel,
    TriggerType,
    TypeFlag,
    TypeTag,
)
from bench.language.module import Module, Node
from bench.language.packer import check_type, map_value, pack_value, pack_value_flat
from bench.language.run import LogEntry, Run, RunError
from bench.language.statement import Statement
from bench.utils.dt import utcnow_with_tz
from bench.utils.utils import DEBUG
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import HasFields, Trigger
    from bench.language.edit import MET, EditData, ModuleEditor

logger = structlog.get_logger(__name__)

SESSION_EDIT_FLUSH_WATERMARK = 512


class ModuleWriter(abc.ABC):
    """Base for writing module/session for type-checking."""

    async def write_module(self, edits: list["EditData"], refresh_index: bool) -> bool:
        raise NotImplementedError

    async def write_session(
        self, session: "Session", runs: list["Run"], logs: list["LogEntry"]
    ) -> bool:
        # nocheckin: write logs directly to os
        # and refactor/rename 'ModuleWriter' concept (module synchronizer?)
        raise NotImplementedError


class Session:
    """A managed context for running a Bench module."""

    def __init__(
        self,
        module: Module,
        writer: ModuleWriter,
        access_level: SessionAccessLevel,
        worker_node_id: str,
        worker_process_id: Optional[str],
        trigger_type: TriggerType,
        trigger_id: Optional[UUID] = None,
        id: UUID = None,
        root_run_id: Optional[UUID] = None,
        root_run_value: dict = None,
        global_run_value: dict = None,
        cache_inferences: bool = True,
        inference_timeout: int = 300,
        inference_retries: int = 5,
    ):
        from bench.language.cache import CacheAsync, CacheSync
        from bench.language.edit import ModuleEditor
        from bench.language.remote import Blobs

        self.id = id or uuid4()
        self.module = module
        self.worker_node_id = worker_node_id
        self.worker_process_id = worker_process_id
        self.trigger_type = trigger_type
        self.trigger_id = trigger_id
        self.cache_inferences = cache_inferences
        self.inference_timeout = inference_timeout
        self.inference_retries = inference_retries
        self.access_level = access_level
        self._writer = writer

        self.cache_sync = CacheSync(module)
        self.cache_async = CacheAsync(module)
        self.blobs = Blobs(module)

        self._executor = ThreadPoolExecutor(max_workers=1)
        self._log = logger.bind(session=self)
        self._editor = ModuleEditor(self.module._local_tree, module.project_id, module.id)
        self._tracer = SessionTracer(
            self,
            editor=self._editor,
            root_run_id=root_run_id,
            root_run_value=root_run_value,
            global_run_value=global_run_value,
        )

        self._opened_at: Optional[datetime] = None
        self._closed_at: Optional[datetime] = None
        self._dangling_nodes_by_ck: dict[UUID, Node] = {}
        self._past_flushes: list[tuple[int, set[MET]]] = []
        self._pending_flushes: list[tuple[int, Awaitable[bool]]] = []
        self._failed_flush: bool = False

    def __str__(self):
        status = "open" if self._opened_at else ("closed" if self._closed_at else "pending")
        return f"{self.module.name} ({self.access_level.name}, {status}, {len(self._editor.edits)} pending edits)"

    def __repr__(self):
        return f"<Session {self}>"

    def sync_to_async(self, fn: Callable) -> Callable[..., Awaitable]:
        return asgiref.sync.sync_to_async(fn, thread_sensitive=False, executor=self._executor)  # type: ignore

    def async_to_sync(self, fn: Awaitable | Callable | Coroutine) -> Callable:
        return asgiref.sync.async_to_sync(fn)  # type: ignore

    @property
    def current_run(self) -> "Run":
        return self._tracer.current_run

    def capture_runs(self) -> "_RunCapture":
        return self._tracer.start_capture()

    def bind_run_value(self, **kwargs):
        return self._tracer.value(**kwargs)

    @contextlib.contextmanager
    def bind_access_level(self, access_level: SessionAccessLevel):
        if access_level > self.access_level:
            raise PermissionError(
                f"cannot increase access level from {self.access_level} to {access_level}"
            )
        old_access = self.access_level
        self.access_level = access_level
        try:
            yield
        finally:
            self.access_level = old_access

    @property
    def dangling(self) -> list[Node]:
        return [n for n in self._dangling_nodes_by_ck.values() if not n.parent]

    def dangling_like(self, type: type[Node]) -> list[Node]:
        return [n for n in self.dangling if isinstance(n, type)]

    @property
    def is_open(self) -> bool:
        return self._opened_at is not None and self._closed_at is None

    async def aopen(self):
        """Opens the session for execution and modification."""
        if self._opened_at is not None:
            raise RuntimeError(f"session already opened {self}")
        self._opened_at = utcnow_with_tz()
        if _active_session.get() is not None:
            raise RuntimeError(f"another session is active: {_active_session.get()}")
        _active_session.set(self)
        await self._tracer.open()
        logger.debug("session.open", session=self)

    async def _do_commit(self, edits: list["EditData"], refresh_index: bool) -> bool:
        """Flush any pending edits to the module"""
        if not edits and not refresh_index:
            return True  # skip if no edits and no index refresh
        # TODO @Robustness: auto-split edits if not in atomic block and too large
        success = await self._writer.write_module(edits, refresh_index)
        if not success:
            self._failed_flush = True
            self.module._reset_from_source()
            if len(edits) > 10:
                edits_str = f"{edits[:5]} ... {edits[-5:]}"
            else:
                edits_str = str(edits)
            raise RuntimeError(f"failed to write {len(edits)} edits {edits_str}")
        else:
            self.module._apply_edits_to_source(edits)
        logger.debug("session.flush.done", session=self, editor=self._editor)
        return success

    @property
    def _needs_flush_before_exit(self):
        # ensure edits are flushed before we exit out of topmost run for error propagation
        return self._editor.edits and len(self._tracer.stacktrace) == 1

    async def acommit(self, optimistic: bool = False, refresh_index: bool = False):
        """
        Flushes all module edits.
        If optimistic, this will return before the flush is complete (but will wait on close).
        """
        edits = [e for e in self._editor.edits if e.mnt not in INTERP_NODE_TYPES]  # :InterpFilter
        if not edits and not refresh_index:
            return  # skip if no edits and no index refresh
        assert not self._failed_flush, f"session {self!r} is broken after failed flush"

        from bench.language.edit import EditBundle

        logger.debug(
            "session.flush",
            session=self,
            editor=self._editor,
            optimistic=optimistic,
            refresh_index=refresh_index,
        )
        edits = EditBundle(edits).compact()
        self._editor.reset()
        self._tracer._new_statement_ids.clear()
        flush = self._do_commit(edits, refresh_index)
        if optimistic:
            self._pending_flushes.append((len(edits), asyncio.create_task(flush)))
        else:
            await flush
        self._past_flushes.append((len(edits), set(m.type for m in edits)))

    def commit(self, optimistic: bool = False):
        if not self._editor.edits:
            return
        asgiref.sync.async_to_sync(self.acommit)(optimistic=optimistic)

    async def aclose(self):
        """Closes the session, flushing any edits and preventing further execution/edit."""
        if self._closed_at is not None:
            raise RuntimeError(f"session already closed {self}")
        self._closed_at = utcnow_with_tz()
        if not self._failed_flush:
            await self.acommit(optimistic=True)
        # TODO @Robustness: flush pending edits inside top level run (to report errors properly)
        # await all pending flushes
        pending_edits_count = sum(count for count, _ in self._pending_flushes)
        logger.debug("session.close.pending", session=self, pending_edits_count=pending_edits_count)
        await asyncio.gather(*(task for _, task in self._pending_flushes))
        _active_session.set(None)
        await self._tracer.close()
        if self.dangling:
            logger.warn("session.close.dangling", session=self, dangling=self.dangling)
        logger.debug("session.close", session=self)

    def close(self):
        asgiref.sync.async_to_sync(self.aclose)()

    def _on_mutated(self, editor: "ModuleEditor", edit: "EditData"):
        if len(self._editor.edits) > SESSION_EDIT_FLUSH_WATERMARK:
            self.commit(optimistic=True)

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


def _redirect_std_streams_if_needed():
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
            statement = active_run.statement
            run = active_run
        else:
            statement = None
            run = None
        log_entry = LogEntry(
            id=UUIDT(),
            module=self.session.module,
            created_at=utcnow_with_tz(),
            stream=self.stream,
            session=self.session,
            statement=statement,
            run=run,
            message=message,
        )
        self.track(log_entry)

    def start(self):
        _redirect_std_streams_if_needed()
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
MAX_STACK_DEPTH = 8 if DEBUG else 16


class PermissionError(Exception):
    pass


class SessionTracer:
    def __init__(
        self,
        session: Session,
        editor: "ModuleEditor",
        root_run_id: UUID = None,
        root_run_value: dict = None,
        global_run_value: dict = None,
    ):
        self.session = session
        self._cached_logs: deque[LogEntry] = deque(maxlen=LOG_CACHE_SIZE)
        self._pending_logs: list[LogEntry] = []
        self._pending_runs: dict[UUID, Run] = {}
        self._commit_cancel: asyncio.Event | None = None
        self._commit_task: asyncio.Task | None = None
        self._root_run_id = root_run_id
        self._new_statement_ids: set[UUID] = set()

        from bench.language.packer import unpack_value

        RunMetadata = symbolx_lib.resolve(".reflect.RunMetadata")
        self._root_run_value = unpack_value(
            root_run_value, RunMetadata, map_k=lambda f: (f.py_ident, f.py_ident)
        )
        self._global_run_value = unpack_value(
            global_run_value, RunMetadata, map_k=lambda f: (f.py_ident, f.py_ident)
        )

        self.stdout_collector = LogCollector(self._track_log, "stdout", session)
        self.stderr_collector = LogCollector(self._track_log, "stderr", session)
        self.session = session
        self.editor = editor
        self._stacktrace: list[Run] = []
        # used to prevent deleting ancestors of running statements
        self._stacktrace_ancestors_cks: dict[UUID, Statement] = {}
        self._tracing_lock = threading.Lock()
        self.runs = {}

    def __str__(self):
        return f"{len(self.stacktrace)} stack, {len(self.runs)} runs"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def stacktrace(self):
        return self._stacktrace

    def _update_stacktrace_ancestors(self):
        self._stacktrace_ancestors_cks.clear()
        for run in self._stacktrace:
            parent = run.statement
            while parent is not None and parent.ck not in self._stacktrace_ancestors_cks:
                self._stacktrace_ancestors_cks[parent.ck] = run.statement
                parent = parent.parent

    def _stacktrace_pop(self) -> Run:
        run = self._stacktrace.pop()
        self._update_stacktrace_ancestors()
        return run

    def _stacktrace_push(self, run: Run) -> None:
        self._stacktrace.append(run)
        self._update_stacktrace_ancestors()

    #
    # Module
    # Edits are actually written to local source in Session._do_commit.
    # We don't track interp edits here because they're manually handled in runtime.
    #

    def node_create(self, *nodes: Node):
        # :InterpFilter
        nodes = [n for n in nodes if n.mnt not in INTERP_NODE_TYPES and n._track & NTL.FULL]
        if nodes and self.session.access_level < SessionAccessLevel.Create:
            raise PermissionError(f"{self.session!r} may not create {nodes!r}")
        self.editor.create_many(*nodes, apply=False)
        for n in nodes:
            if n.ck in self.session._dangling_nodes_by_ck:
                del self.session._dangling_nodes_by_ck[n.ck]
            if n.mnt == MNT.STATEMENT:
                self._new_statement_ids.add(n.id)

    def node_create_preflight(self, *nodes: Node):
        # used to check permission before modifying state locally
        # (only for create since this is the only edit fired 'after' making an irreversible change)
        nodes = [n for n in nodes if n.mnt not in INTERP_NODE_TYPES and n._track & NTL.FULL]
        if nodes and self.session.access_level < SessionAccessLevel.Create:
            raise PermissionError(f"{self.session!r} may not create {nodes!r}")

    def node_update(self, node: Node, properties: list[str]):
        if node.mnt not in INTERP_NODE_TYPES and node._track & NTL.FULL:  # :InterpFilter
            if self.session.access_level < SessionAccessLevel.Update:
                raise PermissionError(f"{self.session!r} may not update {node!r}")
            self.editor.update(node, properties=properties, apply=False)

    def node_delete(self, *nodes: Node):
        # :InterpFilter
        nodes = [n for n in nodes if n.mnt not in INTERP_NODE_TYPES and n._track & NTL.FULL]
        if nodes and self.session.access_level < SessionAccessLevel.Delete:
            raise PermissionError(f"{self.session!r} may not delete {nodes!r}")
        # ensure node is not ancestor of any running statements
        if any(n.ck in self._stacktrace_ancestors_cks for n in nodes):
            ancestor = next(n for n in nodes if n.ck in self._stacktrace_ancestors_cks)
            statement = self._stacktrace_ancestors_cks[ancestor.ck]
            if ancestor == statement:
                raise RuntimeError(f"cannot delete running statement {statement!r}")
            else:
                raise RuntimeError(
                    f"cannot delete ancestor {ancestor!r} of running statement: {statement!r}"
                )
        self.editor.delete_many(*nodes, apply=False)

    def node_truncate(self, node: Node, mnt: MNT):
        if node.mnt not in INTERP_NODE_TYPES and node._track & NTL.FULL:  # :InterpFilter
            if self.session.access_level < SessionAccessLevel.Delete:
                raise PermissionError(f"{self.session!r} may not truncate {node!r}")
            self.editor.truncate(node, mnt, apply=False)

    #
    # Session
    #

    def _track_run(self, run: Run):
        # replace if already exists by id (runs are updated)
        self.runs[run.id] = run
        self._pending_runs[run.id] = run

    @property
    def current_run(self) -> Optional[Run]:
        if self._stacktrace:
            return self._stacktrace[-1]
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
        run = self._stacktrace_pop()
        # update cached info in parent(s)
        if run.value.cached_at is not None:
            self._update_cached_info()
        return run

    def run_enter(self, statement: "Statement", is_async: bool, inputs):
        # we set invalid values to none here unlike in other packing places because
        #  these values may be written even if invalid
        run = self._create_run(
            statement=statement,
            inputs=pack_value(inputs, statement, is_output=False, none_if_invalid=True),
            _is_async=is_async,
        )
        with self._tracing_lock:
            self._stacktrace_push(run)
            _set_active_run(run)
            self._track_run(run)  # tracker may mutate/do other things, so log after it's run
        logger.debug("trace.run.enter", run=run, stackdepth=len(self.stacktrace))

        # pre-run validation
        try:
            if len(self.stacktrace) >= MAX_STACK_DEPTH:
                raise RecursionError(f"maximum stack depth exceeded: {MAX_STACK_DEPTH}")
            check_type(inputs, statement, is_output=False)
        except BaseException as e:
            self.run_exception(statement, e)
            raise e

    def run_exit(self, statement: "Statement", outputs):
        # post-run validation
        try:
            check_type(outputs, statement, is_output=True)
        except BaseException as e:
            self.run_exception(statement, e)
            raise e

        with self._tracing_lock:
            run = self.pop_stacktrace()
            assert run.statement == statement, f"bad stack in {self!r}: {run!r} got {statement!r}"
            run.terminated_at = utcnow_with_tz()
            run.outputs = _pack_and_truncate_value(
                outputs, statement, is_output=True, none_if_invalid=True
            )
            run.status = RunStatus.Completed
            self._track_run(run)
            _clear_active_run(run)
        logger.debug("trace.run.exit", run=run, stackdepth=len(self.stacktrace))

    def run_exception(self, statement: "Statement", exception: BaseException):
        with self._tracing_lock:
            run = self.pop_stacktrace()
            assert run.statement == statement, f"bad stack in {self!r}: {run!r} got {statement!r}"
            run.terminated_at = utcnow_with_tz()
            run.error = RunError.from_exception(exception, statement)
            if isinstance(exception, asyncio.CancelledError):
                run.status = RunStatus.Aborted
            else:
                run.status = RunStatus.Failed
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
        run = self._create_run(statement=statement, trace=True)
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
        with self._tracing_lock:
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
        statement: Optional[Statement] = None,
        inputs: dict[str, Any] | None = None,
        queue_position: int | None = None,
        trace: bool = True,
        trigger_type: TriggerType | None = None,
        trigger: Union["Trigger", UUID, None] = None,
        _is_async: bool = False,
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
            trigger_type = self.session.trigger_type
            trigger = self.session.trigger_id
        run = Run(
            id=self._root_run_id if root is None else UUIDT(),
            module=self.session.module,
            statement=statement,
            statement_path=None,
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
            value=(self._root_run_value or {}) if root is None else {},
            _is_async=_is_async,
        )
        run._activate_inner(self.session, queue_position=queue_position)
        if parent is not None:
            parent.children.append(run)
        custom_value = _custom_value.get()
        if root is None and self._root_run_value:
            run.value.update(self._root_run_value)
        if custom_value:
            run.value.update(custom_value)
        if self._global_run_value:
            run.value.update(self._global_run_value)
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

    async def _commit(self, force: bool = False, kill_pending_runs: bool = False) -> None:
        """Flushes session data."""
        if not force and not self._pending_logs and not self._pending_runs:
            return  # skip if nothing to commit

        with self._tracing_lock:
            logs = self._pending_logs[:]
            self._pending_logs.clear()
            runs = list(self._pending_runs.values())
            self._pending_runs.clear()

            if kill_pending_runs:
                # abort any remaining active runs
                for run in chain(runs, self.runs.values()):
                    run._mark_dead_if_active()

        # force commit module as well if a new statement was run
        #  (since we need those field mappings, lest OS errors)
        if any(r.statement_id in self._new_statement_ids for r in runs):
            await self.session.acommit(optimistic=False, refresh_index=False)

        success = await self.session._writer.write_session(self.session, runs, logs)
        if not success:
            raise RuntimeError(f"failed to write session {self.session}")

    async def open(self, commit_interval: float = SESSION_FLUSH_INTERVAL):
        self.stdout_collector.start()
        self.stderr_collector.start()

        _cancel = asyncio.Event()

        async def _commit_loop():
            while not _cancel.is_set():
                await self._commit()
                await asyncio.sleep(commit_interval)

        self._commit_cancel = _cancel
        self._commit_task = asyncio.create_task(_commit_loop())

    async def close(self):
        self.stdout_collector.stop()
        self.stderr_collector.stop()

        self._commit_cancel.set()
        await self._commit(force=True, kill_pending_runs=True)  # commit pending edits


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
            if type.flags & TypeFlag.IS_ARRAYABLE or type.flags & TypeFlag.IS_ARRAY:
                return []
            return None
        return value

    return map_value(
        value=value,
        type=type,
        map_k=lambda f: (f.py_ident, f._typed_key),
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

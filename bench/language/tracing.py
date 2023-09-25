from __future__ import annotations

import asyncio
import contextlib
import sys
from collections import deque
from contextvars import ContextVar
from dataclasses import dataclass
from datetime import datetime
from itertools import chain
from typing import TYPE_CHECKING, Any, Callable, Optional
from uuid import UUID

import structlog

from bench.language.const import RunStatus, TriggerType, TypeFlag, TypeTag
from bench.language.mapping import map_value, pack_value, pack_value_flat
from bench.language.module import ModuleNode
from bench.language.mutate import ModuleMutator
from bench.language.run import HasRun, Run, RunError
from bench.language.session import LogEntry, Session
from bench.language.statement import Statement
from bench.utils.dt import utcnow_with_tz
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import HasFields, Task, Trigger

logger = structlog.get_logger(__name__)


class Tracer:
    """
    Trace and track everything in a module/session (runs, mutations, etc.).
    """

    # module

    def node_create(self, node: ModuleNode):
        pass

    def node_update(self, node: ModuleNode, properties: list[str]):
        pass

    def node_move(self, node: ModuleNode):
        pass

    def node_delete(self, node: ModuleNode):
        pass

    # session

    def run_enter(self, statement: HasRun, inputs: dict):
        pass

    def run_exit(self, statement: HasRun, outputs: dict):
        pass

    def run_cached(
        self,
        statement: HasRun,
        inputs: dict,
        outputs: dict,
        generated_at: datetime,
        generated_in: UUID,
        duration: float,
    ):
        pass

    def run_exception(self, statement: HasRun, exception: Exception):
        pass


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


class SessionTracer(Tracer):
    def __init__(self, session: Session, mutator: ModuleMutator):
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

    def _track_run(self, run: Run):
        # replace if already exists by id (runs are updated)
        self.runs[run.id] = run
        self._pending_runs[run.id] = run

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
        # nocheckin: check run_enter/run_exit types again
        self.stacktrace.append(run)
        _set_active_run(run)
        self._track_run(run)  # tracker may mutate/do other things, so log after it's run
        logger.debug("trace.run.enter", run=run, stackdepth=len(self.stacktrace))

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
        trigger: Trigger | UUID | None = None,
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
        run._activate(self.session, queue_position=queue_position)
        if parent is not None:
            parent.children.append(run)
        custom_value = _custom_value.get()
        for k, v in (custom_value or {}).items():
            run.value[k] = v
        return run

    @contextlib.contextmanager
    def capture(self) -> _CapturedRuns:
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

    def start_capture(self) -> _RunCapture:
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
    def _is_type_truncated(type: HasFields) -> bool:
        return type.tag in (TypeTag.VECTOR,)

    def _truncate_value(value: Any, type: HasFields, *args, **kwargs) -> Any:
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

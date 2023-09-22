from __future__ import annotations

import asyncio
import contextlib
import sys
from collections import deque
from contextvars import ContextVar
from dataclasses import dataclass
from datetime import datetime
from typing import TYPE_CHECKING, Any, Callable, Optional
from uuid import UUID

import structlog

from bench.language.const import MNT, ModuleOp, RunStatus, TriggerType, TypeFlag, TypeTag
from bench.language.mapping import check_type, map_value, pack_value, pack_value_flat
from bench.language.mutate import ModuleMutator
from bench.language.query import Query, Sort
from bench.language.run import HasRun, Run, RunError
from bench.language.session import LogEntry, Session
from bench.language.statement import Statement
from bench.utils.dt import utcnow_with_tz
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import (
        Code,
        Database,
        Field,
        File,
        HasFields,
        HasValue,
        Model,
        Record,
        RemoteObject,
        Secret,
        Tagging,
        Task,
        Trigger,
        Variable,
    )

logger = structlog.get_logger(__name__)


class Tracer:
    """
    Trace and track everything in a module/session (runs, mutations, etc.).
    """

    # module

    def file_create(self, file: File):
        pass

    def statement_create(self, statement: Statement):
        pass

    def field_append(self, symbol: HasFields, field: Field):
        pass

    def value_update(self, value: Variable, key: Optional[str] = None):
        pass

    def database_clear(self, database: Database):
        pass

    def database_append(self, database: Database, record: Record):
        pass

    def database_extend(self, database: Database, records: list[Record]):
        pass

    def database_remove(self, database: Database, record: Record):
        pass

    def database_update(self, database: Database, record: Record, key: Optional[str] = None):
        pass

    # TODO @Broken: the below trace events aren't fired (or used) yet

    def tagging_set(self, statement: Statement, tagging: Tagging):
        pass

    def tagging_clear(self, statement: Statement, tagging: Tagging):
        pass

    def object_read(self, object: RemoteObject):
        pass

    def object_write(self, object: RemoteObject):
        pass

    def secret_reveal(self, secret: Secret):
        pass

    # execution

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


# TODO @Performance: check performance of contextual stdout/stderr redirect

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
    def __init__(
        self,
        session: Session,
        mutator: ModuleMutator = None,
        validate: bool = True,
    ):
        self.session = session
        self._cached_logs: deque[LogEntry] = deque(maxlen=LOG_CACHE_SIZE)
        self._pending_logs: list[LogEntry] = []
        self._pending_runs: list[Run] = []
        self._flush_cancel: asyncio.Event | None = None
        self._flush_task: asyncio.Task | None = None

        self.run = RunTracer(session=session, track=self._track_run)
        self.tracers: list[Tracer] = [self.run, PermissionCheckingTracer(session)]
        if validate:  # validation tracer must be last
            self.tracers.append(TypeCheckingTracer())
        if mutator:
            self.tracers.append(MutationTracer(mutator))
        self.stdout_collector = LogCollector(self._track_log, "stdout", session)
        self.stderr_collector = LogCollector(self._track_log, "stderr", session)

    def _track_run(self, run: Run):
        # replace if already exists by id (runs are updated)
        for i, existing in enumerate(self._pending_runs):
            if existing.id == run.id:
                self._pending_runs[i] = run
                return
        self._pending_runs.append(run)

    def _track_log(self, log: LogEntry):
        self._pending_logs.append(log)
        self._cached_logs.append(log)

    @property
    def cached_logs(self) -> list[LogEntry]:
        return list(self._cached_logs)

    @property
    def pending_logs(self) -> list[LogEntry]:
        return self._pending_logs

    async def _flush(self, force: bool = False, kill_pending: bool = False) -> None:
        """Flushes session data."""
        if not force and not self._pending_logs and not self._pending_runs:
            return  # skip if nothing to flush

        logs = self._pending_logs[:]
        runs = self._pending_runs[:]
        self._pending_logs.clear()
        self._pending_runs.clear()

        if kill_pending:
            # abort any remaining active runs
            for run in runs:
                if run.active:
                    run.terminated_at = utcnow_with_tz()
                    run.status = RunStatus.Aborted
            for run in self.run.runs.values():
                if run.active:
                    run.terminated_at = utcnow_with_tz()
                    run.status = RunStatus.Aborted
                    runs.append(run)

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

    def value_update(self, value: Statement, key: Optional[str] = None):
        for tracer in self.tracers:
            tracer.value_update(value, key)

    def database_clear(self, database: Database):
        for tracer in self.tracers:
            tracer.database_clear(database)

    def database_append(self, database: Database, record: Record):
        for tracer in self.tracers:
            tracer.database_append(database, record)

    def database_extend(self, database: Database, records: list[Record]):
        for tracer in self.tracers:
            tracer.database_extend(database, records)

    def database_remove(self, database: Database, record: Record):
        for tracer in self.tracers:
            tracer.database_remove(database, record)

    def database_update(self, database: Database, record: Record, key: Optional[str] = None):
        for tracer in self.tracers:
            tracer.database_update(database, record, key)

    def run_enter(self, statement: HasRun, inputs):
        for tracer in self.tracers:
            tracer.run_enter(statement, inputs)

    def run_exit(self, statement: HasRun, outputs):
        for tracer in reversed(self.tracers):
            tracer.run_exit(statement, outputs)

    def run_exception(self, statement: HasRun, exception: Exception):
        for tracer in reversed(self.tracers):
            try:
                tracer.run_exception(statement, exception)
            except BaseException:
                # internal error in tracer, very not good
                logger.exception("trace.run.exception", exc_info=True, tracer=tracer)

    def run_cached(
        self,
        statement: HasRun,
        inputs: dict,
        outputs: dict,
        generated_at: datetime,
        generated_in: UUID,
        duration: float,
    ):
        for tracer in reversed(self.tracers):
            tracer.run_cached(statement, inputs, outputs, generated_at, generated_in, duration)


def _pack_and_truncate_value(
    value: Any,
    type: HasFields,
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


class RunTracer(Tracer):
    """
    A run tracer that records code and model executions.
    """

    def __init__(self, session: Session, track: Callable[[Run], None]):
        self.session = session
        self.stacktrace = []
        self._track = track
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

    def track(self, run: Run):
        self.runs[run.id] = run
        self._track(run)

    def pop_stacktrace(self) -> Run:
        run = self.stacktrace.pop()
        # update cached info in parent(s)
        if run.value.cached_at is not None:
            self._update_cached_info()
        return run

    def run_enter(self, statement: HasRun, inputs):
        # we set invalid values to none here unlike in other packing places because
        #  these values may be written even if invalid
        run = self._create_run(
            runnable=statement,
            inputs=pack_value(inputs, statement, is_output=False, none_if_invalid=True),
        )
        self.stacktrace.append(run)
        _set_active_run(run)
        self.track(run)  # tracker may mutate/do other things, so log after it's run
        logger.debug("trace.run.enter", run=run, stackdepth=len(self.stacktrace))

    def run_exit(self, statement: HasRun, outputs):
        run = self.pop_stacktrace()
        run.terminated_at = utcnow_with_tz()
        run.outputs = _pack_and_truncate_value(
            outputs, statement, is_output=True, none_if_invalid=True
        )
        run.status = RunStatus.Completed
        self.track(run)
        _clear_active_run(run)
        logger.debug("trace.run.exit", run=run, stackdepth=len(self.stacktrace))

    def run_exception(self, statement: HasRun, exception: Exception):
        run = self.pop_stacktrace()
        run.terminated_at = utcnow_with_tz()
        if isinstance(exception, asyncio.CancelledError):
            run.status = RunStatus.Aborted
        else:
            run.status = RunStatus.Failed
            run.error = RunError.from_exception(exception, statement)
        self.track(run)
        _clear_active_run(run)
        logger.debug("trace.run.exception", run=run, stackdepth=len(self.stacktrace))

    def run_cached(
        self,
        statement: HasRun,
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
        self.track(run)
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
        runnable: Optional[Code | Task | Model] = None,
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
        run._activate_in(self.session, queue_position=queue_position)
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


@dataclass
class _CapturedRuns:
    runs: list[Run] = None


class _RunCapture:
    def __init__(self, tracer: RunTracer):
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


class MutationTracer(Tracer):
    """Tracks module mutations."""

    def __init__(self, mutator: ModuleMutator):
        self.mutator = mutator
        # publish not supported yet

    def value_update(self, variable: Statement, key: Optional[str] = None):
        from bench.language import wire

        self.mutator.update(wire.pack_node_flat(variable), properties=["value"])

    def database_clear(self, database: Database):
        self.mutator.truncate(database, MNT.Record)

    def database_append(self, database: Database, record: Record):
        from bench.language import wire

        self.mutator.create(wire.pack_node_flat(record))

    def database_extend(self, database: Database, records: list[Record]):
        from bench.language import wire

        self.mutator.create_many(*[wire.pack_node_flat(record) for record in records])

    def database_remove(self, database: Database, record: Record):
        from bench.language import wire

        self.mutator.delete(wire.pack_node_flat(record))

    def database_update(
        self, database: Database | Variable, record: Record, key: Optional[str] = None
    ):
        from bench.language import wire

        self.mutator.update(wire.pack_node_flat(record), properties=["value"])


class TypeCheckingTracer(Tracer):
    """Validates types (except in inference, which is always checked in the task implementation)."""

    def run_enter(self, statement: HasRun, inputs):
        check_type(inputs, statement, is_output=False)

    def run_exit(self, statement: HasRun, outputs):
        check_type(outputs, statement, is_output=True)

    def value_update(self, value: HasValue, key: Optional[str] = None):
        if key:
            field_ = value.get_field(key)
            if field_ is None:
                raise ValueError(f"{key} does not exist in {value} (available: {value.fields})")
            check_type(value.value.get(key), field_)
        else:
            check_type(value.value, value)

    def database_append(self, database: Database, record: Record):
        check_type(record.value, database, ignore_array=True)

    def database_update(self, database: Database, record: Record, key: Optional[str] = None):
        if key:
            # validate only this key
            field_ = database.get_field(key)
            if field_ is None:
                raise ValueError(
                    f"{key} does not exist in {database} (available: {database.fields})"
                )
            check_type(record.value.get(key), field_)
        else:
            check_type(record.value, database, ignore_array=True)


class PermissionCheckingTracer(Tracer):
    """Validates permissions to access or modify resources in the session."""

    def __init__(self, session: Session):
        self.session = session

    def file_create(self, file: File):
        self.session.check_can(ModuleOp.CREATE, file)

    def statement_create(self, statement: Statement):
        self.session.check_can(ModuleOp.CREATE, statement)

    def value_update(self, value: Variable, key: Optional[str] = None):
        self.session.check_can(ModuleOp.UPDATE, value)

    def database_clear(self, database: Database):
        self.session.check_can(ModuleOp.UPDATE, database)

    def database_append(self, database: Database, record: Record):
        self.session.check_can(ModuleOp.UPDATE, database)

    def database_update(self, database: Database, record: Record, key: Optional[str] = None):
        self.session.check_can(ModuleOp.UPDATE, database)

    def database_delete(self, database: Database, record: Record):
        self.session.check_can(ModuleOp.UPDATE, database)

    def database_search(self, database: Database, query: Query, sort: list[Sort]):
        self.session.check_can(ModuleOp.READ, database)

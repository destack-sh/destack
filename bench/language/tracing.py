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

from bench.language.const import TriggerType, TypeFlag, TypeTag
from bench.language.core import MNT, ModuleOp, Session, Statement
from bench.language.mutate import ModuleMutator
from bench.language.query import Query, Sort
from bench.language.session import LogEntry, Run, RunError
from bench.language.type import TypeBase, check_type, map_value, pack_value, pack_value_flat
from bench.language.utils import Runnable
from bench.utils.dt import utcnow_with_tz
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import (
        Code,
        Dataset,
        Field,
        File,
        HasType,
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

    def field_append(self, symbol: HasType, field: Field):
        pass

    def variable_update(self, value: Variable, key: Optional[str] = None):
        pass

    def dataset_clear(self, dataset: Dataset):
        pass

    def dataset_append(self, dataset: Dataset, record: Record):
        pass

    def dataset_extend(self, dataset: Dataset, records: list[Record]):
        pass

    def dataset_remove(self, dataset: Dataset, record: Record):
        pass

    def dataset_update(self, dataset: Dataset, record: Record, key: Optional[str] = None):
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

    def run_enter(self, statement: Runnable, inputs: dict):
        pass

    def run_exit(self, statement: Runnable, outputs: dict):
        pass

    def run_cached(
        self,
        statement: Runnable,
        inputs: dict,
        outputs: dict,
        generated_at: datetime,
        generated_in: UUID,
        duration: float,
    ):
        pass

    def run_exception(self, statement: Runnable, exception: Exception):
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
        active_run = _active_run_by_root.get()
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

    async def _flush(self, force: bool = False) -> None:
        """Flushes session data."""
        if not force and not self._pending_logs and not self._pending_runs:
            return  # skip if nothing to flush

        logs = self._pending_logs[:]
        runs = self._pending_runs[:]
        self._pending_logs.clear()
        self._pending_runs.clear()
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
        await self._flush(force=True)  # flush pending data

    def variable_update(self, value: Variable, key: Optional[str] = None):
        for tracer in self.tracers:
            tracer.variable_update(value, key)

    def dataset_clear(self, dataset: Dataset):
        for tracer in self.tracers:
            tracer.dataset_clear(dataset)

    def dataset_append(self, dataset: Dataset, record: Record):
        for tracer in self.tracers:
            tracer.dataset_append(dataset, record)

    def dataset_extend(self, dataset: Dataset, records: list[Record]):
        for tracer in self.tracers:
            tracer.dataset_extend(dataset, records)

    def dataset_remove(self, dataset: Dataset, record: Record):
        for tracer in self.tracers:
            tracer.dataset_remove(dataset, record)

    def dataset_update(self, dataset: Dataset, record: Record, key: Optional[str] = None):
        for tracer in self.tracers:
            tracer.dataset_update(dataset, record, key)

    def run_enter(self, statement: Runnable, inputs):
        for tracer in self.tracers:
            tracer.run_enter(statement, inputs)

    def run_exit(self, statement: Runnable, outputs):
        for tracer in reversed(self.tracers):
            tracer.run_exit(statement, outputs)

    def run_exception(self, statement: Runnable, exception: Exception):
        for tracer in reversed(self.tracers):
            try:
                tracer.run_exception(statement, exception)
            except Exception:
                # internal error in tracer, very not good
                logger.exception("trace.run.exception", exc_info=True, tracer=tracer)

    def run_cached(
        self,
        statement: Runnable,
        inputs: dict,
        outputs: dict,
        generated_at: datetime,
        generated_in: UUID,
        duration: float,
    ):
        for tracer in reversed(self.tracers):
            tracer.run_cached(statement, inputs, outputs, generated_at, generated_in, duration)


# We track the active root in a contextvar but not children
#  because they may be in different contexts, and we cannot reset across contexts.
# This will need to be expanded when we get to parallel runs.
_active_root_run: ContextVar[Run | None] = ContextVar("active_root_run", default=None)
_active_run_by_root: dict[UUID, Run] = {}


def _get_active_run() -> Run | None:
    root = _active_root_run.get()
    if root is not None:
        return _active_run_by_root[root.id]
    return None


def _clear_active_run(run: Run):
    root = run.root or run
    if root.id in _active_run_by_root:
        del _active_run_by_root[root.id]
    if _active_root_run.get() == root:
        _active_root_run.set(None)


def _set_active_run(run: Run):
    root = run.root or run
    _active_run_by_root[root.id] = run
    if _active_root_run.get() is None:
        _active_root_run.set(root)


def _pack_and_truncate_value(
    value: Any,
    type: TypeBase,
    ignore_array: bool = False,
    ignore_outer_map: bool = False,
    is_output: bool = None,
) -> Any:
    def _is_type_truncated(type: TypeBase) -> bool:
        return type.tag in (TypeTag.VECTOR,)

    def _truncate_value(value: Any, type: TypeBase, *args, **kwargs) -> Any:
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
        is_output=is_output,
    )


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
        if run.cached_at is not None:
            self._update_cached_info()
        return run

    def _update_cached_info(self):
        for run in self.stacktrace:
            run.cached_at = min(f.cached_at for f in run.walk_descendants() if f.cached_at)
            run.cached_duration = sum(
                f.cached_duration for f in run.walk_descendants() if f.cached_duration
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
            metadata=None,
        )
        if queue_position is not None:
            run.queue_position = queue_position
        if parent is not None:
            parent.children.append(run)
        return run

    def run_enter(self, statement: Runnable, inputs):
        run = self._create_run(
            runnable=statement, inputs=pack_value(inputs, statement, is_output=False)
        )
        self.stacktrace.append(run)
        _set_active_run(run)
        self.track(run)  # tracker may mutate/do other things, so log after it's run
        logger.debug("trace.run.enter", run=run, stackdepth=len(self.stacktrace))

    def run_exit(self, statement: Runnable, outputs):
        run = self.pop_stacktrace()
        run.terminated_at = utcnow_with_tz()
        run.outputs = _pack_and_truncate_value(outputs, statement, is_output=True)
        run._update_status()
        self.track(run)
        _clear_active_run(run)
        logger.debug("trace.run.exit", run=run, stackdepth=len(self.stacktrace))

    def run_exception(self, statement: Runnable, exception: Exception):
        run = self.pop_stacktrace()
        run.terminated_at = utcnow_with_tz()
        run.error = RunError.from_exception(exception, statement)
        run._update_status()
        self.track(run)
        _clear_active_run(run)
        logger.debug("trace.run.exception", run=run, stackdepth=len(self.stacktrace))

    def run_cached(
        self,
        statement: Runnable,
        inputs,
        outputs,
        generated_at: datetime,
        generated_in: UUID,
        duration: float,
    ):
        run = self._create_run(runnable=statement, trace=True)
        run.terminated_at = utcnow_with_tz()
        run.cached_at = generated_at
        run.cached_in = generated_in
        run.cached_duration = duration
        run.inputs = _pack_and_truncate_value(inputs, statement, is_output=False)
        run.outputs = _pack_and_truncate_value(outputs, statement, is_output=True)
        run._update_status()
        self.track(run)
        self._update_cached_info()
        logger.debug("trace.run.cached", run=run, stackdepth=len(self.stacktrace))

    @contextlib.contextmanager
    def capture(self) -> list[Run]:
        """Get all runs that are created within the context."""
        start_ids = set(self.runs.keys())
        capture = _RunCapture()
        try:
            yield
        finally:
            capture.runs = [run for run in self.runs.values() if run.id not in start_ids]


@dataclass
class _RunCapture:
    runs: list[Run] = None


class MutationTracer(Tracer):
    """Tracks module mutations."""

    def __init__(self, mutator: ModuleMutator):
        self.mutator = mutator
        # publish not supported yet

    def variable_update(self, variable: Variable, key: Optional[str] = None):
        from bench.language import wire

        self.mutator.update(wire.pack_node_flat(variable), properties=["value"])

    def dataset_clear(self, dataset: Dataset):
        self.mutator.truncate(dataset, MNT.Record)

    def dataset_append(self, dataset: Dataset, record: Record):
        from bench.language import wire

        self.mutator.create(wire.pack_node_flat(record))

    def dataset_extend(self, dataset: Dataset, records: list[Record]):
        from bench.language import wire

        self.mutator.create_many(*[wire.pack_node_flat(record) for record in records])

    def dataset_remove(self, dataset: Dataset, record: Record):
        from bench.language import wire

        self.mutator.delete(wire.pack_node_flat(record))

    def dataset_update(
        self, dataset: Dataset | Variable, record: Record, key: Optional[str] = None
    ):
        from bench.language import wire

        self.mutator.update(wire.pack_node_flat(record))


class TypeCheckingTracer(Tracer):
    """Validates types (except in inference, which is always checked in the task implementation)."""

    def run_enter(self, statement: Runnable, inputs):
        check_type(inputs, statement, is_output=False)

    def run_exit(self, statement: Runnable, outputs):
        check_type(outputs, statement, is_output=True)

    def variable_update(self, value: Variable, key: Optional[str] = None):
        check_type(value.value, value)

    def dataset_append(self, dataset: Dataset, record: Record):
        check_type(record.value, dataset, ignore_array=True)

    def dataset_update(self, dataset: Dataset, record: Record, key: Optional[str] = None):
        if key is not None and key != "":
            # validate only this key
            field_ = dataset.get_field(key)
            if field_ is None:
                raise ValueError(f"{key} does not exist in {dataset} (available: {dataset.fields})")
            check_type(record.value.get(key), field_)
        else:
            check_type(record.value, dataset, ignore_array=True)


class PermissionCheckingTracer(Tracer):
    """Validates permissions to access or modify resources in the session."""

    def __init__(self, session: Session):
        self.session = session

    def file_create(self, file: File):
        self.session.check_can(ModuleOp.CREATE, file)

    def statement_create(self, statement: Statement):
        self.session.check_can(ModuleOp.CREATE, statement)

    def symbol_create(self, symbol: Statement):
        self.session.check_can(ModuleOp.CREATE, symbol)

    def variable_update(self, value: Variable, key: Optional[str] = None):
        self.session.check_can(ModuleOp.UPDATE, value)

    def dataset_clear(self, dataset: Dataset):
        self.session.check_can(ModuleOp.UPDATE, dataset)

    def dataset_append(self, dataset: Dataset, record: Record):
        self.session.check_can(ModuleOp.UPDATE, dataset)

    def dataset_update(self, dataset: Dataset, record: Record, key: Optional[str] = None):
        self.session.check_can(ModuleOp.UPDATE, dataset)

    def dataset_delete(self, dataset: Dataset, record: Record):
        self.session.check_can(ModuleOp.UPDATE, dataset)

    def dataset_search(self, dataset: Dataset, query: Query, sort: list[Sort]):
        self.session.check_can(ModuleOp.READ, dataset)

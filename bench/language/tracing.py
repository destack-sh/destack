from __future__ import annotations

import asyncio
import sys
from collections import deque
from contextvars import ContextVar
from datetime import datetime
from typing import TYPE_CHECKING, Any, Callable, Optional
from uuid import UUID

import pytz
import structlog

from bench.language.core import MOT, ModuleOp, Session
from bench.language.mutate import ModuleMutator
from bench.language.query import Query, Sort
from bench.language.session import LogEntry, Run, RunError
from bench.language.type import (
    TypeBase,
    TypeTag,
    check_type,
    map_value,
    strip_py_value,
    strip_py_value_flat,
)
from bench.language.utils import Runnable
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
        Statement,
        Tagging,
        Task,
        Value,
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

    def value_update(self, value: Value, key: Optional[str] = None):
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

    def run_queue(self, statement: Runnable, inputs: dict[str, Any], queue_position: int):
        pass

    def run_cancel(self, statement: Runnable, inputs: dict[str, Any]):
        pass

    def run_enter(self, statement: Runnable, inputs):
        pass

    def run_exit(self, statement: Runnable, result):
        pass

    def run_cached(
        self, statement: Runnable, inputs, result, generated_at: datetime, duration: float
    ):
        pass

    def run_exception(self, statement: Runnable, exception: Exception):
        pass


# TODO @Performance: investigate performance implication of contextual stdout/stderr redirect

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
        active_run = _active_run.get()
        if active_run:
            runnable = active_run.runnable
            run = active_run
        else:
            runnable = None
            run = None
        log_entry = LogEntry(
            id=UUIDT(),
            module=self.session.module,
            created_at=datetime.now(pytz.utc),
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

    def value_update(self, value: Value, key: Optional[str] = None):
        for tracer in self.tracers:
            tracer.value_update(value, key)

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

    def run_queue(self, statement: Runnable, inputs: dict[str, Any], queue_position: int):
        for tracer in self.tracers:
            tracer.run_queue(statement, inputs, queue_position)

    def run_cancel(self, statement: Runnable, inputs: dict[str, Any]):
        for tracer in self.tracers:
            tracer.run_cancel(statement, inputs)

    def run_enter(self, statement: Runnable, inputs):
        for tracer in self.tracers:
            tracer.run_enter(statement, inputs)

    def run_exit(self, statement: Runnable, result):
        for tracer in reversed(self.tracers):
            tracer.run_exit(statement, result)

    def run_exception(self, statement: Runnable, exception: Exception):
        for tracer in reversed(self.tracers):
            try:
                tracer.run_exception(statement, exception)
            except Exception:
                # internal error in tracer, very not good
                logger.exception("trace.run.exception", exc_info=True, tracer=tracer)

    def run_cached(
        self, statement: Runnable, inputs, result, generated_at: datetime, duration: float
    ):
        for tracer in reversed(self.tracers):
            tracer.run_cached(statement, inputs, result, generated_at, duration)


_active_run: ContextVar[Run | None] = ContextVar("_active_run", default=None)


def is_run_value_truncated(value: Any, type: TypeBase) -> bool:
    return type.tag == TypeTag.VECTOR


def _strip_and_truncate_py_value_flat(value: Any, type: TypeBase, *args, **kwargs) -> Any:
    stripped = strip_py_value_flat(value, type, *args, **kwargs)
    if is_run_value_truncated(stripped, type):
        return None  # can't use OMITTED_SENTINEL because of type mismatch... hmm
    return stripped


def _strip_and_truncate_py_value(
    value: Any,
    type: TypeBase,
    ignore_array: bool = False,
    ignore_outer_map: bool = False,
    is_output: bool = None,
) -> Any:
    return map_value(
        value=value,
        type=type,
        map_k=lambda f: (f.py_ident, f.typed_key),
        map_v=_strip_and_truncate_py_value_flat,
        ignore_array=ignore_array,
        ignore_outer_map=ignore_outer_map,
        is_output=is_output,
    )


class RunTracer(Tracer):
    """
    A worker-side tracer that records code and model executions.
    """

    def __init__(self, session: Session, track: Callable[[Run], None]):
        self.session = session
        self.stacktrace = []
        self._track = track
        self.runs = {}
        self._cvar_tokens: dict[UUID, Any] = {}

    def __str__(self):
        return f"{len(self.stacktrace)} stack, {len(self.runs)} runs"

    def __repr__(self):
        return f"<RunTracer {self}>"

    @property
    def current_frame(self) -> Optional[Run]:
        if self.stacktrace:
            return self.stacktrace[-1]
        return None

    def track(self, frame: Run):
        self.runs[frame.id] = frame
        self._track(frame)

    def pop_stacktrace(self) -> Run:
        frame = self.stacktrace.pop()
        # update cached info in parent(s)
        if frame.cached_generated_at is not None:
            self._update_cached_info()
        return frame

    def _update_cached_info(self):
        for frame in self.stacktrace:
            frame.cached_generated_at = min(
                f.cached_generated_at for f in frame.walk_descendants() if f.cached_generated_at
            )
            frame.cached_duration = sum(
                f.cached_duration for f in frame.walk_descendants() if f.cached_duration
            )

    def _create_frame(
        self,
        runnable: Optional[Code | Task | Model] = None,
        inputs: dict[str, Any] | None = None,
        queue_position: int | None = None,
        trace: bool = True,
    ):
        if trace and _active_run.get() is not None:
            root = _active_run.get().root or _active_run.get()
            parent = _active_run.get()
        else:
            root = None
            parent = None
        frame = Run(
            id=self.session.ctx.first_run_id if root is None else UUIDT(),
            module=self.session.module,
            runnable=runnable,
            session=self.session,
            root=root,
            parent=parent,
            started_at=datetime.utcnow().replace(tzinfo=pytz.utc),
            terminated_at=None,
            inputs=inputs,
            outputs=None,
            error=None,
            metadata=None,
        )
        if queue_position is not None:
            frame.queue_position = queue_position
        if parent is not None:
            parent.children.append(frame)
        return frame

    def run_queue(self, statement: Runnable, inputs: dict[str, Any], queue_position: int):
        # don't trace this because it's not part of the stacktrace
        frame = self._create_frame(
            runnable=statement,
            inputs=strip_py_value(inputs, statement, is_output=False),
            trace=False,
            queue_position=queue_position,
        )
        self.track(frame)
        logger.debug("trace.run.queue", frame=frame)

    def run_cancel(self, statement: Runnable, inputs: dict[str, Any]):
        # don't trace this because it's not part of the stacktrace
        frame = self._create_frame(
            runnable=statement,
            inputs=strip_py_value(inputs, statement, is_output=False),
            trace=False,
        )
        self.track(frame)
        logger.debug("trace.run.cancel", frame=frame)

    def run_enter(self, statement: Runnable, inputs):
        frame = self._create_frame(
            runnable=statement, inputs=strip_py_value(inputs, statement, is_output=False)
        )
        self.stacktrace.append(frame)
        self._cvar_tokens[frame.id] = _active_run.set(frame)
        self.track(frame)  # tracker may mutate/do other things, so log after it's run
        logger.debug("trace.run.enter", frame=frame, stackdepth=len(self.stacktrace))

    def run_exit(self, statement: Runnable, result):
        frame = self.pop_stacktrace()
        frame.terminated_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.outputs = _strip_and_truncate_py_value(result, statement, is_output=True)
        frame._update_status()
        self.track(frame)
        if _active_run.get() is frame:
            _active_run.reset(self._cvar_tokens.pop(frame.id))
        logger.debug("trace.run.exit", frame=frame, stackdepth=len(self.stacktrace))

    def run_exception(self, statement: Runnable, exception: Exception):
        frame = self.pop_stacktrace()
        frame.terminated_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.error = RunError.from_exception(exception, statement)
        frame._update_status()
        self.track(frame)
        if _active_run.get() is frame:
            _active_run.reset(self._cvar_tokens.pop(frame.id))
        logger.debug("trace.run.exception", frame=frame, stackdepth=len(self.stacktrace))

    def run_cached(
        self, statement: Runnable, inputs, result, generated_at: datetime, duration: float
    ):
        frame = self._create_frame(runnable=statement, trace=True)
        frame.terminated_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.cached_generated_at = generated_at
        frame.cached_duration = duration
        frame.inputs = _strip_and_truncate_py_value(inputs, statement, is_output=False)
        frame.outputs = _strip_and_truncate_py_value(result, statement, is_output=True)
        frame._update_status()
        self.track(frame)
        self._update_cached_info()
        logger.debug("trace.run.cached", frame=frame, stackdepth=len(self.stacktrace))


class MutationTracer(Tracer):
    """Tracks module mutations."""

    def __init__(self, mutator: ModuleMutator):
        self.mutator = mutator
        # publish not supported yet

    def value_update(self, value: Value, key: Optional[str] = None):
        from bench.language import wire

        self.mutator.update(wire.pack_node_flat(value), properties=["value"])

    def dataset_clear(self, dataset: Dataset):
        self.mutator.truncate(dataset, MOT.RECORD)

    def dataset_append(self, dataset: Dataset, record: Record):
        from bench.language import wire

        self.mutator.create(wire.pack_node_flat(record))

    def dataset_extend(self, dataset: Dataset, records: list[Record]):
        from bench.language import wire

        self.mutator.create_many(*[wire.pack_node_flat(record) for record in records])

    def dataset_remove(self, dataset: Dataset, record: Record):
        from bench.language import wire

        self.mutator.delete(wire.pack_node_flat(record))

    def dataset_update(self, dataset: Dataset | Value, record: Record, key: Optional[str] = None):
        from bench.language import wire

        self.mutator.update(wire.pack_node_flat(record))


class TypeCheckingTracer(Tracer):
    """Validates types (except in inference, which is always checked in the task implementation)."""

    def run_enter(self, statement: Runnable, inputs):
        check_type(inputs, statement, is_output=False)

    def run_exit(self, statement: Runnable, result):
        check_type(result, statement, is_output=True)

    def value_update(self, value: Value, key: Optional[str] = None):
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

    def value_update(self, value: Value, key: Optional[str] = None):
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
        self.session.check_can(ModuleOp.SEARCH, dataset)

from __future__ import annotations

import typing
from datetime import datetime
from typing import Any

import pytz
import structlog

from bench.bench.core import MOT, ModuleOp, Session, SessionTracingLevel
from bench.bench.execution import ExecutionFrame
from bench.bench.mutate import ModuleMutator
from bench.bench.query import Query, Sort
from bench.bench.type import check_type, strip_py_value
from bench.bench.wire import ExecutionFrameData
from bench.utils.uuidt import UUIDT

if typing.TYPE_CHECKING:
    from bench.bench import (
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

    Runnable = Code | Task | Model

logger = structlog.get_logger(__name__)


class Tracer:
    """
    Trace and track everything in a module/session (executions, mutations, etc.).
    """

    # module

    def file_create(self, file: File):
        pass

    def statement_create(self, statement: Statement):
        pass

    def field_append(self, symbol: HasType, field: Field):
        pass

    def value_update(self, value: Value, key: typing.Optional[str] = None):
        pass

    def dataset_clear(self, dataset: Dataset):
        pass

    def dataset_append(self, dataset: Dataset, record: Record):
        pass

    def dataset_extend(self, dataset: Dataset, records: list[Record]):
        pass

    def dataset_remove(self, dataset: Dataset, record: Record):
        pass

    def dataset_update(self, dataset: Dataset, record: Record, key: typing.Optional[str] = None):
        pass

    # TODO @Broken: the below trace events aren't fired (or used) yet

    def tagging_set(self, statement: Statement, tagging: Tagging):
        pass

    def tagging_clear(self, statement: Statement, tagging: Tagging):
        pass

    def remote_object_read(self, object: RemoteObject):
        pass

    def remote_object_write(self, object: RemoteObject):
        pass

    def secret_reveal(self, secret: Secret):
        pass

    # execution

    def run_queue(self, statement: Runnable, inputs: dict[str, Any], queue_position: int):
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


class SessionTracer(Tracer):
    """ """

    def __init__(
        self,
        session: Session,
        mutator: ModuleMutator = None,
        publish: bool = True,
        validate: bool = True,
    ):
        self.session = session
        self.execution = ExecutionTracer(
            session=session,
            publish=publish and session.ctx.tracing_level & SessionTracingLevel.EXECUTION,
        )
        self.tracers: list[Tracer] = [self.execution, PermissionCheckingTracer(session)]
        if validate:  # validation tracer must be last
            self.tracers.append(TypeCheckingTracer())
        if mutator:
            self.tracers.append(MutationTracer(mutator))

    def value_update(self, value: Value, key: typing.Optional[str] = None):
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

    def dataset_update(self, dataset: Dataset, record: Record, key: typing.Optional[str] = None):
        for tracer in self.tracers:
            tracer.dataset_update(dataset, record, key)

    def run_queue(self, statement: Runnable, inputs: dict[str, Any], queue_position: int):
        for tracer in self.tracers:
            tracer.run_queue(statement, inputs, queue_position)

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


class ExecutionTracer(Tracer):
    """
    A worker-side tracer that records code and model executions.
    """

    def __init__(self, session: Session, publish: bool = True):
        self.session = session
        self.publish = publish
        self.stacktrace = []
        self.frames = {}

    def __str__(self):
        return f"{len(self.stacktrace)} stack, {len(self.frames)} frames"

    def __repr__(self):
        return f"<ExecutionTracer {self}>"

    @property
    def current_frame(self) -> typing.Optional[ExecutionFrame]:
        if self.stacktrace:
            return self.stacktrace[-1]
        return None

    def track(self, frame: ExecutionFrame):
        if self.publish:
            from bench.msg.core import publish_soon
            from bench.msg.messages import ExecutionChangedPayload, NMessageType

            frame_data = ExecutionFrameData.from_frame(frame, session=self.session)
            logger.debug("execution.track", frame=frame_data.id)
            publish_soon(
                NMessageType.EXECUTION_CHANGED,
                ExecutionChangedPayload(frame.module_id, frames=[frame_data]),
            )
        self.frames[frame.id] = frame

    def pop_stacktrace(self) -> ExecutionFrame:
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
        runnable: typing.Optional[Code | Task | Model] = None,
        inputs: dict[str, Any] | None = None,
        queue_position: int | None = None,
        trace: bool = True,
    ):
        if trace:
            root = self.stacktrace[0] if self.stacktrace else None
            parent = self.stacktrace[-1] if self.stacktrace else None
        else:
            root = None
            parent = None
        frame = ExecutionFrame(
            id=self.session.ctx.root_id if root is None else UUIDT(),
            module_id=self.session.module.id,
            runnable=runnable,
            root=root,
            parent=parent,
            entered_at=datetime.utcnow().replace(tzinfo=pytz.utc),
            exited_at=None,
            cached_generated_at=None,
            cached_duration=None,
            inputs=inputs,
            outputs=None,
            error=None,
            queue_position=queue_position,
        )
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
        logger.debug("trace.queue", frame=frame)

    def run_enter(self, statement: Runnable, inputs):
        frame = self._create_frame(
            runnable=statement, inputs=strip_py_value(inputs, statement, is_output=False)
        )
        self.stacktrace.append(frame)
        self.track(frame)  # tracker may mutate/do other things, so log after it's run
        logger.debug("trace.run.enter", frame=frame, stackdepth=len(self.stacktrace))

    def run_exit(self, statement: Runnable, result):
        frame = self.pop_stacktrace()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.outputs = strip_py_value(result, statement, is_output=True)
        self.track(frame)
        logger.debug("trace.run.exit", frame=frame, stackdepth=len(self.stacktrace))

    def run_exception(self, statement: Runnable, exception: Exception):
        frame = self.pop_stacktrace()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.error = exception
        self.track(frame)
        logger.debug("trace.run.exception", frame=frame, stackdepth=len(self.stacktrace))

    def run_cached(
        self, statement: Runnable, inputs, result, generated_at: datetime, duration: float
    ):
        frame = self._create_frame(runnable=statement, trace=True)
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.cached_generated_at = generated_at
        frame.cached_duration = duration
        frame.inputs = strip_py_value(inputs, statement, is_output=False)
        frame.outputs = strip_py_value(result, statement, is_output=True)
        self.track(frame)
        self._update_cached_info()
        logger.debug("trace.run.cached", frame=frame, stackdepth=len(self.stacktrace))


class MutationTracer(Tracer):
    """Tracks module mutations."""

    def __init__(self, mutator: ModuleMutator):
        self.mutator = mutator
        # publish not supported yet

    def value_update(self, value: Value, key: typing.Optional[str] = None):
        from bench.bench import wire

        self.mutator.update(wire.pack_node_flat(value), properties=["value"])

    def dataset_clear(self, dataset: Dataset):
        self.mutator.truncate(dataset, MOT.RECORD)

    def dataset_append(self, dataset: Dataset, record: Record):
        from bench.bench import wire

        self.mutator.create(wire.pack_node_flat(record))

    def dataset_extend(self, dataset: Dataset, records: list[Record]):
        from bench.bench import wire

        self.mutator.create_many(*[wire.pack_node_flat(record) for record in records])

    def dataset_remove(self, dataset: Dataset, record: Record):
        from bench.bench import wire

        self.mutator.delete(wire.pack_node_flat(record))

    def dataset_update(
        self, dataset: Dataset | Value, record: Record, key: typing.Optional[str] = None
    ):
        from bench.bench import wire

        self.mutator.update(wire.pack_node_flat(record))


class TypeCheckingTracer(Tracer):
    """Validates types (except in inference, which is always checked in the task implementation)."""

    def run_enter(self, statement: Runnable, inputs):
        check_type(inputs, statement, is_output=False)

    def run_exit(self, statement: Runnable, result):
        check_type(result, statement, is_output=True)

    def value_update(self, value: Value, key: typing.Optional[str] = None):
        check_type(value.value, value)

    def dataset_append(self, dataset: Dataset, record: Record):
        check_type(record.value, dataset, ignore_array=True)

    def dataset_update(self, dataset: Dataset, record: Record, key: typing.Optional[str] = None):
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

    def value_update(self, value: Value, key: typing.Optional[str] = None):
        self.session.check_can(ModuleOp.UPDATE, value)

    def dataset_clear(self, dataset: Dataset):
        self.session.check_can(ModuleOp.UPDATE, dataset)

    def dataset_append(self, dataset: Dataset, record: Record):
        self.session.check_can(ModuleOp.UPDATE, dataset)

    def dataset_update(self, dataset: Dataset, record: Record, key: typing.Optional[str] = None):
        self.session.check_can(ModuleOp.UPDATE, dataset)

    def dataset_delete(self, dataset: Dataset, record: Record):
        self.session.check_can(ModuleOp.UPDATE, dataset)

    def dataset_search(self, dataset: Dataset, query: Query, sort: list[Sort]):
        self.session.check_can(ModuleOp.SEARCH, dataset)

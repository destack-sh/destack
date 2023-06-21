from __future__ import annotations

import typing
from datetime import datetime
from typing import Any

import pytz
import structlog

from bench.bench.const import MOT, ModuleOp
from bench.bench.dataset import Query, Sort
from bench.bench.mutate import ModuleMutator
from bench.bench.typer import check_type
from bench.bench.wire import ExecutionFrameData
from bench.msg.core import publish_soon
from bench.msg.messages import ExecutionChangedPayload, NMessageType
from bench.runtime.common.type import ExecutionFrame
from bench.utils.serialize import to_dict
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
        Statement,
        Symbol,
        Task,
        Value,
    )
    from bench.bench.build import XBlock
    from bench.bench.inference import Inference
    from bench.bench.session import Session

    Runnable = Code | Task

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

    def dataset_clear(self, table: Dataset):
        pass

    def dataset_append(self, table: Dataset, record: Record):
        pass

    def dataset_extend(self, table: Dataset, records: list[Record]):
        pass

    def dataset_remove(self, table: Dataset, record: Record):
        pass

    def dataset_update(self, dataset: Dataset, record: Record, key: typing.Optional[str] = None):
        pass

    def dataset_search(self, dataset: Dataset, query: Query, sort: list[Sort]):
        pass

    # execution

    def queue_enter(self, code: Runnable, inputs: dict[str, Any], queue_position: int):
        pass

    def code_enter(self, code: Runnable, args, kwargs):
        pass

    def code_exit(self, code: Runnable, args, kwargs, result):
        pass

    def code_exception(self, code: Runnable, args, kwargs, exception: Exception):
        pass

    def inference_enter(self, model: Model, blocks: list[XBlock], settings: Any):
        pass

    def inference_exit(self, model: Model, blocks: list[XBlock], settings: Any, result: Any):
        pass

    def inference_exception(
        self, model: Model, blocks: list[XBlock], settings: Any, exception: Exception
    ):
        pass

    def inference_cached(
        self, model: Model, blocks: list[XBlock], settings: Any, inference: Inference
    ):
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
        from bench.bench.const import SessionTracingLevel

        self.session = session
        self.execution = ExecutionTracer(
            session=session,
            publish=publish and session.ctx.tracing_level & SessionTracingLevel.EXECUTION,
        )
        self.tracers: list[Tracer] = [self.execution, PermissionCheckingTracer(session)]
        if mutator:
            self.tracers.append(MutationTracer(mutator))
        if validate:  # validation tracer must be last
            self.tracers.append(TypeCheckingTracer())

    def value_update(self, value: Value, key: typing.Optional[str] = None):
        for tracer in self.tracers:
            tracer.value_update(value, key)

    def dataset_clear(self, table: Dataset):
        for tracer in self.tracers:
            tracer.dataset_clear(table)

    def dataset_append(self, table: Dataset, record: Record):
        for tracer in self.tracers:
            tracer.dataset_append(table, record)

    def dataset_extend(self, table: Dataset, records: list[Record]):
        for tracer in self.tracers:
            tracer.dataset_extend(table, records)

    def dataset_remove(self, table: Dataset, record: Record):
        for tracer in self.tracers:
            tracer.dataset_remove(table, record)

    def dataset_update(self, dataset: Dataset, record: Record, key: typing.Optional[str] = None):
        for tracer in self.tracers:
            tracer.dataset_update(dataset, record, key)

    def queue_enter(self, code: Runnable, inputs: dict[str, Any], queue_position: int):
        for tracer in self.tracers:
            tracer.queue_enter(code, inputs, queue_position)

    def code_enter(self, code: Runnable, args, kwargs):
        for tracer in self.tracers:
            tracer.code_enter(code, args, kwargs)

    def code_exit(self, code: Runnable, args, kwargs, result):
        for tracer in reversed(self.tracers):
            tracer.code_exit(code, args, kwargs, result)

    def code_exception(self, code: Runnable, args, kwargs, exception: Exception):
        for tracer in reversed(self.tracers):
            try:
                tracer.code_exception(code, args, kwargs, exception)
            except Exception:
                # internal error in tracer, very bad
                logger.exception("trace.code.exception", exc_info=True, tracer=tracer)

    def inference_enter(self, model: Model, blocks: list[XBlock], settings: Any):
        for tracer in self.tracers:
            tracer.inference_enter(model, blocks, settings)

    def inference_exit(self, model: Model, blocks: list[XBlock], settings: Any, result: Any):
        for tracer in reversed(self.tracers):
            tracer.inference_exit(model, blocks, settings, result)

    def inference_exception(
        self, model: Model, blocks: list[XBlock], settings: Any, exception: Exception
    ):
        for tracer in reversed(self.tracers):
            try:
                tracer.inference_exception(model, blocks, settings, exception)
            except Exception:
                # internal error in tracer, very bad
                logger.exception("trace.inference.exception", exc_info=True, tracer=tracer)

    def inference_cached(
        self, model: Model, blocks: list[XBlock], settings: Any, inference: Inference
    ):
        for tracer in self.tracers:
            tracer.inference_cached(model, blocks, settings, inference)


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

    def track(self, frame):
        if self.publish:
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

    def queue_enter(self, code: Runnable, inputs: dict[str, Any], queue_position: int):
        # don't trace this because it's not part of the stacktrace
        frame = self._create_frame(
            runnable=code,
            inputs=code.rekey(inputs, is_output=False),
            trace=False,
            queue_position=queue_position,
        )
        self.track(frame)
        logger.debug("trace.queue", frame=frame)

    def code_enter(self, code: Runnable, args, kwargs):
        # map args into kwargs
        combined_kwargs = {**kwargs}
        for input_t, input in zip(code.inputs, args):
            combined_kwargs[input_t.name] = input
        frame = self._create_frame(
            runnable=code, inputs=code.rekey(combined_kwargs, is_output=False)
        )
        self.stacktrace.append(frame)
        self.track(frame)  # tracker may mutate/do other things, so log after it's run
        logger.debug("trace.code.enter", frame=frame, stackdepth=len(self.stacktrace))

    def code_exit(self, code: Runnable, args, kwargs, result):
        frame = self.pop_stacktrace()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.outputs = code.rekey(result, is_output=True)
        self.track(frame)
        logger.debug("trace.code.exit", frame=frame, stackdepth=len(self.stacktrace))

    def code_exception(self, code: Runnable, args, kwargs, exception: Exception):
        frame = self.pop_stacktrace()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.error = exception
        self.track(frame)
        logger.debug("trace.code.exception", frame=frame, stackdepth=len(self.stacktrace))

    def inference_enter(self, model: Model, blocks: list[XBlock], settings: Any):
        frame = self._create_frame(runnable=model)
        frame.inputs = [to_dict(block) for block in blocks]
        self.stacktrace.append(frame)
        self.track(frame)
        logger.debug("trace.inference.enter", frame=frame, stackdepth=len(self.stacktrace))

    def inference_exit(self, model: Model, blocks: list[XBlock], settings: Any, result: Any):
        frame = self.pop_stacktrace()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        self.track(frame)
        logger.debug("trace.inference.exit", frame=frame, stackdepth=len(self.stacktrace))

    def inference_cached(
        self, model: Model, blocks: list[XBlock], settings: Any, inference: Inference
    ):
        # track a complete frame, don't add to stacktrace
        frame = self._create_frame(runnable=model, trace=True)
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.cached_generated_at = inference.generated_at
        frame.cached_duration = inference.duration
        frame.inputs = [to_dict(block) for block in blocks]
        frame.outputs = inference.result
        self._update_cached_info()
        self.track(frame)
        logger.debug("trace.inference.cached", frame=frame, stackdepth=len(self.stacktrace))

    def inference_exception(
        self, model: Model, blocks: list[XBlock], settings: Any, exception: Exception
    ):
        frame = self.pop_stacktrace()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.error = exception
        self.track(frame)
        logger.debug("trace.inference.exception", frame=frame, stackdepth=len(self.stacktrace))


class MutationTracer(Tracer):
    """Tracks module mutations."""

    def __init__(self, mutator: ModuleMutator):
        self.mutator = mutator
        # publish not supported yet

    def value_update(self, value: Value, key: typing.Optional[str] = None):
        from bench.bench import wire

        self.mutator.update(wire.pack_node_flat(value), properties=["value"])

    def dataset_clear(self, table: Dataset):
        self.mutator.truncate(table.id, MOT.RECORD)

    def dataset_append(self, table: Dataset, record: Record):
        self.mutator.create(record._to_wire(include_data=True))

    def dataset_extend(self, table: Dataset, records: list[Record]):
        self.mutator.create_many(*[record._to_wire(include_data=True) for record in records])

    def dataset_remove(self, table: Dataset, record: Record):
        self.mutator.delete(record._id)

    def dataset_update(
        self, dataset: Dataset | Value, record: Record, key: typing.Optional[str] = None
    ):
        self.mutator.update(record.id, record._to_wire(include_data=True))


class TypeCheckingTracer(Tracer):
    """Validates types (except in inference, which is always checked in the task implementation)."""

    def code_enter(self, code: Runnable, args, kwargs):
        combined_kwargs = {**kwargs}
        for input_t, input in zip(code.inputs, args):
            combined_kwargs[input_t.name] = input
        check_type(combined_kwargs, code, is_output=False)

    def code_exit(self, code: Runnable, args, kwargs, result):
        check_type(result, code, is_output=True)

    def value_update(self, value: Value, key: typing.Optional[str] = None):
        check_type(value.value, value)

    def dataset_append(self, table: Dataset, record: Record):
        check_type(record._data, table, ignore_array=True)

    def dataset_update(self, dataset: Dataset, record: Record, key: typing.Optional[str] = None):
        if key is not None and key != "":
            # validate only this key
            if key not in dataset:
                raise ValueError(f"{key} does not exist on {dataset.type}")
            check_type(record._data.get(key), dataset.type[key])
        else:
            check_type(record._data, dataset.type, ignore_array=True)


class PermissionCheckingTracer(Tracer):
    """Validates permissions to access or modify resources in the session."""

    def __init__(self, session: Session):
        self.session = session

    def file_create(self, file: File):
        self.session.check_can(ModuleOp.CREATE, file)

    def statement_create(self, statement: Statement):
        self.session.check_can(ModuleOp.CREATE, statement)

    def symbol_create(self, symbol: Symbol):
        self.session.check_can(ModuleOp.CREATE, symbol)

    def value_update(self, value: Value, key: typing.Optional[str] = None):
        self.session.check_can(ModuleOp.UPDATE, value)

    def dataset_clear(self, table: Dataset):
        self.session.check_can(ModuleOp.UPDATE, table)

    def dataset_append(self, table: Dataset, record: Record):
        self.session.check_can(ModuleOp.UPDATE, table)

    def dataset_update(self, dataset: Dataset, record: Record, key: typing.Optional[str] = None):
        self.session.check_can(ModuleOp.UPDATE, dataset)

    def dataset_delete(self, table: Dataset, record: Record):
        self.session.check_can(ModuleOp.UPDATE, table)

    def dataset_search(self, dataset: Dataset, query: Query, sort: list[Sort]):
        self.session.check_can(ModuleOp.SEARCH, dataset)

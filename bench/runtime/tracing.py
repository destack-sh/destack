from __future__ import annotations

import typing
from datetime import datetime
from typing import Any

import pytz
import structlog

from bench.language.mutate import ModuleMutator
from bench.language.type import Model, Record, XBlock
from bench.language.typer import check_type
from bench.msg.core import publish_soon
from bench.msg.messages import ExecutionChangedPayload, NMessageType
from bench.runtime.type import ExecutionFrame, ExecutionFrameData
from bench.utils.serialize import to_dict
from bench.utils.uuidt import UUIDT

if typing.TYPE_CHECKING:
    from bench.runtime.inference import Inference
    from bench.runtime.instance import (
        CodeInstance,
        DataRecordInstance,
        DataTableInstance,
        RecordInstance,
        Session,
        TaskInstance,
    )

logger = structlog.get_logger(__name__)


class Tracer:
    """
    Trace and track a session with executions, mutations, etc.
    """

    def queue_enter(self, code: CodeInstance, inputs: dict[str, Any], queue_position: int):
        pass

    def code_enter(self, code: CodeInstance, args, kwargs):
        pass

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        pass

    def code_exception(self, code: CodeInstance, args, kwargs, exception: Exception):
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

    def table_clear(self, table: DataTableInstance):
        pass

    def table_append(self, table: DataTableInstance, record: Record):
        pass

    def table_extend(self, table: DataTableInstance, records: list[Record]):
        pass

    def table_remove(self, table: DataTableInstance, record: Record):
        pass

    def record_update(
        self,
        record: RecordInstance,
        owner: DataTableInstance | DataRecordInstance,
        key: typing.Optional[str] = None,
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
        from bench.runtime.instance import SessionTracingLevel

        self.session = session
        self.execution = ExecutionTracer(
            session=session,
            publish=publish and session.ctx.tracing_level & SessionTracingLevel.EXECUTION,
        )
        self.tracers: list[Tracer] = [self.execution, PermissionTracer(session)]
        if mutator:
            self.tracers.append(MutationTracer(mutator))
        if validate:  # validation tracer must be last
            self.tracers.append(ValidationTracer())

    def queue_enter(self, code: CodeInstance, inputs: dict[str, Any], queue_position: int):
        for tracer in self.tracers:
            tracer.queue_enter(code, inputs, queue_position)

    def code_enter(self, code: CodeInstance, args, kwargs):
        for tracer in self.tracers:
            tracer.code_enter(code, args, kwargs)

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        for tracer in reversed(self.tracers):
            tracer.code_exit(code, args, kwargs, result)

    def code_exception(self, code: CodeInstance, args, kwargs, exception: Exception):
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

    def table_clear(self, table: DataTableInstance):
        for tracer in self.tracers:
            tracer.table_clear(table)

    def table_append(self, table: DataTableInstance, record: Record):
        for tracer in self.tracers:
            tracer.table_append(table, record)

    def table_extend(self, table: DataTableInstance, records: list[Record]):
        for tracer in self.tracers:
            tracer.table_extend(table, records)

    def table_remove(self, table: DataTableInstance, record: Record):
        for tracer in self.tracers:
            tracer.table_remove(table, record)

    def record_update(
        self,
        record: RecordInstance,
        owner: DataTableInstance | DataRecordInstance,
        key: typing.Optional[str] = None,
    ):
        for tracer in self.tracers:
            tracer.record_update(record, owner, key)


class ExecutionTracer(Tracer):
    """
    A worker-side tracer that records code and model executions.
    """

    def __init__(self, session: Session, publish: bool = True):
        self.session = session
        self.publish = publish
        self.stacktrace = []
        self.frames = []

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

        if not any(f.id == frame.id for f in self.frames):
            self.frames.append(frame)

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
        runnable: typing.Optional[CodeInstance | TaskInstance | Model] = None,
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

    def queue_enter(self, code: CodeInstance, inputs: dict[str, Any], queue_position: int):
        # don't trace this because it's not part of the stacktrace
        frame = self._create_frame(
            runnable=code, inputs=inputs, trace=False, queue_position=queue_position
        )
        self.track(frame)
        logger.debug("trace.queue", frame=frame)

    def code_enter(self, code: CodeInstance, args, kwargs):
        # map args into kwargs
        combined_kwargs = {**kwargs}
        for input_t, input in zip(code.inputs, args):
            combined_kwargs[input_t.name] = input
        frame = self._create_frame(
            runnable=code, inputs=code.type.rekey(combined_kwargs, is_output=False)
        )
        self.stacktrace.append(frame)
        self.track(frame)  # tracker may mutate/do other things, so log after it's run
        logger.debug("trace.code.enter", frame=frame, stackdepth=len(self.stacktrace))

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        frame = self.pop_stacktrace()
        frame.exited_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        frame.outputs = code.type.rekey(result, is_output=True)
        self.track(frame)
        logger.debug("trace.code.exit", frame=frame, stackdepth=len(self.stacktrace))

    def code_exception(self, code: CodeInstance, args, kwargs, exception: Exception):
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

    def table_clear(self, table: DataTableInstance):
        self.mutator.truncate_records(table.id)

    def table_append(self, table: DataTableInstance, record: RecordInstance):
        self.mutator.create(record._to_wire(include_data=True))

    def table_extend(self, table: DataTableInstance, records: list[Record]):
        self.mutator.create_many(*[record._to_wire(include_data=True) for record in records])

    def table_remove(self, table: DataTableInstance, record: Record):
        self.mutator.delete(record.id)

    def record_update(
        self,
        record: RecordInstance,
        owner: DataTableInstance | DataRecordInstance,
        key: typing.Optional[str] = None,
    ):
        self.mutator.update(record.id, record._to_wire(include_data=True))


class ValidationError(RuntimeError):
    pass


class ValidationTracer(Tracer):
    """Validates types (except in inference, which is always checked in the task implementation)."""

    def code_enter(self, code: CodeInstance, args, kwargs):
        try:
            combined_kwargs = {**kwargs}
            for input_t, input in zip(code.inputs, args):
                combined_kwargs[input_t.name] = input
            check_type(combined_kwargs, code, is_output=False)
        except (KeyError, ValueError, TypeError) as e:
            raise ValidationError(f"invalid arguments for {code.name}: {e}", e)

    def code_exit(self, code: CodeInstance, args, kwargs, result):
        try:
            check_type(result, code, is_output=True)
        except TypeError as e:
            raise ValidationError(f"invalid return value for {code.name}: {e}", e)

    def table_append(self, table: DataTableInstance, record: Record):
        check_type(record.data, table)

    def record_update(
        self,
        record: RecordInstance,
        owner: DataTableInstance | DataRecordInstance,
        key: typing.Optional[str] = None,
    ):
        if key is not None and key != "":
            # validate only this key
            if key not in owner.type:
                raise ValidationError(f"{key} does not exist on {owner.type}")
            check_type(record.data.get(key), owner.type[key])
        else:
            check_type(record.data, owner.type)


class PermissionTracer(Tracer):
    """Validates permissions to access or modify resources in the session."""

    def __init__(self, session: Session):
        self.session = session

    def table_clear(self, table: DataTableInstance):
        self.session.check_can_write(table)

    def table_append(self, table: DataTableInstance, record: Record):
        self.session.check_can_write(table)

    def table_delete(self, table: DataTableInstance, record: RecordInstance):
        self.session.check_can_write(table)

    def record_update(
        self,
        record: RecordInstance,
        owner: DataTableInstance | DataRecordInstance,
        key: typing.Optional[str] = None,
    ):
        self.session.check_can_write(owner)

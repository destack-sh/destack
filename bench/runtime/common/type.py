from __future__ import annotations

import enum
import re
import sys
import traceback
from dataclasses import dataclass, field
from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

from bench.language import SymbolType
from bench.language.const import LiteralValue
from bench.language.type import Code, Model, Task
from bench.language.wire import ExecutionTriggerType
from bench.utils.utils import to_pyidentifier_multi

if TYPE_CHECKING:
    from bench.runtime.worker.instance import CodeInstance, Session

DATASET_RECORDS_IN_MEMORY_LIMIT = 512


@dataclass(slots=True)
class ExecutionFrame:
    id: UUID
    module_id: UUID
    runnable: Union[Code, Model, Task]
    root: Optional[ExecutionFrame]
    parent: Optional[ExecutionFrame]
    entered_at: datetime
    exited_at: Optional[datetime]
    cached_generated_at: Optional[datetime]
    cached_duration: Optional[float]
    inputs: Optional[dict[str, LiteralValue]]
    outputs: Optional[LiteralValue]
    error: Optional[Exception]
    queue_position: Optional[int]
    children: list[ExecutionFrame] = field(default_factory=list)

    @property
    def duration(self) -> float:
        if self.exited_at is None:
            return 0
        return (self.exited_at - self.entered_at).total_seconds()

    @property
    def duration_with_cache(self) -> float:
        if self.exited_at is None:
            return 0
        return self.duration + (self.cached_duration or 0)

    def walk_descendants(self):
        yield self
        for child in self.children:
            yield from child.walk_descendants()

    def __str__(self):
        # get str of all non-null fields
        fields_strs = [
            f"module={self.module_id}",
            f"runnable={self.runnable}" if self.runnable else None,
            f"root={self.root.id}" if self.root else None,
            f"parent={self.parent.id}" if self.parent else None,
        ]
        fields_str = [s for s in fields_strs if s]
        return f"id={self.id} ({', '.join(fields_str)})"

    def __repr__(self):
        return f"<ExecutionFrame {self}>"


IGNORED_PACKAGE_PREFIXES = [
    "bench.runtime",
    "bench.language",
    "asgiref",
    "concurrent",
]
IGNORED_PACKAGE_PATHS = [package.replace(".", "/") for package in IGNORED_PACKAGE_PREFIXES]


@dataclass(slots=True)
class PyFrameData:
    filename: str
    lineno: int
    name: str
    locals: dict[str, Any] = None
    line: str = None

    @staticmethod
    def from_traceback(frame: traceback.FrameSummary):
        return PyFrameData(
            filename=frame.filename,
            lineno=frame.lineno,
            name=frame.name,
            locals=frame.locals,
            line=frame.line,
        )

    @staticmethod
    def from_stack(stack: traceback.StackSummary) -> list[PyFrameData]:
        return [PyFrameData.from_traceback(frame) for frame in stack]

    @staticmethod
    def clean(
        stack: list[PyFrameData], from_code: "CodeInstance", session: "Session"
    ) -> list[PyFrameData]:
        from bench.runtime.instance import CodeInstance

        code_instances_by_method_name: dict[str, CodeInstance] = {
            instance.transform.method_name: cast(CodeInstance, instance)
            for instance in session.instances.values()
            if instance.symbol_type == SymbolType.CODE and instance.transform is not None
        }

        transform = from_code.transform
        found_start = False
        cleaned_stack = []
        for frame in stack:
            if any(prefix in frame.filename for prefix in IGNORED_PACKAGE_PATHS):
                continue  # skip support code
            if not found_start:
                # impute bench source info into instantiated code callables
                code = code_instances_by_method_name.get(frame.name)
                if code is not None:
                    if code == from_code:
                        found_start = True
                    elif not found_start:
                        continue  # ignore
                    if from_code.source is not None:
                        frame.filename = to_pyidentifier_multi(
                            from_code.source.file.path, from_code.source.name
                        )
                    frame.name = from_code.name
                    frame.line = transform.transformed_code.splitlines()[frame.lineno - 1]
                    frame.lineno = frame.lineno - transform.start_offset
                    frame.locals = frame.locals or {}
                    for ident, var in code.context.items():
                        if ident not in frame.locals and var.id in session.instances:
                            frame.locals[ident] = repr(session.instances[var.id])
            if found_start:
                # trim file path for python modules
                python_version = f"{sys.version_info.major}.{sys.version_info.minor}"
                if python_version in frame.filename:
                    frame.filename = frame.filename.split(python_version)[-1][1:]  # skip slash
                cleaned_stack.append(frame)
        return cleaned_stack


@dataclass(slots=True)
class RunErrorData:
    """Wire-able representation of an exception."""

    type: str
    message: str
    symbol: Optional[str]
    traceback: list[PyFrameData]

    @staticmethod
    def from_dict(data: dict[str, Any]) -> "RunErrorData":
        return RunErrorData(
            type=data["type"],
            message=data["message"],
            symbol=data.get("symbol"),
            traceback=[PyFrameData(**frame) for frame in data["traceback"]]
            if data.get("traceback")
            else [],
        )


@dataclass(slots=True)
class ExecutionFrameData:
    """Wire-able representation of an execution frame."""

    id: UUID
    module_id: UUID
    runnable_id: UUID
    root_id: Optional[UUID]
    parent_id: Optional[UUID]
    entered_at: datetime
    exited_at: Optional[datetime]
    cached_generated_at: Optional[datetime]
    cached_duration: Optional[float]
    inputs: Optional[Any]
    outputs: Optional[Any]
    error: Optional[RunErrorData]
    queue_position: Optional[int]
    # additional context data not in ExecutionFrame
    project_id: UUID
    tracing_level: Optional[int]
    worker_id: UUID
    trigger_type: Optional[ExecutionTriggerType]
    trigger_id: Optional[UUID]

    @staticmethod
    def from_frame(frame: ExecutionFrame, *, session: Session) -> ExecutionFrameData:
        if frame.error:
            if frame.runnable is None:
                raise ValueError(f"error outside code: {frame}")
            stack_summary = traceback.StackSummary.extract(
                traceback.walk_tb(frame.error.__traceback__), capture_locals=True
            )
            if isinstance(frame.runnable, Code):
                stack = PyFrameData.from_stack(stack_summary)
                stack = PyFrameData.clean(stack, frame.runnable, session=session)
            else:
                stack = []
            error_str = str(frame.error)
            # remove (source=...) from error message
            error_str = re.sub(r"\(source=.+\)", "", error_str)
            error_data = RunErrorData(
                type=type(frame.error).__name__,
                symbol=str(frame.runnable),
                message=f"{type(frame.error).__name__}: {error_str}",
                traceback=stack,
            )
        else:
            error_data = None
        return ExecutionFrameData(
            id=frame.id,
            module_id=frame.module_id,
            runnable_id=frame.runnable.id if frame.runnable else None,
            root_id=frame.root.id if frame.root else None,
            parent_id=frame.parent.id if frame.parent else None,
            entered_at=frame.entered_at,
            exited_at=frame.exited_at,
            cached_generated_at=frame.cached_generated_at,
            cached_duration=frame.cached_duration,
            inputs=frame.inputs,
            outputs=frame.outputs,
            error=error_data,
            queue_position=frame.queue_position,
            project_id=session.ctx.project_id,
            tracing_level=session.ctx.tracing_level,
            worker_id=session.ctx.worker_id,
            trigger_type=session.ctx.trigger_type,
            trigger_id=session.ctx.trigger_id,
        )


class WorkerTenancy(enum.StrEnum):
    COMMUNITY = "COMMUNITY"
    DEDICATED = "DEDICATED"

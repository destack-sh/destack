import enum
import sys
import traceback
from dataclasses import dataclass, field
from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union
from uuid import UUID

from bench.bench.core import Session
from bench.bench.reflect import reflect_enum, reflect_struct
from bench.utils.utils import IdentifierType, to_pyidentifier_multi

if TYPE_CHECKING:
    from bench.bench.code_ import Code
    from bench.bench.model import Model
    from bench.bench.task import Task


@dataclass
class ExecutionFrame:
    id: UUID
    module_id: UUID
    runnable: Union["Code", "Model", "Task"]
    root: Optional["ExecutionFrame"]
    parent: Optional["ExecutionFrame"]
    entered_at: datetime
    exited_at: Optional[datetime]
    cached_generated_at: Optional[datetime]
    cached_duration: Optional[float]
    inputs: Optional[dict[str, Any]]
    outputs: Optional[dict[str, Any]]
    error: Optional[Exception]
    queue_position: Optional[int]
    children: list["ExecutionFrame"] = field(default_factory=list)

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
    "bench.bench",
    "asgiref",
    "concurrent",
]
IGNORED_PACKAGE_PATHS = [package.replace(".", "/") for package in IGNORED_PACKAGE_PREFIXES]


@reflect_struct("RunCodeFrame", "The frame of code that was executed for a traceback")
class ExecutionCodeFrame:
    filename: str
    lineno: int
    name: str
    locals: dict[str, Any] = None  # locals should be richer for deep linking (with ids)
    line: str = None

    @staticmethod
    def from_stack(stack: traceback.StackSummary) -> list["ExecutionCodeFrame"]:
        return [
            ExecutionCodeFrame(
                filename=frame.filename,
                lineno=frame.lineno,
                name=frame.name,
                locals=frame.locals,
                line=frame.line,
            )
            for frame in stack
        ]

    @staticmethod
    def clean(
        stack: list["ExecutionCodeFrame"], from_code: "Code", session: "Session"
    ) -> list["ExecutionCodeFrame"]:
        from bench.bench.code_ import Code

        code_by_method: dict[str, Code] = {
            symbol._transform.method_name: symbol
            for symbol in session.instances.values()
            if isinstance(symbol, Code) and symbol._transform is not None
        }

        transform = from_code._transform
        found_start = False
        cleaned_stack = []
        for frame in stack:
            if any(prefix in frame.filename for prefix in IGNORED_PACKAGE_PATHS):
                continue  # skip support code
            if not found_start:
                # impute bench source info into instantiated code callables
                code = code_by_method.get(frame.name)
                if code is not None:
                    if code == from_code:
                        found_start = True
                    elif not found_start:
                        continue  # ignore
                    frame.filename = to_pyidentifier_multi(
                        from_code.file.name, from_code.name, type=IdentifierType.PATH
                    )
                    frame.name = from_code.name
                    frame.line = transform.transformed_code.splitlines()[frame.lineno - 1]
                    frame.lineno = frame.lineno - transform.start_offset
                    frame.locals = frame.locals or {}
                    for ident, var in code._references.items():
                        if ident not in frame.locals and var.id in session.instances:
                            frame.locals[ident] = repr(session.instances[var.id])
            if found_start:
                # trim file path for python modules
                python_version = f"{sys.version_info.major}.{sys.version_info.minor}"
                if python_version in frame.filename:
                    frame.filename = frame.filename.split(python_version)[-1][1:]  # skip slash
                cleaned_stack.append(frame)
        return cleaned_stack


@reflect_enum("RunErrorType", "Error type of a run")
class RunErrorKind(enum.StrEnum):
    INTERNAL = "INTERNAL"
    PARSE = "PARSE"
    VALIDATION = "VALIDATION"
    RUNTIME = "RUNTIME"
    UNTRUSTED = "UNTRUSTED"


@reflect_struct("RunError", "An error while running a statement")
class RunError(Exception):  # can this really be a subclass of Exception?
    """Wire-able representation of an exception."""

    kind: RunErrorKind
    type: str
    message: Optional[str] = None
    statement_id: Optional[UUID] = None
    traceback: list[ExecutionCodeFrame] = None


@reflect_struct("LogEntry", "A single log entry from a run")
class LogEntry:
    module_id: UUID
    created_at: datetime
    level: str
    logger: str
    session_id: Optional[UUID] = None
    statement_id: Optional[UUID] = None
    execution_id: Optional[UUID] = None
    message: Optional[str] = None
    metadata: dict[str, Any] = None

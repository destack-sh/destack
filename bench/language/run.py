import abc
import enum
import hashlib
import sys
import traceback
from dataclasses import dataclass, field
from datetime import datetime
from functools import cached_property
from typing import TYPE_CHECKING, Any, Optional, Union
from uuid import UUID

import msgpack

from bench.language.const import RunStatus, TriggerType
from bench.language.module import Module
from bench.language.reflect import reflect_struct
from bench.utils.dt import utcnow_with_tz
from bench.utils.utils import IdentifierType, to_pyidentifier_multi

if TYPE_CHECKING:
    from bench.language import Code, Field, Model, Session, Statement, Task, Trigger
    from bench.language.session import LogSearch, RunSearch


class HasRun(abc.ABC):
    """A runnable statement"""

    id: UUID
    ck: UUID
    module: "Module"
    _is_async: bool

    @property
    def logs(self) -> "LogSearch":
        from bench.language.session import LogSearch

        return LogSearch.from_runnable(self)

    @property
    def runs(self) -> "RunSearch":
        from bench.language.session import RunSearch

        return RunSearch.from_runnable(self)

    @cached_property
    def cache(self):
        from bench.language.cache import CacheAsync, CacheSync

        if self._is_async:
            return CacheAsync(self.module, subkey=self.ck.hex)
        else:
            return CacheSync(self.module, subkey=self.ck.hex)

    @property
    def current_run(self):
        return self.module.session.current_run

    def __call__(self, *args, **kwargs):
        raise NotImplementedError

    def to_sync(self) -> "_RunnableProxy":
        if not self._is_async:
            return self
        return _RunnableProxy.to_sync(self)

    def to_async(self) -> "_RunnableProxy":
        if self._is_async:
            return self
        return _RunnableProxy.to_async(self)


class _RunnableProxy:  # :SyncProxy
    """
    A simple proxy for Statement to enable to_sync/to_async while keeping the original Statement object.
    """

    def __init__(self, statement: "HasRun", is_async: bool):
        self._statement = statement
        self._is_async = is_async

    def __call__(self, *args, **kwargs):
        if self._is_async:
            return self.__call_async__(*args, **kwargs)
        else:
            return self.__call_sync__(*args, **kwargs)

    def __getattr__(self, item):
        return getattr(self._statement, item)

    @classmethod
    def to_sync(cls, statement: "Statement") -> "_RunnableProxy":
        proxy = cls(statement, is_async=False)
        proxy.__call_sync__ = statement.session.async_to_sync(statement.__call_async__)
        return proxy

    @classmethod
    def to_async(cls, statement: "Statement") -> "_RunnableProxy":
        proxy = cls(statement, is_async=True)
        proxy.__call_async__ = statement.session.sync_to_async(statement.__call_sync__)
        return proxy


@dataclass(slots=True)
class CachedRun:
    """A cached run of a code statement."""

    generated_at: datetime
    duration: float
    inputs: dict[str, Any]
    outputs: dict[str, Any]

    @staticmethod
    def bytes_from_run(inputs: dict, outputs: dict, started_at: datetime):
        now = utcnow_with_tz()
        run = CachedRun(
            generated_at=now,
            duration=(now - started_at).total_seconds(),
            inputs=inputs,
            outputs=outputs,
        )
        return run.to_json_bytes()

    def to_json_bytes(self) -> bytes:
        run_json = {
            "generated_at": self.generated_at.isoformat(),
            "duration": self.duration,
            "inputs": self.inputs,
            "outputs": self.outputs,
        }
        return msgpack.packb(run_json, use_bin_type=True)

    @staticmethod
    def from_json_bytes(json_bytes: bytes) -> "CachedRun":
        run_json = msgpack.unpackb(json_bytes, raw=False)
        return CachedRun(
            generated_at=datetime.fromisoformat(run_json["generated_at"]),
            duration=run_json["duration"],
            inputs=run_json["inputs"],
            outputs=run_json["outputs"],
        )


def get_run_cache_subkey(inputs_raw: Any, content_id: Optional[str] = None):
    inputs_bytes = msgpack.packb(inputs_raw, use_bin_type=True)
    input_hash = hashlib.sha256(inputs_bytes).hexdigest()
    if content_id:
        return f"run.{content_id}.{input_hash}"
    else:
        return f"run.{input_hash}"


@reflect_struct("RunMetadata", "Metadata for a run", return_type=True)
class RunMetadata:
    name: Optional[str]
    test: Optional[bool]
    queue_position: Optional[int]
    cached_at: Optional[datetime]
    cached_in: Optional[UUID]
    cached_duration: Optional[float]
    progress: Optional[float]


@dataclass
class Run:
    id: UUID
    runnable: Union["Code", "Model", "Task"]
    module: Module
    session: Optional["Session"]
    root: Optional["Run"]
    parent: Optional["Run"]
    scheduled_at: Optional[datetime]
    started_at: Optional[datetime]
    terminated_at: Optional[datetime]
    trigger_type: Optional[TriggerType]
    trigger: Union["Trigger", UUID]
    status: RunStatus = field(init=False)
    inputs: Optional[dict[str, Any]]
    outputs: Optional[dict[str, Any]]
    error: Optional["RunError"]
    metadata: Optional[dict[str, Any]]
    created_at: datetime = field(default_factory=utcnow_with_tz)
    updated_at: datetime = field(default_factory=utcnow_with_tz)
    children: list["Run"] = field(default_factory=list)

    def __post_init__(self):
        self._update_status()
        self.updated_at = utcnow_with_tz()

    def __str__(self):
        metadata_keys_str = ", ".join(self.metadata.keys()) if self.metadata else ""
        return f"{self.runnable} ({self.status}, metadata={metadata_keys_str or '<none>'})"

    def __repr__(self):
        return f"<Run {self}>"

    def _update_status(self):
        if self.error:
            self.status = RunStatus.Failed
        elif self.terminated_at:
            self.status = RunStatus.Completed
        elif self.queue_position:
            self.status = RunStatus.Queued
        else:
            self.status = RunStatus.Running

    @property
    def duration(self) -> float:
        if self.terminated_at is None:
            return 0
        return (self.terminated_at - self.started_at).total_seconds()

    @property
    def duration_with_cache(self) -> float:
        if self.terminated_at is None:
            return 0
        return self.duration + (self.cached_duration or 0)

    def walk_descendants(self):
        yield self
        for child in self.children:
            yield from child.walk_descendants()

    def get_metadata(self, key: Union[str, "Field"], default: Any = None) -> Any:
        if not isinstance(key, str):
            key = key.typed_key
        if self.metadata is None:
            return default
        return self.metadata.get(key, default)

    def set_metadata(self, key: Union[str, "Field"], value: Any):
        if not isinstance(key, str):
            key = key.typed_key
        if self.metadata is None:
            self.metadata = {}
        self.metadata[key] = value

    # direct accessors for default metadata (not great but good enough for now)
    # TODO @Cleanup: wrap all default run metadata and task metadata here

    @property
    def cached_duration(self) -> Optional[float]:
        return self.get_metadata(RunMetadata.cached_duration)

    @cached_duration.setter
    def cached_duration(self, value: Optional[float]):
        self.set_metadata(RunMetadata.cached_duration, value)

    @property
    def queue_position(self) -> Optional[int]:
        return self.get_metadata(RunMetadata.queue_position)

    @queue_position.setter
    def queue_position(self, value: Optional[int]):
        self.set_metadata(RunMetadata.queue_position, value)

    @property
    def cached_at(self) -> Optional[datetime]:
        cached_at = self.get_metadata(RunMetadata.cached_at)
        return datetime.fromisoformat(cached_at) if cached_at else None

    @cached_at.setter
    def cached_at(self, value: Optional[datetime]):
        self.set_metadata(RunMetadata.cached_at, value.isoformat() if value else None)

    @property
    def cached_in(self) -> Optional[UUID]:
        return self.get_metadata(RunMetadata.cached_in)

    @cached_in.setter
    def cached_in(self, value: Optional[UUID]):
        self.set_metadata(RunMetadata.cached_in, value)

    @property
    def test(self) -> Optional[bool]:
        return self.get_metadata(RunMetadata.test)

    @test.setter
    def test(self, value: Optional[bool]):
        self.set_metadata(RunMetadata.test, value)


_IGNORED_PACKAGE_PREFIXES = [
    "bench.runtime",
    "bench.bench",
    "asgiref",
    "concurrent",
]
_IGNORED_PACKAGE_PATHS = [package.replace(".", "/") for package in _IGNORED_PACKAGE_PREFIXES]


# RunError/LogEntry and many others should be reflect types as well, but missing Statement and such
# @reflect_struct("RunError", "An error while running a statement")


class RunCodeFrame:
    filename: str
    lineno: int
    name: str
    locals: dict[str, Any] = None  # locals should be richer for deep linking (with ids)
    line: str = None

    @staticmethod
    def from_dict(data: dict[str, Any]) -> "RunCodeFrame":
        return RunCodeFrame(
            filename=data["filename"],
            lineno=data["lineno"],
            name=data["name"],
            locals=data["locals"],
            line=data["line"],
        )

    @staticmethod
    def from_stack(stack: traceback.StackSummary) -> list["RunCodeFrame"]:
        return [
            RunCodeFrame(
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
        stack: list["RunCodeFrame"], from_statement: "Statement", session: "Session"
    ) -> list["RunCodeFrame"]:
        code_by_method: dict[str, Code] = {
            symbol._transform.method_name: symbol
            for symbol in session.instances_by_id.values()
            if isinstance(symbol, Code) and symbol._transform is not None
        }

        found_start = False
        cleaned_stack = []
        for frame in stack:
            if any(prefix in frame.filename for prefix in _IGNORED_PACKAGE_PATHS):
                continue  # skip support code
            if not found_start:
                # impute bench source info into instantiated code callables
                code = code_by_method.get(frame.name)
                if code is not None:
                    if code == from_statement:
                        found_start = True
                    elif not found_start:
                        continue  # ignore
                    frame.filename = to_pyidentifier_multi(
                        from_statement.file.name, from_statement.name, type=IdentifierType.PATH
                    )
                    frame.name = from_statement.name
                    frame.lineno = frame.lineno - code._transform.start_offset
                    frame.line = code.code.splitlines()[frame.lineno - 1]
                    frame.locals = frame.locals or {}
                    for ident, var in code._statement_references.items():
                        if ident not in frame.locals and var.id in session.instances_by_id:
                            frame.locals[ident] = repr(session.instances_by_id[var.id])
            if found_start:
                # trim file path for python modules
                python_version = f"{sys.version_info.major}.{sys.version_info.minor}"
                if python_version in frame.filename:
                    frame.filename = frame.filename.split(python_version)[-1][1:]  # skip slash
                cleaned_stack.append(frame)
        return cleaned_stack


class RunErrorKind(enum.StrEnum):
    Internal = "Internal"
    Parse = "Parse"
    Validation = "Validation"
    Runtime = "Runtime"
    Untrusted = "Untrusted"


@dataclass
class RunError(Exception):  # can this really be a subclass of Exception?
    """Wire-able representation of an exception."""

    kind: RunErrorKind
    type: str
    message: Optional[str] = None
    runnable: Optional["Statement"] = None
    traceback: list[RunCodeFrame] = None

    @staticmethod
    def from_exception(e: Exception, runnable: Optional["Statement"]) -> "RunError":
        if isinstance(e, RunError):
            return e
        stack = RunCodeFrame.from_stack(traceback.extract_tb(e.__traceback__))
        stack = RunCodeFrame.clean(stack, runnable, runnable.session)
        return RunError(
            kind=RunErrorKind.Runtime,
            type=type(e).__name__,
            message=str(e),
            runnable=runnable,
            traceback=stack,
        )

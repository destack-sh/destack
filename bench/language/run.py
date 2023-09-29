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

from bench.language.const import TERMINAL_RUN_STATUSES, RunStatus, TriggerType
from bench.language.module import NS, Module, ModuleNode, node_component
from bench.utils.dt import utcnow_with_tz
from bench.utils.proxy import proxy_value
from bench.utils.utils import IdentifierType, to_pyidentifier_multi

if TYPE_CHECKING:
    from bench.language import Session, Statement, Trigger
    from bench.language.session import LogSearch, RunSearch


@node_component
class HasRun(ModuleNode):
    """A runnable statement"""

    @property
    def _is_async(self) -> Optional[bool]:
        return None

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
        assert self._status == NS.Tracked, f"cannot call untracked {self!r}"
        if self._is_async:
            return self.__call_async__(*args, **kwargs)
        else:
            return self.__call_sync__(*args, **kwargs)

    def __call_sync__(self, *args, **kwargs):
        raise NotImplementedError

    async def __call_async__(self, *args, **kwargs):
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
        assert self._statement._status == NS.Tracked, f"cannot call untracked {self!r}"
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


# TODO @Architecture: sessions/runs are kind of like module nodes, but also kind of not
#  (have parent run/session, need revisions for value, no ck, activation, ..?)


@dataclass
class Run:
    id: UUID
    runnable: "Statement"
    module: Module
    session: Optional["Session"]
    root: Optional["Run"]
    parent: Optional["Run"]
    scheduled_at: Optional[datetime]
    started_at: Optional[datetime]
    terminated_at: Optional[datetime]
    trigger_type: Optional[TriggerType]
    trigger: Union["Trigger", UUID]
    status: RunStatus
    inputs: Optional[dict[str, Any]]
    outputs: Optional[dict[str, Any]]
    error: Optional["RunError"]
    value: dict[str, Any] = field(default_factory=dict)
    created_at: datetime = field(default_factory=utcnow_with_tz)
    updated_at: datetime = field(default_factory=utcnow_with_tz)
    children: list["Run"] = field(default_factory=list)
    _value_unpacked: bool = False

    def __post_init__(self):
        self.updated_at = utcnow_with_tz()

    def __str__(self):
        value_keys_str = ", ".join(self.value.keys()) if self.value else ""
        return f"{self.runnable} ({self.status}, value={value_keys_str or '<none>'}, {self.id})"

    def __repr__(self):
        return f"<Run {self}>"

    def mark_dead_if_active(self):
        if self.active:
            self.terminated_at = utcnow_with_tz()
            self.status = RunStatus.Aborted if self.started_at else RunStatus.Cancelled

    def _activate_inner(self, session: "Session", **kwargs):
        from bench.language.libs import symbolx_lib
        from bench.language.mapping import check_type, unpack_value

        metatype = symbolx_lib.resolve(".reflect.RunMetadata")

        def _onwrite_value(key: str):
            check_type(self.value, metatype)

        value = unpack_value(self.value, metatype, ignore_array=True, ignore_outer_map=True)
        self.value = proxy_value(
            value, onread=lambda *args: None, onwrite=_onwrite_value, default_none=True
        )
        self._value_unpacked = True

        for key, value in kwargs.items():
            self.value[key] = value

    def _raw_value(self) -> dict:
        from bench.language.libs import symbolx_lib
        from bench.language.mapping import pack_value

        run_value = symbolx_lib.resolve(".reflect.RunMetadata")

        if not self._value_unpacked:
            return self.value
        else:
            return pack_value(self.value, run_value, ignore_array=True, ignore_outer_map=True)

    @property
    def active(self) -> bool:
        return self.status not in TERMINAL_RUN_STATUSES

    @property
    def duration(self) -> float:
        if self.terminated_at is None:
            return 0
        return (self.terminated_at - self.started_at).total_seconds()

    def walk_descendants(self):
        yield self
        for child in self.children:
            yield from child.walk_descendants()


_IGNORED_PACKAGE_PREFIXES = [
    "bench.runtime",
    "bench.bench",
    "asgiref",
    "concurrent",
]
_IGNORED_PACKAGE_PATHS = [package.replace(".", "/") for package in _IGNORED_PACKAGE_PREFIXES]


@dataclass
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
        from bench.language.code_ import HasCode

        code_by_method: dict[str, HasCode] = {
            node._transform.method_name: node
            for node in session.module._nodes
            if isinstance(node, HasCode) and node._transform is not None
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
                        if ident not in frame.locals and var.id in session.module._tree:
                            frame.locals[ident] = repr(session.module._tree[var.id])
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


@dataclass
class LogEntry:
    id: UUID
    module: Module
    created_at: datetime
    stream: str
    session: "Session"
    level: Optional[str] = None
    logger: Optional[str] = None
    runnable: Optional["Statement"] = None
    run: Optional["Run"] = None
    message: Optional[str] = None
    value: dict[str, Any] = None

    def __str__(self):
        return f"'{self.message}' ({self.created_at})"

    def __repr__(self):
        return f"<LogEntry {self}>"

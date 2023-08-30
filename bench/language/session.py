import enum
import sys
import traceback
from dataclasses import dataclass, field
from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union
from uuid import UUID

from bench.language.const import TriggerType
from bench.language.core import Module, Session, Statement, StatementType
from bench.language.query import Query, Sort, SortOrder
from bench.language.reflect import reflect_enum, reflect_struct
from bench.language.search import Search
from bench.utils.dt import utcnow_with_tz
from bench.utils.utils import IdentifierType, to_pyidentifier_multi

if TYPE_CHECKING:
    from bench.language.code_ import Code
    from bench.language.flow import Trigger
    from bench.language.model import Model
    from bench.language.task import Task
    from bench.language.type import Field
    from bench.language.wire import LogEntryData, RunData


class WorkerProfile(enum.StrEnum):
    TINY = "TINY"
    SMALL = "SMALL"
    MEDIUM = "MEDIUM"
    LARGE = "LARGE"
    XLARGE_CPU = "XLARGE_CPU"
    XLARGE_MEM = "XLARGE_MEM"


class WorkerRegion(enum.StrEnum):
    US_CENTRAL = "US_CENTRAL"
    EU_CENTRAL = "EU_CENTRAL"


class WorkerSetStatus(enum.StrEnum):
    SLEEPING = "SLEEPING"
    PENDING = "PENDING"
    UPDATING = "UPDATING"
    HEALTHY = "HEALTHY"
    UNHEALTHY = "UNHEALTHY"
    UNAVAILABLE = "UNAVAILABLE"
    UNKNOWN = "UNKNOWN"


@reflect_enum("RunStatus", "The status of a run")
class RunStatus(enum.StrEnum):
    Scheduled = "Scheduled"
    Queued = "Queued"
    Running = "Running"
    Suspended = "Suspended"
    Aborting = "Aborting"
    # terminal statuses
    Cancelled = "Cancelled"
    Aborted = "Aborted"
    Failed = "Failed"
    Completed = "Completed"


TERMINAL_RUN_STATUSES = {
    RunStatus.Cancelled,
    RunStatus.Aborted,
    RunStatus.Failed,
    RunStatus.Completed,
}
PENDING_RUN_STATUSES = set(RunStatus) - TERMINAL_RUN_STATUSES


@dataclass
class LazyRun:
    """Run that's not loaded."""

    id: UUID

    def load(self) -> "Run":
        raise NotImplementedError


@reflect_struct("RunMetadata", "Default metadata of a run", return_type=True)
class RunMetadata:
    test: Optional[bool]
    queue_position: Optional[int]
    cached_generated_at: Optional[datetime]
    cached_duration: Optional[float]
    progress: Optional[float]


@dataclass
class Run:
    id: UUID
    runnable: Union["Code", "Model", "Task"]
    module: Module
    session: Optional[Session]
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
    def cached_generated_at(self) -> Optional[datetime]:
        cached_generated_at = self.get_metadata(RunMetadata.cached_generated_at)
        return datetime.fromisoformat(cached_generated_at) if cached_generated_at else None

    @cached_generated_at.setter
    def cached_generated_at(self, value: Optional[datetime]):
        self.set_metadata(RunMetadata.cached_generated_at, value.isoformat() if value else None)


_IGNORED_PACKAGE_PREFIXES = [
    "bench.runtime",
    "bench.bench",
    "asgiref",
    "concurrent",
]
_IGNORED_PACKAGE_PATHS = [package.replace(".", "/") for package in _IGNORED_PACKAGE_PREFIXES]


@reflect_struct("RunCodeFrame", "The frame of code that was executed for a traceback")
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
        from bench.language.code_ import Code

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


@reflect_enum("RunErrorType", "Error type of a run")
class RunErrorKind(enum.StrEnum):
    INTERNAL = "INTERNAL"
    PARSE = "PARSE"
    VALIDATION = "VALIDATION"
    RUNTIME = "RUNTIME"
    UNTRUSTED = "UNTRUSTED"


# RunError/LogEntry and many others should be reflect types as well, but missing Statement and such
# @reflect_struct("RunError", "An error while running a statement")
@dataclass
class RunError(Exception):  # can this really be a subclass of Exception?
    """Wire-able representation of an exception."""

    kind: RunErrorKind
    type: str
    message: Optional[str] = None
    runnable: Optional[Statement] = None
    traceback: list[RunCodeFrame] = None

    @staticmethod
    def from_exception(e: Exception, runnable: Optional[Statement]) -> "RunError":
        if isinstance(e, RunError):
            return e
        stack = RunCodeFrame.from_stack(traceback.extract_tb(e.__traceback__))
        stack = RunCodeFrame.clean(stack, runnable, runnable.session)
        return RunError(
            kind=RunErrorKind.RUNTIME,
            type=type(e).__name__,
            message=str(e),
            runnable=runnable,
            traceback=stack,
        )


# @reflect_struct("LogEntry", "A single log entry from a run")
@dataclass
class LogEntry:
    id: UUID
    module: Module
    created_at: datetime
    stream: str
    session: Session
    level: Optional[str] = None
    logger: Optional[str] = None
    runnable: Optional[Statement] = None
    run: Optional[Run] = None
    message: Optional[str] = None
    metadata: dict[str, Any] = None

    def __str__(self):
        return f"'{self.message}' ({self.created_at})"

    def __repr__(self):
        return f"<LogEntry {self}>"


class RunSearch(Search["RunData", Run]):
    """Search over runs."""

    def __init__(
        self,
        module: Module,
        runnables: list[Statement] | None,
        query: Query | None,
        sort: list[Sort] | None,
        limit: Optional[int],
    ):
        super().__init__(module, query, sort, limit)
        self.runnables = runnables

    async def _do_search(
        self, after: list[Any] = None, limit: Optional[int] = None, count: bool = False
    ):
        from bench.msg import NMessage
        from bench.msg.core import request
        from bench.msg.messages import NMessageType, RepSearchRunPayload, ReqSearchRunsPayload

        batch_limit = min(self.RESULT_BATCH_SIZE, limit or self._limit or self.RESULT_BATCH_SIZE)
        runnables_cks = [runnable.ck for runnable in self.runnables] if self.runnables else None
        rep: NMessage[RepSearchRunPayload] = await request(
            NMessageType.SEARCH_RUNS,
            ReqSearchRunsPayload(
                module_id=self.module.id,
                runnables_cks=runnables_cks,
                query=self._query,
                sort=self._sort,
                after=after,
                limit=batch_limit,
                count=count,
            ),
            reply_t=RepSearchRunPayload,
        )
        if rep.p.error:
            raise RuntimeError(f"{self} failed (after={after}, limit={limit}): {rep.p.error}")
        return rep

    def _unpack_element_data(self, element_data: "RunData") -> Run:
        from bench.language import wire

        return wire.unpack_data(element_data, module=self.module)

    def filter(self, query: Query) -> "RunSearch":
        combined_query = Query.and_if_set(self._query, query)
        return RunSearch(self.module, self.runnables, combined_query, self._sort, self._limit)

    def sort(self, sort: list[Sort] | Sort) -> "RunSearch":
        sort = [sort] if isinstance(sort, Sort) else sort
        return RunSearch(self.module, self.runnables, self._query, sort, self._limit)

    def limit(self, limit: int) -> "RunSearch":
        return RunSearch(self.module, self.runnables, self._query, self._sort, limit)

    @staticmethod
    def from_runnable(runnable: Statement) -> "RunSearch":
        return RunSearch(
            module=runnable.module,
            runnables=[runnable],
            query=None,
            sort=[Sort("created_at", SortOrder.DESCENDING)],
            limit=None,
        )


class LogSearch(Search["LogEntryData", LogEntry]):
    """Search over logs."""

    def __init__(
        self,
        module: Module,
        runnables: list[Statement] | None,
        query: Query | None,
        sort: list[Sort] | None,
        limit: Optional[int],
    ):
        super().__init__(module, query, sort, limit)
        self.runnables = runnables

    async def _do_search(
        self, after: list[Any] = None, limit: Optional[int] = None, count: bool = False
    ):
        from bench.msg import NMessage
        from bench.msg.core import request
        from bench.msg.messages import NMessageType, RepSearchLogPayload, ReqSearchLogPayload

        batch_limit = min(self.RESULT_BATCH_SIZE, limit or self._limit or self.RESULT_BATCH_SIZE)
        runnables_ids = [runnable.id for runnable in self.runnables] if self.runnables else None
        rep: NMessage[RepSearchLogPayload] = await request(
            NMessageType.SEARCH_LOGS,
            ReqSearchLogPayload(
                module_id=self.module.id,
                runnables_ids=runnables_ids,
                query=self._query,
                sort=self._sort,
                after=after,
                limit=batch_limit,
                count=count,
            ),
            reply_t=RepSearchLogPayload,
        )
        if rep.p.error:
            raise RuntimeError(f"{self} failed (after={after}, limit={limit}): {rep.p.error}")
        return rep

    def _unpack_element_data(self, element_data: "LogEntryData") -> LogEntry:
        from bench.language import wire

        return wire.unpack_data(element_data, module=self.module)

    def filter(self, query: Query) -> "LogSearch":
        combined_query = Query.and_if_set(self._query, query)
        return LogSearch(self.module, self.runnables, combined_query, self._sort, self._limit)

    def sort(self, sort: list[Sort] | Sort) -> "LogSearch":
        sort = [sort] if isinstance(sort, Sort) else sort
        return LogSearch(self.module, self.runnables, self._query, sort, self._limit)

    def limit(self, limit: int) -> "LogSearch":
        return LogSearch(self.module, self.runnables, self._query, self._sort, limit)

    @staticmethod
    def from_runnable(runnable: Statement) -> "LogSearch":
        return LogSearch(
            module=runnable.module,
            runnables=[runnable],
            query=None,
            sort=[Sort("created_at", SortOrder.DESCENDING)],
            limit=None,
        )


@dataclass
class MissingStatement:
    id: UUID
    type: Optional[StatementType] = None

    def __str__(self):
        return str(self.id)

    def __repr__(self):
        return f"<MissingStatement {self.id} {self.type or '<unknown type>'}>"

    def __getattr__(self, item):
        if item == "id":
            return self.id
        raise AttributeError(f"statement {self.id} not found, so {item} cannot be accessed")

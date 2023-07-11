import enum
import sys
import traceback
from dataclasses import dataclass, field
from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union
from uuid import UUID

from bench.bench.core import Session, Statement, Module
from bench.bench.query import Query, Sort
from bench.bench.reflect import reflect_enum, reflect_struct
from bench.bench.search import Search
from bench.utils.utils import IdentifierType, to_pyidentifier_multi

if TYPE_CHECKING:
    from bench.bench.code_ import Code
    from bench.bench.model import Model
    from bench.bench.task import Task
    from bench.bench.wire import RunData, LogEntryData


@reflect_enum("RunStatus", "The status of a run")
class RunStatus(enum.StrEnum):
    Created = "Created"
    Scheduled = "Scheduled"
    Queued = "Queued"
    Running = "Running"
    Aborting = "Aborting"
    # terminal statuses
    Aborted = "Aborted"
    Failed = "Failed"
    Completed = "Completed"


TERMINAL_RUN_STATUSES = {
    RunStatus.Aborted,
    RunStatus.Failed,
    RunStatus.Completed,
}
PENDING_RUN_STATUSES = set(RunStatus) - TERMINAL_RUN_STATUSES


@dataclass
class Run:
    id: UUID
    runnable: Union["Code", "Model", "Task"]
    session: Session
    root: Optional["Run"]
    parent: Optional["Run"]
    started_at: datetime
    terminated_at: Optional[datetime]
    cached_generated_at: Optional[datetime]
    cached_duration: Optional[float]
    queue_position: Optional[int]
    inputs: Optional[dict[str, Any]]
    outputs: Optional[dict[str, Any]]
    error: Optional["RunError"]
    metadata: Optional[dict[str, Any]]
    children: list["Run"] = field(default_factory=list)

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
        stack: list["RunCodeFrame"], from_code: "Code", session: "Session"
    ) -> list["RunCodeFrame"]:
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
            if any(prefix in frame.filename for prefix in _IGNORED_PACKAGE_PATHS):
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


# @reflect_struct("LogEntry", "A single log entry from a run")
@dataclass
class LogEntry:
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
        super().__init__(query, sort, limit)
        self.module = module
        self.runnables = runnables

    async def _do_search(
        self, after: list[Any] = None, limit: Optional[int] = None, count: bool = False
    ):
        from bench.msg import NMessage
        from bench.msg.core import request
        from bench.msg.messages import NMessageType, ReqSearchRunPayload, RepSearchRunPayload

        batch_limit = min(self.RESULT_BATCH_SIZE, limit or self._limit or self.RESULT_BATCH_SIZE)
        runnables_ids = [runnable.id for runnable in self.runnables] if self.runnables else None
        rep: NMessage[RepSearchRunPayload] = await request(
            NMessageType.REQUEST_SEARCH_RUN,
            ReqSearchRunPayload(
                module_id=self.module.id,
                runnables_ids=runnables_ids,
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
        parent = self.module._statements_by_id.get(element_data.parent_id)
        if parent is None:
            parent = MissingStatement(element_data.parent_id)
        raise NotImplementedError

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
            sort=[Sort("created_at", desc=True)],
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
        super().__init__(query, sort, limit)
        self.module = module
        self.runnables = runnables

    async def _do_search(
        self, after: list[Any] = None, limit: Optional[int] = None, count: bool = False
    ):
        from bench.msg import NMessage
        from bench.msg.core import request
        from bench.msg.messages import NMessageType, ReqSearchLogPayload, RepSearchLogPayload

        batch_limit = min(self.RESULT_BATCH_SIZE, limit or self._limit or self.RESULT_BATCH_SIZE)
        runnables_ids = [runnable.id for runnable in self.runnables] if self.runnables else None
        rep: NMessage[RepSearchLogPayload] = await request(
            NMessageType.REQUEST_SEARCH_LOG,
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
        from bench.bench import wire

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
            sort=[Sort("created_at", desc=True)],
            limit=None,
        )


@dataclass
class MissingStatement:
    id: UUID

    def __str__(self):
        return str(self.id)

    def __repr__(self):
        return f"<MissingStatement {self.id}>"

    def __getattr__(self, item):
        if item == "id":
            return self.id
        raise AttributeError(f"statement {self.id} not found, so {item} cannot be accessed")

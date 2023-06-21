from __future__ import annotations

import abc
import contextvars
import enum
import re
import typing
import uuid
from collections import defaultdict
from concurrent.futures import Executor, ThreadPoolExecutor
from dataclasses import dataclass, field
from datetime import datetime
from functools import cached_property
from typing import Callable, NamedTuple, Optional, Union
from uuid import UUID, uuid4

import structlog
from asgiref.sync import async_to_sync, sync_to_async
from more_itertools import first

from bench.bench.const import ExecutionTriggerType
from bench.bench.issue import BenchError, Issue, IssueHandler, IssueKind, IssueType
from bench.settings import logging
from bench.utils.utils import required_field, to_pyidentifier

if typing.TYPE_CHECKING:
    from bench.bench.mutate import ModuleMutation

logger = structlog.get_logger(__name__)


class ModuleObjectType(enum.StrEnum):
    # source
    MODULE = "MODULE"
    FILE = "FILE"
    STATEMENT = "STATEMENT"
    FIELD = "FIELD"
    RECORD = "RECORD"
    DATASET_VIEW = "DATASET_VIEW"
    DATASET_VIEW_FIELD = "DATASET_VIEW_FIELD"
    # interp
    ISSUE = "ISSUE"
    RESOLVED_FIELD = "RESOLVED_FIELD"
    # user
    COMMENT = "COMMENT"


class InterpScope(enum.StrEnum):  # not sure if we still need this?
    MODULE = "module"
    FILE = "file"
    STATEMENT = "statement"


class StatementType(enum.StrEnum):
    """The type of Bench statement."""

    TEXT = "text"
    BLANK = "blank"
    TYPE = "type"
    TASK = "task"
    EXPECTATION = "expect"
    CODE = "code"
    MODEL = "model"
    VALUE = "value"
    DATASET = "dataset"
    REQUIREMENT = "require"
    BLOCK = "block"


MOT = ModuleObjectType
ModuleReference = typing.NamedTuple(
    "ModuleReference", [("name", str), ("version", str), ("id", typing.Optional[UUID])]
)


def statement_path_as_str(statement_path: StatementPath) -> str:
    return f"{statement_path.path}:{statement_path.name}"


def parse_statement_path(statement_path: str) -> StatementPath:
    if ":" not in statement_path:
        raise ValueError(f"invalid statement path: {statement_path}")
    path, name = statement_path.split(":")
    return StatementPath(path, name)


REFERENCE_REGEX = re.compile(
    r"^((?P<module_owner>[\w\- ]+)\.(?P<module_name>[\w\- ]+))?(\.(?P<path>[\w.\- ]+)\.)?(?P<name>[\w\- ]+)$"
)
RELATIVE_REFERENCE_REGEX = re.compile(r"^\.(?P<path>[\w.\- ]+)$")
ABSOLUTE_IMPORT_SOURCE_REGEX = re.compile(
    r"^(?P<module_owner>[\w\- ]+)\.(?P<module_name>[\w\- ]+)\.(?P<path>[\w.\- ]+)$"
)


class LookupBy(enum.StrEnum):
    Name = "name"
    PyIdent = "py_ident"


if typing.TYPE_CHECKING:
    node = dataclass
else:

    def node(cls):
        """Decorator alias for dataclass."""
        cls = dataclass(cls, repr=False, eq=False)
        cls._PROPERTIES = [f.name for f in cls.__dataclass_fields__.values()]
        return cls


@node
class ModuleNode(abc.ABC):
    id: UUID = field(default_factory=uuid.uuid4)
    parent: Optional[ModuleNode] = None
    revision: int = 0

    def __eq__(self, other):
        return self.id == other.id

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.parent.id if self.parent is not None else None

    def walk(self) -> typing.Iterator[ModuleNode]:
        from bench.bench.wire import walk_node

        return walk_node(self)

    @property
    def attached(self) -> bool:
        return self.parent is not None


@node
class HasCrud(abc.ABC):
    created_at: datetime = field(default_factory=datetime.utcnow)
    updated_at: datetime = field(default_factory=datetime.utcnow)
    last_edited_at: datetime = field(default_factory=datetime.utcnow)
    last_changed_at: datetime = field(default_factory=datetime.utcnow)
    revision: int = 0


@node
class HasSession(abc.ABC):
    id: UUID = field(default_factory=uuid.uuid4)
    _session: "Session" = None

    def __post_init__(self):
        if self._session is None:
            self._session = active_session.get()
            if self._session is not None:
                self._session.add(self, new=True)
        else:
            self._session.add(self, new=False)

    def __del__(self):
        if self._session is not None:
            self._session.remove(self)

    def instantiate_in(self, session: "Session") -> None:
        """'Instantiate' this object in the given session."""
        if self._session is not None:
            self._session.remove(self)
        self._session = session
        session.add(self, new=False)

    @property
    def session(self) -> "Session":
        """Access the session, error-ing if there is none."""
        if self._session is None:
            raise RuntimeError(f"no active session for {self}")
        return self._session

    @session.setter
    def session(self, session: Optional["Session"]):
        """Set the session."""
        self._session = session

    @property
    def logger(self) -> logging.Logger:
        return self.session.logger


@node
class HasIssues(abc.ABC):
    issues: list[Issue] | None = field(default_factory=list)

    @property
    def errors(self) -> list[Issue]:
        if self.issues is None:
            return []
        return [i for i in self.issues if i.kind == IssueKind.ERROR]

    def _on_issue(
        self,
        issue: "Issue" = None,
        *,
        subject: Union["Symbol", "Statement", "File", None] = None,
        type: IssueType = None,
        **kwargs,
    ):
        if issue is None:
            issue = Issue(type=type, subject=subject, **kwargs)
        if self.issues is None:
            self.issues = []
        self.issues.append(issue)
        if self.parent is not None:
            self.parent._on_issue(issue)


SymbolT = typing.TypeVar("SymbolT", bound="Symbol")


@node
class Scope:
    parent: Optional[Scope] = None
    scopes_by_name: dict[str, Scope] = field(default_factory=dict)
    symbols_by_id: dict[UUID, Symbol] = field(default_factory=dict)
    names_by_identifier: dict[str, str] = field(default_factory=dict)

    @cached_property
    def root_scope(self) -> Scope:
        if self.parent is None:
            return self
        return self.parent.root_scope

    def get_in_scope(self, name: str, by: LookupBy) -> Scope | None:
        if by == LookupBy.Name:
            return self.scopes_by_name.get(name)
        elif by == LookupBy.PyIdent:
            if name in self.names_by_identifier:
                name = self.names_by_identifier[name]
                return self.scopes_by_name.get(name)
        else:
            raise ValueError(f"unexpected lookup type: {by}")
        return None

    def find_scope(self, name: str, by: LookupBy) -> Scope | None:
        scope = self.get_in_scope(name, by)
        if scope is not None:
            return scope
        if self.parent is not None:
            return self.parent.find_scope(name, by=by)
        return None

    def find_symbol(self, name: str, by: LookupBy) -> SymbolT | None:
        """Find the statement recursively in this scope and its parents."""
        scope = self.find_scope(name, by)
        if scope is not None and not isinstance(scope, Symbol):
            raise TypeError(f"expected symbol, got {type(scope)}")
        return scope

    def lookup_symbol(
        self,
        path: StatementPath | UUID | str,
        symbol_t: StatementType | typing.Type[SymbolT] | None = None,
        by: LookupBy = LookupBy.Name,
    ) -> SymbolT | None:
        """Lookup the symbol either by path or id. If path is a string, it can be
        it can be a name (lookup upwards) or a full relative/absolute path).
        """
        if isinstance(path, UUID):
            return self.root_scope.symbols_by_id.get(path)
        elif isinstance(path, str):
            if ":" not in path and "." not in path:
                return self.find_symbol(path, by=by)
            path = parse_statement_path(path)
        if path.path == ".":
            return self.find_symbol(path.name, by=by)
        # strip leading . in path
        path = StatementPath(path.path[1:], path.name)
        first_part = path.path.split(".")[0]
        scope = self.find_scope(first_part, by=by)
        if scope is None:
            return None
        return scope.lookup_symbol(path, symbol_t=symbol_t)

    def _add_statement(self, statement: Statement, by_name: bool) -> None:
        if statement.name is not None and by_name:
            if statement.name in self.scopes_by_name:
                self._on_issue(
                    type=IssueType.AMBIGUOUS_DEFINITION, subject=statement, path=statement.path
                )
            else:
                self.scopes_by_name[statement.name] = statement
                self.names_by_identifier[statement.ident] = statement.name

        self.symbols_by_id.update(statement.symbols_by_id)
        if isinstance(statement, Symbol):
            self.symbols_by_id[statement.id] = statement

    def _clear(self):
        """Resets this scope and all child scopes."""
        self.scopes_by_name.clear()
        self.symbols_by_id.clear()
        self.names_by_identifier.clear()
        for scope in self.scopes_by_name.values():
            scope._clear()


@node
class Module(ModuleNode, HasCrud, HasSession, HasIssues, Scope):
    name: str = required_field()
    files: list[File] = field(default_factory=list)
    dependencies: dict[str, Module | ModuleReference] = field(default_factory=dict)
    parent: None = None
    parent_scope: Scope = None
    committed: bool = False

    @property
    def attached(self) -> bool:
        return True  # root is always "attached"

    def add_dependency(self, module: Module | ModuleReference) -> None:
        if module.name in self.dependencies:
            raise ValueError(
                f"{self} has dependency {module.name}: {self.dependencies[module.name]}"
            )
        self.dependencies[module.name] = module

    def lookup_symbol(
        self,
        path: StatementPath | UUID | str,
        symbol_t: typing.Type[SymbolT] | None = None,
        by: LookupBy = LookupBy.Name,
    ) -> SymbolT:
        if isinstance(path, UUID):
            return self.symbols_by_id.get(path)
        elif path.startswith("."):
            return super().lookup_symbol(path, symbol_t=symbol_t, by=by)
        else:
            match = REFERENCE_REGEX.match(path)
            module_name = match.group("module_owner") + "." + match.group("module_name")
            dependency = self.dependencies.get(module_name)
            if dependency is None:
                raise LookupError(f"could not find dependency {module_name}")
            localized_path = "." + match.group("path") + ":" + match.group("name")
            return dependency.lookup_symbol(localized_path, symbol_t=symbol_t, by=by)

    def __str__(self):
        return f"{self.name} ({len(self.files)} files)"

    def __repr__(self):
        return f"<Module {str(self)}>"

    def instantiate_in(self, session: Session):
        if self._session is not None:
            self._session.remove(self)
        self._session = session
        self.clear()
        self.index()
        self.interp()
        for n in self.walk():
            if n != self and isinstance(n, HasSession):
                n.instantiate_in(session)

    def clear(self):
        super()._clear()
        for file in self.files:
            file._clear()

    def index(self):
        for file in self.files:
            file._index()
            if file.name in self.scopes_by_name:
                self._on_issue(type=IssueType.AMBIGUOUS_DEFINITION, subject=file, path=file.name)
            else:
                self.scopes_by_name[file.name] = file
            self.symbols_by_id.update(file.symbols_by_id)

    def interp(self):
        for file in self.files:
            file._interp()


@node
class File(ModuleNode, HasCrud, HasSession, HasIssues, Scope):
    module: Module = required_field()
    name: str = required_field()
    parent: File | Module = None
    children: list[File] | None = None
    statements: list[Statement] = field(default_factory=list)
    # index
    statements_by_parent_id: dict[UUID | None, list[Statement]] | None = None

    def __post_init__(self):
        super().__post_init__()
        if self.parent is None:
            self.parent = self.module
        self._sort()

    def __str__(self):
        return f"{self.module.name}/{self.name}"

    def __repr__(self):
        return f"<File {str(self)}>"

    @property
    def path(self) -> str:
        if isinstance(self.parent, File):
            return f"{self.parent.path}.{self.name}"
        else:
            return self.name

    def append(self, *statements: Statement):
        """Appends the statements to this file."""
        for statement in statements:
            statement.file = self
            statement.parent_scope = self
            self.statements.append(statement)

    def _sort(self):
        """Sorts the files statements in-place according to parent & order keys."""
        # per parent (incl. root = None) sort by order key
        sorted_statements = []
        self.statements_by_parent_id: dict[UUID, list[Statement]] = defaultdict(list)
        for statement in self.statements:
            self.statements_by_parent_id[statement.parent_id].append(statement)

        def walk_dfs(statement: Statement):
            sorted_statements.append(statement)
            children = self.statements_by_parent_id.get(statement.id)
            if children is not None:
                for child in sorted(children, key=lambda s: s.order_key):
                    walk_dfs(child)

        roots = self.statements_by_parent_id.get(self.id, [])
        for statement in sorted(roots, key=lambda s: s.order_key):
            walk_dfs(statement)

        self.statements = sorted_statements

    def _clear(self):
        """Resets this scope and all child scopes."""
        super()._clear()
        for statement in self.statements:
            statement._clear()

    def _index(self):
        """Indexes all statements in this file into the scope."""
        self._sort()
        for statement in self.statements:
            statement._index()
            is_root = statement.parent == self
            self._add_statement(statement, by_name=is_root)

    def _interp(self):
        for statement in self.statements:
            statement._interp(statement)


@node
class Statement(ModuleNode, HasCrud, HasSession, HasIssues, Scope):
    """A Bench statement."""

    file: File | None = None
    parent: Statement | File = None
    children: list[Statement] | None = None
    order_key: str | None = None
    type: StatementType = StatementType.BLANK
    name: Optional[str] = None
    id: UUID = field(default_factory=uuid.uuid4)

    def __post_init__(self):
        super().__post_init__()
        if self.file is None and self.parent is not None:
            self.file = self.parent.file
        if self.parent is None:
            self.parent = self.file

    def __str__(self):
        return f"{self.path} {self.name or '<anon>'}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def module(self) -> Module:
        if self.file is None:
            raise ValueError("statement is detached")
        return self.file.module

    @property
    def path(self) -> str:
        if self.file is None:
            return f"<detached>:{self.infile_path}"
        else:
            return self.file.path + ":" + str(self.infile_path)

    @property
    def infile_path(self) -> str:
        parent = self.parent
        ancestor_parts = [self.name or "<anon>"]
        seen_ids = {self.id}
        while isinstance(parent, Statement):
            if parent.id in seen_ids:
                # :CircularAncestry
                # circuit breaker: ignore here because this is an error in indexing
                ancestor_parts.append("<!loop>")
                break
            ancestor_parts.append(parent.name or "<anon>")
            seen_ids.add(parent.id)
            parent = parent.parent
        return ".".join(reversed(ancestor_parts))

    @property
    def fqn(self) -> str:
        if self.file is None:
            raise ValueError(f"cannot get fqn of detached statement {self}")
        return f"{self.file.module.name}.{self.file.name.replace('/', '.')}.{self.name}"

    @property
    def ident(self) -> str:
        return to_pyidentifier(self.name)

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.parent.id if self.parent is not None else None

    def _index(self):
        self._clear()
        self.children = self.file.statements_by_parent_id.get(self.id, [])
        for child in self.children:
            # only index self, not children
            # (unlike in file/module, statement nesting is only semantic, not structural)
            self._add_statement(child, by_name=True)

    def _interp(self, scope: Scope) -> None:
        pass


@node
class Blank(Statement):
    """A blank statement."""

    type: StatementType = StatementType.BLANK


@node
class Text(Statement):
    """A comment that's not semantic/interpreted by default."""

    type: StatementType = StatementType.TEXT
    text: str | None = None


StatementPath = NamedTuple("StatementPath", [("path", str), ("name", str)])
StatementReference = typing.Union[Statement, StatementPath, UUID]


class SymbolBase(abc.ABC):
    """Base for interpretable symbols for type-checking."""

    parent: Statement | File
    session: Session

    def _clear(self) -> None:
        raise NotImplementedError

    def _interp(self, scope: Scope) -> None:
        raise NotImplementedError

    def _reinterp(self, scope: Scope = None, raise_errors: bool = True) -> None:
        raise NotImplementedError

    _on_issue: IssueHandler


@node
class Symbol(Statement, SymbolBase):
    """An interpretable and semantic statement (symbol) in Bench source."""

    issues: list[Issue] | None = None

    def _clear(self) -> None:
        """Clears any derived/interpreted values on this symbol."""
        Statement._clear(self)
        self.issues = None

    def _interp(self, scope: Scope) -> None:
        """Updates, resolves and checks any derived/interpreted values on this symbol."""
        pass

    def _reinterp(self, scope: Scope = None, raise_errors: bool = True) -> None:
        """Clears and re-interprets this symbol in scope."""
        self._clear()
        self._index()
        self._interp(scope or self)
        if raise_errors and self.errors:
            raise BenchError(self.errors[0])


@node
class Requirement(Symbol):
    module_name: Optional[str] = None
    module_id: Optional[UUID] = None
    version: Optional[str] = None

    async def arequire(self) -> None:
        raise NotImplementedError

    def require(self) -> None:
        async_to_sync(self.arequire)()

    def __str__(self):
        return f"{self.module_name or '<unspecified>'}@{self.version or '<any>'}"

    def _interp(self, scope: Scope) -> None:
        pass


@node
class Block(Symbol):
    contents: list[Symbol] = field(default_factory=list)

    def __iter__(self):
        return iter(self.contents)

    def __getattr__(self, item: str):
        content = first(self.contents, lambda c: c.name == item, None)
        if content is not None:
            return content
        else:
            raise AttributeError(item)

    def _interp(self, scope: Scope) -> None:
        pass


class ModuleOp(enum.StrEnum):
    READ = "read"
    SEARCH = "search"
    CREATE = "create"
    UPDATE = "update"
    DELETE = "delete"


class SessionMode(enum.StrEnum):
    READ_ONLY = "ro"
    WRITE = "w"


active_session: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)


class SessionTracingLevel(enum.IntFlag):
    NONE = 0
    EXECUTION = 1
    MUTATION = 2
    VALIDATION = 4
    ALL = EXECUTION | MUTATION | VALIDATION


@dataclass(slots=True)
class SessionContext:
    module_id: UUID
    project_id: UUID
    worker_id: UUID
    tracing_level: SessionTracingLevel
    trigger_type: ExecutionTriggerType
    trigger_id: typing.Optional[UUID]
    root_id: typing.Optional[UUID] = None


class SessionBase(abc.ABC):
    module: Module

    @property
    def is_open(self) -> bool:
        raise NotImplementedError

    def add(self, obj: ModuleNode):
        raise NotImplementedError

    def remove(self, obj: ModuleNode):
        raise NotImplementedError


class Session:
    """A managed context for running code in a module (may mutate)."""

    def __init__(
        self,
        module: Module,
        ctx: SessionContext | None = None,
        cache_inferences: bool = True,
        inference_timeout: int = 30,
        inference_retries: int = 5,
        mode: SessionMode = SessionMode.READ_ONLY,
        write: Callable[[list[ModuleMutation]], typing.Awaitable[bool]] = None,
        executor: Executor = None,
    ):
        from bench.bench.mutate import ModuleMutator
        from bench.bench.tracing import SessionTracer

        if mode != SessionMode.READ_ONLY and write is None:
            raise ValueError("write must be provided for non-readonly sessions")
        self.id = uuid4()
        self.ctx = ctx
        self.module = module
        self.instances: dict[UUID, "HasSession"] = {}
        self.default_models = [
            module.lookup_symbol("openai.std.text.gpt3"),
            module.lookup_symbol("anthropic.std.text.claude-instant"),
        ]
        self.cache_inferences = cache_inferences
        self.inference_timeout = inference_timeout
        self.inference_retries = inference_retries
        self.mode = mode
        self.write = write

        self.anonymous_scope = Scope(parent=self.module)
        self.executor = executor or ThreadPoolExecutor(max_workers=1)
        self.logger = logger.bind(session=self)
        self.mutator = ModuleMutator(self.module)
        self.tracer = SessionTracer(self, mutator=self.mutator, publish=True, validate=True)
        self.opened_at: Optional[datetime] = None
        self.closed_at: Optional[datetime] = None

    def __str__(self):
        status = "open" if self.opened_at else ("closed" if self.closed_at else "pending")
        return (
            f"{self.module.name} {self.id} ({self.mode}, {status}, {len(self.mutator.mutations)})"
        )

    def __repr__(self):
        return f"<Session {self}>"

    def sync_to_async(self, fn: Callable) -> Callable[..., typing.Awaitable]:
        return sync_to_async(fn, thread_sensitive=False, executor=self.executor)

    def async_to_sync(self, fn: typing.Awaitable) -> Callable:
        return async_to_sync(fn)

    @property
    def is_open(self) -> bool:
        return self.opened_at is not None and self.closed_at is None

    def add(self, *objs: "HasSession", new: bool = False) -> None:
        if new:
            for obj in objs:
                # permissions are checked in tracer
                if isinstance(obj, File):
                    self.tracer.file_create(obj)
                elif isinstance(obj, Statement):
                    self.tracer.statement_create(obj)
                    if isinstance(obj, Symbol):
                        # it feels like this should be done in some tracer? also (re?)-index?
                        self.module.symbols_by_id[obj.id] = obj
        for obj in objs:
            self.instances[obj.id] = obj

    def remove(self, *objs: "HasSession") -> None:
        for obj in objs:
            if isinstance(obj, (Symbol, Statement, File)) and obj.id in self.instances:
                del self.instances[obj.id]
                # not doing anything yet?

    def check_can(self, op: ModuleOp, thing: File | Statement):
        if not self.can(op, thing):
            raise RuntimeError(f"cannot {op} {thing} in {self}")

    def can(self, op: ModuleOp, thing: File | Statement) -> bool:
        if self.mode == SessionMode.READ_ONLY:
            return op in (ModuleOp.READ, ModuleOp.SEARCH)
        elif self.mode == SessionMode.WRITE:
            return True
        else:
            raise RuntimeError(f"unknown session mode {self.mode}")

    def open(self):
        """Opens the session for execution and modification."""
        if self.opened_at is not None:
            raise RuntimeError(f"session already opened {self}")
        self.opened_at = datetime.now()
        if active_session.get() is not None:
            raise RuntimeError(f"another session is active: {active_session.get()}")
        active_session.set(self)
        logger.debug("session.open", session=self)

    async def aflush(self, keep_open: bool = True):
        """Flushes all module mutations."""
        if not self.mutator.mutations:
            return
        if self.mode == SessionMode.READ_ONLY:
            raise RuntimeError(f"cannot mutate read-only session {self}")
        logger.debug("session.flush", session=self, mutator=self.mutator)
        mutations = self.mutator.bundle().compact()
        success = await self.write(mutations)
        if not success:
            raise RuntimeError(f"failed to write mutations {self.mutator.mutations}")
        logger.debug("session.flush.done", session=self)

    def flush(self):
        async_to_sync(self.aflush)()

    async def aclose(self, flush: bool = True):
        """Closes the session, flushing any mutations and preventing further execution/mutation."""
        if self.closed_at is not None:
            raise RuntimeError(f"session already closed {self}")
        self.closed_at = datetime.now()
        if flush:
            await self.aflush(keep_open=False)
        active_session.set(None)
        logger.debug("session.close", session=self)

    def close(self, flush: bool = True):
        async_to_sync(self.aclose)(flush=flush)

    async def __aenter__(self):
        self.open()
        return self

    async def __aexit__(self, exc_type, exc_value, traceback):
        await self.aclose()

    def sync(self):
        """A sync context manager for this session."""
        session = self

        class SyncSession:
            def __enter__(self):
                session.open()
                return session

            def __exit__(self, exc_type, exc_value, traceback):
                session.close()

        return SyncSession()

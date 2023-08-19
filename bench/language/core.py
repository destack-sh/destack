import abc
import asyncio
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
from logging import Logger
from typing import Callable, NamedTuple, Optional, Union
from uuid import UUID, uuid4

import structlog
from asgiref.sync import async_to_sync, sync_to_async

from bench.language.const import StatementType, TriggerType
from bench.language.issue import BenchError, Issue, IssueHandler, IssueKind, IssueType
from bench.utils.dt import utcnow_with_tz
from bench.utils.fractional import generate_n_keys_between
from bench.utils.func import did_you_mean_str
from bench.utils.utils import IdentifierType, required_field, to_pyidentifier

if typing.TYPE_CHECKING:
    from bench.language.mutate import ModuleMutation, ModuleMutator
    from bench.language.session import LogEntry, Run
    from bench.language.wire import ModuleTreeData

logger = structlog.get_logger(__name__)


class ModuleObjectType(enum.StrEnum):
    # source
    MODULE = "MODULE"
    FILE = "FILE"
    STATEMENT = "STATEMENT"
    TRIGGER = "TRIGGER"
    TAGGING = "TAGGING"
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


MOT = ModuleObjectType
ModuleReference = typing.NamedTuple(
    "ModuleReference", [("name", str), ("version", str), ("id", typing.Optional[UUID])]
)

StatementPath = NamedTuple("StatementPath", [("path", str), ("name", str)])
StatementReference = typing.Union["Statement", StatementPath, UUID]
STATEMENT_REFERENCE_REGEX = re.compile(
    r"^((?P<module_owner>[\w\- ]+)\.(?P<module_name>[\w\- ]+))?\.(?P<path>[\w.\- ]+)"
)


def parse_absolute_statement_reference(path: str) -> tuple[str, str]:
    match = STATEMENT_REFERENCE_REGEX.match(path)
    if not match:
        raise ValueError(f"invalid absolute statement reference: {path}")
    module_name = match.group("module_owner") + "." + match.group("module_name")
    localized_path = "." + match.group("path")
    return module_name, localized_path


def parse_statement_path(statement_path: str) -> "StatementPath":
    path, name = statement_path.rsplit(".", 1)
    return StatementPath(path, name)


def statement_path_as_str(statement_path: "StatementPath") -> str:
    return f"{statement_path.path}:{statement_path.name}"


class LookupBy(enum.StrEnum):
    Name = "name"
    PyIdent = "py_ident"


@typing.dataclass_transform()
def node(cls: Optional[typing.Type] = None, tracked: list[str] = None):
    """
    Decorator alias for module node.
    Only tracked properties may be mutated during a session (by the user).
    """

    def decorate(cls):
        cls = dataclass(cls, repr=False, eq=False)  # type: ignore
        cls._PROPERTIES = [f.name for f in cls.__dataclass_fields__.values()]  # type: ignore
        # add @property methods to _PROPERTIES
        for name, attr in cls.__dict__.items():
            if isinstance(attr, property):
                cls._PROPERTIES.append(name)  # type: ignore
        # check that all tracked properties are actually properties
        for prop in tracked or []:
            if prop not in cls._PROPERTIES:  # type: ignore
                raise ValueError(f"invalid tracked property: {prop}")
        cls._TRACKED = tracked or []  # type: ignore
        # add tracked properties from base classes
        for base in cls.__bases__:  # type: ignore
            if hasattr(base, "_TRACKED"):
                cls._TRACKED.extend(base._TRACKED)  # type: ignore
        # can only track properties inside a session
        if cls._TRACKED and not issubclass(cls, HasSession):  # type: ignore
            raise ValueError("cannot have tracked properties without HasSession")

        return cls

    if cls is not None:
        return decorate(cls)

    return decorate


@node
class ModuleNode(abc.ABC):
    id: UUID = field(default_factory=uuid.uuid4)
    parent: Optional["ModuleNode"] = None
    revision: int = 0

    def __eq__(self, other):
        return isinstance(other, self.__class__) and self.id == other.id

    def __hash__(self):
        return hash(self.id)

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.parent.id if self.parent is not None else None

    def walk(self) -> typing.Iterator["ModuleNode"]:
        from bench.language.wire import walk_node

        return walk_node(self)

    def copy(self):
        from bench.language import wire

        _, node_datas = wire.pack_node(self)
        return wire.unpack_node(node_datas, parent=self.parent, session=None)

    @property
    def attached(self) -> bool:
        return self.parent is not None

    @property
    def path(self) -> str:
        raise NotImplementedError


@node
class HasCrud(abc.ABC):
    created_at: datetime = field(default_factory=utcnow_with_tz)
    updated_at: datetime = field(default_factory=utcnow_with_tz)
    last_edited_at: datetime = field(default_factory=utcnow_with_tz)
    last_changed_at: datetime = field(default_factory=utcnow_with_tz)
    revision: int = 0


CRUD_PROPERTIES = HasCrud._PROPERTIES  # type: ignore


@node
class HasSession(abc.ABC):
    id: UUID = field(default_factory=uuid.uuid4)
    _session: Optional["Session"] = None
    _tracked: bool = False

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

    def activate_in(self, session: "Session") -> None:
        """'Instantiate' this object in the given session."""
        if self._session is not None:
            self.deactivate()
        self._session = session
        session.add(self, new=False)
        self._tracked = True

    def deactivate(self) -> None:
        """'Deinstantiate' this object."""
        self._tracked = False  # need to set this first as some statements may redirect set/get
        if self._session is not None:
            self._session.remove(self)
        self._session = None

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
    def logger(self) -> Logger:
        return self.session.logger


@node
class HasIssues(abc.ABC):
    issues: list[Issue] = field(default_factory=list)

    @property
    def errors(self) -> list[Issue]:
        if self.issues is None:
            return []
        return [i for i in self.issues if i.kind == IssueKind.ERROR]

    @property
    def self_errors(self):
        return [i for i in self.errors if i.subject == self]

    def _on_issue(
        self,
        issue: "Issue" = None,
        *,
        subject: Union["Statement", "Statement", "File", None] = None,
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


StatementT = typing.TypeVar("StatementT", bound="Statement")


@node
class Scope:
    parent: Optional["Scope"] = None
    _scopes_by_name: dict[str, "Scope"] = field(default_factory=dict)
    _statements_by_id: dict[UUID, "Statement"] = field(default_factory=dict)
    _names_by_py_ident: dict[str, str] = field(default_factory=dict)

    @cached_property
    def _root_scope(self) -> "Scope":
        if self.parent is None:
            return self
        return self.parent._root_scope

    def _on_issue(
        self,
        issue: "Issue" = None,
        *,
        subject: Union["Statement", "Statement", "File", None] = None,
        type: IssueType = None,
        **kwargs,
    ):
        raise NotImplementedError

    def _get_scope(self, name: str, by: LookupBy) -> Union["Scope", None]:
        if by == LookupBy.Name:
            return self._scopes_by_name.get(name)
        elif by == LookupBy.PyIdent:
            if name in self._names_by_py_ident:
                name = self._names_by_py_ident[name]
                return self._scopes_by_name.get(name)
        else:
            raise ValueError(f"unexpected lookup type: {by}")
        return None

    def _find_scope(self, name: str, by: LookupBy) -> Union["Scope", None]:
        scope = self._get_scope(name, by)
        if scope is not None:
            return scope
        if self.parent is not None:
            return self.parent._find_scope(name, by=by)
        return None

    def _find_statement(self, name: str, by: LookupBy = LookupBy.Name) -> StatementT | None:
        """Find the statement recursively in this scope and its parents."""
        scope = self._find_scope(name, by)
        if scope is not None and not isinstance(scope, Statement):
            return None
        return typing.cast(StatementT, scope)

    def lookup(
        self,
        path: Union["StatementPath", UUID, str],
        by: LookupBy = LookupBy.Name,
        statement_t: StatementType | typing.Type[StatementT] | None = None,
    ) -> StatementT | None:
        """
        Lookup the symbol either by path or id. If path is a string, it can be
        it can be a name (lookup upwards) or a full relative/absolute path.
        """
        if isinstance(path, UUID):
            return self._root_scope._statements_by_id.get(path)
        elif isinstance(path, str):
            if "." not in path:
                return self._find_statement(path, by=by)
            path = parse_statement_path(path)
        if path.path == ".":
            return self._find_statement(path.name, by=by)
        # strip leading . in path
        if path.path.startswith("."):
            path = StatementPath(path.path[1:], path.name)
        parts = path.path.split(".", 2)
        if len(parts) > 1:
            first_part, inner_part = parts[0], StatementPath(parts[1], path.name)
        else:
            first_part, inner_part = parts[0], path.name
        scope = self._find_scope(first_part, by=by)
        if scope is None:
            return None
        return scope.lookup(inner_part, statement_t=statement_t, by=by)

    def _add_statement(self, statement: "Statement", by_name: bool) -> None:
        if statement.name is not None and by_name:
            if statement.name in self._scopes_by_name:
                self._on_issue(
                    type=IssueType.AMBIGUOUS_DEFINITION, subject=statement, path=statement.path
                )
            else:
                self._scopes_by_name[statement.name] = statement
                self._names_by_py_ident[statement.py_ident] = statement.name

        self._statements_by_id.update(statement._statements_by_id)
        if isinstance(statement, Statement):
            self._statements_by_id[statement.id] = statement

    def _add_file(self, file: "File", by_name: bool) -> None:
        if file.name is not None and by_name:
            if file.name in self._scopes_by_name:
                self._on_issue(type=IssueType.AMBIGUOUS_DEFINITION, subject=file, path=file.path)
            else:
                self._scopes_by_name[file.name] = file
                self._names_by_py_ident[file.py_ident] = file.name
        self._statements_by_id.update(file._statements_by_id)

    def _clear(self):
        """Resets this scope and all child scopes."""
        self._scopes_by_name = {}
        self._statements_by_id = {}
        self._names_by_py_ident = {}
        for scope in self._scopes_by_name.values():
            scope._clear()


class ModuleStatus(enum.IntEnum):
    Raw = 0
    Index = 1
    Interp = 2
    Instance = 3


@node
class Module(ModuleNode, HasCrud, HasSession, HasIssues, Scope):
    name: str = required_field()
    files: list["File"] = field(default_factory=list)
    dependencies: dict[str, Union["Module", ModuleReference]] = field(default_factory=dict)
    builtins: list["File"] = field(default_factory=list)
    parent: None = None
    parent_scope: Scope = None
    committed: bool = False
    status: ModuleStatus = ModuleStatus.Raw

    def __str__(self):
        issues_str = f", {len(self.issues)} issues" if self.issues is not None else ""
        return f"{self.name} ({self.status.name}, {len(self.files)} files{issues_str})"

    def __repr__(self):
        return f"<Module {str(self)}>"

    @property
    def attached(self) -> bool:
        return True  # root is always "attached"

    @property
    def path(self) -> str:
        return self.py_ident

    @property
    def py_ident(self) -> str:
        return to_pyidentifier(self.name, IdentifierType.PATH)

    def add_builtin(self, file: "File") -> None:
        self.builtins.append(file)

    def add_dependency(self, module: Union["Module", ModuleReference]) -> None:
        if module.name in self.dependencies:
            raise ValueError(
                f"{self} has dependency {module.name}: {self.dependencies[module.name]}"
            )
        self.dependencies[module.name] = module

    def lookup(
        self,
        path: Union["StatementPath", UUID, str],
        by: LookupBy = LookupBy.Name,
        statement_t: StatementType | typing.Type[StatementT] | None = None,
    ) -> StatementT | None:
        if isinstance(path, UUID):
            return self._statements_by_id.get(path)  # type: ignore
        elif isinstance(path, str) and path.startswith("."):
            return super().lookup(path, statement_t=statement_t, by=by)
        else:
            module_name, localized_path = parse_absolute_statement_reference(path)
            if module_name == self.name:
                dependency = self
            else:
                dependency = self.dependencies.get(module_name)
            if dependency is None:
                return None
            return dependency.lookup(localized_path, statement_t=statement_t, by=by)

    def lookup_or_error(
        self,
        path: Union["StatementPath", UUID, str],
        by: LookupBy = LookupBy.Name,
        statement_t: typing.Type[StatementT] | None = None,
    ) -> StatementT:
        result = self.lookup(path, by=by, statement_t=statement_t)
        if result is None:
            raise LookupError(f"{path} not found in {self}")
        return result

    def create_file(self, name: str) -> "File":
        if name in self._scopes_by_name:
            raise ValueError(f"{name} already exists in {self}: {self._scopes_by_name[name]}")
        file = File(name=name, parent=self, module=self)
        self.files.append(file)
        return file

    def add_file(self, file: "File") -> None:
        if file.parent is not None and file.parent != self:
            raise ValueError(f"{file} is already in {file.parent}")
        file.module = self
        file.parent = self
        self.files.append(file)

    def get_file(self, name: str) -> "File":
        scope = self._scopes_by_name.get(name)
        if not isinstance(scope, File):
            raise ValueError(f"expected file, got {type(scope)}")
        return scope

    def _expect_status(self, status: ModuleStatus):
        if self.status != status:
            raise ValueError(f"need {self} to be {status.name}")

    def activate_in(self, session: "Session"):
        if self._session is not None:
            self._session.remove(self)
        self.clear()
        self.index()
        self.interp()
        for n in self.walk():
            if n.id != self.id and isinstance(n, HasSession):
                n.activate_in(session)
        for dependency in self.dependencies.values():
            dependency.activate_in(session)
        self._session = session

    def deactivate(self) -> None:
        self._session = None
        for n in self.walk():
            if n.id != self.id and isinstance(n, HasSession):
                n.deactivate()

    def clear(self):
        super()._clear()
        self._session = None
        for file in self.files:
            file._clear()
        self.status = ModuleStatus.Raw

    def index(self):
        self._expect_status(ModuleStatus.Raw)
        for builtin in self.builtins:
            for statement in builtin.statements:
                self._add_statement(statement, by_name=True)
        for dependency in self.dependencies.values():
            self._statements_by_id.update(dependency._statements_by_id)
        for file in self.files:
            file._index()
            self._add_file(file, by_name=True)
        self.status = ModuleStatus.Index

    def interp(self):
        self._expect_status(ModuleStatus.Index)
        for file in self.files:
            file._interp()
        self.status = ModuleStatus.Interp

    def copy(self):
        from bench.language import wire

        module_data = wire.pack_module(self)
        module_copy = wire.unpack_module(module_data, session=None)
        for builtin in self.builtins:
            module_copy.add_builtin(builtin)  # also copy?
        for dependency in self.dependencies.values():
            module_copy.add_dependency(dependency)  # also copy?
        if self.status >= ModuleStatus.Index:
            module_copy.index()
        if self.status >= ModuleStatus.Interp:
            module_copy.interp()
        if len(module_copy.issues or []) != len(self.issues or []):
            raise RuntimeError(
                f"{self} copy expected {len(self.issues or [])} issues, got {len(module_copy.issues or [])}: {module_copy.issues}"
            )
        return module_copy

    @staticmethod
    def interp_from(
        maybe_module: Union["ModuleTreeData", "Module"], session: Optional["Session"]
    ) -> "Module":
        from bench.language import libs, wire

        # copy default dependencies
        dependencies = {name: dep.copy() for name, dep in libs.DEFAULT_MODULES.items()}

        logger.debug("module.interp", module=maybe_module)
        if isinstance(maybe_module, wire.ModuleTreeData):
            module: Module = wire.unpack_module(maybe_module, session=session)
        else:
            module = maybe_module
        module.add_builtin(dependencies["symbolx.lib"].get_file("builtins"))
        for dependency in dependencies.values():
            module.add_dependency(dependency)
        logger.debug("module.interp.index", module=module)
        module.index()
        module.interp()
        logger.debug("module.interp.done", module=module)
        return module


@node(tracked=["name"])
class File(ModuleNode, HasCrud, HasSession, HasIssues, Scope):
    name: str = required_field()
    module: Optional[Module] = None
    parent: Union["File", Module] = None
    children: list["File"] = field(default_factory=list)
    statements: list["Statement"] = field(default_factory=list)
    # index
    statements_by_parent_id: dict[UUID | None, list["Statement"]] | None = None

    def __post_init__(self):
        super().__post_init__()
        if self.parent is None:
            self.parent = self.module

    def __str__(self):
        return f"{self.path} '{self.name}' ({len(self.statements)} statements)"

    def __repr__(self):
        return f"<File {str(self)}>"

    @property
    def path(self) -> str:
        if isinstance(self.parent, Module):
            return f"{self.parent.name}.{self.py_ident}"
        elif isinstance(self.parent, File):
            return f"{self.parent.path}.{self.py_ident}"
        else:
            return self.py_ident  # detached file

    @property
    def py_ident(self) -> Optional[str]:
        if self.name is None:
            return None
        else:
            return to_pyidentifier(self.name, IdentifierType.PATH)

    def append_statement(self, *statements: "Statement"):
        """Appends the statements to this file."""
        last_ok = self.statements[-1].order_key if self.statements else None
        oks = generate_n_keys_between(last_ok, None, len(statements))
        for ok, statement in zip(oks, statements):
            if statement.parent is not None and statement.parent != self:
                raise ValueError(f"statement {statement} belongs to {statement.parent}")
            statement.order_key = ok
            statement.parent = self
            for descendant in statement.walk_descendants():
                descendant.file = self
                self.statements.append(descendant)

    append = append_statement  # alias for File

    def _assign_oks(self):
        for statements in self.statements_by_parent_id.values():
            oks = generate_n_keys_between(None, None, len(statements))
            for ok, statement in zip(oks, statements):
                statement.order_key = ok

    def _clear(self):
        """Resets this scope and all child scopes."""
        super()._clear()
        for statement in self.statements:
            statement._clear()

    def _index(self):
        """Indexes all statements in this file into the scope."""
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

        if len(sorted_statements) != len(self.statements):
            raise RuntimeError(f"invalid order: {len(sorted_statements)} != {len(self.statements)}")
        self.statements = sorted_statements

        for statement in self.statements:
            statement._index()
            self._add_statement(statement, by_name=statement.parent == self)

    def _interp(self):
        for statement in self.statements:
            statement._interp(statement)


@node(tracked=["name"])
class Statement(ModuleNode, HasCrud, HasSession, HasIssues, Scope):
    """A Bench statement."""

    file: File | None = None
    parent: Union["Statement", File] = None
    children: list["Statement"] | None = None
    resolved_children: list["Statement"] | None = None
    order_key: str | None = None
    type: StatementType = required_field()  # set by subclasses
    name: Optional[str] = None
    issues: list[Issue] | None = None
    id: UUID = field(default_factory=uuid.uuid4)

    def __post_init__(self):
        super().__post_init__()
        if self.file is None and self.parent is not None:
            self.file = self.parent.file
        if self.parent is None:
            self.parent = self.file

    def __str__(self):
        return f"{self.path} '{self.name}'" if self.name else self.path

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
            return self.file.path + "." + str(self.infile_path)

    @property
    def infile_path(self) -> str:
        parent = self.parent
        ancestor_parts = [self.py_ident or "<anon>"]
        seen_ids = {self.id}
        while isinstance(parent, Statement):
            if parent.id in seen_ids:
                # :CircularAncestry
                # circuit breaker: ignore here because this is an error in indexing
                ancestor_parts.append("<!loop>")
                break
            ancestor_parts.append(parent.py_ident or "<anon>")
            seen_ids.add(parent.id)
            parent = parent.parent
        return ".".join(reversed(ancestor_parts))

    @property
    def py_ident(self) -> Optional[str]:
        if self.name is None:
            return None
        else:
            return to_pyidentifier(self.name, IdentifierType.VARIABLE)

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.parent.id if self.parent is not None else None

    def append_statement(self, *statements: "Statement"):
        last_ok = self.children[-1].order_key if self.children else None
        oks = generate_n_keys_between(last_ok, None, len(statements))
        for ok, statement in zip(oks, statements):
            if statement.parent is not None and statement.parent != self:
                raise ValueError(f"statement {statement} belongs to {statement.parent}")
            statement.order_key = ok
            statement.file = self.file
            statement.parent = self
        if self.children is None:
            self.children = [*statements]
        else:
            self.children.extend(statements)
        self._resolve_children()

    def _resolve_children(self):
        resolved_children = []
        for statement in self.children or []:
            if statement.type == StatementType.REFERENCE:
                if isinstance(statement.reference, Statement):
                    resolved_children.append(statement.reference)
            elif statement.type == StatementType.BLOCK:
                resolved_children.extend(statement.resolved_children)
            else:
                resolved_children.append(statement)
        self.resolved_children = resolved_children

    def get_children_by_type(
        self, *types: typing.Union[StatementType, typing.Type["Statement"]]
    ) -> list["Statement"]:
        if self.resolved_children is None:
            raise RuntimeError(f"{self} is not indexed")
        types = [t if isinstance(t, StatementType) else t.type for t in types]
        return [s for s in self.resolved_children if s.type in types]

    def walk_descendants(self) -> typing.Iterator["Statement"]:
        """Yields all descendant statements in DFS order."""
        yield self
        if self.children is not None:
            for child in self.children:
                yield from child.walk_descendants()

    def __getattr__(self, item):
        if item in self._PROPERTIES:
            return super().__getattribute__(item)
        else:
            scope = self._scopes_by_name.get(item)
            if scope is not None:
                return scope
        candidates = {
            **{s: s for s in self._PROPERTIES},
            **{s.name: s for s in self._scopes_by_name.values()},
        }
        did_you_mean = did_you_mean_str(candidates, item)
        raise AttributeError(f"{self} has no attribute {item} ({did_you_mean})")

    def _index(self):
        self._clear()
        self.children = self.file.statements_by_parent_id.get(self.id, [])
        self._resolve_children()
        for child in self.children:
            # only index self, not children
            # (unlike in file/module, statement nesting is only semantic, not structural)
            self._add_statement(child, by_name=True)

    def _clear(self) -> None:
        """Clears any derived/interpreted values on this statement."""
        super()._clear()
        self.issues = None

    def _interp(self, scope: Scope) -> None:
        """Updates, resolves and checks any derived/interpreted values on this statement."""
        pass

    def _reinterp(self, scope: Scope = None, raise_errors: bool = True) -> None:
        """Clears and re-interprets this statement in scope."""
        self._clear()
        self._index()
        self._interp(scope or self)
        if raise_errors and self.errors:
            raise BenchError(self.errors[0])


class StatementBase(abc.ABC):
    """Base for statements for type-checking."""

    parent: Statement | File
    session: "Session"
    resolved_children: list["Statement"]
    _scopes_by_name: dict[str, Scope] | None
    _names_by_py_ident: dict[str, str] | None

    def _clear(self) -> None:
        raise NotImplementedError

    def _interp(self, scope: Scope) -> None:
        raise NotImplementedError

    def _reinterp(self, scope: Scope = None, raise_errors: bool = True) -> None:
        raise NotImplementedError

    _on_issue: IssueHandler


#
# Sessions
#


class ModuleOp(enum.StrEnum):
    READ = "read"
    SEARCH = "search"
    CREATE = "create"
    UPDATE = "update"
    DELETE = "delete"
    RUN = "run"


class SessionMode(enum.StrEnum):
    READ_ONLY = "ro"
    WRITE = "w"


active_session: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)


@dataclass(slots=True)
class SessionContext:
    module_id: UUID
    project_id: UUID
    worker_node_id: str
    worker_process_id: Optional[str]
    trigger_type: TriggerType
    trigger_id: typing.Optional[UUID]
    first_run_id: typing.Optional[UUID] = None


class SessionBase(abc.ABC):
    """Base for sessions for type-checking."""

    module: Module

    @property
    def is_open(self) -> bool:
        raise NotImplementedError

    def add(self, obj: ModuleNode):
        raise NotImplementedError

    def remove(self, obj: ModuleNode):
        raise NotImplementedError


SESSION_MUTATION_FLUSH_WATERMARK = 500


class ModuleWriter(abc.ABC):
    """Base for writing module/session for type-checking."""

    async def write_module(self, mutations: list["ModuleMutation"]) -> bool:
        raise NotImplementedError

    async def write_session(
        self, session: "Session", runs: list["Run"], logs: list["LogEntry"]
    ) -> bool:
        raise NotImplementedError


class NoopModuleWriter(ModuleWriter):
    async def write_module(self, mutations: list["ModuleMutation"]) -> bool:
        return True

    async def write_session(
        self, session: "Session", runs: list["Run"], logs: list["LogEntry"]
    ) -> bool:
        return True


class Session:
    """A managed context for running code in a module (may mutate)."""

    def __init__(
        self,
        module: Module,
        writer: ModuleWriter,
        id: UUID = None,
        ctx: SessionContext | None = None,
        cache_inferences: bool = True,
        inference_timeout: int = 120,
        inference_retries: int = 5,
        mode: SessionMode = SessionMode.READ_ONLY,
        executor: Executor = None,
    ):
        from bench.language.cache import CacheAsync, CacheSync
        from bench.language.mutate import ModuleMutator
        from bench.language.remote import Storage
        from bench.language.tracing import SessionTracer

        self.id = id or uuid4()
        self.ctx = ctx
        self.module = module
        self.instances: dict[UUID, "HasSession"] = {}
        self.default_models: list[Statement] = [
            module.lookup_or_error("openai.lib.chat.gpt4"),
            module.lookup_or_error("openai.lib.chat.gpt3"),
        ]
        self.cache_inferences = cache_inferences
        self.inference_timeout = inference_timeout
        self.inference_retries = inference_retries
        self.mode = mode
        self.writer = writer

        self.cache_sync = CacheSync(module, project_id=ctx.project_id)
        self.cache_async = CacheAsync(module, project_id=ctx.project_id)
        self.storage = Storage(module)

        self.anonymous_scope = Scope(parent=self.module)
        self.executor = executor or ThreadPoolExecutor(max_workers=1)
        self.logger = logger.bind(session=self)
        self.mutator = ModuleMutator(self.module, hooks=[self._on_mutated])
        self.tracer = SessionTracer(self, mutator=self.mutator, validate=True)
        self.opened_at: Optional[datetime] = None
        self.closed_at: Optional[datetime] = None
        self.metadata: dict[str, typing.Any] = {}
        self._pending_flushes: list[tuple[int, typing.Awaitable[bool]]] = []

    def __str__(self):
        status = "open" if self.opened_at else ("closed" if self.closed_at else "pending")
        return (
            f"{self.module.name} {self.id} ({self.mode}, {status}, {len(self.mutator.mutations)})"
        )

    def __repr__(self):
        return f"<Session {self}>"

    def sync_to_async(self, fn: Callable) -> Callable[..., typing.Awaitable]:
        return sync_to_async(fn, thread_sensitive=False, executor=self.executor)  # type: ignore

    def async_to_sync(self, fn: typing.Awaitable | typing.Callable | typing.Coroutine) -> Callable:
        return async_to_sync(fn)  # type: ignore

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
                    if isinstance(obj, Statement):
                        # it feels like this should be done in some tracer? also (re?)-index?
                        self.module._statements_by_id[obj.id] = obj
        for obj in objs:
            self.instances[obj.id] = obj

    def remove(self, *objs: "HasSession") -> None:
        for obj in objs:
            if isinstance(obj, (Statement, Statement, File)) and obj.id in self.instances:
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

    async def aopen(self):
        """Opens the session for execution and modification."""
        if self.opened_at is not None:
            raise RuntimeError(f"session already opened {self}")
        self.opened_at = utcnow_with_tz()
        if active_session.get() is not None:
            raise RuntimeError(f"another session is active: {active_session.get()}")
        active_session.set(self)
        await self.tracer.open()
        logger.debug("session.open", session=self)

    async def _do_flush(self, mutations: list["ModuleMutation"]) -> bool:
        # TODO @Robustness: auto-split mutations if not in atomic block and too large
        success = await self.writer.write_module(mutations)
        if not success:
            if len(mutations) > 20:
                mutations_str = f"{mutations[:10]} ... {mutations[-10:]}"
            else:
                mutations_str = str(mutations)
            raise RuntimeError(f"failed to write {len(mutations)} mutations {mutations_str}")
        logger.debug("session.flush.done", session=self, mutator=self.mutator)
        return success

    async def aflush(self, optimistic: bool = False):
        """
        Flushes all module mutations.
        If optimistic, this will return before the flush is complete (but will wait on close).
        """
        if not self.mutator.mutations:
            return
        if self.mode == SessionMode.READ_ONLY:
            raise RuntimeError(f"cannot mutate read-only session {self}")
        logger.debug("session.flush", session=self, mutator=self.mutator, optimistic=optimistic)
        mutations = self.mutator.bundle().compact()
        self.mutator.reset()
        flush = self._do_flush(mutations)
        if optimistic:
            self._pending_flushes.append((len(mutations), asyncio.create_task(flush)))
        else:
            await flush

    def flush(self, optimistic: bool = False):
        async_to_sync(self.aflush)(optimistic=optimistic)

    async def aclose(self):
        """Closes the session, flushing any mutations and preventing further execution/mutation."""
        if self.closed_at is not None:
            raise RuntimeError(f"session already closed {self}")
        self.closed_at = utcnow_with_tz()
        await self.aflush(optimistic=True)
        # await all pending flushes
        pending_mutations_count = sum(count for count, _ in self._pending_flushes)
        logger.debug(
            "session.close.pending", session=self, pending_mutations_count=pending_mutations_count
        )
        await asyncio.gather(*(task for _, task in self._pending_flushes))
        active_session.set(None)
        await self.tracer.close()
        logger.debug("session.close", session=self)

    def close(self):
        async_to_sync(self.aclose)()

    def _on_mutated(self, mutator: "ModuleMutator", mutation: "ModuleMutation"):
        if len(self.mutator.mutations) > SESSION_MUTATION_FLUSH_WATERMARK:
            self.flush(optimistic=True)

    async def __aenter__(self):
        await self.aopen()
        return self

    async def __aexit__(self, exc_type, exc_value, traceback):
        await self.aclose()

    def sync(self):
        """A sync context manager for this session."""
        session = self

        class SyncSession:
            def __enter__(self):
                async_to_sync(session.aopen)()
                return session

            def __exit__(self, exc_type, exc_value, traceback):
                async_to_sync(session.aclose)()

        return SyncSession()

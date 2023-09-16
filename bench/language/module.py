import abc
import enum
import itertools
import typing
import uuid
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime
from functools import cached_property
from logging import Logger
from typing import Optional, Union
from uuid import UUID, uuid4

import structlog

from bench.language.const import (
    MNT,
    IssueKind,
    IssueType,
    ModuleReference,
    NodePath,
    StatementType,
    parse_absolute_node_reference,
    parse_node_path,
)
from bench.utils.dt import utcnow_with_tz
from bench.utils.utils import IdentifierType, required_field, to_pyidentifier

if typing.TYPE_CHECKING:
    from bench.language import Field, File, Issue, Session, Statement
    from bench.language.wire import ModuleTreeData

logger = structlog.get_logger(__name__)

SESSION_NOT_READY = object()


class LookupBy(enum.StrEnum):
    Name = "name"
    PyIdent = "py_ident"


def node(cls: Optional[typing.Type] = None, mnt: MNT = None, tracked: list[str] | None = None):
    """
    Decorator alias for module node.
    Only tracked properties may be mutated during a session (by the 'user'/run).
    """

    def decorate(cls):
        cls = dataclass(cls, repr=False, eq=False)  # type: ignore
        if mnt:
            cls.mnt = mnt
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
        if cls._TRACKED and not issubclass(cls, ModuleNode):  # type: ignore
            raise ValueError(f"{cls} cannot have tracked properties without ModuleNode")

        return cls

    if cls is not None:
        return decorate(cls)

    return decorate


def new_node_identity(module_id: UUID) -> tuple[UUID, UUID]:
    ck = uuid4()
    id = get_node_id(module_id, ck)
    return id, ck


def new_detached_node_identity() -> tuple[UUID, UUID]:
    ck = uuid4()
    return ck, ck


def get_node_id(module_id: UUID, ck: UUID):
    """Derive the version-specific node id from its constant key"""
    return uuid.uuid5(module_id, str(ck))


class ModuleVisitor:
    def __init__(self):
        self._descendant_by_ck: dict[UUID, ModuleNode] = {}
        self._reference_by_ck: dict[UUID, ModuleNode] = {}

    def __str__(self):
        return f"{len(self._descendant_by_ck)} nodes"

    def __repr__(self):
        return f"<ModuleVisitor {str(self)}>"

    @property
    def subtree(self) -> typing.Collection["ModuleNode"]:
        return self._descendant_by_ck.values()

    @property
    def references(self) -> typing.Collection["ModuleNode"]:
        return self._reference_by_ck.values()

    def visit_child(self, node: "ModuleNode"):
        if node.ck in self._descendant_by_ck and self._descendant_by_ck[node.ck].id != node.id:
            raise ValueError(f"cannot visit child {node} twice: {self._descendant_by_ck[node.ck]}")
        self._descendant_by_ck[node.ck] = node

    def visit_reference(self, node: "ModuleNode"):
        self._reference_by_ck[node.ck] = node


@node
class ModuleNode(abc.ABC):
    """
    A node in a Bench module tree.
    A node has a per-version unique id (id) and a constant identifier key (ck).
    """

    mnt: typing.ClassVar[MNT]  # set in @node decorator
    id: UUID = field(default=None)
    ck: UUID = field(default_factory=uuid.uuid4)
    parent: Optional["ModuleNode"] = None
    # prototype: Optional["ModuleNode"] / instance_of_ck: UUID
    created_at: datetime = field(default_factory=utcnow_with_tz)
    updated_at: datetime = field(default_factory=utcnow_with_tz)
    last_edited_at: datetime = field(default_factory=utcnow_with_tz)
    last_changed_at: datetime = field(default_factory=utcnow_with_tz)
    revision: int = 0

    _session: Optional["Session"] = None
    _tracked: bool = False

    def __post_init__(self):
        if self._session is SESSION_NOT_READY:
            pass
        elif self._session is None:
            from bench.language.session import active_session

            self._session = active_session.get()
            if self._session is not None:
                self._session.add(self, new=True)
        else:
            self._session.add(self, new=False)

    def __del__(self):
        if self._session is not None:
            self._session.remove(self)

    def _assign_id(self, module_id: UUID):
        if module_id is None:
            raise ValueError(f"cannot assign id to {self} without module_id")
        assert self.id is None, f"cannot assign id to {self} twice"
        assert self.ck is not None, f"cannot assign id to {self} without ck"
        self.id = get_node_id(module_id, self.ck)

    def _assign_id_if_none(self):
        if self.id is None and self.module is not None:
            self._assign_id(self.module.id)

    def __eq__(self, other):
        return isinstance(other, self.__class__) and self.id == other.id

    def __hash__(self):
        return hash(self.id)

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.parent.id if self.parent is not None else None

    @property
    def module(self) -> Optional["Module"]:
        if self.parent is not None:
            return self.parent.module
        elif isinstance(self, Module):
            return self
        return None

    def _clear(self) -> None:
        pass

    def _interp(self, scope: "Scope") -> None:
        pass

    def _visit(self, visitor: ModuleVisitor) -> None:
        """Visit any child nodes."""
        raise NotImplementedError(f"{self.__class__.__name__} does not implement _visit")

    def _activate_in(self, session: "Session") -> None:
        """'Instantiate' this object in the given session."""
        if self._session is not None:
            self._deactivate()
        self._session = session
        session.add(self, new=False)
        self._tracked = True

    def _deactivate(self) -> None:
        """'Deinstantiate' this object."""
        self._tracked = False  # need to set this first as some statements may redirect set/get
        if self._session is not None:
            self._session.remove(self)
        self._session = None

    def _notify_added(self, *nodes: "ModuleNode") -> None:
        """When this node adds another node."""
        if self.module is not None:
            for n in nodes:
                self.module._on_added(n)

    def _walk(self) -> typing.Iterator["ModuleNode"]:
        visitor = ModuleVisitor()
        visitor.visit_child(self)

        seen: dict[UUID, ModuleNode] = {}
        to_visit = [self]
        while to_visit:
            for node in to_visit:
                seen[node.ck] = node
                node._visit(visitor)
            to_visit = [n for n in visitor.subtree if n.ck not in seen]

        yield from visitor._descendant_by_ck.values()

    def copy(self):
        from bench.language import wire

        _, node_datas = wire.pack_node(self)
        return wire.unpack_node(node_datas, parent=self.parent, session=None)

    @property
    def attached(self) -> bool:
        return self.parent is not None

    @property
    def path(self) -> str:
        raise NotImplementedError(f"{self.__class__.__name__} does not implement path")

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


NodeT = typing.TypeVar("NodeT", bound="ModuleNode")


@node
class Scope:
    parent: Optional["Scope"] = None
    issues: list["Issue"] = None
    _scopes_by_name: dict[str, "Scope"] = field(default_factory=dict)
    _nodes_by_id: dict[UUID, "ModuleNode"] = field(default_factory=dict)
    _nodes_by_ck: dict[UUID, "Statement"] = field(default_factory=dict)
    _names_by_ident: dict[str, str] = field(default_factory=dict)

    def _get_scope(self, name: str, by: LookupBy) -> Union["Scope", None]:
        if by == LookupBy.Name:
            return self._scopes_by_name.get(name)
        elif by == LookupBy.PyIdent:
            if name in self._names_by_ident:
                name = self._names_by_ident[name]
                return self._scopes_by_name.get(name)
        else:
            raise ValueError(f"{self} got unexpected lookup type: {by}")
        return None

    def _find_scope(self, name: str, by: LookupBy) -> Union["Scope", None]:
        scope = self._get_scope(name, by)
        if scope is not None:
            return scope
        if self.parent is not None:
            return self.parent._find_scope(name, by=by)
        return None

    def _find_node(self, name: str, by: LookupBy = LookupBy.Name) -> NodeT | None:
        """Find the node recursively in this scope and its parents."""
        scope = self._find_scope(name, by)
        if scope is not None and not isinstance(scope, Statement):
            return None
        return typing.cast(NodeT, scope)

    def lookup(
        self,
        path: Union["NodePath", UUID, str],
        by: LookupBy = LookupBy.Name,
        node_t: MNT | StatementType | typing.Type[NodeT] | None = None,
    ) -> NodeT | None:
        """
        Lookup the symbol either by path or id. If path is a string, it can be
        it can be a name (lookup upwards) or a full relative/absolute path.
        """
        if isinstance(path, UUID):
            return self._root_scope.lookup(path, by=by, node_t=node_t)
        elif isinstance(path, str):
            if "." not in path:
                return self._find_node(path, by=by)
            path = parse_node_path(path)
        if path.path == ".":
            return self._find_node(path.name, by=by)
        # strip leading . in path
        if path.path.startswith("."):
            path = NodePath(path.path[1:], path.name)
        parts = path.path.split(".", 2)
        if len(parts) > 1:
            first_part, inner_part = parts[0], NodePath(parts[1], path.name)
        else:
            first_part, inner_part = parts[0], path.name
        scope = self._find_scope(first_part, by=by)
        if scope is None:
            return None
        return scope.lookup(inner_part, node_t=node_t, by=by)

    def _add_child_scope(self, scope: "Scope", by_name: bool) -> None:
        self._add_child_node(scope, by_name)
        self._nodes_by_id.update(scope._nodes_by_id)
        self._nodes_by_ck.update(scope._nodes_by_ck)

    def _add_child_node(self, node: ModuleNode, by_name: bool) -> None:
        if node.name is not None and by_name:
            if node.name in self._scopes_by_name or node.py_ident in self._names_by_ident:
                self._on_issue(type=IssueType.AMBIGUOUS_DEFINITION, subject=node, path=node.path)
            else:
                self._scopes_by_name[node.name] = node
                self._names_by_ident[node.py_ident] = node.name
        self._nodes_by_id[node.id] = node
        self._nodes_by_ck[node.ck] = node

    def _clear(self):
        """Resets this scope and all child scopes."""
        self._scopes_by_name = {}
        self._nodes_by_id = {}
        self._nodes_by_ck = {}
        self._names_by_ident = {}
        for scope in self._scopes_by_name.values():
            scope._clear()

    @property
    def errors(self) -> list["Issue"]:
        if self.issues is None:
            return []
        return [i for i in self.issues if i.kind == IssueKind.Error]

    @property
    def self_errors(self):
        return [i for i in self.errors if i.parent == self]

    def _on_issue(
        self,
        issue: "Issue" = None,
        *,
        subject: Union["Statement", "File", "Field", None] = None,
        type: IssueType = None,
        **kwargs,
    ):
        from bench.language.field import Field
        from bench.language.issue import Issue

        if isinstance(subject, Field):
            subject = subject.parent  # fields don't have issues (yet)
        if issue is None:
            issue = Issue(type=type, parent=subject, **kwargs)
        if self.issues is None:
            self.issues = []
        self.issues.append(issue)
        if self.parent is not None:
            self.parent._on_issue(issue)

    @cached_property
    def _root_scope(self) -> "Scope":
        if self.parent is None:
            return self
        return self.parent._root_scope


class ModuleStatus(enum.IntEnum):
    Raw = 0
    Index = 1
    Interp = 2
    Instance = 3


@node(mnt=MNT.Module)
class Module(ModuleNode, Scope):
    name: str = required_field()
    files: list["File"] = field(default_factory=list)
    dependencies: dict[str, Union["Module", ModuleReference]] = field(default_factory=dict)
    builtins: list["File"] = field(default_factory=list)
    parent: None = None
    parent_scope: Scope = None
    committed: bool = False
    status: ModuleStatus = ModuleStatus.Raw
    # index
    _files_by_parent_id: dict[UUID, list["File"]] | None = None

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

    @property
    def all_statements(self):
        return itertools.chain.from_iterable(file.statements for file in self.files)

    def _visit(self, visitor: ModuleVisitor) -> None:
        for file in self._files_by_parent_id.get(self.id, []):
            visitor.visit_child(file)

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
        path: Union["NodePath", UUID, str],
        by: LookupBy = LookupBy.Name,
        node_t: MNT | typing.Type[NodeT] | None = None,
    ) -> NodeT | None:
        if isinstance(path, UUID):
            return self._nodes_by_id.get(path) or self._nodes_by_ck.get(path)
        elif isinstance(path, str) and path.startswith("."):
            return Scope.lookup(self, path, node_t=node_t, by=by)
        else:
            module_name, localized_path = parse_absolute_node_reference(path)
            if module_name == self.name:
                dependency = self
            else:
                dependency = self.dependencies.get(module_name)
            if dependency is None:
                return None
            return dependency.lookup(localized_path, node_t=node_t, by=by)

    def lookup_or_error(
        self,
        path: Union["NodePath", UUID, str],
        by: LookupBy = LookupBy.Name,
        node_t: typing.Type[NodeT] | None = None,
    ) -> NodeT:
        result = self.lookup(path, by=by, node_t=node_t)
        if result is None:
            raise LookupError(f"{path} not found in {self}")
        return result

    def create_file(self, name: str) -> "File":
        from bench.language.file import File

        if name in self._scopes_by_name:
            raise ValueError(f"{name} already exists in {self}: {self._scopes_by_name[name]}")
        file = File(name=name, parent=self, module=self)
        file._assign_id(self.id)
        self.files.append(file)
        self.files.sort(key=lambda f: f.name)
        file._index()
        return file

    def add_file(self, file: "File") -> None:
        if file.parent is not None and file.parent != self:
            raise ValueError(f"{file} is already in {file.parent}")
        if not file.id:
            file._assign_id(self.id)
        file.module = self
        file.parent = self
        file._index()
        self.files.append(file)
        self.files.sort(key=lambda f: f.name)
        self._on_added(file)

    def get_file(self, name: str) -> "File":
        scope = self._scopes_by_name.get(name)
        if not isinstance(scope, File):
            raise ValueError(f"expected file, got {type(scope)}")
        return scope

    def _on_added(self, node: ModuleNode) -> None:
        for n in node._walk():
            n._assign_id_if_none()

    def _expect_status(self, status: ModuleStatus):
        if self.status != status:
            raise ValueError(f"need {self} to be {status.name}")

    def _activate_in(self, session: "Session"):
        if self._session is not None:
            self._session.remove(self)
        self.clear()
        self.index()
        self.interp()
        for n in self._walk():
            if n.id != self.id:
                n._activate_in(session)
        for dependency in self.dependencies.values():
            dependency._activate_in(session)
        self._session = session

    def _deactivate(self) -> None:
        self._session = None
        for n in self._walk():
            if n.id != self.id:
                n._deactivate()

    def clear(self):
        super()._clear()
        self._session = None
        for file in self.files:
            file._clear()
        self.status = ModuleStatus.Raw
        self._files_by_parent_id = None

    def index(self):
        self._expect_status(ModuleStatus.Raw)
        for builtin in self.builtins:
            for statement in builtin.statements:
                self._add_child_scope(statement, by_name=True)
        for dependency in self.dependencies.values():
            self._nodes_by_id.update(dependency._nodes_by_id)
            self._nodes_by_ck.update(dependency._nodes_by_ck)
        self._files_by_parent_id = defaultdict(list)
        self.files.sort(key=lambda f: f.name)
        for file in self.files:
            self._files_by_parent_id[file.parent_id].append(file)
            file._index()
            self._add_child_scope(file, by_name=True)
        self.status = ModuleStatus.Index

    def interp(self):
        self._expect_status(ModuleStatus.Index)
        for file in self.files:
            file._interp(file)
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

        logger.debug("module.interp", module=maybe_module)
        # no need to copy these since worker processes are isolated?
        dependencies = {name: dep for name, dep in libs.DEFAULT_MODULES.items()}

        if isinstance(maybe_module, wire.ModuleTreeData):
            logger.debug("module.interp.unpack", module=maybe_module)
            module: Module = wire.unpack_module(maybe_module, session=session)
        else:
            module = maybe_module
        module.add_builtin(dependencies["symbolx.lib"].get_file("builtins"))
        for dependency in dependencies.values():
            module.add_dependency(dependency)
        logger.debug("module.interp.index", module=module)
        module.index()
        logger.debug("module.interp.interp", module=module)
        module.interp()
        logger.debug("module.interp.done", module=module)
        return module

import abc
import dataclasses
import enum
import inspect
import itertools
import typing
from typing import Iterator
import uuid
from dataclasses import dataclass
from datetime import datetime
from logging import Logger
from typing import TYPE_CHECKING, ClassVar, Collection, Optional, Union
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
from bench.utils.utils import IdentifierType, to_pyidentifier, required_field

if TYPE_CHECKING:
    from bench.language import Field, File, Issue, Session, Statement
    from bench.language.wire import ModuleTreeData, NodeTree
    from bench.language.issue import ValidationHandler

logger = structlog.get_logger(__name__)

SESSION_NOT_READY = object()


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


class LookupBy(enum.StrEnum):
    Name = "name"
    PyIdent = "py_ident"


class NodeRelationType(enum.IntFlag):
    """Parent relation between node and descendants."""

    ZERO = 0
    INLINE = 2**0  # fully loaded: File->Statement, Statement->Field, ...
    SHARED = 2**1  # across versions: Statement->Comment, Statement[versioned=False]->Record, ...
    FLAT = 2**2  # flattened inner hierarchy: Module->File, File->Statement, ...
    CUMULATIVE = 2**3  # sum of descendants: Module->Issue, File->Issue, ...
    NAMED = 2**4  # scoped by name: Module->File, File->Statement, ...
    KEYED = 2**5  # scoped by key: File->Tagging, Statement->Tagging, ...
    ORDERED = 2**6  # ordered: File->Statement, Statement->Field, ...


NRel = NodeRelationType

UNSET = object()


@dataclass
class NodeProperty:
    """A property of a module node."""

    name: str | None = None  # name from LHS of assignment
    is_internal: bool = False
    is_runtime: bool = False
    is_child_of: list[MNT] | None = None
    is_descendant_of: MNT | None = None
    default: typing.Any = UNSET
    default_factory: typing.Callable[[], typing.Any] | None = None
    annotation: typing.Any = None  # type annotation on LHS of assignment
    # for relations
    is_ancestor_of: MNT | None = None
    is_allowed: typing.Callable[["NodeT"], bool] | None = None
    relation_flags: NodeRelationType = NodeRelationType.ZERO

    def __str__(self):
        non_default = []
        for k, v in self.__dict__.items():
            if k not in ("name", "annotation") and v is not UNSET and v:
                if isinstance(v, bool):
                    non_default.append(k)
                else:
                    non_default.append(f"{k}={v}")
        attrs_str = ", ".join(non_default)
        return f"{self.name} ({attrs_str})" if attrs_str else self.name

    def __repr__(self):
        return f"<NodeProperty {self}>"


def nproperty(
    *, default: typing.Any = UNSET, default_factory: typing.Callable[[], typing.Any] = None
):
    """Standard user facing node property."""
    return NodeProperty(default=default, default_factory=default_factory)


def ninternal(
    *, default: typing.Any = UNSET, default_factory: typing.Callable[[], typing.Any] = None
):
    """Internal only, persisted node property."""
    return NodeProperty(is_internal=True, default=default, default_factory=default_factory)


def nruntime(
    *, default: typing.Any = UNSET, default_factory: typing.Callable[[], typing.Any] = None
):
    """Internal only, non-persisted runtime node property."""
    return NodeProperty(
        is_internal=True, is_runtime=True, default=default, default_factory=default_factory
    )


def nparent(*mnt: MNT):
    """The parent of a node, must be of one of the given types."""
    return NodeProperty(is_child_of=list(mnt), default=None, is_internal=True)


def nancestor(mnt: MNT):
    """Computed nearest ancestor of the given type."""
    return NodeProperty(is_descendant_of=mnt, default=None, is_internal=True)


def nchildren(
    mnt: MNT, flags: NRel = NRel.INLINE, is_allowed: typing.Callable[["NodeT"], bool] = None
):
    """Computed read/write children or descendants of the given type."""
    return NodeProperty(
        is_ancestor_of=mnt, relation_flags=flags, is_allowed=is_allowed, is_internal=True
    )


class NodeStatus(enum.IntEnum):
    Raw = 0
    Indexed = 1
    Interpreted = 2
    Tracked = 3


class NodeMethod(enum.Enum):
    init = "init"
    clear = "clear"
    index = "index"
    interp = "interp"
    visit = "visit"
    validate = "validate"
    activate = "activate"
    deactivate = "deactivate"

    @property
    def inner(self) -> str:
        return f"_{self}_inner"

    @property
    def self(self) -> str:
        return f"_{self}_self"

    @property
    def rec(self) -> str:
        return f"_{self}_rec"


# :NodeMethods
_NODE_INNER_METHODS: list[str] = [m.inner for m in NodeMethod]
_FORBIDDEN_NODE_METHODS = (
    [m.self for m in NodeMethod]
    + [m.rec for m in NodeMethod]
    + ["__post_init__", "__init__", "__del__"]
)
_NODE_CLASS_BY_MNT: dict[MNT, type] = {}


@typing.dataclass_transform()
def node_component(cls: Optional[typing.Type] = None, mnt: MNT = None, dynamic: bool = False):
    """
    Mark a class as a node component (or concrete node for a MNT).
    """

    def decorate(cls):
        properties: dict[str, NodeProperty] = {}
        mutable_properties: dict[str, NodeProperty] = {}
        static_components: list[type["ModuleNode"]] = []

        # collect properties
        for name, prop in cls.__dict__.items():
            if (
                name.startswith("__")
                or type(prop).__name__.startswith("_")
                or inspect.ismethod(properties)
                or inspect.isfunction(prop)
                or isinstance(prop, property)
                or isinstance(prop, classmethod)
                or isinstance(prop, staticmethod)
            ):
                continue  # ignore reserved names and non-fields
            if not isinstance(prop, NodeProperty):
                raise TypeError(f"{cls}.{name} is not a NodeProperty: {prop} ({type(prop)})")
            prop.name = name
            properties[name] = prop
            if not prop.is_internal:
                mutable_properties[name] = prop

        # add any parent classes properties
        for base in reversed(cls.__bases__):
            if base.__name__ in ("ModuleNode", "ABC") or not hasattr(base, "__properties__"):
                continue
            for name, prop in base.__properties__.items():
                if name not in properties:
                    properties[name] = prop
                elif prop != properties[name]:
                    raise ValueError(f"property conflict for {name}: {prop}, {properties[name]}")
            static_components.append(base)

        if dynamic:
            # ensure that all properties are dynamic
            static_props = [prop for prop in properties.values() if not prop.is_runtime]
            if static_props:
                raise TypeError(f"expected only dynamic props for {cls}, got {static_props}")

        # create dataclass
        for name, prop in properties.items():
            if not hasattr(cls, name):  # may be inherited
                continue
            if prop.default is not UNSET:
                setattr(cls, name, dataclasses.field(default=prop.default))
            elif prop.default_factory is not None:
                setattr(cls, name, dataclasses.field(default_factory=prop.default_factory))
            else:
                setattr(cls, name, required_field())
            cls.__annotations__[name] = prop.annotation
        cls = dataclass(cls, repr=False, eq=False)  # type: ignore
        cls.__properties__ = properties
        cls.__mutable_properties__ = mutable_properties
        cls.__static_components__ = tuple(static_components)

        # register as concrete node class for mnt
        if mnt:
            cls.mnt = mnt
            if mnt in _NODE_CLASS_BY_MNT:
                raise ValueError(f"node class conflict for {mnt}: {cls}, {_NODE_CLASS_BY_MNT[mnt]}")
            _NODE_CLASS_BY_MNT[mnt] = cls

        return cls

    if cls is not None:
        return decorate(cls)

    return decorate


def node(mnt: MNT):
    def decorate(cls):
        return node_component(cls, mnt=mnt)

    return decorate


NodeT = typing.TypeVar("NodeT", bound="ModuleNode")


class NodeList(Collection, typing.Generic[NodeT]):
    def __init__(self, parent: "ModuleNode", property: NodeProperty):
        self._parent = parent
        self._prop = property
        self._flags = property.relation_flags
        self._children: list[NodeT] = []

    def __str__(self):
        return f"{self._parent}.{self._prop.name} ({len(self._children)} items, {self._prop})"

    def __repr__(self):
        return f"<NodeList {self}>"

    def _init_from(self, scope: "ScopedNode"):
        self._children = []  # nocheckin: implement NodeList._init_from
        # old impl of File.append_statement:
        # last_ok = self.statements[-1].order_key if self.statements else None
        # oks = generate_n_keys_between(last_ok, None, len(statements))
        # for ok, statement in zip(oks, statements):
        # if statement.parent is not None and statement.parent != self:
        #         raise ValueError(f"statement {statement} belongs to {statement.parent}")
        #     statement.order_key = ok
        #             statement.parent = self
        #             statement.file = self
        #             self.module._on_added(statement)
        #             statement._index()
        #             for descendant in statement.walk_descendants():
        #                 descendant.file = self
        #                 self.statements.append(descendant)
        #                 self.module._on_added(descendant)

    def create(self, **kwargs):
        node = None  # nocheckin: implement NodeList.create
        return self.append(node)

    def append(self, _node: NodeT):
        # nocheckin: implement NodeList.append
        raise NotImplementedError

    def clear(self):
        raise NotImplementedError(f"{self} does not support clear")

    def get(self, some_id: str) -> Optional[NodeT]:
        if not (self._flags & NRel.KEYED) and not (self._flags & NRel.NAMED):
            raise ValueError(f"cannot get {some_id} from {self}")
        for child in self._children:
            if child.key == some_id or child.name == some_id or child.py_ident == some_id:
                return child
        return None

    def __contains__(self, some_id: object) -> bool:
        if isinstance(some_id, str) and (self._flags & NRel.KEYED or self._flags & NRel.NAMED):
            return self.get(some_id) is not None
        elif isinstance(some_id, ModuleNode):
            return some_id in self._children
        else:
            return False

    def __iter__(self) -> Iterator[NodeT]:
        yield from self._children

    def __len__(self) -> int:
        return len(self._children)


@node_component
class ModuleNode(abc.ABC):
    """
    A node in a Bench module tree.
    A node has a per-version unique id (id) and a constant identifier key (ck).
    """

    mnt: ClassVar[MNT]  # set in @node decorator
    __properties__: ClassVar[dict[str, NodeProperty]] = {}
    __static_components__: ClassVar[tuple[type["ModuleNode"]]] = []
    __mutable_properties__: ClassVar[dict[str, NodeProperty]] = {}

    id: UUID = ninternal(default=None)
    ck: UUID = ninternal(default_factory=uuid.uuid4)
    parent: Optional["ModuleNode"] = nparent()
    # prototype: Optional["ModuleNode"] / instance_of_ck: UUID

    created_at: datetime = ninternal(default_factory=utcnow_with_tz)
    updated_at: datetime = ninternal(default_factory=utcnow_with_tz)
    last_edited_at: datetime = ninternal(default_factory=utcnow_with_tz)
    last_changed_at: datetime = ninternal(default_factory=utcnow_with_tz)
    revision: int = ninternal(default=0)

    module: Optional["Module"] = nancestor(MNT.Module)
    issues: NodeList["Issue"] = nchildren(MNT.Issue, NRel.INLINE | NRel.CUMULATIVE)

    _session: Optional["Session"] = nruntime(default=None)
    _status: NodeStatus = nruntime(default=NodeStatus.Interpreted)

    def __post_init__(self):
        # nocheckin: probably need to update __post_init__ to track node relations
        if self._session is SESSION_NOT_READY:
            pass
        elif self._session is None:
            from bench.language.session import active_session

            self._session = active_session.get()
            if self._session is not None:
                self._session.tracer.node_create(self)
            else:
                raise RuntimeError(f"no active session for {self}")
        self._init_self()

    @property
    def _components(self) -> tuple[type["ModuleNode"]]:
        return self.__static_components__

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

    # abstract :ComponentMethods

    def _init_inner(self) -> None:
        """Initialize this node."""
        pass

    def _clear_inner(self) -> None:
        """Resets this node index and interp state."""
        pass

    def _index_inner(self) -> None:
        """Index this node."""
        for prop in self.__properties__.values():
            if prop.is_ancestor_of is not None:
                pass  # nocheckin: auto implement index

    def _interp_inner(self, scope: "ScopedNode") -> None:
        """Interpret this node."""
        pass

    def _validate_inner(self, properties: Collection[str], on_issue: "ValidationHandler") -> None:
        """Validate the given properties of this node."""
        pass  # nocheckin: implement validate
        # how does this work with _interp? is it part of interp?

    def _visit_inner(self, visitor: "NodeVisitor") -> None:
        """Visit any referenced nodes."""
        pass

    def _activate_inner(self, session: "Session") -> None:
        """'Instantiate' this object in the given session."""
        assert self._status == NodeStatus.Interpreted, f"cannot activate {self} in {self._status}"
        if self._session is not None:
            self._deactivate_rec()
        self._session = session
        self._status = NodeStatus.Tracked

    def _deactivate_inner(self) -> None:
        """'Deinstantiate' this object."""
        assert self._status == NodeStatus.Tracked, f"cannot deactivate {self} in {self._status}"
        self._session = None
        self._status = NodeStatus.Interpreted

    # final :ComponentMethods

    @staticmethod
    def _make_self_method(method: NodeMethod):
        """Creates method that calls _method_inner for all components"""

        def self_method(self, *args, **kwargs):
            for component in self._components:
                getattr(component, method.inner)(self, *args, **kwargs)

        self_method.__name__ = method.self
        return self_method

    _init_self = _make_self_method(NodeMethod.init)
    _clear_self = _make_self_method(NodeMethod.clear)
    _index_self = _make_self_method(NodeMethod.index)
    _interp_self = _make_self_method(NodeMethod.interp)
    _visit_self = _make_self_method(NodeMethod.visit)
    _validate_self = _make_self_method(NodeMethod.validate)
    _activate_self = _make_self_method(NodeMethod.activate)
    _deactivate_self = _make_self_method(NodeMethod.deactivate)

    def _notify_added(self, *nodes: "ModuleNode") -> None:
        """When this node adds another node."""
        if self.module is not None:
            for n in nodes:
                self.module._on_added(n)

    def _walk(self) -> typing.Iterator["ModuleNode"]:
        visitor = NodeVisitor()
        visitor.visit_child(self)

        seen: dict[UUID, ModuleNode] = {}
        to_visit = [self]
        while to_visit:
            for node in to_visit:
                seen[node.ck] = node
                node._visit(visitor)
            to_visit = [n for n in visitor.subtree if n.ck not in seen]

        yield from visitor._descendant_by_ck.values()

    def _on_issue(
        self,
        issue: "Issue" = None,
        *,
        subject: Union["Statement", "File", "Field", None] = None,
        type: IssueType = None,
        **kwargs,
    ):
        from bench.language import Statement, File
        from bench.language.issue import Issue

        if not isinstance(subject, (Statement, File)):
            subject = subject.parent  # fields don't have issues (yet)
        if issue is None:
            issue = Issue(type=type, parent=subject, **kwargs)
        self.issues.append(issue)

    def copy(self):
        from bench.language import wire

        _, node_datas = wire.pack_node(self)
        return wire.unpack_node(node_datas, parent=self.parent, session=None)

    @property
    def errors(self) -> list["Issue"]:
        if self.issues is None:
            return []
        return [i for i in self.issues or [] if i.kind == IssueKind.Error]

    @property
    def self_errors(self):
        return [i for i in self.errors or [] if i.parent == self]

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


@node_component
class ScopedNode(ModuleNode):
    """A scope for hosting and looking up nodes. Required for any node with children."""

    _scopes_by_name: dict[str, "ScopedNode"] = nruntime(default_factory=dict)
    _names_by_ident: dict[str, str] = nruntime(default_factory=dict)
    # TODO @Performance: technically we only need id tree in scope at the root level
    #  (but root may be detached from module, so would need attach/detach logic)
    _tree: Optional["NodeTree"] = nruntime(default=None)  # inline nodes tree

    def _init_inner(self) -> None:
        self._tree = NodeTree()

    def _get_scope(self, name: str, by: Optional[LookupBy]) -> Union["ScopedNode", None]:
        if by is None and name in self._scopes_by_name or by == LookupBy.Name:
            return self._scopes_by_name.get(name)
        if by is None and name in self._names_by_ident or by == LookupBy.PyIdent:
            if name in self._names_by_ident:
                name = self._names_by_ident[name]
                return self._scopes_by_name.get(name)
        return None

    def _find_scope(self, name: str, by: Optional[LookupBy]) -> Union["ScopedNode", None]:
        scope = self._get_scope(name, by)
        if scope is not None:
            return scope
        if self.parent is not None:
            return self.parent._find_scope(name, by=by)
        return None

    def _find_node(self, name: str, by: Optional[LookupBy] = None) -> NodeT | None:
        """Find the node recursively in this scope and its parents."""
        return self._find_scope(name, by)

    def lookup(
        self,
        path: Union["NodePath", UUID, str],
        by: Optional[LookupBy] = None,
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

    def _add_child_scope(self, scope: "ScopedNode", by_name: bool) -> None:
        self._add_child_node(scope, by_name)
        self._tree.add_tree(scope._tree)

    def _add_child_node(self, node: ModuleNode, by_name: bool) -> None:
        self._tree.add(node)
        # index name
        if node.name is not None and by_name:
            if node.name in self._scopes_by_name or node.py_ident in self._names_by_ident:
                self._on_issue(type=IssueType.AMBIGUOUS_DEFINITION, subject=node, path=node.path)
            else:
                self._scopes_by_name[node.name] = node
                self._names_by_ident[node.py_ident] = node.name

    def _clear_inner(self):
        """Resets this scope and all child scopes."""
        self._scopes_by_name = {}
        self._names_by_ident = {}
        self._tree.clear()

    @property
    def _root_scope(self) -> "ScopedNode":
        if self.parent is None:
            return self
        return self.parent._root_scope


class NodeVisitor:
    def __init__(self):
        self._descendant_by_ck: dict[UUID, ModuleNode] = {}
        self._reference_by_ck: dict[UUID, ModuleNode] = {}

    def __str__(self):
        return f"{len(self._descendant_by_ck)} nodes"

    def __repr__(self):
        return f"<NodeVisitor {str(self)}>"

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


@node(mnt=MNT.Module)
class Module(ScopedNode):
    parent: None = nparent()
    name: str = ninternal()  # can't change this yet
    committed: bool = ninternal()
    files: NodeList["File"] = nchildren(MNT.File, NodeRelationType.INLINE | NodeRelationType.FLAT)
    dependencies: dict[str, Union["Module", ModuleReference]] = nruntime(default_factory=dict)
    builtins: list["File"] = nruntime(default_factory=list)

    def __str__(self):
        issues_str = f", {len(self.issues)} issues" if self.issues is not None else ""
        return f"{self.name} ({self._status.name}, {len(self.files)} files{issues_str})"

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

    def _visit(self, visitor: NodeVisitor) -> None:
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
        by: Optional[LookupBy] = None,
        node_t: MNT | typing.Type[NodeT] | None = None,
    ) -> NodeT | None:
        if isinstance(path, UUID):
            return self._tree.get(path)
        elif isinstance(path, str) and path.startswith("."):
            return ScopedNode.lookup(self, path, node_t=node_t, by=by)
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
        by: Optional[LookupBy] = None,
        node_t: typing.Type[NodeT] | None = None,
    ) -> NodeT:
        result = self.lookup(path, by=by, node_t=node_t)
        if result is None:
            raise LookupError(f"{path} not found in {self}")
        return result

    def get_file(self, name: str) -> "File":
        from bench.language.file import File

        scope = self._scopes_by_name.get(name)
        if not isinstance(scope, File):
            raise ValueError(f"expected file, got {type(scope)}")
        return scope

    def _on_added(self, node: ModuleNode) -> None:
        for n in node._walk():
            n._assign_id_if_none()

    def _activate_inner(self, session: "Session"):
        for dependency in self.dependencies.values():
            dependency._activate(session)

    def _index_inner(self):
        for builtin in self.builtins:
            for statement in builtin.statements:
                self._add_child_scope(statement, by_name=True)
        for dependency in self.dependencies.values():
            self._nodes_by_id.update(dependency._nodes_by_id)
            self._nodes_by_ck.update(dependency._nodes_by_ck)

    def copy(self):
        from bench.language import wire

        module_data = wire.pack_module(self)
        module_copy = wire.unpack_module(module_data, session=None)
        for builtin in self.builtins:
            module_copy.add_builtin(builtin)  # also copy?
        for dependency in self.dependencies.values():
            module_copy.add_dependency(dependency)  # also copy?
        if self._status >= NodeStatus.Indexed:
            module_copy._index_rec()
        if self._status >= NodeStatus.Interpreted:
            module_copy._interp_rec(module_copy)
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
        if isinstance(maybe_module, wire.ModuleTreeData):
            logger.debug("module.interp.unpack", module=maybe_module)
            module: Module = wire.unpack_module(maybe_module, session=session)
        else:
            module = maybe_module
        # no need to copy deps since worker processes are isolated?
        dependencies = {name: dep for name, dep in libs.DEFAULT_MODULES.items()}
        module.add_builtin(dependencies["symbolx.lib"].get_file("builtins"))
        for dependency in dependencies.values():
            module.add_dependency(dependency)
        logger.debug("module.interp.index", module=module)
        module.index()
        logger.debug("module.interp.interp", module=module)
        module._interp()
        logger.debug("module.interp.done", module=module)
        return module

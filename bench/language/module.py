import abc
import dataclasses
import enum
import functools
import inspect
import typing
import uuid
from collections import defaultdict, deque
from dataclasses import dataclass
from datetime import datetime
from importlib import import_module
from logging import Logger
from typing import TYPE_CHECKING, ClassVar, Collection, Iterator, Optional, Union
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
from bench.utils.fractional import BIGGEST_INTEGER, generate_key_between
from bench.utils.utils import IdentifierType, required_field, to_pyidentifier

if TYPE_CHECKING:
    from bench.language import Field, File, Issue, Session, Statement
    from bench.language.issue import ValidationHandler
    from bench.language.wire import ModuleTreeData

logger = structlog.get_logger(__name__)


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

    Default = 0  # default inline relation
    Remote = 2**0  # not inline: Statement->Record, ...
    Shared = 2**1  # across versions: Statement->Comment, Statement[versioned=False]->Record, ...
    Flat = 2**2  # flattened inner hierarchy: Module->File, File->Statement, ...
    Cumulative = 2**3  # sum of descendants: Module->Issue, File->Issue, ...
    Named = 2**4  # scoped by name: Module->File, File->Statement, ...
    Keyed = 2**5  # scoped by key: File->Tagging, Statement->Tagging, ...
    Ordered = 2**6  # ordered: File->Statement, Statement->Field, ...


NRel = NodeRelationType

UNSET = object()


@dataclass
class NodeProperty:
    """A property of a module node."""

    name: str | None = None  # name from LHS of assignment
    is_internal: bool = False
    is_runtime: bool = False
    parent_mnts: list[MNT] | None = None
    ancestor_mnt: MNT | None = None
    default: typing.Any = UNSET
    default_factory: typing.Callable[[], typing.Any] | None = None
    copy_value: typing.Callable[[typing.Any], typing.Any] | None = None
    annotation: typing.Any = None  # type annotation on LHS of assignment
    # for relations
    child_mnt: MNT | None = None
    is_allowed: typing.Callable[["NodeT"], bool] | None = None
    children_flags: NodeRelationType = NodeRelationType.Default

    def __post_init__(self):
        if (
            not self.is_relation
            and self.is_runtime
            and self.default is UNSET
            and self.default_factory is None
        ):
            raise ValueError(f"missing default for {self}")
        if self.child_mnt and (self.default is not UNSET or self.default_factory is not None):
            raise ValueError(f"cannot set default for {self}")

    def __str__(self):
        non_default = []
        for k, v in self.__dict__.items():
            if k == "children_flags":
                flags_str = ", ".join(f.name for f in NodeRelationType if v & f)
                if flags_str:
                    non_default.append(flags_str)
            elif k not in ("name", "annotation") and v is not UNSET and v:
                if isinstance(v, bool):
                    non_default.append(k)
                else:
                    non_default.append(f"{k}={v}")
        attrs_str = ", ".join(non_default)
        return f"{self.name} ({attrs_str})" if attrs_str else self.name

    def __repr__(self):
        return f"<NodeProperty {self}>"

    def new(self) -> typing.Any:
        if self.default is not UNSET:
            return self.default
        elif self.default_factory is not None:
            return self.default_factory()
        else:
            raise ValueError(f"no default for {self}")

    def copy(self, value: typing.Any) -> typing.Any:
        if self.is_relation:
            raise ValueError(f"cannot copy relation {self}")
        elif self.copy_value is not None:
            return self.copy_value(value)
        # auto-copy if it's trivial (primitives, immutable, enum, ...)
        elif isinstance(value, (type(None), bool, int, float, str, UUID, datetime, enum.Enum)):
            return value
        else:
            raise ValueError(f"cannot copy {self}")

    @property
    def is_relation(self) -> bool:
        return self.parent_mnts or self.child_mnt or self.ancestor_mnt


def nproperty(
    *,
    default: typing.Any = UNSET,
    default_factory: typing.Callable[[], typing.Any] = None,
    copy_value: typing.Callable[[typing.Any], typing.Any] = None,
):
    """Standard user facing node property."""
    return NodeProperty(default=default, default_factory=default_factory, copy_value=copy_value)


def ninternal(
    *,
    default: typing.Any = UNSET,
    default_factory: typing.Callable[[], typing.Any] = None,
    copy_value: typing.Callable[[typing.Any], typing.Any] = None,
):
    """Internal only, persisted node property."""
    return NodeProperty(
        is_internal=True, default=default, default_factory=default_factory, copy_value=copy_value
    )


def nruntime(
    *,
    default: typing.Any = UNSET,
    default_factory: typing.Callable[[], typing.Any] = None,
    copy_value: typing.Callable[[typing.Any], typing.Any] = None,
) -> object:
    """Internal only, non-persisted runtime node property."""
    return NodeProperty(
        is_internal=True,
        is_runtime=True,
        default=default,
        default_factory=default_factory,
        copy_value=copy_value,
    )


def nparent(*mnt: MNT):
    """The parent of a node, must be of one of the given types."""
    return NodeProperty(parent_mnts=list(mnt), default=None, is_internal=True)


def nancestor(mnt: MNT):
    """Computed nearest ancestor of the given type."""
    return NodeProperty(ancestor_mnt=mnt, default=None, is_internal=True)


def nchildren(
    mnt: MNT, flags: NRel = NRel.Default, is_allowed: typing.Callable[["NodeT"], bool] = None
):
    """Computed read/write children or descendants of the given type."""
    return NodeProperty(
        child_mnt=mnt, children_flags=flags, is_allowed=is_allowed, is_internal=True
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
        return f"_{self.value}_inner"

    @property
    def self(self) -> str:
        return f"_{self.value}_self"

    @property
    def rec(self) -> str:
        return f"_{self.value}_rec"


# :NodeMethods
_NODE_INNER_METHODS: list[str] = [m.inner for m in NodeMethod]
_FORBIDDEN_NODE_METHODS = (
    [m.self for m in NodeMethod] + [m.rec for m in NodeMethod] + ["__post_init__", "__del__"]
)
_NODE_CLASS_BY_MNT: dict[MNT, type["NodeT"]] = {}


def _get_node_class(mnt: MNT):
    if len(_NODE_CLASS_BY_MNT) < len(MNT):
        m = import_module("bench.language")
        getattr(m, mnt)  # noqa
    return _NODE_CLASS_BY_MNT[mnt]


@typing.dataclass_transform()
def node_component(cls: Optional[typing.Type] = None, mnt: MNT = None, dynamic: bool = False):
    """
    Mark a class as a node component (or concrete node for a MNT).
    """

    def decorate(cls):
        properties: dict[str, NodeProperty] = {}
        static_components: list[type["ModuleNode"]] = [cls]

        # check that no forbidden methods are defined
        if cls.__name__ not in ("ModuleNode", "ScopeNode"):
            for name in _FORBIDDEN_NODE_METHODS:
                meth = getattr(cls, name, None)
                good_meth = getattr(ModuleNode, name, getattr(ScopeNode, name, None))
                if meth is not None and meth is not good_meth:
                    raise ValueError(f"forbidden method {name} defined in {cls}")
        if cls.__name__ != "ModuleNode":
            static_components.append(ModuleNode)

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
                or type(prop) == functools.cached_property
            ):
                continue  # ignore reserved names and non-fields
            if not isinstance(prop, NodeProperty):
                raise TypeError(f"{cls}.{name} is not a NodeProperty: {prop} ({type(prop)})")
            prop.name = name
            properties[name] = prop

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

        # create class
        for name, prop in properties.items():
            if not hasattr(cls, name):  # may be inherited
                continue
            if prop.ancestor_mnt:
                setattr(cls, name, _node_ancestor_prop(prop))
                if name in cls.__annotations__:  # ensure property doesn't have a dataclass field
                    del cls.__annotations__[name]
            elif prop.child_mnt:
                setattr(cls, name, dataclasses.field(init=False, default=None))
            elif prop.default is not UNSET:
                setattr(cls, name, dataclasses.field(default=prop.default))
            elif prop.default_factory is not None:
                setattr(cls, name, dataclasses.field(default_factory=prop.default_factory))
            else:
                setattr(cls, name, required_field())
            if not prop.ancestor_mnt:
                cls.__annotations__[name] = prop.annotation
        cls = dataclass(cls, repr=False, eq=False)  # type: ignore
        cls.__properties__ = properties
        cls.__static_components__ = tuple(static_components)

        # register properties
        mutable_properties: dict[str, NodeProperty] = {}
        ancestor_properties: dict[str, NodeProperty] = {}
        list_properties: dict[str, NodeProperty] = {}
        list_properties_by_child: dict[MNT, list[NodeProperty]] = defaultdict(list)
        for prop in properties.values():
            if prop.child_mnt:
                if (
                    cls.__name__ != "ScopeNode"
                    and not issubclass(cls, ScopeNode)
                    and mnt is not None
                ):
                    raise ValueError(f"{cls} is not ScopeNode for {prop}")
                list_properties[prop.name] = prop
                list_properties_by_child[prop.child_mnt].append(prop)
            elif prop.ancestor_mnt:
                ancestor_properties[prop.name] = prop
            elif not prop.is_internal:
                mutable_properties[prop.name] = prop
        cls.__tracked_properties__ = mutable_properties
        cls.__ancestor_properties__ = ancestor_properties
        cls.__list_properties__ = list_properties
        cls.__list_properties_by_child__ = list_properties_by_child

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


def _node_ancestor_prop(prop: NodeProperty) -> property:
    def get(self: NodeT) -> Optional[NodeT]:
        parent = self.parent
        while parent is not None:
            if parent.mnt == prop.ancestor_mnt:
                return parent
            parent = parent.parent
        return None

    def set(self: NodeT, value: NodeT):
        raise NotImplementedError(f"cannot set computed ancestor property {prop}")

    return property(get, set)


class NodeList(Collection, typing.Generic[NodeT]):
    def __init__(self, parent: "ScopeNode", property: NodeProperty):
        self._parent = parent
        self._child_t: type[NodeT] = _get_node_class(property.child_mnt)
        self._prop = property
        self._flags = property.children_flags
        self._children: list[NodeT] = []

    def __str__(self):
        return str(self._children)

    def __repr__(self):
        return f"<NodeList {self._parent.path}->{self._prop.name}: {self}>"

    def _update(self, scope: "ScopeNode"):
        """Recomputes the list from the given scope."""

        # _children is effectively a computed property which is replaced wholesale,
        # we don't do diff updates to keep it simple with all the relation types.
        if self._flags & NRel.Cumulative:
            # all matching children of parent's descendants
            #  e.g. Module->Issue, File->Issue, ... -> all issues
            self._children = scope._local_root_tree.get_descendants(
                scope.ck, self._child_t, recursive=True, prefilter=False
            )
            assert not self._flags & NRel.Ordered, f"cannot order cumulative {self}"
        elif self._flags & NRel.Flat:
            # all matching descendants of matching children of parent
            #  e.g. Module->File, File->File, ... -> all files
            self._children = scope._local_root_tree.get_descendants(
                scope.ck, self._child_t, recursive=True, prefilter=True
            )
            if self._flags & NRel.Ordered:
                self._children.sort(key=lambda n: n.order_key)
                # nocheckin: incorrect for flat
        else:
            # only matching children of parent
            self._children = scope._local_root_tree.get_descendants(
                scope.ck, self._child_t, recursive=False
            )
            if self._flags & NRel.Ordered:
                self._children.sort(key=lambda n: n.order_key or BIGGEST_INTEGER)

    def create(self, *args, **kwargs):
        node_cls = _NODE_CLASS_BY_MNT[self._prop.child_mnt]
        if hasattr(node_cls, "_coerce_from"):
            node = node_cls._coerce_from(*args, **kwargs)
        else:
            node = node_cls(*args, **kwargs)
        self.append(node)
        return node

    def append(self, _node: NodeT, _create: bool = True) -> None:
        """
        Attaches a child node to a parent through a list. This is for users adding nodes.
        A node may be 'append'-ed to a list at most once,
         but may exist in multiple lists (through _init_from collection).
        """
        if _node.parent is not None:  # maybe copy?
            raise ValueError(f"cannot append {_node} to {self}: already has parent {_node.parent}")
        was_attached = _node.attached
        _node.parent = self._parent

        # assign order key to ordered nodes
        if self._flags & NRel.Ordered and _node.order_key is None:
            last_ok = self._children[-1].order_key if self._children else None
            _node.order_key = generate_key_between(last_ok, None)
            # nocheckin: incorrect for flat

        # assign ids if newly attached to the module (ids are derived from ck + module)
        if not was_attached and _node.attached:
            module_id = _node.module.id
            for n in _node._walk():
                if n.id is None:
                    n._assign_id(module_id)
        # 'create' node in session
        if _create and self._parent._session:
            self._parent._session.tracer.node_create(_node)
        # subsume node into parent scope
        if not was_attached and _node.attached and isinstance(_node, ScopeNode):
            # take over node's children
            self._parent._import_scope(_node)
            _node._local_tree = None
        else:
            # index node into tree
            self._parent._local_root_tree.add(_node)
        if self._flags & NRel.Named and _node.name is not None:
            self._parent._register_named_child(_node)

        # and update every affect node & list
        self._parent._trigger_update([_node])

        assert _node in self._children, f"node {_node} not in {self}"

    def extend(self, nodes: Collection[NodeT]):
        """Attaches a list of child nodes to a parent. See append."""
        for node in nodes:
            self.append(node)

    def remove(self, _node: NodeT, _delete: bool = True):
        """Removes a child node from a parent. See append for reverse."""
        if _delete and self._parent.session:
            self._parent.session.tracer.node_delete(_node)
        self._parent._local_root_tree.remove(_node)
        raise NotImplementedError(f"{self!r}.remove not supported yet")

    def clear(self):
        """Removes all child nodes from a parent. See append for reverse."""
        children = list(self._children)
        for child in children:
            self.remove(child)

    def get(self, some_id: str) -> Optional[NodeT]:
        if not (self._flags & NRel.Keyed) and not (self._flags & NRel.Named):
            raise ValueError(f"cannot get {some_id} from {self}")
        for child in self._children:
            if child.key == some_id or child.name == some_id or child.py_ident == some_id:
                return child
        return None

    def __contains__(self, obj: object) -> bool:
        if isinstance(obj, str) and (self._flags & NRel.Keyed or self._flags & NRel.Named):
            return self.get(obj) is not None
        elif isinstance(obj, ModuleNode):
            return obj in self._children
        else:
            return False

    def __getitem__(self, item: int | slice | str) -> NodeT | list[NodeT]:
        if isinstance(item, int):
            return self._children[item]
        elif isinstance(item, slice):
            return self._children[item]
        elif isinstance(item, str):
            return self.get(item)
        else:
            raise TypeError(f"invalid index for {self}: {item} ({type(item)})")

    def __iter__(self) -> Iterator[NodeT]:
        yield from self._children

    def __len__(self) -> int:
        return len(self._children)


NT = typing.TypeVar("NT")


class NodeTree(typing.Generic[NT]):
    """An indexed tree of module nodes"""

    def __init__(self, nodes: list[NT] = None):
        self.nodes_by_id: dict[UUID, NT] = {}
        self.nodes_by_ck: dict[UUID, NT] = {}
        self.node_id_by_parent_id: dict[UUID, list[UUID]] = {}
        for node in nodes or []:
            self.add(node)

    def __str__(self):
        return f"{len(self.nodes_by_id)} nodes"

    def __repr__(self):
        return f"<ModuleTree {self}>"

    #
    # Mutations
    #

    def clear(self):
        """Clear the tree"""
        self.nodes_by_id.clear()
        self.nodes_by_ck.clear()
        self.node_id_by_parent_id.clear()

    def add(self, node: NT):
        """Add a node to the tree (error if node already exists)"""
        if node.id is None:
            raise ValueError(f"node {node!r} has no id")
        if node.id in self.nodes_by_id:
            existing = self.nodes_by_id[node.id]
            raise ValueError(
                f"node {node!r} (id={node.id}) already exists in {self!r}: {existing!r}"
            )
        self.nodes_by_id[node.id] = node
        self.nodes_by_ck[node.ck] = node
        if node.parent_id is not None:
            if node.parent_id not in self.node_id_by_parent_id:
                self.node_id_by_parent_id[node.parent_id] = []
            self.node_id_by_parent_id[node.parent_id].append(node.id)

    def replace(self, node: NT):
        """Upsert a node in the tree (replace if node already exists)"""
        old_node = self.nodes_by_id.get(node.id)
        if old_node is not None and old_node.parent_id is not None:
            self.node_id_by_parent_id[old_node.parent_id].remove(node.id)
        self.nodes_by_id[node.id] = node
        self.nodes_by_ck[node.ck] = node
        if node.parent_id not in self.node_id_by_parent_id:
            self.node_id_by_parent_id[node.parent_id] = []
        self.node_id_by_parent_id[node.parent_id].append(node.id)

    def remove(self, node: NT):
        """Remove a node from the tree (incl. all descendants if recursive)"""
        descendants = self.get_descendants(node.id, recursive=True, include_self=True)
        for descendant in descendants:
            if descendant.id in self.nodes_by_id:
                self.nodes_by_id.pop(descendant.id)
            if descendant.ck in self.nodes_by_ck:
                self.nodes_by_ck.pop(descendant.ck)
            if descendant.id in self.node_id_by_parent_id:
                self.node_id_by_parent_id.pop(descendant.id)
            if descendant.parent_id in self.node_id_by_parent_id:
                self.node_id_by_parent_id[descendant.parent_id].remove(descendant.id)

    def truncate(self, node: NT, t: type[NT] | None = None, recursive: bool = True):
        """Truncate descendants of a node"""
        descendants = self.get_descendants(node.id, t, recursive=recursive)
        for descendant in descendants:
            if descendant.id in self.node_id_by_parent_id:
                self.node_id_by_parent_id.pop(descendant.id)
            if descendant.parent_id in self.node_id_by_parent_id:
                self.node_id_by_parent_id[descendant.parent_id].remove(descendant.id)
            self.nodes_by_id.pop(descendant.id)
            self.nodes_by_ck.pop(descendant.ck)

    def prune(self, t: type[NT]):
        """Prune all nodes of the given type"""
        for node in list(self.nodes_by_id.values()):
            if isinstance(node, t):
                self.remove(node, recursive=True)

    def add_tree(self, tree: Union["NodeTree", "DetachedNodeTree"]):
        if isinstance(tree, DetachedNodeTree):
            for node in tree.nodes_by_ck.values():
                self.add(node)
        else:
            self.nodes_by_id.update(tree.nodes_by_id)
            self.nodes_by_ck.update(tree.nodes_by_ck)
            self.node_id_by_parent_id.update(tree.node_id_by_parent_id)

    def remove_tree(self, tree: "NodeTree"):
        for node in tree.nodes_by_id.values():
            if node.id in self.nodes_by_id:
                self.nodes_by_id.pop(node.id)
            if node.ck in self.nodes_by_ck:
                self.nodes_by_ck.pop(node.ck)
            if node.id in self.node_id_by_parent_id:
                self.node_id_by_parent_id.pop(node.id)

    #
    # Read only
    #

    def get(self, node_id_or_ck: UUID) -> Optional[NT]:
        """Gets a node by id"""
        node = self.nodes_by_id.get(node_id_or_ck)
        return node if node is not None else self.nodes_by_ck.get(node_id_or_ck)

    def path_of(self, node: NT) -> list[NT]:
        """Returns the path from the root to the node"""
        path = []
        while node:
            path.insert(0, node)
            node = self.nodes_by_id.get(node.parent_id)
        return path

    @property
    def roots(self) -> list[NT]:
        return [
            node
            for node in self.nodes_by_id.values()
            if node.parent_id is None or node.parent_id not in self.nodes_by_id
        ]

    @property
    def root(self) -> Optional[NT]:
        roots = self.roots
        if len(roots) > 1:
            raise ValueError(f"expected 0 or 1 root nodes, got {roots}")
        return roots[0] if roots else None

    def walk_bfs(self, roots: list[NT] = None) -> typing.Generator[NT, None, None]:
        """Walks the tree in breadth-first order"""
        num_traversed = 0
        queue = deque(roots or self.roots)
        while queue:
            current_node = queue.popleft()
            num_traversed += 1
            yield current_node
            for child_id in self.node_id_by_parent_id.get(current_node.id, []):
                queue.append(self.nodes_by_id[child_id])
        if roots == self.roots and num_traversed != len(self.nodes_by_id):
            raise ValueError(
                f"expected {len(self.nodes_by_id)} nodes, but traversed {num_traversed}"
            )

    def walk_bfs_batched(self, roots: list[NT] = None) -> typing.Generator[list[NT], None, None]:
        """Walks the tree in breadth-first order, yielding all nodes at each level"""
        num_traversed = 0
        queue = deque(roots or self.roots)
        while queue:
            level = []
            for _ in range(len(queue)):
                current_node = queue.popleft()
                level.append(current_node)
                for child_id in self.node_id_by_parent_id.get(current_node.id, []):
                    queue.append(self.nodes_by_id[child_id])
            num_traversed += len(level)
            yield level
        if roots == self.roots and num_traversed != len(self.nodes_by_id):
            raise ValueError(
                f"expected {len(self.nodes_by_id)} nodes, but traversed {num_traversed}"
            )

    def get_descendants(
        self,
        node_id_or_ck: UUID,
        t: type[NT] | None = None,
        recursive: bool = False,
        prefilter: bool = False,
        include_self: bool = False,
    ) -> list["NT"]:
        """Finds all children (or descendants) of the given type"""
        if node_id_or_ck in self.nodes_by_id:
            node_id = node_id_or_ck
        elif node_id_or_ck in self.nodes_by_ck:
            node_id = self.nodes_by_ck[node_id_or_ck].id
        else:
            raise ValueError(f"node {node_id_or_ck} is not in {self!r}")
        children = [
            self.nodes_by_id[child_id]
            for child_id in self.node_id_by_parent_id.get(node_id, [])
            if t is None or not prefilter or isinstance(self.nodes_by_id[child_id], t)
        ]
        descendants = children[:]
        if recursive:
            for child in children:
                if child.id not in self.node_id_by_parent_id:
                    continue
                descendants.extend(
                    self.get_descendants(child.id, t, prefilter=prefilter, recursive=True)
                )
        if include_self and node_id in self.nodes_by_id:
            descendants.append(self.nodes_by_id[node_id])
        if not prefilter and t is not None:
            descendants = [n for n in descendants if isinstance(n, t)]
        return descendants

    def get_ancestor(self, node_id: UUID, t: type[NT] | None = None) -> Optional["NT"]:
        """Finds the next ancestor of the given type"""
        node = self.nodes_by_id.get(node_id)
        if node is None:
            raise ValueError(f"node {node_id} is not in {self}")
        while node:
            if t is None or isinstance(node, t):
                return node
            if node.parent_id is None:
                return None
            node = self.nodes_by_id[node.parent_id]
        return None

    def get_ancestors(
        self,
        node_id: UUID,
        t: type[NT] | None = None,
        include_self: bool = False,
    ) -> list["NT"]:
        """Finds all ancestors of the given type"""
        ancestors = []
        node = self.nodes_by_id.get(node_id)
        if node is None:
            raise ValueError(f"node {node_id} is not in {self}")
        if include_self:
            ancestors.append(node)
        while node:
            if t is None or isinstance(node, t):
                ancestors.append(node)
            if node.parent_id is None:
                break
            node = self.nodes_by_id[node.parent_id]
        return ancestors


class DetachedNodeTree:
    """
    A minimal NodeTree for working with instantiated nodes who may not have ids yet.
    We have a separate tree for this because wire nodes work with ids only (for parent),
     and we don't need to support all operations since it's only for detached nodes.
    """

    def __init__(self):
        self.nodes_by_ck: dict[UUID, "ModuleNode"] = {}
        self.node_ck_by_parent_ck: dict[UUID, list[ModuleNode]] = defaultdict(list)

    def __str__(self):
        return f"{len(self.nodes_by_ck)} nodes"

    def __repr__(self):
        return f"<DetachedNodeTree {self}>"

    def add(self, node: "ModuleNode"):
        """Add a node to the tree (error if node already exists)"""
        if node.ck in self.nodes_by_ck and self.nodes_by_ck[node.ck] is not node:
            raise ValueError(f"node {node!r} (ck={node.ck}) already exists in {self!r}")
        self.nodes_by_ck[node.ck] = node
        if node.parent is not None:
            self.node_ck_by_parent_ck[node.parent.ck].append(node)

    def add_tree(self, tree: "DetachedNodeTree"):
        assert type(self) == type(tree), f"cannot add {tree!r} to {self!r}"
        self.nodes_by_ck.update(tree.nodes_by_ck)
        for parent_ck, children in tree.node_ck_by_parent_ck.items():
            self.node_ck_by_parent_ck[parent_ck].extend(children)

    def remove(self, node: "ModuleNode"):
        """Remove a node from the tree (incl. all descendants if recursive)"""
        descendants = self.get_descendants(node.ck, recursive=True, include_self=True)
        for descendant in descendants:
            if descendant.ck in self.node_ck_by_parent_ck:
                self.node_ck_by_parent_ck.pop(descendant.ck)
            if descendant.parent.ck in self.node_ck_by_parent_ck:
                self.node_ck_by_parent_ck[descendant.parent.ck].remove(descendant)

    def get_descendants(
        self,
        node_id_or_ck: UUID,
        t: type[NT] | None = None,
        recursive: bool = False,
        prefilter: bool = False,
        include_self: bool = False,
    ) -> list["NT"]:
        """Finds all children (or descendants) of the given type"""
        children = [
            child
            for child in self.node_ck_by_parent_ck.get(node_id_or_ck, [])
            if t is None or not prefilter or isinstance(child, t)
        ]
        descendants = children[:]
        if recursive:
            for child in children:
                if child.ck not in self.node_ck_by_parent_ck:
                    continue
                descendants.extend(
                    self.get_descendants(child.ck, t, prefilter=prefilter, recursive=True)
                )
        if include_self and node_id_or_ck in self.nodes_by_ck:
            descendants.append(self.nodes_by_ck[node_id_or_ck])
        if not prefilter and t is not None:
            descendants = [n for n in descendants if isinstance(n, t)]
        return descendants


def _make_self_method(
    method: NodeMethod, wraps, from_status: NodeStatus = None, to_status: NodeStatus = None
):
    """Creates method that calls _method_inner for all components"""

    @functools.wraps(wraps)
    def self_method(self: "ModuleNode", *args, **kwargs):
        if from_status is not None and self._status != from_status:
            raise RuntimeError(f"cannot {method.name} {self!r} (status={self._status.name})")
        for component in self._components:
            if hasattr(component, method.inner):
                getattr(component, method.inner)(self, *args, **kwargs)
        if to_status is not None:
            self._status = to_status

    self_method.__name__ = method.self
    return self_method


@node_component
class ModuleNode(abc.ABC):
    """
    A node in a Bench module tree.
    A node has a per-version unique id (id) and a constant identifier key (ck).
    """

    mnt: ClassVar[MNT]  # set in @node decorator
    __static_components__: ClassVar[tuple[type["ModuleNode"]]] = []
    __properties__: ClassVar[dict[str, NodeProperty]] = {}
    __ancestor_properties__: ClassVar[dict[str, NodeProperty]] = {}
    __list_properties__: ClassVar[dict[str, NodeProperty]] = {}
    __list_properties_by_child__: ClassVar[dict[MNT, list[NodeProperty]]] = defaultdict(list)
    __tracked_properties__: ClassVar[dict[str, NodeProperty]] = {}

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

    _session: Optional["Session"] = nruntime(default=None)
    _status: NodeStatus = nruntime(default=None)

    def __post_init__(self):
        self._init_self()
        if self._session is None:
            from bench.language.session import active_session

            self._session = active_session.get()
        if self._status is None:
            self._status = NodeStatus.Interpreted if self._session else NodeStatus.Raw

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.parent.id if self.parent is not None else None

    @property
    def _components(self) -> tuple[type["ModuleNode"]]:
        return self.__static_components__

    @property
    def _local_root(self) -> "ModuleNode":
        parent = self
        while parent.parent is not None:
            parent = parent.parent
        return parent

    def _assign_id(self, module_id: UUID):
        assert module_id, f"cannot assign id to {self} without a module id"
        assert self.id is None, f"cannot assign id to {self} twice"
        assert self.ck is not None, f"cannot assign id to {self} without ck"
        self.id = get_node_id(module_id, self.ck)

    def __eq__(self, other):
        return isinstance(other, self.__class__) and self.id == other.id

    def __hash__(self):
        return hash(self.id)

    def __setattr__(self, key, value):
        if self._status != NodeStatus.Tracked:
            super().__setattr__(key, value)
        elif key in self.__tracked_properties__:
            super().__setattr__(key, value)
            # nocheckin: track and validate mutation
        elif key in self.__list_properties__:
            raise AttributeError(f"cannot set {self.__properties__[key]} (use NodeList)")
        else:
            raise AttributeError(f"cannot set {key} on {self}")

    def _trigger_update(self, changed: list["ModuleNode"]):
        """Trigger list updates and re-interps (now or later) in all relevant nodes."""
        # reinit children for any affected parent nodes
        # TODO @Performance: use mark dirty in node list to avoid reinit
        parent = self
        affected_mnts = set([n.mnt for n in changed])
        while parent is not None:
            for prop in parent.__list_properties__.values():
                if prop.child_mnt in affected_mnts:
                    getattr(parent, prop.name)._update(parent)
            parent = parent.parent
        # nocheckin: also reinterp

    # abstract :ComponentMethods

    def _init_inner(self) -> None:
        """Initialize this node."""
        for name, prop in self.__list_properties__.items():
            setattr(self, name, NodeList(self, prop))

    def _clear_inner(self) -> None:
        """Resets this node's index and interp state."""
        pass

    def _index_inner(self) -> None:
        """Index this node."""
        pass

    def _interp_inner(self, scope: "ScopeNode") -> None:
        """Interpret this node."""
        pass

    def _validate_inner(self, properties: Collection[str], on_issue: "ValidationHandler") -> None:
        """Validate the given properties of this node."""
        pass  # nocheckin: validate
        # how does this work with _interp? is it part of interp?

    def _visit_inner(self, visitor: "NodeVisitor") -> None:
        """Visit any referenced nodes."""
        for prop in self.__list_properties__.values():
            for child in getattr(self, prop.name):
                visitor.visit_child(child)

    def _activate_inner(self, session: "Session") -> None:
        """'Instantiate' this object in the given session."""
        assert self._status == NodeStatus.Interpreted, f"cannot activate {self} in {self._status}"
        assert self._session is None, f"cannot activate {self} while active in {self._session}"
        self._session = session
        self._status = NodeStatus.Tracked

    def _deactivate_inner(self) -> None:
        """'Deinstantiate' this object."""
        assert self._status == NodeStatus.Tracked, f"cannot deactivate {self} in {self._status}"
        self._status = NodeStatus.Interpreted
        self._session = None

    # final :ComponentMethods

    _init_self = _make_self_method(NodeMethod.init, _init_inner)
    _clear_self = _make_self_method(NodeMethod.clear, _clear_inner, to_status=NodeStatus.Raw)
    _index_self = _make_self_method(
        NodeMethod.index, _index_inner, from_status=NodeStatus.Raw, to_status=NodeStatus.Indexed
    )
    _interp_self = _make_self_method(
        NodeMethod.interp,
        _interp_inner,
        from_status=NodeStatus.Indexed,
        to_status=NodeStatus.Interpreted,
    )
    _visit_self = _make_self_method(NodeMethod.visit, _visit_inner)
    _validate_self = _make_self_method(NodeMethod.validate, _validate_inner)
    _activate_self = _make_self_method(NodeMethod.activate, _activate_inner)
    _deactivate_self = _make_self_method(NodeMethod.deactivate, _deactivate_inner)

    def _copy_self(self, keep_parent: bool = False, reset_id: bool = True) -> "ModuleNode":
        """
        Copies this node without any descendants.
        If keep parent we attach the copy to the same parent.
        All non-relational properties are copied using NodeProperty.copy.
        """
        props = {}
        for name, prop in self.__properties__.items():
            if prop.is_relation:
                continue
            props[name] = prop.copy(getattr(self, name))
        if keep_parent:
            props["parent"] = self.parent
        if reset_id:
            props["id"] = None
            props["ck"] = uuid.uuid4()
        copy = self.__class__(**props)
        return copy

    def _walk(self) -> typing.Iterator["ModuleNode"]:
        """Walks this node and all descendants in breadth-first order"""
        visitor = NodeVisitor()
        visitor.visit_child(self)

        seen: dict[UUID, ModuleNode] = {}
        to_visit = [self]
        while to_visit:
            for node in to_visit:
                seen[node.ck] = node
                node._visit_self(visitor)
            to_visit = [n for n in visitor.subtree if n.ck not in seen]

        yield from visitor._descendant_by_ck.values()

    @property
    def attached(self) -> bool:
        return self.module is not None

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


def _make_rec_method(method: NodeMethod, wraps, pass_scope: bool = False):
    """Creates method that calls _method_self for self and all descendants"""

    @functools.wraps(wraps)
    def rec_method(self: "ScopeNode"):
        descendants = self._local_root_tree.get_descendants(self.ck, recursive=True)
        if pass_scope:
            for node in descendants:
                scope = node if isinstance(node, ScopeNode) else node.parent
                getattr(node, method.self)(scope)
            getattr(self, method.self)(node)
        else:
            for node in descendants:
                getattr(node, method.self)()
            getattr(self, method.self)()

    rec_method.__name__ = method.rec
    return rec_method


@node_component
class ScopeNode(ModuleNode):
    """A scope for hosting and looking up nodes. Required for any node with children."""

    issues: NodeList["Issue"] = nchildren(MNT.Issue, NRel.Cumulative)
    _scopes_by_name: dict[str, "ScopeNode"] = nruntime(default_factory=dict)
    _names_by_ident: dict[str, str] = nruntime(default_factory=dict)
    # the local tree is maintained at the local root (usually module, maybe a detached root node)
    _local_tree: Union["NodeTree", "DetachedNodeTree", None] = nruntime(default=None)

    def _init_inner(self) -> None:
        if self.parent is None:
            if not isinstance(self, Module):
                self._local_tree = DetachedNodeTree()
            else:
                self._local_tree = NodeTree()
            self._local_tree.add(self)

    _clear_rec = _make_rec_method(NodeMethod.clear, ModuleNode._clear_self)
    _index_rec = _make_rec_method(NodeMethod.index, ModuleNode._index_self)
    _interp_rec = _make_rec_method(NodeMethod.interp, ModuleNode._interp_self, pass_scope=True)

    def _get_scope(self, name: str, by: Optional[LookupBy]) -> Union["ScopeNode", None]:
        if by is None and name in self._scopes_by_name or by == LookupBy.Name:
            return self._scopes_by_name.get(name)
        if by is None and name in self._names_by_ident or by == LookupBy.PyIdent:
            if name in self._names_by_ident:
                name = self._names_by_ident[name]
                return self._scopes_by_name.get(name)
        return None

    def _find_scope(self, name: str, by: Optional[LookupBy]) -> Union["ScopeNode", None]:
        scope = self._get_scope(name, by)
        if scope is not None:
            return scope
        if self.parent is not None:
            return self.parent._find_scope(name, by=by)
        return None

    def _find_node(self, name: str, by: Optional[LookupBy] = None) -> NodeT | None:
        """Find the node recursively in this scope and its parents."""
        return self._find_scope(name, by)

    def _update_lists(self, scope: "ScopeNode"):
        for prop in self.__list_properties__.values():
            getattr(self, prop.name)._update(scope)

    def _clear_inner(self):
        self._scopes_by_name = {}
        self._names_by_ident = {}

    def _index_inner(self) -> None:
        for prop in self.__list_properties__.values():
            if prop.children_flags & NRel.Named:
                for child in getattr(self, prop.name):
                    if child.name is not None:
                        self._register_named_child(child)

    def _register_named_child(self, node: ModuleNode) -> None:
        """
        Adds a child node into this scope. Idempotent for the same node.
        """
        if node.name in self._scopes_by_name or node.py_ident in self._names_by_ident:
            if node.py_ident in self._names_by_ident:
                existing = self._scopes_by_name[self._names_by_ident[node.py_ident]]
            else:
                existing = self._scopes_by_name[node.name]
            if existing.id != node.id:
                self._on_issue(type=IssueType.AMBIGUOUS_DEFINITION, subject=node, path=node.path)
        else:
            self._scopes_by_name[node.name] = node
            self._names_by_ident[node.py_ident] = node.name

    def _import_scope(self, scope: "ScopeNode") -> None:
        """Adds the given tree into this scope."""
        self._local_root_tree.add_tree(scope._local_tree)

    @property
    def _local_root_scope(self) -> "ScopeNode":
        if self.parent is None:
            return self
        return self.parent._local_root_scope

    @property
    def _local_root_tree(self) -> Union["NodeTree", "DetachedNodeTree"]:
        tree = self._local_root_scope._local_tree
        assert tree is not None, f"no local tree for {self} in {self._local_root_scope}"
        return tree

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
            return self._local_root_scope.lookup(path, by=by, node_t=node_t)
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

    def _on_issue(
        self,
        *,
        subject: Union["Statement", "File", "Field", None] = None,
        type: IssueType = None,
        message: str = None,
        **kwargs,
    ):
        from bench.language import File, Statement
        from bench.language.issue import Issue

        if not isinstance(subject, (Statement, File)):
            subject = subject.parent  # fields don't have issues (yet)
        issue_ck = uuid.uuid5(subject.id, (type.value + (message or "")))
        issue_id = issue_ck  # not sure?
        issue = Issue(id=issue_id, ck=issue_ck, type=type, parent=None, **kwargs)
        subject.issues.append(issue)

    @property
    def errors(self) -> list["Issue"]:
        if self.issues is None:
            return []
        return [i for i in self.issues or [] if i.kind == IssueKind.Error]

    @property
    def self_errors(self):
        return [i for i in self.errors or [] if i.parent == self]


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
class Module(ScopeNode):
    parent: None = nparent()
    name: str = ninternal()  # can't change this yet
    committed: bool = ninternal(default=False)
    files: NodeList["File"] = nchildren(MNT.File, NRel.Flat | NRel.Named)
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
            return self._local_tree.get(path)
        elif isinstance(path, str) and path.startswith("."):
            return ScopeNode.lookup(self, path, node_t=node_t, by=by)
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

    def _activate_inner(self, session: "Session"):
        for dependency in self.dependencies.values():
            dependency._activate(session)

    def _deactivate_inner(self) -> None:
        for dependency in self.dependencies.values():
            dependency._deactivate()

    def _index_inner(self):
        for builtin in self.builtins:
            for statement in builtin.statements:
                self._import_scope(statement, by_name=True)
        for dependency in self.dependencies.values():
            self._local_root_tree.add_tree(dependency._local_tree)

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

import abc
import dataclasses
import enum
import functools
import inspect
import typing
import uuid
from collections import defaultdict, deque
from copy import deepcopy
from dataclasses import dataclass
from datetime import datetime
from importlib import import_module
from itertools import chain
from logging import Logger
from typing import TYPE_CHECKING, Callable, ClassVar, Collection, Iterator, Optional, Union
from uuid import UUID, uuid4

import structlog
from cachetools import cached

from bench.language.const import (
    INTERP_NODE_TYPES,
    MNT,
    IssueKind,
    IssueType,
    ModuleReference,
    NodePath,
    StatementType,
    parse_absolute_node_reference,
    parse_node_path,
)
from bench.language.validation import (
    PropertyValidationHandler,
    ValidationError,
    ValidationHandler,
    on_issue_raise,
)
from bench.utils.dt import utcnow_with_tz
from bench.utils.fractional import BIGGEST_INTEGER, generate_key_between
from bench.utils.func import did_you_mean_str
from bench.utils.utils import DEBUG, IdentifierType, frozendict, required_field, to_pyidentifier

if TYPE_CHECKING:
    from bench.language import File, Issue, Session
    from bench.language.mutate import ModuleMutation
    from bench.language.wire import NodeData

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
    Named = 2**4  # indexed by name: Module->File, File->Statement, ...
    Scoped = 2**5  # scoped by name: Module->File, File->Statement, ...
    Keyed = 2**6  # indexed by key: File->Tagging, Statement->Tagging, ...
    Ordered = 2**7  # ordered: File->Statement, Statement->Field, ...


NRel = NodeRelationType

UNSET = object()


@dataclass
class NodeProperty:
    """A property of a module node."""

    name: str | None = None  # name from LHS of assignment
    component: type["ModuleNode"] | None = None  # source component class
    is_required: bool = False
    is_internal: bool = False
    is_runtime: bool = False
    is_cru: bool = False
    parent_mnts: list[MNT] | None = None
    ancestor_mnt: MNT | None = None
    default: typing.Any = UNSET
    default_factory: Callable[[], typing.Any] | None = None
    list_type: type["NodeListBase"] | None = None
    custom_copy: Callable[[typing.Any], typing.Any] | None = None
    custom_validate: Callable[[typing.Any, "PropertyValidationHandler"], bool | None] | None = None
    annotation: typing.Any = None  # type annotation on LHS of assignment
    # for relations
    child_mnt: MNT | None = None
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
        return f"{self.component.__name__}.{self.name}>"

    def __repr__(self):
        non_default = []
        for k, v in self.__dict__.items():
            if k == "children_flags":
                flags_str = ", ".join(f.name for f in NodeRelationType if v & f)
                if flags_str:
                    non_default.append(flags_str)
            elif k not in ("name", "annotation", "component") and v is not UNSET and v:
                if isinstance(v, bool):
                    non_default.append(k)
                else:
                    non_default.append(f"{k}={v}")
        attrs_str = ", ".join(non_default)
        attrs_str = f" ({attrs_str})" if attrs_str else ""
        return f"<NodeProperty {self}{attrs_str}>"

    def equals_type(self, other: "NodeProperty") -> bool:
        """Compares everything but the source component."""
        for k in dataclasses.fields(self):
            if k.name == "component":
                continue
            if getattr(self, k.name) != getattr(other, k.name):
                return False
        return True

    def new(self) -> typing.Any:
        if self.default is not UNSET:
            return self.default
        elif self.default_factory is not None:
            return self.default_factory()
        else:
            raise ValueError(f"no default for {self!r}")

    def copy(self, value: typing.Any) -> typing.Any:
        if self.is_relation:
            raise ValueError(f"cannot copy relation {self!r}")
        elif self.custom_copy is not None:
            return self.custom_copy(value)
        # auto-copy if it's trivial (primitives, immutable, enum, ...)
        elif isinstance(value, (type(None), bool, int, float, str, UUID, datetime, enum.Enum)):
            return value
        else:
            raise ValueError(f"cannot copy {self!r}")

    def validate(self, value: typing.Any, on_issue: "PropertyValidationHandler") -> bool | None:
        if self.custom_validate is not None:
            return self.custom_validate(value, on_issue)
        else:
            return None

    @property
    def is_relation(self) -> bool:
        return bool(self.parent_mnts) or self.child_mnt or self.ancestor_mnt


def nproperty(
    *,
    default: typing.Any = UNSET,
    default_factory: Callable[[], typing.Any] = None,
    copy: Callable[[typing.Any], typing.Any] = None,
    validate: Callable[[typing.Any, "PropertyValidationHandler"], bool | None] = None,
):
    """Standard user facing node property."""
    return NodeProperty(
        default=default,
        default_factory=default_factory,
        custom_copy=copy,
        custom_validate=validate,
    )


def ninternal(
    *,
    default: typing.Any = UNSET,
    default_factory: Callable[[], typing.Any] = None,
    copy: Callable[[typing.Any], typing.Any] = None,
    is_cru: bool = False,
):
    """Internal only, persisted node property."""
    return NodeProperty(
        is_internal=True,
        is_required=True,
        default=default,
        default_factory=default_factory,
        custom_copy=copy,
        is_cru=is_cru,
    )


def nruntime(
    *,
    default: typing.Any = UNSET,
    default_factory: Callable[[], typing.Any] = None,
    copy: Callable[[typing.Any], typing.Any] = None,
) -> object:
    """Internal only, non-persisted runtime node property."""
    return NodeProperty(
        is_internal=True,
        is_runtime=True,
        is_required=False,
        default=default,
        default_factory=default_factory,
        custom_copy=copy,
    )


def nparent(*mnt: MNT):
    """The parent of a node, must be of one of the given types."""
    return NodeProperty(parent_mnts=list(mnt), default=None, is_internal=True)


def nancestor(mnt: MNT):
    """Computed nearest ancestor of the given type."""
    return NodeProperty(ancestor_mnt=mnt, default=None, is_internal=True)


def nchildren(mnt: MNT, flags: NRel = NRel.Default, custom_list: type["NodeListBase"] = None):
    """Computed read/write children or descendants of the given type."""
    return NodeProperty(
        child_mnt=mnt,
        children_flags=flags,
        is_internal=True,
        is_required=True,
        list_type=custom_list or NodeList,
    )


class NodeStatus(enum.IntEnum):
    Source = 0
    Indexed = 1
    Interpreted = 2
    Tracked = 3


NS = NodeStatus


class NodeMethod(enum.Enum):
    init = "init"
    clear = "clear"
    index = "index"
    interp = "interp"
    visit = "visit"
    validate = "validate"
    activate = "activate"
    deactivate = "deactivate"
    call = "call"
    iter = "iter"
    aiter = "aiter"
    len = "len"

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
_COMPONENT_CLASS_BY_NAME: dict[str, type["ModuleNode"]] = {}
_COMPONENT_METHODS: dict[[NodeMethod, type["ModuleNode"]], typing.Any] = {}
_COMPONENT_CALL_ORDER: list[str] = [
    "ModuleNode",
    "ScopeNode",
    "HasFields",  # for resolved_fields
    # the rest
]


@cached(cache={})
def _sort_components_in_call_order(
    components: list[type["ModuleNode"]],
) -> list[type["ModuleNode"]]:
    """Sorts components by call order. Nodes without call order are left as-is."""
    sorted_components = []
    for component in components:
        if component.__name__ in _COMPONENT_CALL_ORDER:
            sorted_components.append(component)
    sorted_components.sort(key=lambda c: _COMPONENT_CALL_ORDER.index(c.__name__))
    for component in components:
        if component.__name__ not in _COMPONENT_CALL_ORDER:
            sorted_components.append(component)
    return sorted_components


_concrete_component_methods: dict[str, list[typing.Any]] = {}


def _get_component_methods(
    components: list[type["ModuleNode"]], method: NodeMethod, concrete_key: str
) -> list[typing.Any]:
    """Get the actually implemented methods in the given components in call order."""
    cache_key = f"{concrete_key}.{method.name}"
    if cache_key not in _concrete_component_methods:
        methods = []
        for component in _sort_components_in_call_order(components):
            if _COMPONENT_METHODS.get((method, component), None) is not None:
                methods.append(getattr(component, method.inner))
        _concrete_component_methods[cache_key] = methods

    return _concrete_component_methods[cache_key]


def _get_node_class(mnt: MNT):
    if len(_NODE_CLASS_BY_MNT) < len(MNT):
        m = import_module("bench.language")
        getattr(m, mnt)  # noqa check that the node class is defined
    return _NODE_CLASS_BY_MNT[mnt]


@typing.dataclass_transform()
def node_component(
    cls: Optional[typing.Type] = None,
    mnt: MNT = None,
    passthrough: tuple[tuple[str, "Passthrough"]] = (),
    dynamic_components: tuple[type["ModuleNode"]] = (),
):
    """
    Mark a class as a node component (or concrete node for a MNT).
    """

    def decorate(cls):
        properties: dict[str, NodeProperty] = {}
        static_components: list[type["ModuleNode"]] = [cls]

        # check that no forbidden methods are defined
        CORE_TYPES = ("ModuleNode", "ScopeNode")
        if cls.__name__ not in CORE_TYPES:
            for name in _FORBIDDEN_NODE_METHODS:
                meth = getattr(cls, name, None)
                good_meth = getattr(ModuleNode, name, getattr(ScopeNode, name, None))
                if meth is not None and meth is not good_meth:
                    raise ValueError(f"forbidden method {name} defined in {cls}")

        # collect static components from class hierarchy
        for base in cls.__bases__:
            if base.__name__ in ("ModuleNode", "ABC"):
                continue
            if hasattr(base, "__properties__"):
                static_components.append(base)
                for gp in base.__static_components__:
                    if gp.__name__ not in CORE_TYPES and gp not in static_components:
                        static_components.append(gp)
        if cls.__name__ != "ModuleNode":
            static_components.append(ModuleNode)

        # collect properties from this
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
            prop.component = cls
            properties[name] = prop

        # collect properties from all components (static and dynamic, least to most specific)
        cls.__properties__ = {**properties}  # copy own properties
        for component in chain(reversed(static_components), reversed(dynamic_components)):
            for name, prop in component.__properties__.items():
                if name not in properties or name == "parent":  # override parent with more specific
                    # register all static and any non-runtime dynamic properties
                    if not prop.is_runtime or component not in dynamic_components:
                        properties[name] = prop
                elif not prop.equals_type(properties[name]):
                    raise ValueError(f"property conflict '{name}': {prop!r}, {properties[name]!r}")

        # collect methods implemented in this class (specifically)
        for meth_type in NodeMethod:
            meth = getattr(cls, meth_type.inner, None)
            if meth is not None and not any(
                meth is getattr(base, meth_type.inner, None) for base in cls.__bases__
            ):
                _COMPONENT_METHODS[(meth_type, cls)] = meth

        # create class (map to dataclass)
        for name, prop in properties.items():
            if not hasattr(cls, name):  # may be inherited
                continue
            if prop.ancestor_mnt:
                setattr(cls, name, _node_ancestor_prop(prop))
                if name in cls.__annotations__:  # computed property doesn't need a dataclass field
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
        cls.__static_components__ = tuple(static_components)
        cls.__dynamic_components__ = tuple(dynamic_components or ())
        cls.__static_passthrough__ = passthrough

        # register properties
        props = properties.values()
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
        f = frozendict
        cls.__properties__ = f(properties)
        cls.__list_properties__ = f(list_properties)
        cls.__list_properties_by_child__ = f(list_properties_by_child)
        cls.__tracked_properties__ = f({p.name: p for p in props if not p.is_internal})
        cls.__ancestor_properties__ = f({p.name: p for p in props if p.ancestor_mnt})
        cls.__internal_properties__ = f({p.name: p for p in props if p.is_internal})

        # register as concrete node class for mnt
        if mnt:
            cls.mnt = mnt
            if mnt in _NODE_CLASS_BY_MNT:
                raise ValueError(f"node class conflict for {mnt}: {cls}, {_NODE_CLASS_BY_MNT[mnt]}")
            _NODE_CLASS_BY_MNT[mnt] = cls
        _COMPONENT_CLASS_BY_NAME[cls.__name__] = cls

        return cls

    if cls is not None:
        return decorate(cls)

    return decorate


def node(
    mnt: MNT,
    passthrough: tuple[tuple[str, "Passthrough"]] = (),
    dynamic_components: tuple[type["ModuleNode"]] = (),
):
    def decorate(cls):
        return node_component(
            cls, mnt=mnt, passthrough=passthrough, dynamic_components=dynamic_components
        )

    return decorate


NodeT = typing.TypeVar("NodeT", bound="ModuleNode")


def _node_ancestor_prop(prop: NodeProperty) -> property:
    """Computed ancestor property for ModuleNode instances."""

    def get(self: NodeT) -> Optional[NodeT]:
        parent = self  # include self in search
        while parent is not None:
            if parent.mnt == prop.ancestor_mnt:
                return parent
            parent = parent.parent
        return None

    def set(self: NodeT, value: NodeT):
        raise NotImplementedError(f"cannot set computed ancestor property {prop}")

    return property(get, set)


def _sort_nested_ordered_list(root_ck: UUID, nodes: list[NodeT]) -> list[NodeT]:
    """
    Sort a list of ordered, hierarchical nodes.
    Each node is ordered within its 'parent' (by 'order_key'). Start at the root.
    """
    ordered = []

    nodes_by_parent_ck: dict[UUID, list[NodeT]] = defaultdict(list)
    for node in nodes:
        nodes_by_parent_ck[node.parent.ck].append(node)

    def _walk_dfs(parent_ck: UUID):
        children = nodes_by_parent_ck.get(parent_ck, None)
        if children:
            children.sort(key=lambda n: n.order_key or BIGGEST_INTEGER)
            for child in children:
                ordered.append(child)
                _walk_dfs(child.ck)

    _walk_dfs(root_ck)

    if len(ordered) != len(nodes):
        missing_nodes = [n for n in nodes if n not in ordered]
        assert not missing_nodes, f"missing {len(missing_nodes)} nodes {missing_nodes} in {ordered}"
    return ordered


class NodeListBase(abc.ABC, Collection, typing.Generic[NodeT]):
    """
    Base node list for custom implementation (right now just for database).
    """

    def __init__(self, parent: "ScopeNode", property: NodeProperty):
        self._parent = parent
        self._property = property

    def __repr__(self):
        return f"<{self.__class__.__name__} {self._parent.path}->{self._property.name}: {self}>"

    def _update(self, scope: "ScopeNode"):
        """Recomputes the list from the given scope."""
        raise NotImplementedError

    def create(self, *args, **kwargs):
        """Creates a new node in the list."""
        node_cls = _NODE_CLASS_BY_MNT[self._property.child_mnt]
        if hasattr(node_cls, "new"):
            node = node_cls.new(*args, **kwargs, for_parent=self._parent)
        else:
            node = node_cls(*args, **kwargs)
        self.append(node)
        return node

    def append(self, node: NodeT, _create: bool = True, _trigger: bool = True) -> None:
        """
        Attaches a child node to a parent through a list. This is for users adding nodes.
        A node may be 'append'-ed to a list at most once,
         but may exist in multiple lists (through _init_from collection).
        """
        raise NotImplementedError

    def extend(self, nodes: Collection[NodeT], _create: bool = True, _trigger: bool = True):
        """Attaches a list of child nodes to a parent. See append."""
        raise NotImplementedError

    def remove(self, node: NodeT, _delete: bool = True, _trigger: bool = True):
        """Removes a child node from a parent. See append for reverse."""
        raise NotImplementedError

    def clear(self, _delete: bool = True, _trigger: bool = True):
        """Removes all child nodes from a parent. See append for reverse."""
        raise NotImplementedError

    def set(self, nodes: Collection[NodeT]):
        """Replaces all child nodes of a parent."""
        self.clear(_trigger=False)
        self.extend(nodes)

    def get(self, some_id: str) -> Optional[NodeT]:
        """Gets a node by some id (as determined by the logic of the list)."""
        raise NotImplementedError


class NodeList(NodeListBase[NodeT]):
    """
    A list of node descendants for a parent's property.
    This is the primary way of adding, removing and accessing inline node relations.
    """

    def __init__(self, parent: "ScopeNode", property: NodeProperty):
        super().__init__(parent, property)
        self._child_t: type[NodeT] | None = None
        self._flags = property.children_flags
        self._nodes: list[NodeT] = []

    if DEBUG:
        # for debugger inspection
        nodes = property(lambda self: self._nodes)

    def __str__(self):
        return str(self._nodes)

    def _scope(self) -> dict[str, "ModuleNode"]:
        """Gets the visible scope for error reporting"""
        if self._flags & NRel.Named:
            return {n.py_ident: n for n in self._nodes}
        return {}

    def _update(self, scope: "ScopeNode"):
        if self._child_t is None:
            # late bind to avoid circular import when getting node class
            self._child_t = _get_node_class(self._property.child_mnt)

        # _children is effectively a computed property which is replaced wholesale,
        # we don't do diff updates to keep it simple with all the relation types.
        if self._flags & NRel.Cumulative:
            # all matching children of parent's descendants
            #  e.g. Module->Issue, File->Issue, ... -> all issues
            self._nodes = scope._local_root_tree.get_descendants(
                scope.ck, self._child_t, recursive=True, prefilter=False
            )
            assert not self._flags & NRel.Ordered, f"cannot order cumulative {self}"
        elif self._flags & NRel.Flat:
            # all matching descendants of matching children of parent
            #  e.g. Module->File, File->File, ... -> all files
            self._nodes = scope._local_root_tree.get_descendants(
                scope.ck, self._child_t, recursive=True, prefilter=True
            )
            if self._flags & NRel.Ordered:
                self._nodes = _sort_nested_ordered_list(self._parent.ck, self._nodes)
        else:
            # only matching children of parent
            self._nodes = scope._local_root_tree.get_descendants(
                scope.ck, self._child_t, recursive=False
            )
            if self._flags & NRel.Ordered:
                self._nodes.sort(key=lambda n: n.order_key or BIGGEST_INTEGER)

    def append(self, _node: NodeT, _create: bool = True, _trigger: bool = True) -> None:
        if _node.parent is not None:  # maybe copy?
            raise ValueError(f"cannot append {_node!r} to {self}!r: has parent {_node.parent!r}")

        # assign ids if newly attached to the module (ids are derived from ck + module)
        if not _node.attached and self._parent.attached:
            module_id = self._parent.module.id
            for n in _node._walk_rec():
                if n.id is None:
                    n._assign_id(module_id)
        # update parent after updating ids (need to walk in the node's tree, which may differ)
        _node.parent = self._parent

        # index node into parent scope (either subsume if previously detached or just add)
        if _node.attached and isinstance(_node, ScopeNode) and _node._local_tree is not None:
            self._parent._import_scope_tree(_node)
            _node._local_tree = None
        else:
            self._parent._local_root_tree.add(_node)
        # register node scope
        if self._flags & NRel.Scoped and _node.name is not None:
            self._parent._add_node_to_scope(_node)

        # assign order key to ordered nodes
        if self._flags & NRel.Ordered and _node.order_key is None:
            if self._flags & NRel.Flat:  # add after last root node
                last_ok = next(
                    (n.order_key for n in reversed(self._nodes) if n.parent == self._parent), None
                )
            else:
                last_ok = self._nodes[-1].order_key if self._nodes else None
            _node.order_key = generate_key_between(last_ok, None)
        if _trigger:
            # and update every affect node & list
            self._parent._trigger_update([_node])
            assert _node in self._nodes, f"node {_node} not in {self!r}"

        # activate node in session
        if self._parent._status == NS.Tracked and _node._status != NS.Tracked:
            _node._activate_self(self._parent._session)
        # 'create' node in session if it's attached
        if _node.attached and _create and self._parent._session:
            self._parent._session.tracer.node_create(_node)

    def extend(self, nodes: Collection[NodeT], _create: bool = True, _trigger: bool = True):
        nodes = list(nodes) if not isinstance(nodes, list) else nodes
        if nodes:
            for node in nodes:
                self.append(node, _trigger=False)
            self._parent._trigger_update(nodes)
            assert all(n in self._nodes for n in nodes), f"nodes {nodes} not in {self!r}"

    def remove(self, _node: NodeT, _delete: bool = True, _trigger: bool = True):
        if _delete and self._parent._session:
            self._parent.session.tracer.node_delete(_node)
        self._parent._local_root_tree.remove(_node)

        if _trigger:
            self._parent._trigger_update([_node])

    def clear(self, _delete: bool = True, _trigger: bool = True):
        if self._nodes:
            removed = list(self._nodes)
            for _node in removed:
                self.remove(_node, _delete=_delete, _trigger=False)
            self._parent._trigger_update(removed)

    def get(self, some_id: str) -> Optional[NodeT]:
        if not (self._flags & NRel.Keyed) and not (self._flags & NRel.Named):
            raise ValueError(f"cannot get {some_id!r} from {self!r}")
        for child in self._nodes:
            if (self._flags & NRel.Keyed and child.key == some_id) or (
                self._flags & NRel.Named and (child.name == some_id or child.py_ident == some_id)
            ):
                return child
        return None

    def __bool__(self):
        return bool(self._nodes)

    def __contains__(self, obj: object) -> bool:
        # special case to unwrap key (e.g. for tagging/tag objects)
        if self._flags & NRel.Keyed and hasattr(obj, "key"):
            obj = obj.key
        if isinstance(obj, str) and (self._flags & NRel.Keyed or self._flags & NRel.Named):
            return self.get(obj) is not None
        elif isinstance(obj, ModuleNode):
            if obj.mnt != self._property.child_mnt:
                raise TypeError(f"{self!r} cannot contain {obj!r}")
            return obj in self._nodes
        else:
            return False

    def __getitem__(self, item: int | slice | str) -> NodeT | list[NodeT]:
        if isinstance(item, int):
            return self._nodes[item]
        elif isinstance(item, slice):
            return self._nodes[item]
        elif isinstance(item, str):
            return self.get(item)
        else:
            raise TypeError(f"invalid index for {self!r}: {item} ({type(item)})")

    def __getattr__(self, item):
        if item.startswith("_"):
            return super().__getattr__(item)
        node = self.get(item)
        if node is None:
            raise AttributeError(f"no node '{item}' in {self!r}")
        return node

    def __iter__(self) -> Iterator[NodeT]:
        yield from self._nodes

    def __len__(self) -> int:
        return len(self._nodes)

    def __eq__(self, other: object) -> bool:
        if isinstance(other, NodeList):
            return self._nodes == other._nodes
        elif isinstance(other, list):
            return self._nodes == other
        else:
            return False


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

    @property
    def nodes(self) -> Collection[NT]:
        return self.nodes_by_ck.values()

    def deepcopy(self):
        nodes = [deepcopy(node) for node in self.nodes]
        return NodeTree(nodes)

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

    def set(self, nodes: Collection[NT]):
        """Replaces all nodes in the tree"""
        self.clear()
        for node in nodes:
            self.add(node)

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

    def __getitem__(self, item):
        return self.get(item)

    def __contains__(self, item):
        return item in self.nodes_by_id or item in self.nodes_by_ck

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

    @property
    def nodes(self) -> Collection[NT]:
        return self.nodes_by_ck.values()

    def __getitem__(self, item):
        return self.nodes_by_ck.get(item)

    def __contains__(self, item):
        return item in self.nodes_by_ck

    def clear(self):
        """Clear the tree"""
        self.nodes_by_ck.clear()
        self.node_ck_by_parent_ck.clear()

    def add(self, node: "ModuleNode"):
        """Add a node to the tree (error if node already exists)"""
        if node.ck in self.nodes_by_ck and self.nodes_by_ck[node.ck] is not node:
            raise ValueError(f"node {node!r} (ck={node.ck}) already exists in {self!r}")
        self.nodes_by_ck[node.ck] = node
        if node.parent is not None:
            self.node_ck_by_parent_ck[node.parent.ck].append(node)

    def set(self, nodes: Collection[NT]):
        """Replaces all nodes in the tree"""
        self.clear()
        for node in nodes:
            self.add(node)

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
    """Creates method that calls _method_inner for all components in call order"""

    @functools.wraps(wraps)
    def self_method(self: "ModuleNode", *args, _coerce: bool = True, **kwargs):
        if from_status is not None and self._status != from_status:
            # auto coerce the node into the desired to_status if allowed and feasible
            if _coerce and self._status <= to_status:  # automatically index if needed
                if self._status == NS.Source and to_status >= NS.Indexed:
                    self._index_self()
                if self._status == NS.Indexed and to_status > NS.Interpreted:
                    self._interp_self(self)
                if self._status < from_status:
                    raise RuntimeError(
                        f"cannot coerce {method.name} {self!r} (status={self._status.name})"
                    )
                if self._status == to_status:
                    return  # nothing to do
            else:
                raise RuntimeError(f"cannot {method.name} {self!r} (status={self._status.name})")

        for meth in _get_component_methods(self._components, method, self._concrete_cache_key):
            meth(self, *args, **kwargs)
        if to_status is not None:
            self._status = to_status

    self_method.__name__ = method.self
    return self_method


def _make_inner_dunder_method(method: NodeMethod):
    """Creates method that proxies a builtin dunder method to the first _method_inner"""

    def inner_method(self: "ModuleNode", *args, **kwargs):
        meths = _get_component_methods(self._components, method, self._concrete_cache_key)
        if len(meths) < 2:  # includes this one
            raise RuntimeError(f"{self!r} does not support {method.name}")
        return meths[1](self, *args, **kwargs)

    inner_method.__name__ = method.inner
    return inner_method


class Passthrough(enum.StrEnum):
    Full = "full"
    Scope = "scope"


@node_component
class ModuleNode(abc.ABC):
    """
    A node in a Bench module tree.
    A node has a per-version unique id (id) and a constant identifier key (ck).
    """

    mnt: ClassVar[MNT]  # set in @node decorator
    __static_components__: ClassVar[tuple[type["ModuleNode"]]] = []
    __dynamic_components__: ClassVar[tuple[type["ModuleNode"]]] = ()
    __properties__: ClassVar[dict[str, NodeProperty]] = {}
    __ancestor_properties__: ClassVar[dict[str, NodeProperty]] = {}
    __list_properties__: ClassVar[dict[str, NodeProperty]] = {}
    __list_properties_by_child__: ClassVar[dict[MNT, list[NodeProperty]]] = defaultdict(list)
    __tracked_properties__: ClassVar[dict[str, NodeProperty]] = {}
    __internal_properties__: ClassVar[dict[str, NodeProperty]] = {}
    __static_passthrough__: ClassVar[tuple[tuple[str, Passthrough]]] = ()

    id: UUID = ninternal(default=None)
    ck: UUID = ninternal(default_factory=uuid.uuid4)
    parent: Optional["ModuleNode"] = nparent()
    # prototype: Optional["ModuleNode"] / instance_of_ck: UUID
    module: Optional["Module"] = nancestor(MNT.Module)

    created_at: datetime = ninternal(default_factory=utcnow_with_tz, is_cru=True)
    updated_at: datetime = ninternal(default_factory=utcnow_with_tz, is_cru=True)
    last_edited_at: datetime = ninternal(default_factory=utcnow_with_tz, is_cru=True)
    last_changed_at: datetime = ninternal(default_factory=utcnow_with_tz, is_cru=True)
    revision: int = ninternal(default=0, is_cru=True)

    _session: Optional["Session"] = nruntime(default=None)
    _status: NodeStatus = nruntime(default=None)

    def __post_init__(self):
        if self._session is None:
            from bench.language.builtin import active_session

            self._session = active_session.get()
        if self._status is None:
            self._status = NS.Interpreted if self._session is not None else NS.Source
        if self.id is None and self.attached:
            self._assign_id(self.module.id)
        self._init_self()

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.parent.id if self.parent is not None else None

    @property
    def _components(self) -> tuple[type["ModuleNode"]]:
        return self.__static_components__

    @property
    def _dynamic_components(self) -> tuple[type["ModuleNode"]]:
        return ()

    @property
    def _concrete_cache_key(self) -> str:
        """Identifier for dynamic components"""
        return type(self).__name__

    @property
    def _passthrough_targets(self) -> tuple[tuple[str, Passthrough]] | None:
        """Pass through __getattr__/__setattr__ properties (before defaulting to usual)"""
        return self.__static_passthrough__

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

    def _trigger_update(self, changed: list["ModuleNode"]):
        """Trigger list updates and re-interps in all relevant nodes."""
        assert changed, f"cannot trigger update on {self} with no changed nodes"
        # reinit children for any affected parent nodes
        # TODO @Performance: use mark dirty in node list to avoid reinit
        parent = self
        affected_mnts = set([n.mnt for n in changed])
        while parent is not None:
            for prop in parent.__list_properties__.values():
                if prop.child_mnt in affected_mnts:
                    getattr(parent, prop.name)._update(parent)
            parent = parent.parent
        # nocheckin: also reinterp if active in session

    def _set_untracked(self, key, value):
        self.__dict__[key] = value

    def __setattr__(self, key, value):
        if self._status != NS.Tracked:
            return super().__setattr__(key, value)

        # tracked set
        if key in self.__internal_properties__:
            if key in self.__list_properties__:
                raise AttributeError(f"cannot set {self.__properties__[key]} (use NodeList)")
            else:
                return super().__setattr__(key, value)
        elif key in self.__tracked_properties__:
            prev = getattr(self, key)
            super().__setattr__(key, value)
            try:
                self._validate_self([key], on_issue=on_issue_raise)
            except ValidationError as e:  # reset on error
                super().__setattr__(key, prev)
                raise e
            self._session.tracer.node_update(self, [key])
            return
        elif key in self.__dict__:
            return super().__setattr__(key, value)

        # try first full passthrough target (if any)
        for target, mode in self._passthrough_targets:
            target = getattr(self, target)
            if mode == Passthrough.Full:
                setattr(target, key, value)
                return  # success

        # report set error with additional info
        candidates = {
            **(self.__tracked_properties__ if self._status == NS.Tracked else self.__properties__),
            **{s.name: s for s in self._scopes_by_name.values()},
        }
        did_you_mean = did_you_mean_str(candidates, key)
        raise AttributeError(f"Cannot set '{key}' on {self!r}. {did_you_mean}")

    def __getattr__(self, item):
        if item in self.__dict__:  # 'native' property or method
            return super().__getattribute__(item)

        attr = UNSET
        # prefer dynamic components own methods
        for component in self._dynamic_components:
            attr = getattr(component, item, UNSET)
            if attr is not UNSET:
                break
        # check passthrough targets if tracked in session
        if attr is UNSET and self._status == NS.Tracked:
            for target, mode in self._passthrough_targets:
                target = getattr(self, target)
                if mode == Passthrough.Full:
                    attr = getattr(target, item, UNSET)
                elif mode == Passthrough.Scope:
                    assert isinstance(target, NodeList), f"invalid scope passthrough: {attr!r}"
                    attr = target.get(item) or UNSET
                if attr is not UNSET:
                    break

        # attribute be property, method, or just plain value
        if attr is not UNSET:
            if isinstance(attr, property):
                return attr.fget(self)
            elif not isinstance(attr, ModuleNode) and callable(attr) and not inspect.ismethod(attr):
                return functools.partial(attr, self)
            else:
                return attr

        # report lookup error with additional info
        candidates = {**self.__properties__}
        if isinstance(self, ScopeNode):
            candidates.update(self._scopes_by_name)
        did_you_mean = did_you_mean_str(candidates, item)
        raise AttributeError(f"{self!r} has no attribute '{item}'. {did_you_mean}")

    # abstract :ComponentMethods

    def _init_inner(self) -> None:
        """Initialize this node."""
        for name, prop in self.__list_properties__.items():
            setattr(self, name, prop.list_type(self, prop))

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
        """Validate cross-property constraints given the modified properties."""
        # since this is the root module, we also validate the properties directly
        for name in properties:
            prop = self.__properties__.get(name)
            assert prop is not None, f"unknown property '{name}' on {self!r}"
            value = getattr(self, name)
            if value is None:
                if prop.is_required:
                    on_issue(self, f"{prop.name}: is required")
            elif prop.custom_validate is not None:
                handler = PropertyValidationHandler(self, prop, on_issue)
                valid = prop.validate(value, handler)
                if valid is False:
                    on_issue(self, f"{prop.name}: invalid value")

    def _visit_inner(self, visitor: "NodeVisitor") -> None:
        """Visit any referenced nodes."""
        for prop in self.__list_properties__.values():
            if prop.children_flags & NRel.Remote:
                continue
            for child in getattr(self, prop.name):
                visitor.visit_child(child)

    def _activate_inner(self, session: "Session") -> None:
        """'Instantiate' this object in the given session."""
        self._session = session
        self._status = NS.Tracked

    def _deactivate_inner(self) -> None:
        """'Deinstantiate' this object."""
        self._session = None

    _call_inner = _make_inner_dunder_method(NodeMethod.call)
    _iter_inner = _make_inner_dunder_method(NodeMethod.iter)
    _aiter_inner = _make_inner_dunder_method(NodeMethod.aiter)
    _len_inner = _make_inner_dunder_method(NodeMethod.len)

    __call__ = _call_inner
    __iter__ = _iter_inner
    __aiter__ = _aiter_inner
    __len__ = _len_inner

    def __bool__(self):
        return True  # allow truthy checks for nodes

    # final :ComponentMethods

    _init_self = _make_self_method(NodeMethod.init, _init_inner)
    _clear_self = _make_self_method(NodeMethod.clear, _clear_inner, to_status=NS.Source)
    _index_self = _make_self_method(
        NodeMethod.index, _index_inner, from_status=NS.Source, to_status=NS.Indexed
    )
    _interp_self = _make_self_method(
        NodeMethod.interp,
        _interp_inner,
        from_status=NS.Indexed,
        to_status=NS.Interpreted,
    )
    _visit_self = _make_self_method(NodeMethod.visit, _visit_inner)
    _validate_self = _make_self_method(NodeMethod.validate, _validate_inner)
    _activate_self = _make_self_method(
        NodeMethod.activate, _activate_inner, from_status=NS.Interpreted, to_status=NS.Tracked
    )
    _deactivate_self = _make_self_method(
        NodeMethod.deactivate, _deactivate_inner, from_status=NS.Tracked, to_status=NS.Interpreted
    )

    def _copy_self(self, keep_parent: bool = False, reset_id: bool = True) -> "ModuleNode":
        """
        Copies this node without any descendants.
        All non-relational properties are copied using NodeProperty.copy, relations are reset.
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

    def _walk_rec(self) -> Collection["ModuleNode"]:
        """
        Walks this node and all descendants in breadth-first order.
        """
        return [self]

    def _on_issue(
        self,
        *,
        subject: Optional["ModuleNode"] = None,
        type: IssueType = None,
        message: str = None,
        **kwargs,
    ):
        self.parent._on_issue(subject=self, type=type, message=message, **kwargs)

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
            raise RuntimeError(f"no active session for {self!r}")
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
    def rec_method(self: "ScopeNode", *args, **kwargs):
        descendants = self._local_root_tree.get_descendants(self.ck, recursive=True)
        method_name = method.self
        if pass_scope:
            for node in descendants:
                scope = node if isinstance(node, ScopeNode) else node.parent
                getattr(node, method_name)(scope)
            getattr(self, method_name)(self, *args, **kwargs)
        else:
            for node in descendants:
                getattr(node, method_name)(*args, **kwargs)
            getattr(self, method_name)(*args, **kwargs)

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
    _activate_rec = _make_rec_method(NodeMethod.activate, ModuleNode._activate_self)
    _deactivate_rec = _make_rec_method(NodeMethod.deactivate, ModuleNode._deactivate_self)

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
            if prop.children_flags & NRel.Scoped:
                for child in getattr(self, prop.name):
                    if child.name is not None:
                        self._add_node_to_scope(child)

    def _walk_rec(self) -> Collection["ModuleNode"]:
        return self._local_root_tree.get_descendants(self.ck, recursive=True, include_self=True)

    def _add_node_to_scope(self, node: ModuleNode) -> None:
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

    def _import_scope_tree(self, scope: "ScopeNode") -> None:
        """Adds the given tree into this scope."""
        assert scope._local_tree is not None, f"no local tree to import {scope!r} into {self!r}"
        self._local_root_tree.add_tree(scope._local_tree)

    @property
    def _local_root_scope(self) -> "ScopeNode":
        if self.parent is None:
            return self
        return self.parent._local_root_scope

    @property
    def _local_root_tree(self) -> Union["NodeTree", "DetachedNodeTree"]:
        tree = self._local_root_scope._local_tree
        assert tree is not None, f"no local tree for {self!r} in {self._local_root_scope!r}"
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

    def resolve(
        self,
        path: Union["NodePath", UUID, str],
        by: Optional[LookupBy] = None,
        node_t: typing.Type[NodeT] | None = None,
    ) -> NodeT:
        result = self.lookup(path, by=by, node_t=node_t)
        if result is None:
            raise LookupError(f"{path} not found in {self!r}")
        return result

    def _on_issue(
        self,
        *,
        subject: Optional["ModuleNode"] = None,
        type: IssueType = None,
        message: str = None,
        **kwargs,
    ):
        from bench.language import File, Statement
        from bench.language.issue import Issue

        issue_ck = uuid.uuid5(subject.id, (type.value + (message or "")))
        issue_id = issue_ck  # not sure?
        if not isinstance(subject, (Statement, File)):
            subject = subject.parent  # fields don't have issues (yet)
        issue = Issue(id=issue_id, ck=issue_ck, type=type, parent=None, **kwargs)
        if issue not in subject.issues:  # dedup
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


@dataclass
class ModuleChange:
    source_mutations: list["ModuleMutation"]  # incoming external mutations
    interp_mutations: list["ModuleMutation"]  # resulting interp state change
    added: list[ModuleNode]
    updated: list[ModuleNode]
    removed: list[ModuleNode]

    @property
    def touched(self) -> typing.Iterable[ModuleNode]:
        return chain(self.added, self.updated, self.removed)


@node(mnt=MNT.Module, passthrough=(("files", Passthrough.Scope),))
class Module(ScopeNode):
    parent: None = nparent()
    name: str = ninternal()  # can't change this yet
    committed: bool = ninternal(default=False)
    files: NodeList["File"] = nchildren(MNT.File, NRel.Flat | NRel.Named | NRel.Scoped)
    dependencies: dict[str, Union["Module", ModuleReference]] = nruntime(default_factory=dict)
    builtins: list["File"] = nruntime(default_factory=list)
    _lookup_cache: dict[str, NodeT] = nruntime(default_factory=dict)
    _source: Optional[NodeTree] = nruntime(default=None)
    _project_id: Optional[UUID] = nruntime(default=None)

    def __str__(self):
        if self.issues:
            issue_strs = []
            for k in (IssueKind.Error, IssueKind.Warning, IssueKind.Notice):
                issues_of_kind = [i for i in self.issues if i.kind == k]
                if issues_of_kind:
                    issue_strs.append(f"{len(issues_of_kind)} {k.name.lower()}s")
            issues_str = f", {', '.join(issue_strs)}"
        else:
            issues_str = ""
        return f"{self.name} ({len(self.files)} files{issues_str})"

    def __repr__(self):
        return f"<Module {str(self)}>"

    @property
    def _tree(self) -> NodeTree:
        return self._local_tree

    @property
    def _nodes(self) -> Collection[NT]:
        return self._tree.nodes_by_ck.values()

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
        if not any(dep == file.module for dep in self.dependencies.values()):
            raise ValueError(f"cannot add builtin {file!r} to {self!r} without {file.module!r}")
        self.builtins.append(file)

    def add_dependency(self, dependency: Union["Module", ModuleReference]) -> None:
        if dependency.name in self.dependencies:
            raise ValueError(
                f"{self} has dependency {dependency.name}: {self.dependencies[dependency.name]}"
            )
        self.dependencies[dependency.name] = dependency

    def lookup(
        self,
        path: Union["NodePath", UUID, str],
        by: Optional[LookupBy] = None,
        node_t: MNT | typing.Type[NodeT] | None = None,
    ) -> NodeT | None:
        if path in self._lookup_cache:
            return self._lookup_cache[path]

        # extended lookup with dependencies, defaults to regular scope lookup
        if isinstance(path, UUID):
            resolved = self._local_tree.get(path)
            if resolved is None:
                for dependency in self.dependencies.values():
                    if path in dependency._tree:
                        resolved = dependency._tree[path]
                        break
        elif isinstance(path, str) and path.startswith("."):
            resolved = ScopeNode.lookup(self, path, node_t=node_t, by=by)
        else:
            module_name, localized_path = parse_absolute_node_reference(path)
            if module_name == self.name:
                dependency = self
            else:
                dependency = self.dependencies.get(module_name)
            if dependency is None:
                resolved = None
            else:
                resolved = dependency.lookup(localized_path, node_t=node_t, by=by)

        # cache result
        if self.committed:
            self._lookup_cache[path] = resolved
        return resolved

    def _activate_inner(self, session: "Session"):
        for dependency in self.dependencies.values():
            if dependency._status != NS.Tracked:
                # multiple modules can depend on the same module, only activate once
                dependency._activate_rec(session)

    def _deactivate_inner(self) -> None:
        for dependency in self.dependencies.values():
            if dependency._status == NS.Tracked:  # see above
                dependency._deactivate_rec()

    def _index_inner(self):
        for builtin in self.builtins:
            self._add_node_to_scope(builtin)

    def _apply_mutations(self, mutations: list["ModuleMutation"]) -> ModuleChange:
        """
        Applies the given external mutations to the module.
        Any changed nodes are returned.
        TODO @Performance @UX: :HotReload patch mutations locally
        """
        assert self._source is not None, f"cannot apply mutations to {self!r} without source"

        # update source
        old_nodes_by_ck: dict[UUID, ModuleNode] = {**self.module._tree.nodes_by_ck}
        self._apply_source_mutations(mutations)

        # update self (this is obviously inefficient, but performs surprisingly okay)
        self._reset_from_source()

        # compute change, apply interp source changes if any
        change = self._compute_change(mutations, old_nodes_by_ck)
        if change.interp_mutations:
            self._apply_source_mutations(change.interp_mutations)
        return change

    def _reset_from_source(self):
        """Resets the module completely from the source."""
        from bench.language.wire import unpack_node

        prev_session = self.module._session
        if prev_session:
            self.module._deactivate_self()

        self.module._clear_rec()
        self.module._tree.clear()
        unpack_node(self._source, parent=self, session=None, exclude=INTERP_NODE_TYPES)
        self.module._interp_rec()

        if prev_session:
            self.module._activate_rec(prev_session)

    def _apply_source_mutations(self, mutations: list["ModuleMutation"]) -> None:
        """Applies the mutations directly to the source without any interp."""
        from bench.language.mutate import ModuleMutator

        mutator = ModuleMutator(self._source, self._project_id, self.id)
        # errors are fine here since e.g. a deleted issue's parent may have disappeared
        #  (we could filter that, but it's easier this way since it's more explicit for clients)
        mutator.apply_all(mutations, raise_on_error=False)

    def _compute_change(
        self, source_mutations: list["ModuleMutation"], old_nodes_by_ck: dict[UUID, ModuleNode]
    ) -> ModuleChange:
        """Computes the change between the old and new module state."""
        from bench.language.mutate import ModuleMutator

        new_nodes: dict[UUID, ModuleNode] = self.module._tree.nodes_by_ck
        added = []
        updated = []
        for n in new_nodes.values():
            if n.ck in old_nodes_by_ck:
                if n.revision != old_nodes_by_ck[n.ck].revision:
                    updated.append(n)
            else:
                added.append(n)
        removed = [n for n in old_nodes_by_ck.values() if n.ck not in new_nodes]

        # gather interp mutations
        mutator = ModuleMutator(self._source, self._project_id, self.id)
        for node in added:
            if node.mnt in INTERP_NODE_TYPES:
                mutator.create(node, apply=False)
        for node in removed:
            if node.mnt in INTERP_NODE_TYPES:
                mutator.delete(node, apply=False)

        return ModuleChange(
            source_mutations=source_mutations,
            interp_mutations=mutator.mutations,
            added=added,
            updated=updated,
            removed=removed,
        )

    @staticmethod
    def interp(source: list["NodeData"], project_id: UUID) -> "Module":
        """Create an interpreted Module from a source module node tree."""
        from bench.language import libs, wire

        module = wire.unpack_module(source, exclude=INTERP_NODE_TYPES, session=None)
        module._source = NodeTree(source)
        module._project_id = project_id
        old_nodes_by_ck = {**module._tree.nodes_by_ck}

        for dependency in libs.DEFAULT_MODULES.values():
            module.add_dependency(dependency)
        module.add_builtin(libs.symbolx_lib.files.get("builtins"))
        module._interp_rec()

        # update source with interp mutations (doesn't have them)
        change = module._compute_change([], old_nodes_by_ck)
        if change.interp_mutations:
            module._apply_source_mutations(change.interp_mutations)

        return module

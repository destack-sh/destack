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
    NodeTrackingLevel,
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
from bench.utils.fractional import BIGGEST_INTEGER, generate_key_between, generate_n_keys_between
from bench.utils.func import did_you_mean_str, nextn
from bench.utils.utils import (
    DEBUG,
    IdentifierType,
    flatten_list,
    frozendict,
    required_field,
    to_pyidentifier,
)

if TYPE_CHECKING:
    from bench.language import File, Issue, NodeVisitor, Session
    from bench.language.edit import EditData
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
    component: type["Node"] | None = None  # source component class
    annotation: typing.Any = None  # type annotation on LHS of assignment
    # config
    is_required: bool = False
    is_internal: bool = False
    is_runtime: bool = False
    is_cru: bool = False
    parent_mnts: tuple[MNT] | None = None
    ancestor_mnt: MNT | None = None
    default: typing.Any = UNSET
    default_factory: Callable[[], typing.Any] | None = None
    list_type: type["NodeListBase"] | None = None
    custom_copy: Callable[[typing.Any], typing.Any] | None = None
    custom_validate: Callable[[typing.Any, "PropertyValidationHandler"], bool | None] | None = None
    # for manual handling in dynamic nodes
    ignore_conflicts_with: tuple[type["Node"], ...] | None = None
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
        return f"{self.component.__name__}.{self.name}"

    def __repr__(self):
        non_default = []
        for k, v in self.__dict__.items():
            if v is UNSET or not v:
                continue
            elif k in ("custom_copy", "custom_validate"):
                func_str = f"{v.__name__}@{hex(id(v))}"
                non_default.append(f"{k}={func_str}")
            elif k == "children_flags":
                flags_str = ", ".join(f.name for f in NodeRelationType if v & f)
                if flags_str:
                    non_default.append(flags_str)
            elif k not in ("name", "annotation", "component", "ignore_conflicts_with"):
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
            if k.name in ("component", "ignore_conflicts_with"):
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
    is_required: bool = False,
    ignore_conflicts_with: tuple[type["Node"], ...] = None,
):
    """Standard user facing node property."""
    return NodeProperty(
        default=default,
        default_factory=default_factory,
        custom_copy=copy,
        custom_validate=validate,
        is_required=is_required,
        ignore_conflicts_with=ignore_conflicts_with,
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
    return NodeProperty(parent_mnts=tuple(mnt), default=None, is_internal=True)


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
_COMPONENT_CLASS_BY_NAME: dict[str, type["Node"]] = {}
_COMPONENT_METHODS: dict[[NodeMethod, type["Node"]], typing.Any] = {}
_COMPONENT_CALL_ORDER: list[str] = [
    "Node",
    "ScopeNode",
    "HasFields",  # for resolved_fields
    # the rest
]


@cached(cache={})
def _sort_components_in_call_order(
    components: list[type["Node"]],
) -> list[type["Node"]]:
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
    components: list[type["Node"]], method: NodeMethod, concrete_key: str
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
    passthrough: tuple[tuple[str, "_Passthrough"]] = (),
    dynamic_components: tuple[type["Node"], ...] = (),
):
    """
    Mark a class as a node component (or concrete node for a MNT).
    """

    def decorate(cls):
        properties: dict[str, NodeProperty] = {}
        static_components: list[type["Node"]] = [cls]

        # check that no forbidden methods are defined
        CORE_TYPES = ("Node", "ScopeNode")
        if cls.__name__ not in CORE_TYPES:
            for name in _FORBIDDEN_NODE_METHODS:
                meth = getattr(cls, name, None)
                good_meth = getattr(Node, name, getattr(ScopeNode, name, None))
                if meth is not None and meth is not good_meth:
                    raise ValueError(f"forbidden method {name} defined in {cls}")

        # collect static components from class hierarchy
        for base in cls.__bases__:
            if base.__name__ in ("Node", "ABC"):
                continue
            if hasattr(base, "__properties__"):
                static_components.append(base)
                for gp in base.__static_components__:
                    if gp.__name__ not in CORE_TYPES and gp not in static_components:
                        static_components.append(gp)
        if cls.__name__ != "Node":
            static_components.append(Node)

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
                existing = properties.get(name, None)
                if existing is None or name == "parent":  # override parent with more specific
                    # register all static and any non-runtime dynamic properties
                    if not prop.is_runtime or component not in dynamic_components:
                        properties[name] = prop
                elif not prop.equals_type(existing):
                    if existing.ignore_conflicts_with and any(
                        issubclass(component, c) for c in existing.ignore_conflicts_with
                    ):
                        continue
                    raise ValueError(f"property conflict '{name}': {prop!r}, {existing!r}")

        # collect methods implemented in this class (specifically)
        for meth_type in NodeMethod:
            meth = getattr(cls, meth_type.inner, None)
            if meth is not None and not any(
                meth is getattr(base, meth_type.inner, None) for base in cls.__bases__
            ):
                _COMPONENT_METHODS[(meth_type, cls)] = meth

        # create class (map to dataclass)
        for name, prop in properties.items():
            if not prop.child_mnt and not hasattr(cls, name):  # may be inherited
                continue
            if prop.ancestor_mnt:
                setattr(cls, name, _node_ancestor_prop(prop))
                if name in cls.__annotations__:  # computed property doesn't need a dataclass field
                    del cls.__annotations__[name]
            elif prop.child_mnt:
                setattr(cls, name, dataclasses.field(default=None))
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
    passthrough: tuple[tuple[str, "_Passthrough"]] = (),
    dynamic_components: tuple[type["Node"], ...] = (),
):
    def decorate(cls):
        return node_component(
            cls, mnt=mnt, passthrough=passthrough, dynamic_components=dynamic_components
        )

    return decorate


NodeT = typing.TypeVar("NodeT", bound="Node")


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


class _NodeChange(enum.IntFlag):
    """The kind of reactive change effect to trigger in a node."""

    Ignore = 0
    UpdateLists = 2**0
    Detach = 2**1
    Attach = 2**2
    Full = UpdateLists | Detach | Attach


_NC = _NodeChange


@dataclass
class _ChangeEffect:
    """
    The effect of a change in nodes.
    TODO @Performance: use mark dirty in to batch change effects
    """

    prev_session: Optional["Session"]
    prev_status: Optional[NS]
    level: _NC
    affected_mnts: set[MNT] | None
    ancestors: list["Node"] | None
    affected: list["Node"] | None

    @staticmethod
    def _collect(
        from_parent: Optional["Node"],
        to_parent: Optional["Node"],
        changed: list["Node"],
        level: _NC,
    ) -> "_ChangeEffect":
        """Collects nodes affected by a change in the given children."""
        assert changed, f"cannot create update on {to_parent!r} without changed nodes"
        assert from_parent or to_parent, f"cannot create update on {changed!r} without parent"

        # collect ancestors to update their affected node lists
        affected_mnts = set([n.mnt for n in changed])
        ancestors = []
        if level >= _NC.UpdateLists:
            parent = from_parent
            while parent is not None:
                ancestors.append(parent)
                parent = parent.parent
            parent = to_parent
            while parent is not None:
                ancestors.append(parent)
                parent = parent.parent

        # collect nodes to reinterp following attach/detach
        if level & (_NC.Detach | _NC.Attach):
            affected_nodes: list[Node] = ancestors[:]
            for child in changed:
                affected_nodes.append(child)
                if isinstance(child, ScopeNode):
                    affected_nodes.extend(
                        child._local_root_tree.get_descendants(  # :NodeViews
                            child.ck, recursive=True, include_self=False
                        )
                    )
            # filter out interp types
            affected_nodes = [n for n in affected_nodes if n.mnt not in INTERP_NODE_TYPES]
        else:
            affected_nodes = None

        return _ChangeEffect(
            prev_session=to_parent._session if to_parent else None,
            prev_status=to_parent._status if to_parent else None,
            affected_mnts=affected_mnts,
            ancestors=ancestors,
            affected=affected_nodes,
            level=level,
        )

    def _effect(self, level: _NC | None = None) -> None:
        """Applies the effect of a trigger to update the affected nodes."""
        if level & _NC.UpdateLists:
            for ancestor in self.ancestors:
                for prop in ancestor.__list_properties__.values():
                    if prop.child_mnt in self.affected_mnts:
                        getattr(ancestor, prop.name)._update(ancestor)

        if level & _NC.Detach:
            for _node in self.affected:
                if _node._session and _node._status == NS.Tracked:
                    _node._deactivate_self()
            for _node in self.affected:
                _node._clear_self(_node.scope)

        if level & _NC.Attach:
            for _node in self.affected:
                _node._index_self()
            for _node in self.affected:
                _node._interp_self(_node.scope)
                if self.prev_session and self.prev_status == NS.Tracked:
                    _node._activate_self(self.prev_session)


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

    def create(self, *args, _append: bool = True, **kwargs) -> NodeT:
        """Creates a new node in the list."""
        if len(args) == 1 and isinstance(args[0], Node):
            raise ValueError(f"cannot create {args[0]!r}, use append for existing nodes")
        node_cls = _NODE_CLASS_BY_MNT[self._property.child_mnt]
        # set new node status to source to prevent activation before it's appended
        if hasattr(node_cls, "new"):
            node = node_cls.new(*args, **kwargs, for_parent=self._parent, _status=NS.Source)
        else:
            node = node_cls(*args, **kwargs, _status=NS.Source)
        if _append:
            self.append(node)
        return node

    def create_many(self, *nodes: Collection[typing.Any | dict]) -> list[NodeT]:
        """Creates a new node in the list."""
        created = []
        for n in nodes:
            if isinstance(n, dict):
                node = self.create(**n, _append=False)
            elif isinstance(n, tuple):
                node = self.create(*n, _append=False)
            else:
                node = self.create(n, _append=False)
            created.append(node)
        self.extend(*created)
        return created

    def append(self, node: NodeT, _create: bool = True, _trigger: _NC = _NC.Full) -> None:
        """
        Attaches a child node to a parent through a list. This is for users adding nodes.
        A node may be 'append'-ed to a list at most once,
         but may exist in multiple lists (through _init_from collection).
        """
        raise NotImplementedError

    def extend(
        self,
        *nodes: Collection[NodeT],
        _create: bool = True,
        _trigger: _NC = _NC.Full,
    ):
        """Attaches a list of child nodes to a parent. See append."""
        raise NotImplementedError

    def remove(self, node: NodeT, _delete: bool = True, _trigger: _NC = _NC.Full):
        """Removes a child node from a parent. See append for reverse."""
        raise NotImplementedError

    def clear(self, _delete: bool = True, _trigger: _NC = _NC.Full):
        """Removes all child nodes from a parent. See append for reverse."""
        raise NotImplementedError

    def set(self, nodes: Collection[NodeT], _trigger: _NC = _NC.Full):
        """Replaces all child nodes of a parent."""
        self.clear(_trigger=_NC.Ignore)
        self.extend(*nodes, _trigger=_trigger)

    def get(self, some_id: str) -> Optional[NodeT]:
        """Gets a node by some id (as determined by the logic of the list)."""
        raise NotImplementedError

    def index(self, node: NodeT) -> int:
        """Gets the index of a node in the list."""
        raise NotImplementedError


class NodeList(NodeListBase[NodeT]):
    """
    A list of node descendants for a parent's property.
    This is the primary way of adding, removing and accessing inline node relations.
    """

    def __init__(self, parent: "ScopeNode", property: NodeProperty):
        super().__init__(parent, property)
        self._child_mnt: MNT = property.child_mnt
        self._flags = property.children_flags
        self._nodes: list[NodeT] = []

    if DEBUG:
        # for debugger inspection
        nodes = property(lambda self: self._nodes)

    def __str__(self):
        return str(self._nodes)

    def _scope(self) -> dict[str, "Node"]:
        """Gets the visible scope for error reporting"""
        if self._flags & NRel.Named:
            return {n.py_ident: n for n in self._nodes}
        return {}

    def _ok_bounds(
        self, after: NodeT = None, before: NodeT = None
    ) -> tuple[Optional[str], Optional[str]]:
        """Gets the order key bounds after the given (default to last)."""
        assert self._flags & NRel.Ordered, f"cannot get order key for {self!r}"
        if after is not None:
            next_ok = nextn(
                n.order_key
                for n in self._nodes
                if n.order_key > after.order_key and n.parent == after.parent
            )
            return after.order_key, next_ok
        elif before is not None:
            last_ok = nextn(
                n.order_key
                for n in reversed(self._nodes)
                if n.order_key < before.order_key and n.parent == before.parent
            )
            return last_ok, before.order_key
        else:
            last_ok = nextn(
                (n.order_key for n in reversed(self._nodes) if n.parent == self._parent)
            )
            return last_ok, None

    def _update(self, scope: "ScopeNode"):
        # _children is effectively a computed property which is replaced wholesale,
        # we don't do diff updates to keep it simple with all the relation types.
        if self._flags & NRel.Cumulative:
            # all matching children of parent's descendants
            #  e.g. Module->Issue, File->Issue, ... -> all issues
            self._nodes = scope._local_root_tree.get_descendants(
                scope.ck, self._child_mnt, recursive=True, prefilter=False
            )
            assert not self._flags & NRel.Ordered, f"cannot order cumulative {self}"
        elif self._flags & NRel.Flat:
            # all matching descendants of matching children of parent
            #  e.g. Module->File, File->File, ... -> all files
            self._nodes = scope._local_root_tree.get_descendants(
                scope.ck, self._child_mnt, recursive=True, prefilter=True
            )
            if self._flags & NRel.Ordered:
                self._nodes = _sort_nested_ordered_list(self._parent.ck, self._nodes)
        else:
            # only matching children of parent
            self._nodes = scope._local_root_tree.get_descendants(
                scope.ck, self._child_mnt, recursive=False
            )
            if self._flags & NRel.Ordered:
                self._nodes.sort(key=lambda n: n.order_key or BIGGEST_INTEGER)

    def append(
        self,
        _node: NodeT,
        _create: bool = True,
        after: NodeT = None,
        before: NodeT = None,
        _trigger: _NC = _NC.Full,
    ) -> list[NodeT]:
        assert isinstance(_node, Node), f"cannot append {_node!r} to {self!r}"
        if _node.parent is not None:
            raise ValueError(f"cannot attach {_node!r} to {self!r}: attached to {_node.parent!r}")

        # assign ids if newly attached to the module (ids are derived from ck + module)
        if not _node.attached and self._parent.attached:
            module_id = self._parent.module.id
            for n in _node._walk_rec():
                if n.id is None:
                    n._assign_id(module_id)
        change = _ChangeEffect._collect(None, self._parent, [_node], _trigger)
        # update parent after updating ids (the above walks tree, which is changed here)
        _node.parent = self._parent

        # index node into parent scope
        if isinstance(_node, ScopeNode) and _node._local_tree is not None:
            # subsume if previously detached (ignores out of line nodes, see :NodeViews)
            added = _node._local_tree.get_descendants(_node.ck, recursive=True, include_self=True)
            _node._local_tree.update(_node)  # parent changed
            self._parent._import_scope_tree(_node)
            _node._local_tree = None
        else:  # or just add
            added = [_node]
            self._parent._local_root_tree.add(_node)
        # register node scope
        if (
            self._flags & NRel.Scoped
            and _node.name
            and (not self._flags & NRel.Flat or _node.parent == self._parent)
        ):
            self._parent._add_node_to_scope(_node)

        # assign order key to ordered nodes
        if self._flags & NRel.Ordered and _node.order_key is None:
            _node.order_key = generate_key_between(*self._ok_bounds(after, before))
        if _trigger:
            # and update every affected node (to list/interp as needed)
            change._effect(_trigger)
            assert _node in self._nodes, f"node {_node!r} not in {self!r}"

        # activate node in session if this parent has one
        if self._parent._status == NS.Tracked and _node._status != NS.Tracked:
            _node._activate_self(self._parent._session)
        # 'create' node in session if it's attached
        if _node.attached and _create and self._parent._session:
            self._parent._session.tracer.node_create(*added)
        # temporarily hoisted records may no longer be in tree, so return our added nodes
        return added

    def extend(
        self,
        *nodes: NodeT,
        _create: bool = True,
        after: NodeT = None,
        before: NodeT = None,
        _trigger: _NC = _NC.Full,
    ):
        nodes = flatten_list(*nodes)
        if not nodes:
            return
        # pre-assign order keys since we don't trigger between appends (meaning last_ok is wrong)
        if self._flags & NRel.Ordered:
            oks = generate_n_keys_between(*self._ok_bounds(after, before), n=len(nodes))
            for node, ok in zip(nodes, oks):
                node.order_key = ok

        # as above in append but batched: append, trigger, create
        #  (can we merge them somehow to simplify)?
        change = _ChangeEffect._collect(None, self._parent, nodes, _trigger)
        change._effect(_trigger & ~_NC.Attach)
        added = []
        for node in nodes:
            added.extend(self.append(node, _create=False, _trigger=_NC.Ignore))
        change._effect(_trigger & ~_NC.Detach)
        if _trigger & _NC.UpdateLists:
            assert all(n in self._nodes for n in nodes), f"nodes {nodes} not in {self!r}"
        if self._parent.attached and _create and self._parent._session:
            self._parent._session.tracer.node_create(*added)

    def remove(self, _node: NodeT, _delete: bool = True, _trigger: _NC = _NC.Full):
        change = _ChangeEffect._collect(self._parent, None, [_node], _trigger)
        if _delete and self._parent._session:
            self._parent.session.tracer.node_delete(_node)
        self._parent._local_root_tree.remove(_node)
        _node.parent = None
        change._effect(_trigger)
        if _trigger & _NC.UpdateLists:
            assert _node not in self._nodes, f"node {_node!r} still in {self!r}"

    def clear(self, _delete: bool = True, _trigger: _NC = _NC.Full):
        if not self._nodes:
            return
        change = _ChangeEffect._collect(self._parent, None, self._nodes, _trigger)
        removed = list(self._nodes)
        for _node in removed:
            self.remove(_node, _delete=_delete, _trigger=_NC.Ignore)
        change._effect(_trigger)
        if _trigger & _NC.UpdateLists:
            assert not self._nodes, f"{self!r} is not empty"

    def get(self, some_id: str) -> Optional[NodeT]:
        if not (self._flags & NRel.Keyed) and not (self._flags & NRel.Named):
            raise ValueError(f"cannot get {some_id!r} from {self!r}")
        for child in self._nodes:
            if (self._flags & NRel.Keyed and child.key == some_id) or (
                self._flags & NRel.Named and (child.name == some_id or child.py_ident == some_id)
            ):
                return child
        return None

    def index(self, node: NodeT) -> int:
        return self._nodes.index(node)

    def __bool__(self):
        return bool(self._nodes)

    def __contains__(self, obj: object) -> bool:
        # special case to unwrap key (e.g. for tagging/tag objects)
        if self._flags & NRel.Keyed and hasattr(obj, "key"):
            obj = obj.key
        if isinstance(obj, str) and (self._flags & NRel.Keyed or self._flags & NRel.Named):
            return self.get(obj) is not None
        elif isinstance(obj, Node):
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


class NodeTreeBase(abc.ABC, typing.Generic[NT]):
    @property
    def nodes(self) -> Collection[NT]:
        raise NotImplementedError

    def get(self, node_id_or_ck: UUID) -> Optional[NT]:
        """Gets a node by id or ck"""
        raise NotImplementedError

    def __getitem__(self, item):
        raise NotImplementedError

    def __contains__(self, item):
        raise NotImplementedError

    def clear(self):
        """Clear the tree"""
        raise NotImplementedError

    def add(self, node: "Node"):
        """Add a node to the tree (error if node already exists)"""
        raise NotImplementedError

    def add_many(self, *nodes: Collection[NT]):
        nodes = flatten_list(*nodes)
        for node in nodes:
            self.add(node)

    def update(self, node: "Node"):
        """Updates the node in this tree (must exist)"""
        raise NotImplementedError

    def set(self, nodes: Collection[NT]):
        """Replaces all nodes in the tree"""
        self.clear()
        for node in nodes:
            self.add(node)

    def add_tree(self, tree: "DetachedNodeTree"):
        raise NotImplementedError

    def remove(self, node: "Node"):
        """Remove a node from the tree (incl. all descendants if recursive)"""
        raise NotImplementedError

    def get_descendants(
        self,
        node_id_or_ck: UUID,
        mnt: MNT | None = None,
        recursive: bool = False,
        prefilter: bool = False,
        include_self: bool = False,
    ) -> list["NT"]:
        """Gets all children descendants as filtered in BFS order"""
        raise NotImplementedError

    def get_ancestor(self, node_id_or_ck: UUID, mnt: MNT | None = None) -> Optional["NT"]:
        """Finds the next ancestor of the given type (including self)"""
        raise NotImplementedError


class NodeTree(NodeTreeBase[NT]):
    """An indexed tree of module nodes. Can be either language or data nodes."""

    def __init__(self, nodes: Collection[NT] | "NodeTree" = None):
        self.nodes_by_id: dict[UUID, NT] = {}
        self.nodes_by_ck: dict[UUID, NT] = {}
        self.node_id_by_parent_id: dict[UUID, list[UUID]] = {}
        if isinstance(nodes, list):
            for node in nodes or []:
                self.add(node)
        elif isinstance(nodes, NodeTree):
            self.add_tree(nodes)

    def __str__(self):
        return f"{len(self.nodes_by_id)} nodes"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def nodes(self) -> Collection[NT]:
        return self.nodes_by_ck.values()

    def copy(self):
        return NodeTree(self)

    def deepcopy(self):
        nodes = [deepcopy(node) for node in self.nodes]
        return NodeTree(nodes)

    #
    # Edits
    #

    def clear(self):
        """Clear the tree"""
        self.nodes_by_id.clear()
        self.nodes_by_ck.clear()
        self.node_id_by_parent_id.clear()

    def add(self, node: NT):
        """Add a node to the tree (error if node already exists)"""
        assert node.id is not None, f"cannot add {node!r} to {self!r} without id"
        if node.id in self.nodes_by_id:
            existing = self.nodes_by_id[node.id]
            raise ValueError(
                f"node {node!r} (id={node.id}) already exists in {self!r}: {existing!r} (id={existing.id})"
            )
        self.nodes_by_id[node.id] = node
        self.nodes_by_ck[node.ck] = node
        if node.parent_id is not None:
            if node.parent_id not in self.node_id_by_parent_id:
                self.node_id_by_parent_id[node.parent_id] = []
            self.node_id_by_parent_id[node.parent_id].append(node.id)

    def update(self, node: NT):
        """Updates a node in this tree (must exist)"""
        if node.id not in self.nodes_by_id:
            raise ValueError(f"node {node!r} does not exist in {self!r}")
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

    def truncate(self, node: NT, mnt: MNT, recursive: bool = True):
        """Truncate descendants of a node"""
        descendants = self.get_descendants(node.id, mnt, recursive=recursive)
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
                if node.mnt != MNT.RECORD:  # remove hoisted records :TempRecordTree
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
        mnt: MNT | None = None,
        recursive: bool = False,
        prefilter: bool = False,
        include_self: bool = False,
    ) -> list["NT"]:
        """Gets all children descendants as filtered in BFS order"""
        if node_id_or_ck in self.nodes_by_id:
            node_id = node_id_or_ck
        elif node_id_or_ck in self.nodes_by_ck:
            node_id = self.nodes_by_ck[node_id_or_ck].id
        else:
            raise ValueError(f"node {node_id_or_ck} is not in {self!r}")
        children = [
            self.nodes_by_id[child_id]
            for child_id in self.node_id_by_parent_id.get(node_id, [])
            if not mnt or not prefilter or self.nodes_by_id[child_id].mnt == mnt
        ]

        descendants = []
        if include_self:
            descendants.append(self.nodes_by_id[node_id])
        descendants.extend(children)
        if recursive:
            for child in children:
                if child.id not in self.node_id_by_parent_id:
                    continue
                descendants.extend(
                    self.get_descendants(child.id, mnt, prefilter=prefilter, recursive=True)
                )
        if not prefilter and mnt:
            descendants = [n for n in descendants if n.mnt == mnt]
        return descendants

    def get_ancestor(self, node_id_or_ck: UUID, mnt: MNT | None = None) -> Optional["NT"]:
        """Finds the next ancestor of the given type (including self)"""
        if node_id_or_ck in self.nodes_by_id:
            node_id = node_id_or_ck
        elif node_id_or_ck in self.nodes_by_ck:
            node_id = self.nodes_by_ck[node_id_or_ck].id
        else:
            raise ValueError(f"node {node_id_or_ck} is not in {self!r}")
        node = self.nodes_by_id.get(node_id)
        while node:
            if not mnt or node.mnt == mnt:
                return node
            if node.parent_id is None:
                return None
            node = self.nodes_by_id[node.parent_id]
        return None

    def get_ancestors(
        self,
        node_id: UUID,
        mnt: MNT | None = None,
        include_self: bool = False,
    ) -> list["NT"]:
        """Finds all ancestors of the given type"""
        ancestors = []
        node = self.nodes_by_id.get(node_id)
        if node is None:
            raise ValueError(f"node {node_id} is not in {self!r}")
        if include_self:
            ancestors.append(node)
        while node:
            if not mnt or node.mnt == mnt:
                ancestors.append(node)
            if node.parent_id is None:
                break
            node = self.nodes_by_id[node.parent_id]
        return ancestors


class DetachedNodeTree(NodeTreeBase[NT]):
    """
    A minimal NodeTree for working with instantiated nodes that may not have ids yet.
    We have a separate tree for this because wire nodes work with ids only (for parent),
     and we don't need to support all operations since it's only for detached nodes.
    """

    def __init__(self):
        self.nodes_by_ck: dict[UUID, "Node"] = {}
        self.node_ck_by_parent_ck: dict[UUID, list[Node]] = defaultdict(list)

    def __str__(self):
        return f"{len(self.nodes_by_ck)} nodes"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def nodes(self) -> Collection[NT]:
        return self.nodes_by_ck.values()

    def get(self, node_ck: UUID) -> Optional[NT]:
        """Gets a node by id"""
        return self.nodes_by_ck.get(node_ck)

    def __getitem__(self, item):
        return self.nodes_by_ck.get(item)

    def __contains__(self, item):
        return item in self.nodes_by_ck

    def clear(self):
        """Clear the tree"""
        self.nodes_by_ck.clear()
        self.node_ck_by_parent_ck.clear()

    def add(self, node: "Node"):
        """Add a node to the tree (error if node already exists)"""
        if node.ck in self.nodes_by_ck and self.nodes_by_ck[node.ck] is not node:
            raise ValueError(f"node {node!r} (ck={node.ck}) already exists in {self!r}")
        self.nodes_by_ck[node.ck] = node
        if node.parent is not None:
            self.node_ck_by_parent_ck[node.parent.ck].append(node)

    def update(self, node: "Node"):
        """Updates the node in this tree (must exist)"""
        if node.ck not in self.nodes_by_ck:
            raise ValueError(f"node {node!r} (ck={node.ck}) does not exist in {self!r}")
        self.nodes_by_ck[node.ck] = node
        if node.parent is not None:
            self.node_ck_by_parent_ck[node.parent.ck].append(node)

    def add_tree(self, tree: "DetachedNodeTree"):
        assert type(self) == type(tree), f"cannot add {tree!r} to {self!r}"
        self.nodes_by_ck.update(tree.nodes_by_ck)
        for parent_ck, children in tree.node_ck_by_parent_ck.items():
            self.node_ck_by_parent_ck[parent_ck].extend(children)

    def remove(self, node: "Node"):
        """Remove a node from the tree (incl. all descendants if recursive)"""
        descendants = self.get_descendants(node.ck, recursive=True, include_self=True)
        for descendant in descendants:
            if descendant.ck in self.node_ck_by_parent_ck:
                self.node_ck_by_parent_ck.pop(descendant.ck)
            if descendant.parent and descendant.parent.ck in self.node_ck_by_parent_ck:
                self.node_ck_by_parent_ck[descendant.parent.ck].remove(descendant)
            if descendant.ck in self.nodes_by_ck:
                self.nodes_by_ck.pop(descendant.ck)

    def get_descendants(
        self,
        node_id_or_ck: UUID,
        mnt: MNT | None = None,
        recursive: bool = False,
        prefilter: bool = False,
        include_self: bool = False,
    ) -> list["NT"]:
        """Gets all children descendants as filtered in BFS order"""
        children = [
            child
            for child in self.node_ck_by_parent_ck.get(node_id_or_ck, [])
            if not mnt or not prefilter or child.mnt == mnt
        ]
        descendants = []
        if include_self:
            descendants.append(self.nodes_by_ck[node_id_or_ck])
        descendants.extend(children)
        if recursive:
            for child in children:
                if child.ck not in self.node_ck_by_parent_ck:
                    continue
                descendants.extend(
                    self.get_descendants(child.ck, mnt, prefilter=prefilter, recursive=True)
                )
        if not prefilter and mnt:
            descendants = [n for n in descendants if n.mnt == mnt]
        return descendants

    def get_ancestor(self, node_id_or_ck: UUID, mnt: MNT | None = None) -> Optional["NT"]:
        """Finds the next ancestor of the given type (including self)"""
        node = self.nodes_by_ck.get(node_id_or_ck)
        if node is None:
            raise ValueError(f"node {node_id_or_ck} is not in {self!r}")
        while node:
            if not mnt or node.mnt == mnt:
                return node
            if node.parent is None:
                return None
            node = node.parent
        return None


def _make_self_method(
    method: NodeMethod, wraps, from_status: NodeStatus = None, to_status: NodeStatus = None
):
    """Creates method that calls _method_inner for all components in call order"""

    @functools.wraps(wraps)
    def self_method(self: "Node", *args, _coerce: bool = True, _ignore: bool = False, **kwargs):
        if from_status is not None and self._status != from_status:
            if not _coerce:
                raise RuntimeError(f"cannot {method.name} {self!r} (status={self._status.name})")
            # auto coerce the node into the desired to_status if allowed and feasible
            if self._status == NS.Source and to_status > NS.Indexed:
                self._index_self()
            if self._status == NS.Indexed and to_status > NS.Interpreted:
                self._interp_self(self)
            if from_status <= to_status <= self._status or from_status >= to_status >= self._status:
                return  # nothing to do
            if self._status < from_status:
                raise RuntimeError(
                    f"cannot coerce {method.name} {self!r} (status={self._status.name})"
                )

        for meth in _get_component_methods(self._components, method, self._concrete_cache_key):
            meth(self, *args, **kwargs)
        if to_status is not None:
            self._status = to_status

    self_method.__name__ = method.self
    return self_method


def _make_inner_dunder_method(method: NodeMethod):
    """Creates method that proxies a builtin dunder method to the first _method_inner"""

    def inner_method(self: "Node", *args, **kwargs):
        meths = _get_component_methods(self._components, method, self._concrete_cache_key)
        if len(meths) < 2:  # includes this one
            raise RuntimeError(f"{self!r} does not support {method.name}")
        return meths[1](self, *args, **kwargs)

    inner_method.__name__ = method.inner
    return inner_method


class _Passthrough(enum.StrEnum):
    Full = "full"
    Scope = "scope"


@node_component
class Node(abc.ABC):
    """
    A node in a Bench module tree.
    A node has a per-version unique id (id) and a constant identifier key (ck).
    The id is derived from the module id, so it's only assigned when the node is attached.
    """

    mnt: ClassVar[MNT]  # set in @node decorator
    __static_components__: ClassVar[tuple[type["Node"], ...]] = []
    __dynamic_components__: ClassVar[tuple[type["Node"], ...]] = ()
    __properties__: ClassVar[dict[str, NodeProperty]] = {}
    __ancestor_properties__: ClassVar[dict[str, NodeProperty]] = {}
    __list_properties__: ClassVar[dict[str, NodeProperty]] = {}
    __list_properties_by_child__: ClassVar[dict[MNT, list[NodeProperty]]] = defaultdict(list)
    __tracked_properties__: ClassVar[dict[str, NodeProperty]] = {}
    __internal_properties__: ClassVar[dict[str, NodeProperty]] = {}
    __static_passthrough__: ClassVar[tuple[tuple[str, _Passthrough]]] = ()
    __has_scope__: ClassVar[bool] = False

    id: UUID = ninternal(default=None)
    ck: UUID = ninternal(default=None)
    parent: Optional["Node"] = nparent()
    # prototype: Optional["Node"] / instance_of_ck: UUID
    module: Optional["Module"] = nancestor(MNT.MODULE)

    created_at: datetime = ninternal(default_factory=utcnow_with_tz, is_cru=True)
    updated_at: datetime = ninternal(default_factory=utcnow_with_tz, is_cru=True)
    last_edited_at: datetime = ninternal(default_factory=utcnow_with_tz, is_cru=True)
    last_changed_at: datetime = ninternal(default_factory=utcnow_with_tz, is_cru=True)
    revision: int = ninternal(default=0, is_cru=True)

    _session: Optional["Session"] = nruntime(default=None)
    _status: NodeStatus = nruntime(default=None)
    _track: NodeTrackingLevel = nruntime(default=NodeTrackingLevel.FULL)
    _new: bool = nruntime(default=False)

    def __post_init__(self):
        if self._session is None:
            from bench.language.builtin import _active_session

            self._session = _active_session.get()
        if self._status is None:
            self._status = NS.Interpreted if self._session is not None else NS.Source
        if self.ck is None:
            self.ck = uuid4()
            self._new = True
        if self._session and self._new and not self.parent:
            self._session._dangling_nodes_by_ck[self.ck] = self
        if self.id is None and self.attached:
            self._assign_id(self.module.id)
        self._init_self()
        if self._status == NS.Interpreted and self._session is not None:
            self._activate_self(self._session)

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.parent.id if self.parent is not None else None

    @property
    def _components(self) -> tuple[type["Node"], ...]:
        return self.__static_components__

    @property
    def _dynamic_components(self) -> tuple[type["Node"], ...]:
        return ()

    @property
    def _concrete_cache_key(self) -> str:
        """Identifier for dynamic components"""
        return type(self).__name__

    @property
    def _passthrough_targets(self) -> tuple[tuple[str, _Passthrough]] | None:
        """Pass through __getattr__/__setattr__ properties (before defaulting to usual)"""
        return self.__static_passthrough__

    @property
    def _local_root(self) -> "Node":
        parent = self
        while parent.parent is not None:
            parent = parent.parent
        return parent

    @property
    def _local_root_scope(self) -> "ScopeNode":
        assert self.scope is not None, f"{self!r} has no parent"
        return self.scope._local_root_scope

    @property
    def _local_root_tree(self) -> "NodeTreeBase":
        return self._local_root._local_tree

    def _assign_id(self, module_id: UUID):
        assert module_id, f"cannot assign id to {self} without a module id"
        assert self.id is None, f"cannot assign id to {self} twice"
        assert self.ck is not None, f"cannot assign id to {self} without ck"
        self.id = get_node_id(module_id, self.ck)

    def __eq__(self, other):
        return isinstance(other, self.__class__) and self.id == other.id and self.ck == other.ck

    def __hash__(self):
        return hash(self.id)

    def _set_untracked(self, key, value):
        self.__dict__[key] = value

    def __setattr__(self, key, value):
        if self._status != NS.Tracked:
            return super().__setattr__(key, value)

        # tracked set
        if key in self.__internal_properties__:
            if key in self.__list_properties__:
                return getattr(self, key).set(value)
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
            if self.attached:
                self._session.tracer.node_update(self, [key])
            return
        elif key in self.__dict__:
            return super().__setattr__(key, value)

        # try first full passthrough target (if any)
        for target, mode in self._passthrough_targets:
            target = getattr(self, target)
            if mode == _Passthrough.Full:
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
        # prefer components own methods
        for component in self._components:
            if component is self.__class__ or component is Node:
                continue
            attr = getattr(component, item, UNSET)
            if attr is not UNSET:
                break
        # check passthrough targets if tracked in session
        if attr is UNSET and self._session is not None:
            for target, mode in self._passthrough_targets:
                target = getattr(self, target)
                if mode == _Passthrough.Full:
                    attr = getattr(target, item, UNSET)
                elif mode == _Passthrough.Scope:
                    assert isinstance(target, NodeList), f"invalid scope passthrough: {attr!r}"
                    attr = target.get(item) or UNSET
                if attr is not UNSET:
                    break
        # attribute be property, method, or just plain value
        if attr is not UNSET:
            if isinstance(attr, property):
                return attr.fget(self)
            elif not isinstance(attr, Node) and callable(attr) and not inspect.ismethod(attr):
                return functools.partial(attr, self)
            else:
                return attr

        # report lookup error with additional info
        candidates = {k: v for k, v in self.__properties__.items() if not k.startswith("_")}
        if isinstance(self, ScopeNode):
            candidates.update(self._scopes_by_name)
        did_you_mean = did_you_mean_str(candidates, item)
        raise AttributeError(f"{self!r} has no attribute '{item}'. {did_you_mean}")

    # abstract :ComponentMethods

    def _init_inner(self) -> None:
        """Initialize this node."""
        pass

    def _clear_inner(self, scope: Optional["ScopeNode"]) -> None:
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
        from bench.language.builtin import _should_validate

        if DEBUG and not _should_validate():
            return  # escape hatch for testing
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
        """Visit any non-descendant referenced nodes."""
        pass

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

    def _init_self(self):
        # init lists
        existing_lists: dict[str, typing.Any] | None = None
        for name, prop in self.__list_properties__.items():
            existing = getattr(self, name, None)
            setattr(self, name, prop.list_type(self, prop))
            if existing and not isinstance(existing, NodeList):
                if existing_lists is None:
                    existing_lists = {}
                existing_lists[name] = existing

        # run actual init methods
        for meth in _get_component_methods(
            self._components, NodeMethod.init, self._concrete_cache_key
        ):
            meth(self)

        # keep manually set node lists if passed in
        if existing_lists:
            changed_nodes: list[NodeT] = []
            was_interp = self._status >= NS.Interpreted
            detach_trigger = _NC.UpdateLists | _NC.Detach if was_interp else _NC.UpdateLists
            for name, existing in existing_lists.items():
                if existing and not isinstance(existing, NodeList):
                    getattr(self, name).extend(*existing, _trigger=detach_trigger)
                    changed_nodes.extend(existing)
            if changed_nodes and was_interp:
                _ChangeEffect._collect(None, self, changed_nodes, _NC.Attach)._effect(_NC.Attach)

        # validate if in session after all init are done
        if self._status >= NS.Interpreted and self._session is not None:
            self._validate_self(self.__tracked_properties__.keys(), on_issue=on_issue_raise)

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

    def _copy_self(self, keep_parent: bool = False, reset_id: bool = True) -> "Node":
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

    def _walk_rec(self) -> Collection["Node"]:
        """
        Walks this node and all descendants in breadth-first order.
        """
        return [self]

    def _on_issue(self, subject: "Node", type: IssueType, message: str = None, **kwargs) -> None:
        # only scope nodes can host issues, forward to parent
        self.parent._on_issue(subject=self, type=type, message=message, **kwargs)

    @property
    def attached(self) -> bool:
        return self.parent is not None and self.module is not None

    @property
    def scope(self) -> Optional["ScopeNode"]:
        return self.parent

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
        return self.session._log


def _make_rec_method(method: NodeMethod, wraps, custom_kwargs: Callable[["Node"], dict] = None):
    """Creates method that calls _method_self for self and all descendants"""

    @functools.wraps(wraps)
    def rec_method(self: "ScopeNode", *args, **kwargs):
        # ignores out-of-line descendants (see :NodeViews)
        descendants = self._local_root_tree.get_descendants(self.ck, recursive=True)
        method_name = method.self
        if custom_kwargs:
            for node in descendants:
                node_kwargs = custom_kwargs(node)
                getattr(node, method_name)(*args, **kwargs, **node_kwargs)
            node_kwargs = custom_kwargs(self)
            getattr(self, method_name)(*args, **kwargs, **node_kwargs)
        else:
            for node in descendants:
                getattr(node, method_name)(*args, **kwargs)
            getattr(self, method_name)(*args, **kwargs)

    rec_method.__name__ = method.rec
    return rec_method


@node_component
class ScopeNode(Node):
    """A scope for hosting and looking up nodes. Required for any node with children."""

    __has_scope__: ClassVar[bool] = True
    issues: NodeList["Issue"] = nchildren(MNT.ISSUE, NRel.Cumulative)
    _scopes_by_name: dict[str, "ScopeNode"] = nruntime(default_factory=dict)
    _names_by_ident: dict[str, str] = nruntime(default_factory=dict)
    # the local tree is maintained at the local root (usually module, maybe a detached root node)
    _local_tree: Union["NodeTree", "DetachedNodeTree", None] = nruntime(default=None)

    @property
    def scope(self) -> "ScopeNode":
        return self

    def _init_inner(self) -> None:
        if self.parent is None:
            if not isinstance(self, Module):
                self._local_tree = DetachedNodeTree()
            else:
                self._local_tree = NodeTree()
            self._local_tree.add(self)

    _clear_rec = _make_rec_method(
        NodeMethod.clear, Node._clear_self, custom_kwargs=lambda n: dict(scope=n.scope)
    )
    _index_rec = _make_rec_method(NodeMethod.index, Node._index_self)
    _interp_rec = _make_rec_method(
        NodeMethod.interp, Node._interp_self, custom_kwargs=lambda n: dict(scope=n.scope)
    )
    _visit_rec = _make_rec_method(NodeMethod.visit, Node._visit_self)
    _validate_rec = _make_rec_method(
        NodeMethod.validate,
        Node._validate_self,
        custom_kwargs=lambda n: dict(
            properties=n.__tracked_properties__.keys(), on_issue=on_issue_raise
        ),
    )
    _activate_rec = _make_rec_method(NodeMethod.activate, Node._activate_self)
    _deactivate_rec = _make_rec_method(NodeMethod.deactivate, Node._deactivate_self)

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
            # check that we're not resolving something from an out-of-sync cache
            assert scope.attached == self.attached, f"{scope!r} isn't in the same tree as {self!r}"
            return scope
        if self.parent is not None:
            return self.parent._find_scope(name, by=by)
        return None

    def _update_lists(self, scope: "ScopeNode"):
        for prop in self.__list_properties__.values():
            getattr(self, prop.name)._update(scope)

    def _clear_inner(self, scope: Optional["ScopeNode"]):
        self._scopes_by_name = {}
        self._names_by_ident = {}

    def _index_inner(self) -> None:
        for prop in self.__list_properties__.values():
            if prop.children_flags & NRel.Scoped:
                for child in getattr(self, prop.name):
                    if child.name and (not prop.children_flags & NRel.Flat or child.parent == self):
                        self._add_node_to_scope(child)

    def _walk_rec(self) -> Collection["Node"]:
        return self._local_root_tree.get_descendants(self.ck, recursive=True, include_self=True)

    def _add_node_to_scope(self, node: Node) -> None:
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
        """The root of the 'local' node tree (usually module, but maybe a detached root node)"""
        if self.parent is None:
            return self
        return self.parent._local_root_scope

    @property
    def _local_root_tree(self) -> Union["NodeTree", "DetachedNodeTree"]:
        """The 'local' node tree (see _local_root_scope)"""
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
            if self._local_tree is not None:
                return self._local_tree.get(path)
            else:
                return self._local_root_scope.lookup(path, by=by, node_t=node_t)

        if isinstance(path, str):
            path = parse_node_path(path)
        if path.path == ".":
            return self._find_scope(path.name, by)
        elif path.path.startswith("."):
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

    def _get_visible_scopes(self) -> dict[str, "ScopeNode"]:
        """Returns the scopes visible from this node."""
        scopes = {**self._scopes_by_name}
        if self.parent:
            for name, child in self.parent._get_visible_scopes().items():
                if name not in scopes:  # shadowing
                    scopes[name] = child
        return scopes

    def _on_issue(self, subject: "Node", type: IssueType, message: str = None, **kwargs):
        from bench.language import File, Statement
        from bench.language.issue import Issue

        if not subject.attached:
            return  # no way to derive issue id, so just ignore?
        issue_ck = uuid.uuid5(subject.id, (type.value + (message or "")))
        issue_id = issue_ck  # not sure?
        if not isinstance(subject, (Statement, File)):
            subject = subject.parent  # fields don't have issues (yet)
        issue = Issue(id=issue_id, ck=issue_ck, type=type, parent=None, **kwargs)
        if issue not in subject.issues:  # dedup
            subject.issues.append(issue, _trigger=_NC.UpdateLists)

    @property
    def errors(self) -> list["Issue"]:
        if self.issues is None:
            return []
        return [i for i in self.issues or [] if i.kind == IssueKind.Error]

    @property
    def self_errors(self):
        return [i for i in self.errors or [] if i.parent == self]


@dataclass
class ModuleChange:
    source_edits: list["EditData"]  # incoming external edits
    interp_edits: list["EditData"]  # resulting interp state change
    added: list[Node]
    updated: list[Node]
    removed: list[Node]

    @property
    def touched(self) -> typing.Iterable[Node]:
        return chain(self.added, self.updated, self.removed)


@node(mnt=MNT.MODULE, passthrough=(("files", _Passthrough.Full),))
class Module(ScopeNode):
    parent: None = nparent()
    name: str = ninternal()  # can't change this yet
    committed: bool = ninternal(default=False)
    files: NodeList["File"] = nchildren(MNT.FILE, NRel.Flat | NRel.Named | NRel.Scoped)
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
    def project_id(self):
        assert self._project_id is not None, f"no project id set in {self!r}"
        return self._project_id

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
            if not isinstance(path, NodePath):
                path = parse_absolute_node_reference(path)
            module_name, sub_path = path
            if module_name == self.name:
                dependency = self
            else:
                dependency = self.dependencies.get(module_name)
            if dependency is None:
                resolved = None
            else:
                resolved = dependency.lookup(sub_path, node_t=node_t, by=by)

        if not resolved:
            return resolved
        assert resolved.attached, f"resolved {path} to detached {resolved!r} (index out of sync?)"
        # cache result
        if self.committed:
            self._lookup_cache[path] = resolved
        return resolved

    def _get_visible_scopes(self) -> dict[str, "ScopeNode"]:
        scopes = {**self._scopes_by_name}
        for builtin in self.builtins:
            scopes.update(builtin._get_visible_scopes())
        return scopes

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

    def _apply_edits(self, edits: list["EditData"]) -> ModuleChange:
        """
        Applies the given external edits to the module.
        TODO @Performance @UX: :HotReload patch edits locally
        """
        assert self._source is not None, f"cannot apply edits to {self!r} without source"

        # update source
        old_nodes_by_ck: dict[UUID, Node] = {**self.module._tree.nodes_by_ck}
        old_source = self._source.copy()
        self._apply_edits_to_source(edits)

        # update self (this is obviously inefficient, but performs surprisingly okay)
        self._reset_from_source()

        # compute change, apply interp source changes if any
        change = self._compute_change(edits, old_nodes_by_ck, old_source)
        if change.interp_edits:
            self._apply_edits_to_source(change.interp_edits)
        return change

    def _reset_from_source(self):
        """Resets the module completely from the source."""
        from bench.language.wire import unpack_node

        assert self._source and self.id in self._source, f"cannot reset {self!r} without source"

        prev_session = self.module._session
        if prev_session:
            self.module._deactivate_self()

        self.module._clear_rec()
        self.module._tree.clear()
        _ = unpack_node(self._source, parent=self, session=None, exclude=INTERP_NODE_TYPES)
        self.module._interp_rec()

        if prev_session:
            self.module._activate_rec(prev_session)

    def _apply_edits_to_source(self, edits: list["EditData"]) -> None:
        """Applies the edits directly to the source without any interp."""
        from bench.language.edit import ModuleEditor

        editor = ModuleEditor(self._source, self._project_id, self.id)
        # errors are fine here since e.g. a deleted issue's parent may have disappeared
        #  (we could filter that, but it's easier this way since it's more explicit for clients)
        editor.apply_all(edits, raise_on_error=False)

    def _compute_change(
        self,
        source_edits: list["EditData"],
        old_nodes_by_ck: dict[UUID, Node],
        old_source: NodeTree,
    ) -> ModuleChange:
        """Computes the change between the old and new module state."""
        from bench.language.edit import ModuleEditor

        new_nodes: dict[UUID, Node] = self.module._tree.nodes_by_ck
        added = []
        updated = []
        for n in new_nodes.values():
            if n.ck in old_nodes_by_ck:
                if n.revision != old_nodes_by_ck[n.ck].revision:
                    updated.append(n)
            else:
                added.append(n)
        removed = [n for n in old_nodes_by_ck.values() if n.ck not in new_nodes]

        # gather interp edits (delete from old, create in new)
        old_editor = ModuleEditor(old_source, self._project_id, self.id)
        for node in removed:
            if node.mnt in INTERP_NODE_TYPES:
                if node.ck not in old_source.nodes_by_ck:
                    # need to investigate
                    logger.warning(f"node {node!r} not found in old source for {self!r}")
                    continue
                # recover parent info from source
                old_node = old_source.nodes_by_ck[node.ck]
                old_editor.delete(old_node, apply=False)
        new_editor = ModuleEditor(self._source, self._project_id, self.id)
        for node in added:
            if node.mnt in INTERP_NODE_TYPES:
                new_editor.create(node, apply=False)

        return ModuleChange(
            source_edits=source_edits,
            interp_edits=new_editor.edits + old_editor.edits,
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
        old_source_nodes_by_ck = {**module._source.nodes_by_ck}

        for dependency in libs.DEFAULT_MODULES.values():
            module.add_dependency(dependency)
        module.add_builtin(libs.symbolx_lib.files.get("builtins"))
        module._interp_rec()

        # update source with interp edits (doesn't have them)
        change = module._compute_change([], old_nodes_by_ck, old_source_nodes_by_ck)
        if change.interp_edits:
            module._apply_edits_to_source(change.interp_edits)

        return module

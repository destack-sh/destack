import abc
import dataclasses
import enum
import functools
import inspect
import uuid
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime
from itertools import chain
from sys import intern
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    ClassVar,
    Collection,
    Iterable,
    Iterator,
    Optional,
    TypeVar,
    Union,
    cast,
    dataclass_transform,
    final,
)
from uuid import UUID, uuid4

import math
import structlog
from bitarray import bitarray
from cachetools import cached

from bench.language.const import (
    IN_BENCH_NODE_TYPES,
    IN_PACKAGE_NODE_TYPES,
    NODE_TYPES,
    SUB_BENCH_NODE_TYPES,
    SUB_PACKAGE_NODE_TYPES,
    UNSET,
    NodeSource,
    NodeStatus,
    NodeTrackingLevel,
    NodeType,
    NRel,
    StructType,
    _active_session,
    NodeReferenceKind,
)
from bench.language.property import Property
from bench.language.graph import DetachedNodeGraph, NodeGraph, NodeGraphBase, NodeList
from bench.language.setup import (
    STRUCT_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    NODE_COMPONENT_CLASS_BY_NAME,
    HAS_CHILD_NODE_TYPES,
    _on_completing_setup,
)
from bench.language.property import (
    p_runtime,
    p_parent,
    p_ancestor,
    p_child,
    p_regular,
    p_internal,
    p_system,
    METATYPE_PROPERTY,
    _PROPERTY_SPECIFIERS,
)
from bench.language.validation import (
    PropertyValidationHandler,
    ValidationError,
    ValidationHandler,
    on_invalid_raise,
)
from bench.proto.wire import AnyNodeData, AnyStructData, SomeNodeData
from bench.sql.core import (
    Constraint,
    ConstraintType,
    Index,
    IndexType,
    PrimitiveType,
    Table,
)
from bench.utils.casing import PYTHON_CASING, IdentifierType, to_casing
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import (
    bytetuple,
    did_you_mean_str,
)
from bench.utils.utils import frozendict

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        FieldPath,
        NodeReference,
        NodeVisitor,
        Notice,
        NoticeType,
        Package,
        PropertyReference,
        Session,
        ValueReference,
    )
    from bench.language.expression import _NodeExpressionBase
    from bench.language.notice import NoticeHandler

logger = structlog.get_logger(__name__)


def get_node_id(package_id: UUID, ck: UUID):
    """Derive the version-specific node id from its constant key"""
    return uuid.uuid5(package_id, str(ck))


class _ComponentMethod(enum.Enum):
    # lifecycle
    init = "init"
    clear = "clear"
    interp = "interp"
    validate = "validate"
    updated = "updated"
    track = "track"
    untrack = "untrack"
    visit = "visit"
    # extra
    call = "call"
    iter = "iter"
    aiter = "aiter"
    len = "len"
    getitem = "getitem"

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
_COMPONENT_INNER_METHODS: tuple[str, ...] = tuple(m.inner for m in _ComponentMethod)
_FORBIDDEN_COMPONENT_METHODS = (
    tuple(m.self for m in _ComponentMethod)
    + tuple(m.rec for m in _ComponentMethod)
    + ("__post_init__", "__del__")
)
_COMPONENT_METHODS: dict[[_ComponentMethod, type["Node"]], Any] = {}
_COMPONENT_CALL_ORDER: tuple[str, ...] = ("Node",)  # ... the rest


def _sort_components_in_call_order(
    components: Collection[type["Node"]],
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


@cached(cache={}, key=lambda components, method, concrete_key: f"{concrete_key}.{method.name}")
def _get_component_methods(
    components: Collection[type["Node"]], method: _ComponentMethod, concrete_key: str
) -> tuple[Callable, ...]:
    """Get the actually implemented methods in the given components in call order."""
    methods = []
    for component in _sort_components_in_call_order(components):
        if _COMPONENT_METHODS.get((method, component), None) is not None:
            methods.append(getattr(component, method.inner))
    return tuple(methods)


def _process_struct_base_cls(
    cls: Union[type["Node"], type["Struct"]],
    dynamic_components: tuple[type["Node"], ...] = (),
    reserved: set[str | int] = None,
    is_in_package: bool = False,
    is_in_bench: bool = False,
    is_final: bool = False,
) -> tuple[type["Node"], dict[str, "Property"]]:
    """Process a struct base class and return the processed class and its properties."""
    metatype = METATYPE_PROPERTY.clone()
    metatype.component = cls
    properties_by_name: dict[str, "Property"] = {METATYPE_PROPERTY.name: metatype}
    static_components: list[type["Node"] | type["Struct"]] = [cls]

    # check that no forbidden methods are defined in non-base classes
    CORE_TYPES = ("Struct", "Node")
    if cls.__name__ not in CORE_TYPES:
        for name in _FORBIDDEN_COMPONENT_METHODS:
            meth = getattr(cls, name, None)
            good_meths = (getattr(cls, name, None) for cls in (Struct, Node, Node))
            if meth is not None and meth not in good_meths:
                raise ValueError(f"forbidden method {name} defined in {cls}")

    # collect static components from class hierarchy
    for base in cls.__bases__:
        if base.__name__ in ("Struct", "Node", "ABC"):
            continue
        if hasattr(base, "__properties__"):
            base: type["Struct"]
            static_components.append(base)
            for gp in base.__static_components__:
                if gp.__name__ not in CORE_TYPES and gp not in static_components:
                    static_components.append(gp)
    if cls.__name__ not in ("Struct", "Node"):
        if issubclass(cls, Node):
            static_components.append(Node)
            static_components.append(Struct)
        elif issubclass(cls, Struct):
            static_components.append(Struct)
        else:
            raise ValueError(f"invalid struct base {cls}")

    # collect properties from this
    declared_properties: dict[str, Property] = {}
    for name, prop in list(cls.__dict__.items()):
        if (
            name.startswith("__")
            or type(prop).__name__.startswith("_")
            or inspect.ismethod(prop)
            or inspect.isfunction(prop)
            or isinstance(prop, property)
            or isinstance(prop, classmethod)
            or isinstance(prop, staticmethod)
            or type(prop) == functools.cached_property
        ):
            continue  # ignore reserved names and non-fields
        if not isinstance(prop, Property):
            raise TypeError(f"{cls.__name__}.{name} is not a NodeProperty: {prop} ({type(prop)})")
        prop.name = intern(name)
        prop.component = cls
        prop.py_type_raw = cls.__annotations__.get(name, None)
        properties_by_name[name] = prop
        declared_properties[name] = prop
    cls.__declared_properties__ = frozendict(declared_properties)
    cls.__own_properties__ = frozendict(properties_by_name)  # remember 'own' properties

    # collect properties from all components (static and dynamic, least to most specific)
    is_node_base = cls.__name__ in ("Node",)
    is_struct_base = cls.__name__ == "Struct"
    is_node = not is_struct_base and (is_node_base or issubclass(cls, Node))
    reserved_properties: set[str | int] = set(reserved or ())
    for component in chain(reversed(static_components), reversed(dynamic_components)):
        for name, prop in component.__own_properties__.items():
            existing = properties_by_name.get(name, None)
            # override parent & id with more specific values
            if existing is None or name.startswith("parent") or existing.id is UNSET:
                if prop.is_runtime_only or component not in dynamic_components:
                    prop = prop.clone()
                    prop.component = cls
                    properties_by_name[name] = prop
                else:
                    pass  # ignore
            elif not prop._equals_type(existing):
                if existing.ignore_conflicts:
                    continue
                raise ValueError(f"property conflict '{name}': {prop!r}, {existing!r}")
            if not is_node and prop.is_graph_reference:
                raise ValueError(f"non-node {cls} has node-only relation {prop}")
        reserved_properties.update(component.__reserved_properties__)
    cls.__reserved_properties__ = frozenset(reserved_properties)

    # contribute extra properties
    for prop in tuple(properties_by_name.values()):
        prop: Property
        # collect any extra contributed properties
        if prop.struct_type == StructType.PROPERTY_REFERENCE or prop.reference_kind in (
            NodeReferenceKind.PARENT,
            NodeReferenceKind.ANCESTOR,
            NodeReferenceKind.REGULAR,
        ):
            for p in prop._contribute_ptrs():
                if p.name in properties_by_name:
                    raise ValueError(
                        f"property conflict '{p.name}': {p!r}, {properties_by_name[prop.name]!r}"
                    )
                properties_by_name[p.name] = p
                if not p.is_computed:
                    setattr(cls, p.name, p)

    # create class (map properties to dataclass fields)
    # TODO @Cleanup: the ck/package/bench property removal is a bit hacky & confusing
    for name, prop in list(properties_by_name.items()):
        # remove 'bench'/'package' ancestor property if not actually a descendant :MagicNodeProps
        if ((not is_in_bench or cls.__name__ == "Bench") and prop.name == "bench") or (
            (not is_in_package or cls.__name__ == "Package") and prop.name == "package"
        ):
            attr = None
            del properties_by_name[name]
            # remove contributed reference keys too
            for key in prop.reference_ptrs:
                del properties_by_name[key.name]
        # remove node ck (is == id if outside a package) :MagicNodeProps
        elif prop.name == "ck" and is_node and (not is_in_package or cls.__name__ == "Package"):
            attr = _node_ck_from_id_prop(prop)
            del properties_by_name[name]

        # map property to class attribute or dataclass field
        elif not is_final:
            attr = None  # only set attributes in final class
        elif prop.reference_kind == NodeReferenceKind.ANCESTOR and not is_node_base:
            if prop.reference_source is None:  # actual ancestor property
                attr = _node_computed_ancestor_prop(prop)
            else:  # wired pointer to ancestor property
                attr = _node_computed_ancestor_ptr_prop(prop)
        elif prop.is_computed or not prop.is_runtime:
            attr = UNSET
        elif prop.default is not UNSET:
            attr = dataclasses.field(default=prop.default)
        elif prop.default_factory is not None:
            attr = dataclasses.field(default_factory=prop.default_factory)
        else:
            attr = _required_prop(prop)
        # set attribute and annotation accordingly
        if attr is not UNSET:
            setattr(cls, name, attr)
        if isinstance(attr, dataclasses.Field):
            cls.__annotations__[name] = prop.py_type_raw
        elif name in cls.__annotations__:
            del cls.__annotations__[name]
        # also set extra computed reference properties
        if prop.reference_kind and not prop.reference_source and prop.reference_wired_ptr:
            for postfix, ref_key, ptr_key in (
                ("id", "id", "id"),
                ("ck", "ck", "ck"),
                ("type", "metatype", "type"),
            ):
                computed_prop = _node_ref_computed_prop(
                    ref_key, ptr_key, prop, prop.reference_wired_ptr
                )
                setattr(cls, prop.name + "_" + postfix, computed_prop)

    # collect methods implemented in this class (specifically)
    for meth_type in _ComponentMethod:
        meth = getattr(cls, meth_type.inner, None)
        if meth is not None and not any(
            meth is getattr(base, meth_type.inner, None) for base in cls.__bases__
        ):
            _COMPONENT_METHODS[(meth_type, cls)] = meth

    # register components and index properties
    cls.__static_components__ = tuple(static_components)
    cls.__dynamic_components__ = tuple(dynamic_components or ())
    cls.__properties__ = frozendict(properties_by_name)
    properties_by_id: dict[int, Property] = {}
    for prop in properties_by_name.values():
        if prop.id is not None and prop.id is not UNSET and not prop.reference_source:
            existing = properties_by_id.get(prop.id, None)
            if existing is not None:
                raise ValueError(f"property id conflict: {prop!r}, {existing!r}")
            properties_by_id[prop.id] = prop
    props = properties_by_name.values()
    cls.__properties_by_id__ = frozendict(properties_by_id)
    cls.__properties_name_by_id__ = frozendict(
        {p.id: p.name for p in properties_by_id.values() if p.id is not None}
    )
    cls.__tracked_properties__ = frozendict({p.name: p for p in props if not p.is_runtime_only})
    cls.__internal_properties__ = frozendict({p.name: p for p in props if p.is_internal})
    cls.__reference_properties__ = frozendict(
        {p.name: p for p in props if p.reference_types or p.is_property_reference}
    )
    cls.__sensitive_properties__ = frozendict({p.name: p for p in props if p.is_sensitive})
    cls.__struct_properties__ = frozendict({p.name: p for p in props if p.is_struct})
    cls.__value_properties__ = frozendict({p.name: p for p in props if p.is_value_runtime})
    cls.__properties_in_order__ = tuple(sorted(properties_by_id.values(), key=lambda p: p.id))
    for i, prop in enumerate(cls.__properties_in_order__):
        prop.ord = i
        if prop.reference_wired_ptr:
            prop.reference_wired_ptr.ord = i
        for p in prop.reference_stored_ptrs or ():
            p.ord = i
        if prop.reference_source:
            prop.reference_source.ord = i
    cls.__properties_id_in_order__ = tuple(p.id for p in cls.__properties_in_order__)
    cls.__max_property_ord__ = len(cls.__properties_in_order__) - 1
    cls.__properties_mask__ = bitarray(cls.__max_property_ord__ + 1)
    cls.__properties_mask__.setall(True)

    # TODO @Performance!: use slots for Struct/Node and in wire types (StructData/NodeData/...)
    #  Using slots for our structs bit trickier than it seems because
    #   1) we use dynamic props in Blocks (for now?)
    #   2) lack of betterproto support (unclear how challenging it would be to add)

    # transform class
    cls = dataclass(cls, slots=False, repr=False, eq=False)  # type: ignore
    for prop in props:  # update reference to 'new' class
        prop.component = cls

    return cls, properties_by_name


def struct_component(
    cls: Optional[type] = None,
    struct_type: StructType = None,
    reserved: set[str | int] = None,
    is_final: bool = False,
):
    """
    Mark a class as a struct component (or concrete struct for a StructType).
    """

    @dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
    def decorate(cls):
        cls, properties = _process_struct_base_cls(cls=cls, reserved=reserved, is_final=is_final)

        # register struct
        if struct_type:
            cls.metatype = struct_type
            if struct_type in STRUCT_CLASS_BY_TYPE:
                raise ValueError(
                    f"struct class conflict for {struct_type}: {cls}, {STRUCT_CLASS_BY_TYPE[struct_type]}"
                )
            STRUCT_CLASS_BY_TYPE[struct_type] = cls
        return cls

    if cls is not None:
        return decorate(cls)
    return decorate


def struct(
    struct_type: StructType,
    reserved: set[str | int] = None,
    index_in_search: bool = False,
    identifier: IdentifierType | None = None,
):
    @dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
    def decorate(cls):
        cls = struct_component(cls, struct_type=struct_type, reserved=reserved, is_final=True)
        cls.__is_indexed_in_search__ = index_in_search
        cls.__identifier_type__ = identifier
        return cls

    return decorate


def node_component(
    cls: Optional[type] = None,
    node_type: NodeType = None,
    passthrough: tuple[tuple[str, "_Passthrough"]] = (),
    dynamic_components: tuple[type["Node"], ...] = (),
    reserved: set[str | int] = None,
    is_in_package: bool = False,
    is_in_bench: bool = False,
    is_final: bool = False,
):
    """
    Mark a class as a node component (or concrete node for a NodeType).
    """

    @dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
    def decorate(cls):
        cls, properties = _process_struct_base_cls(
            cls=cls,
            dynamic_components=dynamic_components,
            reserved=reserved,
            is_in_package=is_in_package,
            is_in_bench=is_in_bench,
            is_final=is_final,
        )
        cls.__passthrough_targets__ = passthrough
        # register node properties
        props = properties.values()
        list_properties: dict[str, Property] = {}
        list_properties_by_child: dict[NodeType, list[Property]] = defaultdict(list)
        for prop in properties.values():
            if prop.reference_kind == NodeReferenceKind.CHILD:
                if cls.__name__ != "Node" and not issubclass(cls, Node) and node_type is not None:
                    raise ValueError(f"{cls} is not a Node for {prop}")
                list_properties[prop.name] = prop
                for ref_t in prop.reference_types:
                    list_properties_by_child[ref_t].append(prop)
        cls.__list_properties__ = frozendict(list_properties)
        cls.__list_properties_by_child__ = frozendict(list_properties_by_child)
        cls.__ancestor_properties__ = frozendict(
            {p.name: p for p in props if p.reference_kind == NodeReferenceKind.ANCESTOR}
        )

        # register as concrete node class for node_type
        if node_type:
            cls.metatype = node_type
            if node_type in NODE_CLASS_BY_TYPE:
                raise ValueError(
                    f"node class conflict for {node_type}: {cls}, {NODE_CLASS_BY_TYPE[node_type]}"
                )
            NODE_CLASS_BY_TYPE[node_type] = cls
        NODE_COMPONENT_CLASS_BY_NAME[cls.__name__] = cls

        return cls

    if cls is not None:
        return decorate(cls)

    return decorate


def node(
    node_type: NodeType,
    passthrough: tuple[tuple[str, "_Passthrough"]] = (),
    dynamic_components: tuple[type["Node"], ...] = (),
    stored: bool = True,
    stored_custom: bool = False,
    index_in_search: bool = False,
    local: bool = False,
    roots: tuple[NodeType, ...] = (NodeType.BENCH,),
    reserved: set[str | int] = None,
    indexes: tuple[Index, ...] = (),
    constraints: tuple[Constraint, ...] = (),
    unique_together: tuple[tuple[str, ...], ...] = (),
    identifier: IdentifierType | None = None,
):
    """Register a class as a concrete node for the given node type."""

    in_package = node_type in IN_PACKAGE_NODE_TYPES
    sub_package = node_type in SUB_PACKAGE_NODE_TYPES
    in_bench = node_type in IN_BENCH_NODE_TYPES
    sub_bench = node_type in SUB_BENCH_NODE_TYPES

    @dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
    def decorate(cls):
        cls = node_component(
            cls,
            node_type=node_type,
            passthrough=passthrough,
            dynamic_components=dynamic_components,
            reserved=reserved,
            is_in_package=in_package,
            is_in_bench=in_bench,
            is_final=True,
        )
        cls.__is_stored__ = stored
        cls.__is_stored_custom__ = stored_custom
        cls.__is_indexed_in_search__ = index_in_search
        cls.__is_local__ = local
        cls.__identifier_type__ = identifier

        extra_indexes: list[Index] = [*indexes]
        extra_constraints: list[Constraint] = [*constraints]
        for columns in unique_together:
            columns = tuple(sorted(columns))  # for consistency
            index_name = f"bench_idx_{'_'.join(columns)}"
            index = Index(index_name, type=IndexType.BTREE, is_unique=True, columns=columns)
            constraint = Constraint(
                index.inner_name,
                type=ConstraintType.UNIQUE,
                columns=columns,
                index=index.inner_name,
            )
            extra_indexes.append(index)
            extra_constraints.append(constraint)

        cls.__extra_indexes__ = tuple(extra_indexes)
        cls.__extra_constraints__ = tuple(extra_constraints)

        parent_property = cls.__properties__.get("parent", None)
        if parent_property is None:
            raise ValueError(f"node {cls} has no parent property")
        cls.__parent_property__ = parent_property
        cls.__roots__ = bytetuple(roots, enum_cls=NodeType)
        cls.__is_in_package__ = in_package
        cls.__is_sub_package__ = sub_package
        cls.__is_in_bench__ = in_bench
        cls.__is_sub_bench__ = sub_bench

        return cls

    return decorate


def iter_properties(*types: NodeType | StructType) -> Iterator[Property]:
    """Iterate over all properties of the given node/struct types."""
    for cls in chain(
        (NODE_CLASS_BY_TYPE.get(t) for t in types), (STRUCT_CLASS_BY_TYPE.get(t) for t in types)
    ):
        if cls is not None:
            yield from cls.__properties__.values()


NodeT = TypeVar("NodeT", bound="Node")


def _node_ck_from_id_prop(prop: Property) -> property:
    """Get ck from id (read-only)."""

    def get(self: NodeT) -> UUID:
        return self.id

    def set(self: NodeT, value: UUID):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get, set)


def _node_computed_ancestor_prop(prop: Property) -> property:
    """Computed ancestor property for Node instances."""

    if prop.is_ancestor_nearest:

        def get_nearest(self: NodeT) -> Optional[NodeT]:
            parent = self if prop.is_ancestor_self else self.parent
            while parent is not None:
                if parent.metatype in prop.reference_types:
                    return parent
                parent = parent.parent
            return None

        get = get_nearest
    else:

        def get_farthest(self: NodeT) -> Optional[NodeT]:
            parent = self if prop.is_ancestor_self else self.parent
            farthest = None
            while parent is not None:
                if parent.metatype in prop.reference_types:
                    farthest = parent
                parent = parent.parent
            return farthest

        get = get_farthest

    def set(self: NodeT, value: NodeT):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get, set)


def _required_prop(prop: Property):
    """Hacky way to make a field required when subclassing a dataclass with defaults."""

    _field = None

    def _raise_must_set():
        raise ValueError(f"{prop!r} must be set")

    _field = dataclasses.field(default_factory=_raise_must_set, metadata={"required": True})
    return _field


def _node_computed_ancestor_ptr_prop(prop: Property) -> property:
    def get_ancestor_ptr(self: NodeT) -> Optional["NodeReference"]:
        from bench.language.expression import NodeReference

        ancestor = getattr(self, prop.reference_source.name)
        if ancestor is None:
            return None
        else:
            return NodeReference.from_node(ancestor)

    def set(self: NodeT, value: "NodeReference"):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get_ancestor_ptr, set)


def _node_ref_computed_prop(
    key_ref: str, key_ptr: str, ref_prop: Property, ptr_prop: Property
) -> property:
    """Computed value from another property. If prop is not set, use backup prop."""

    is_array = ref_prop.is_array

    def get(self: NodeT) -> Optional[Any]:
        ref = getattr(self, ref_prop.name)
        ptr = getattr(self, ptr_prop.name)
        if is_array:
            if ref:
                return tuple(getattr(r, key_ref) for r in ref or ())
            else:
                return tuple(getattr(p, key_ptr) for p in ptr or ())
        else:
            if ref is not None:
                return getattr(ref, key_ref)
            elif ptr is not None:
                return getattr(ptr, key_ptr)
            else:
                return None

    def set(self: NodeT, value: Any):
        raise NotImplementedError(f"cannot set computed property {ref_prop!r}: {value!r}")

    return property(get, set)


def _make_self_method(
    method: _ComponentMethod,
    wraps,
    to_status: NodeStatus = None,
):
    """Creates method that calls _method_inner for all components in call order"""

    @functools.wraps(wraps)
    def self_method(self: "Struct", *args, _coerce: bool = True, _ignore: bool = False, **kwargs):
        if self._status == to_status:
            return
        for meth in _get_component_methods(self._components, method, self._instance_cache_key):
            meth(self, *args, **kwargs)
        if to_status is not None:
            self._status = to_status

    self_method.__name__ = method.self
    return self_method


def _make_inner_dunder_method(method: _ComponentMethod):
    """Creates method that proxies a builtin dunder method to the first _method_inner"""

    def inner_method(self: "Node", *args, **kwargs):
        meths = _get_component_methods(self._components, method, self._instance_cache_key)
        if len(meths) <= 1:  # includes this one
            raise RuntimeError(f"{self!r} does not support {method.name}")
        return meths[1](self, *args, **kwargs)

    inner_method.__name__ = method.inner
    return inner_method


class _Passthrough(enum.StrEnum):
    Full = "full"
    Scope = "scope"


@struct_component
class Struct(abc.ABC):
    """
    A non-node data structure, usually inside a node (which is the only way to store/retrieve it).
    Will track, track, etc. when we start using these in nodes.
    """

    metatype: ClassVar[StructType]  # type discriminator is field 0 if needed?
    __static_components__: ClassVar[tuple[type["Node"], ...]] = []
    __dynamic_components__: ClassVar[tuple[type["Node"], ...]] = ()
    __identifier_type__: ClassVar[IdentifierType | None] = None  # for named structs

    __properties__: ClassVar[dict[str, Property]] = {}
    __own_properties__: ClassVar[dict[str, Property]] = {}
    __declared_properties__: ClassVar[dict[str, Property]] = {}
    __properties_by_id__: ClassVar[dict[int, Property]] = {}
    __properties_name_by_id__: ClassVar[dict[int, str]] = {}
    __tracked_properties__: ClassVar[dict[str, Property]] = {}
    __internal_properties__: ClassVar[dict[str, Property]] = {}
    __reference_properties__: ClassVar[dict[str, Property]] = {}
    __sensitive_properties__: ClassVar[dict[str, Property]] = {}
    __struct_properties__: ClassVar[dict[str, Property]] = {}
    __value_properties__: ClassVar[dict[str, Property]] = {}
    __stored_properties__: ClassVar[dict[str, Property]] = {}
    __wired_properties__: ClassVar[dict[str, Property]] = {}
    __runtime_properties__: ClassVar[dict[str, Property]] = {}
    __reserved_properties__: ClassVar[set[int | str]] = set()
    __properties_in_order__: ClassVar[tuple[Property, ...]]
    __properties_id_in_order__: ClassVar[tuple[int, ...]]
    __max_property_ord__: ClassVar[int] = None
    __properties_mask__: ClassVar[bitarray] = None

    __is_struct__: ClassVar[bool] = True
    __is_node__: ClassVar[bool] = False
    __is_indexed_in_search__: ClassVar[bool] = False  # stored in local OS (only for logs really)

    _status: NodeStatus = p_runtime(default=None)

    # _node: Optional["Node"] = p_runtime(default=None) (for real Structs only)

    def __post_init__(self):
        if self._status is None:
            # not sure if this is totally right... where do we get :StructScope?
            self._status = NodeStatus.INTERP if _active_session.get() else NodeStatus.SOURCE
        self._init_self()

    def __content_str__(self) -> str:
        return ""

    @final
    def __str__(self):
        return self.__content_str__()

    @final
    def __repr__(self):
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str}>"
        else:
            return f"<{self.__class__.__name__} @ {id(self)}>"

    @classmethod
    def _unmask_properties_ids(cls, mask: bitarray) -> tuple[int, ...]:
        return tuple(cls.__properties_id_in_order__[i] for i in mask.search(True))

    @classmethod
    def _unmask_properties(cls, mask: bitarray) -> tuple[Property, ...]:
        return tuple(cls.__properties_in_order__[i] for i in mask.search(True))

    @classmethod
    def _mask_properties(cls, properties: Collection[Property]) -> bitarray:
        mask = bitarray(cls.__max_property_ord__ + 1)
        for prop in properties:
            mask[prop.ord] = True
        return mask

    @classmethod
    def _resolve_property(cls, ptr: "PropertyReference") -> Property | None:
        prop = cls._get_property(ptr)
        if prop is None:
            raise ValueError(f"unknown property reference: {ptr!r} in {cls!r}")
        return prop

    @classmethod
    def _get_property(cls, ptr: "PropertyReference") -> Property | None:
        if not ptr.references_type:
            return cls.__properties_by_id__.get(ptr.id, None)
        else:
            for prop in cls.__stored_properties__.values():
                if prop.id == ptr.id and prop.reference_types[0] == ptr.references_type:
                    return prop
            return None

    @property
    def _components(self) -> tuple[type["Node"], ...]:
        return self.__static_components__

    @property
    def _instance_cache_key(self) -> str:
        """Identifier for dynamic components"""
        return type(self).__name__

    def equals_content(self, other: Any) -> bool:
        """Checks if all wired properties of the two structs are equal (recursively)."""
        if other is None or self.metatype != other.metatype:
            return False
        for prop in self.__wired_properties__.values():
            self_value = getattr(self, prop.name)
            other_value = getattr(other, prop.name)
            if self_value != other_value and (
                prop.py_type_stripped != float
                or not math.isclose(self_value, other_value, rel_tol=1e-5)
            ):
                return False
        return True

    def __eq__(self, other):
        return self.equals_content(other)

    def _set_untracked(self, key, value):
        self.__dict__[key] = value

    # TODO @Broken: track in-struct edits (__setattr__) :StructScope

    def _init_inner(self):
        # init reference pointers if references are set :NodeReferences
        for prop in self.__reference_properties__.values():
            ref = getattr(self, prop.name)
            if prop.reference_wired_ptr is not None and ref:
                if prop.is_array:
                    self.__dict__[prop.reference_wired_ptr.name] = [r.to_ref() for r in ref]
                else:
                    self.__dict__[prop.reference_wired_ptr.name] = ref.to_ref()

    def _clear_inner(self, scope: Optional["Node"] = None):
        pass

    def _interp_inner(self, scope: "Node", on_notice: "NoticeHandler"):
        from bench.language.notice import NoticeType

        # resolve node references :NodeReferences
        for prop in self.__reference_properties__.values():
            if prop.is_wired or prop.is_stored or getattr(self, prop.name, None):
                continue  # already resolved
            ptr = getattr(self, prop.reference_wired_ptr.name)
            if ptr is None:
                continue

            # reference to built in property
            if prop.is_property_reference:
                if prop.is_array:
                    ptr = cast(list["PropertyReference"], ptr)
                    setattr(self, prop.name, [p.resolve() for p in ptr])
                else:
                    ptr = cast("PropertyReference", ptr)
                    setattr(self, prop.name, ptr.resolve())
            else:
                # regular node reference
                if prop.is_array:
                    ptr = cast(list["NodeReference"], ptr)
                    resolved = []
                    for p in ptr:
                        resolved = scope.lookup(p.id or p.ck)
                        if resolved is None:
                            on_notice(self, NoticeType.MISSING_REFERENCE, properties=(prop,))
                        resolved.append(resolved)
                    setattr(self, prop.name, resolved)
                else:
                    ptr = cast("NodeReference", ptr)
                    resolved = scope.lookup(ptr.id or ptr.ck)
                    if resolved is None:
                        on_notice(self, NoticeType.MISSING_REFERENCE, properties=(prop,))
                    setattr(self, prop.name, resolved)

    def _visit_inner(self, visitor: "NodeVisitor"):
        # visit node references :NodeReferences
        for prop in self.__reference_properties__.values():
            value = getattr(self, prop.name)
            if isinstance(value, Node):
                visitor.visit_reference(value)

    def _validate_inner(
        self, properties: Collection[Property], on_invalid: "ValidationHandler"
    ) -> None:
        """Validate cross-property constraints given the modified properties."""
        # since this is the root package, we also validate the properties directly
        for prop in properties:
            value = getattr(self, prop.name)
            if value is None:
                if prop.is_required:
                    on_invalid(self, f"{prop.name} is required", [prop])
            elif prop.custom_validate is not None:
                handler = PropertyValidationHandler(self, prop, on_invalid)
                valid = prop.validate(value, handler)
                if valid is False:
                    on_invalid(self, f"{prop.name}: invalid value", [prop])

    # struct has basic set of lifecycle methods (no index because no scope)
    _init_self = _make_self_method(_ComponentMethod.init, _init_inner)
    _clear_self = _make_self_method(_ComponentMethod.clear, _clear_inner, NodeStatus.SOURCE)
    _interp_self = _make_self_method(_ComponentMethod.interp, _interp_inner, NodeStatus.INTERP)
    _visit_self = _make_self_method(_ComponentMethod.visit, _visit_inner)
    _validate_self = _make_self_method(_ComponentMethod.validate, _validate_inner)

    def _walk_self(self) -> Iterable["Struct"]:
        yield self
        for prop in self.__struct_properties__.values():
            value = getattr(self, prop.name)
            if value:
                if prop.is_array:
                    for item in value:
                        yield from item._walk_self()
                else:
                    yield from value._walk_self()

    @staticmethod
    def _make_rec_method(method: _ComponentMethod, wraps):
        """Creates method that calls _method_self for all contained structs"""

        @functools.wraps(wraps)
        def rec_method(self: "Node", *args, **kwargs):
            # just walk self, every struct can only appear once
            for descendant in self._walk_self():
                getattr(descendant, method.self)(*args, **kwargs)

        rec_method.__name__ = method.rec
        return rec_method

    _clear_rec = _make_rec_method(_ComponentMethod.clear, _clear_self)
    _interp_rec = _make_rec_method(_ComponentMethod.interp, _interp_self)
    _visit_rec = _make_rec_method(_ComponentMethod.visit, _visit_self)

    def _to_data(self) -> AnyNodeData | AnyStructData:
        """Convert to wire format"""
        from bench.proto.wiring import pack_struct

        return pack_struct(self)


def _make_rec_method(
    method: _ComponentMethod, wraps, custom_kwargs: Callable[["Node"], dict] = None
):
    """Creates method that calls _method_self for self and all descendants"""

    @functools.wraps(wraps)
    def rec_method(self: "Node", *args, **kwargs):
        # graph has only host and inlined nodes, so this ignores out-of-line descendants (like records)
        descendants = self._root_graph.collect_descendants(self, recursive=True)
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
class Node(Struct, _NodeExpressionBase if TYPE_CHECKING else object):
    """
    A node in the Bench graph: it's a struct with an identity, so it can relate nodes in a graph.
    All nodes have a globally unique id (id).
    Source nodes may also have a constant identifier key (ck) used to derive the id per Package.
    """

    metatype: ClassVar[NodeType]
    __static_components__: ClassVar[tuple[type["Node"], ...]] = []
    __dynamic_components__: ClassVar[tuple[type["Node"], ...]] = ()
    __passthrough_targets__: ClassVar[tuple[tuple[str, _Passthrough]]] = ()
    __identifier_type__: ClassVar[IdentifierType | None] = None

    __ancestor_properties__: ClassVar[dict[str, Property]] = {}
    __list_properties__: ClassVar[dict[str, Property]] = {}
    __list_properties_by_child__: ClassVar[dict[NodeType, tuple[Property, ...]]] = defaultdict(list)
    __parent_property__: ClassVar[Property] = None

    __roots__: ClassVar[bytetuple[NodeType]] = UNSET
    __is_node__: ClassVar[bool] = True
    __is_in_bench__: ClassVar[bool] = UNSET  # part of a Bench
    __is_sub_bench__: ClassVar[bool] = UNSET  # part of a Bench (excludes Bench itself)
    __is_in_package__: ClassVar[bool] = UNSET  # part of a Package
    __is_sub_package__: ClassVar[bool] = UNSET  # part of a Package (excludes Package itself)
    __is_stored__: ClassVar[bool] = False  # stored in primary store (runtime or local)
    __is_stored_custom__: ClassVar[bool] = False  # custom storage logic (for records)
    __is_indexed_in_search__: ClassVar[bool] = False  # stored in local OS
    __is_local__: ClassVar[bool] = False  # stored in Bench-local DB (instead of global Bench DB)
    __extra_indexes__: ClassVar[tuple[Index, ...]] = ()  # extra indexes for PG
    __extra_constraints__: ClassVar[tuple[Constraint, ...]] = ()  # extra constraints for PG
    __table__: ClassVar[Table] = UNSET  # if stored regularly, set after finalization

    # 1-9: reserved for node identity
    # NOTE: ck/package/branch/bench only exist if __is_in_package__/__is_in_bench__ :MagicNodeProps
    id: UUID = p_system(2, default=None, require=True)
    ck: UUID = p_system(3, default=None, require=True)
    parent: Optional["Node"] = p_parent(4)
    # template: Optional["Node"] = node_template(5)
    package: "Package" = p_ancestor(6, NodeType.PACKAGE, require=True, store=True, wire=True)
    bench: "Bench" = p_ancestor(7, NodeType.BENCH, require=True, store=False, wire=False)
    source: NodeSource = p_system(8, default=NodeSource.STORE, store=False, wire=True, require=True)

    # 10-29: reserved for node tracking
    revision: int = p_system(10, default=0, require=True, primitive_type=PrimitiveType.INT64)
    created_at: datetime = p_system(11, default=None, require=True)
    updated_at: datetime = p_system(12, default=None, require=True)
    deleted_at: Optional[datetime] = p_system(13, default=None)
    archived_at: Optional[datetime] = p_system(14, default=None)
    # (only some nodes have some of these properties)
    # changed_at, active_at, ....
    # created_by, updated_by, changed_by, active_by, ...
    # computed_values: dict[int, ValueReference] | None = p_regular(20)
    # for instances of templates (with 'template' set)
    # set_values: list[int] | None = p_regular(21)

    # 30+ for 'user' node/struct properties
    # <... defined in concrete type ...>

    notices: NodeList["Notice"] = p_child(NodeType.NOTICE, NRel.CUMULATIVE)
    links: NodeList["Link"] = p_child(NodeType.LINK, NRel.CUMULATIVE)

    # the node graph is maintained at the highest root node (usually *the* root node, but may be detached)
    _graph: Union["NodeGraphBase", None] = p_runtime(default=None)
    _session: Optional["Session"] = p_runtime(default=None)
    _status: NodeStatus = p_runtime(default=None)
    _track: NodeTrackingLevel = p_runtime(default=NodeTrackingLevel.FULL)
    _is_new: bool = p_runtime(default=False)
    _updated_properties: bitarray | None = p_runtime(default=None)

    def __post_init__(self):
        # init ck/id/timestamps
        if self.__is_in_package__:
            if self.ck is None:
                self.ck = uuid4()
                self._is_new = True
            if self.id is None and self.is_attached:
                self._assign_id(self.package.id)
        elif self.id is None:
            self.id = uuid4()
            self._is_new = True
        if self.created_at is None:
            now = utcnow_with_tz()
            self.created_at = now
            self.updated_at = now
        # init session context
        if self._session is None and self._session is not UNSET:
            self._session = _active_session.get()
        if self._session and self._session is not UNSET and self._is_new and not self.parent:
            self._session._dangling_nodes_by_ck[self.ck] = self
        # init status
        if self._status is None:
            self._status = NodeStatus.INTERP if self._session is not None else NodeStatus.SOURCE
        self._init_self()
        # track if in session
        if self._status == NodeStatus.INTERP and self._session is not None:
            self._interp_self(self, on_notice=self._on_notice)
            self._track_self(self._session)

    @property
    def _components(self) -> tuple[type["Node"], ...]:
        return self.__static_components__

    @property
    def _dynamic_components(self) -> tuple[type["Node"], ...]:
        return ()

    @property
    def _instance_cache_key(self) -> str:
        """Identity for dynamic components"""
        return type(self).__name__

    @property
    def root(self) -> "Node":
        """Current root of this node. May not be *the* "right" root if detached."""
        parent = self
        while parent.parent is not None:
            parent = parent.parent
        return parent

    @property
    def _root_graph(self) -> "NodeGraphBase":
        root = self.root
        graph = root._graph
        assert graph is not None, f"no graph for root {root!r} (from {self!r})"
        return graph

    def _assign_id(self, package_id: UUID):
        assert package_id, f"cannot assign id to {self!r} without a package id"
        assert self.id is None, f"cannot assign id to {self!r} twice"
        assert self.ck is not None, f"cannot assign id to {self!r} without ck"
        self.id = get_node_id(package_id, self.ck)

    @final
    def __str__(self):  # noqa: override the default __str__ for nodes
        content_str = self.__content_str__()
        ident_str = self.py_ident
        if ident_str is None:
            ident_str = str(self.id)
        if content_str:
            content_str = f" ({content_str})"
        if self.archived_at is not None:
            if self.deleted_at is not None:
                status_str = " [archived, soft deleted]"
            else:
                status_str = " [archived]"
        elif self.deleted_at is not None:
            status_str = " [soft deleted]"
        else:
            status_str = ""
        if self.__parent_property__ is None:
            return f"'{ident_str}'{content_str}{status_str}"
        else:
            return f"'{self.absolute_path}'{content_str}{status_str}"

    @final
    def __repr__(self):  # noqa: override the default __repr__ for nodes
        return f"<{self.__class__.__name__} {str(self)}>"

    @property
    def is_attached(self) -> bool:
        if self.__is_in_package__:
            return self.parent is not None and self.package is not None
        elif self.__is_in_bench__:
            return self.parent is not None and self.bench is not None
        elif self.__roots__:
            return self.parent is not None
        else:
            return True

    @property
    def scope(self) -> Optional["Node"]:
        return self.parent

    @property
    def identifier_type(self) -> Optional[IdentifierType]:
        return self.__identifier_type__

    @property
    def bench_ident(self) -> Optional[str]:
        """The Bench identifier of this node (slug if exists, else name if exists)."""
        if self.identifier_type is None:
            return None
        if self.metatype == NodeType.PACKAGE and self.parent is not None:
            return self.parent.bench_ident  # package shares its Bench's identifier
        if "slug" in self.__properties__:
            slug = getattr(self, "slug")
            if slug:  # prefer slug as ident
                return slug
        return getattr(self, "name")

    @property
    def bench_path_ident(self) -> Optional[str]:
        """The Bench *path* identifier of this node (prefers bench_ident, ck/id filter otherwise)"""
        bench_ident = self.bench_ident
        if bench_ident is not None:
            return bench_ident
        elif "ck" in self.__properties__:
            return f"[ck={self.ck}]"
        else:
            return f"[id={self.id}]"

    @property
    def py_ident(self) -> Optional[str]:
        """The standardized python identifier of this node. Derived from slug or name."""
        identifier_type = self.identifier_type
        if identifier_type is None:
            return None
        if self.metatype == NodeType.PACKAGE and self.parent is not None:
            return self.parent.py_ident
        if "slug" in self.__properties__:
            slug = getattr(self, "slug", None)
            if slug:  # prefer slug as ident
                return slug
        name = getattr(self, "name", None)
        if name is None:
            return None
        return to_casing(name, PYTHON_CASING[identifier_type])

    @property
    def absolute_path(self) -> str:
        if self.__parent_property__ is None or not self.__parent_property__.reference_types:
            return self.bench_ident
        elif self.parent is None:
            return f"<detached>/{self.bench_path_ident}"
        else:
            path_segments: list[str] = []
            current = self
            while True:
                # skip bench (same path as pkg)
                path_segments.append(current.bench_path_ident)
                next_parent = current.parent
                has_next = next_parent is not None and next_parent.metatype != NodeType.BENCH
                if not has_next:
                    break
                if current.metatype == NodeType.FIELD:
                    path_segments.append(".")
                elif current.metatype != NodeType.BLOCK and next_parent.metatype == NodeType.BLOCK:
                    path_segments.append(":")
                else:
                    path_segments.append("/")
                current = next_parent

            path_segments.reverse()
            path = "".join(path_segments)
            return path

    @property
    def session(self) -> "Session":
        """Access the session, error-ing if there is none."""
        if self._session is None:
            raise RuntimeError(f"no active session for {self!r}")
        return self._session

    @session.setter
    def session(self, session: Optional["Session"]):
        self._session = session

    def to_ref(self) -> "NodeReference":
        from bench.language.expression import NodeReference

        return NodeReference.from_node(self)

    def __eq__(self, other: "Node"):
        return self.metatype == other.metatype and self.id == other.id and self.ck == other.ck

    def __hash__(self):
        return hash(self.id)

    @property
    def is_extant(self):
        return self.archived_at is None and self.deleted_at is None

    @property
    def is_archived(self) -> bool:
        return self.archived_at is not None or self.parent is not None and self.parent.is_archived

    @property
    def is_soft_deleted(self) -> bool:
        return (
            self.deleted_at is not None or self.parent is not None and self.parent.is_soft_deleted
        )

    def move_to(
        self,
        parent: "Node",
        after: Optional["Node"],
        before: Optional["Node"],
        order_key: str = None,
    ):
        raise NotImplementedError

    def delete(self):
        """Soft delete this node."""
        assert not self.is_soft_deleted, f"{self!r} is already deleted"
        raise NotImplementedError

    def restore(self):
        """Restore this node from soft deletion."""
        assert self.is_soft_deleted, f"{self!r} is not deleted"
        raise NotImplementedError

    def hard_delete_forever(self):
        """Hard delete this node. Forever. Irreversibly."""
        raise NotImplementedError

    def __setattr__(self, key, value):
        """Sets *any* attribute on this node (incl. slots)."""
        is_tracked = self._status == NodeStatus.TRACKED
        prop = self.__properties__.get(key)
        if prop is not None:
            if prop.reference_kind == NodeReferenceKind.CHILD:
                attr = getattr(self, key)
                if attr is None:  # initial set
                    return object.__setattr__(self, key, value)
                else:
                    return attr.set(value)  # has its own set
            elif prop.is_runtime_only:  # untracked
                return object.__setattr__(self, key, value)

            if is_tracked:
                # validated set
                prev = getattr(self, key)
                object.__setattr__(self, key, value)
                try:
                    self._validate_self([prop], on_invalid=on_invalid_raise)
                    self._updated_self((prop,))
                except ValidationError:  # reset on error
                    object.__setattr__(self, key, prev)
                    raise
            else:
                object.__setattr__(self, key, value)

            if prop.reference_wired_ptr:
                # update reference pointer  :NodeReferences
                from bench.language.expression import NodeReference

                object.__setattr__(
                    self, prop.reference_wired_ptr.name, NodeReference.from_node(value)
                )
            if self._session is not None and not self._is_new:
                # update in session
                if self._updated_properties is None:
                    self._updated_properties = bitarray(self.__max_property_ord__ + 1)
                self._updated_properties[prop.ord] = True
                self._session.update(self, properties=(prop,))
            return

        if is_tracked:
            # also try first full passthrough target (if any)
            for target, mode in self.__passthrough_targets__:
                target = getattr(self, target)
                if mode == _Passthrough.Full:
                    setattr(target, key, value)
                    return  # success

        # report set error with additional info
        candidates = {p.name: p for p in self.__properties__.values() if not p.is_computed}
        if isinstance(self, Node):
            candidates.update(cast(Node, self)._get_children_by_ident())
        did_you_mean = did_you_mean_str(candidates, key)
        raise AttributeError(f"Cannot set '{key}' on {self!r}. {did_you_mean}")

    def __getattr__(self, item):
        # we're using slots so this is not an instance attribute

        attr = UNSET
        # prefer components methods
        for component in self._components:
            attr = getattr(component, item, UNSET)
            if attr is not UNSET:
                break
        # check passthrough targets if tracked in session
        if attr is UNSET and self._session is not None:
            for target, mode in self.__passthrough_targets__:
                target = getattr(self, target)
                if mode == _Passthrough.Full:
                    attr = getattr(target, item, UNSET)
                elif mode == _Passthrough.Scope:
                    assert isinstance(target, NodeList), f"invalid scope passthrough: {attr!r}"
                    attr = target.get(item) or UNSET
                if attr is not UNSET:
                    break
        # attribute could be property, method, or just plain value
        if attr is not UNSET:
            if isinstance(attr, property):
                return attr.fget(self)
            elif not isinstance(attr, Node) and callable(attr) and not inspect.ismethod(attr):
                return functools.partial(attr, self)
            else:
                return attr

        # report get error with additional info
        candidates = {
            # own properties
            **{
                p.name: p
                for p in self.__properties__.values()
                if p.reference_kind or not p.is_runtime_only
            },
            # public methods
            **{m: None for m in dir(self) if not m.startswith("_")},
        }
        did_you_mean = did_you_mean_str(candidates, item)
        raise AttributeError(f"{self!r} has no attribute '{item}'. {did_you_mean}")

    def _walk_structs(self) -> Iterable["Struct"]:
        for prop in self.__struct_properties__.values():
            value = getattr(self, prop.name)
            if value is None:
                continue
            elif not prop.is_array:
                yield from value._walk_self()
            elif len(value) > 0:
                for item in value:
                    yield from item._walk_self()

    # abstract :ComponentMethods in addition to Struct

    def _on_notice(
        self,
        subject: "Node",
        type: "NoticeType",
        message: Optional[str] = None,
        path: Optional["FieldPath"] = None,
        properties: Optional[list[Property] | tuple[Property, ...]] = None,
    ) -> None:
        subject.notices.create(type=type, message=message, path=path, properties=properties)

    def _to_data_wrapped(self) -> SomeNodeData:
        """To wire format, wrapped in the generic any node container."""
        from bench.proto.wiring import pack_node, wrap_some_node

        return wrap_some_node(pack_node(self))

    def _init_inner(self) -> None:
        if self.parent is None:
            # if we're not in a graph, start a new one
            if NodeType.BENCH in self.__roots__:
                self._graph = DetachedNodeGraph()
            else:
                self._graph = NodeGraph()
            self._graph.add(self)

    def _clear_inner(self, scope: Optional["Node"] = None):
        """Clear this node."""
        # clear all structs recursive
        for s in self._walk_structs():
            s._clear_rec()

    def _interp_inner(self, scope: "Node", on_notice: "NoticeHandler"):
        """Interp this node."""
        # interp all structs recursive
        for s in self._walk_structs():
            s._interp_rec(scope, on_notice)

    def _track_inner(self, session: "Session") -> None:
        """Track this object in the given session."""
        self._session = session
        self._status = NodeStatus.TRACKED

    def _untrack_inner(self) -> None:
        """Stop tracking this object."""
        self._session = None

    def _attached_inner(self) -> None:
        """Called when this node is attached to a package."""
        pass

    def _detached_inner(self) -> None:
        """Called when this node is detached from a package."""
        pass

    def _updated_inner(self, properties: Collection[Property]) -> None:
        """Called when this node is updated."""
        pass

    _call_inner = _make_inner_dunder_method(_ComponentMethod.call)
    _iter_inner = _make_inner_dunder_method(_ComponentMethod.iter)
    _aiter_inner = _make_inner_dunder_method(_ComponentMethod.aiter)
    _len_inner = _make_inner_dunder_method(_ComponentMethod.len)
    _getitem_inner = _make_inner_dunder_method(_ComponentMethod.getitem)

    __call__ = _call_inner
    __iter__ = _iter_inner
    __aiter__ = _aiter_inner
    __len__ = _len_inner
    __getitem__ = _getitem_inner

    def __bool__(self):
        return True  # allow truthy checks for nodes

    # final :ComponentMethods

    @final
    def _init_self(self):
        # init lists
        existing_lists: dict[str, Any] | None = None
        if len(self.__list_properties__) > 0:
            for name, prop in self.__list_properties__.items():
                existing = getattr(self, name, None)
                node_list = prop.list_type(self, prop)
                setattr(self, name, node_list)
                if prop.alias:
                    setattr(self, prop.alias, node_list)
                if existing and not isinstance(existing, NodeList):
                    if existing_lists is None:
                        existing_lists = {}
                    existing_lists[name] = existing

        # run actual init methods
        for meth in _get_component_methods(
            self._components, _ComponentMethod.init, self._instance_cache_key
        ):
            meth(self)

        # keep manually set node lists if passed in
        if existing_lists:
            was_interp = self._status >= NodeStatus.INTERP
            for name, existing in existing_lists.items():
                if existing and not isinstance(existing, NodeList):
                    getattr(self, name).extend(*existing)

        # validate if in session after all init are done
        if (
            self._status >= NodeStatus.INTERP
            and self._is_new
            and self._session is not None
            and self._session is not UNSET
        ):
            self._validate_self(self.__tracked_properties__.values(), on_invalid=on_invalid_raise)

    # node has extended set of lifecycle methods
    _clear_self = _make_self_method(_ComponentMethod.clear, _clear_inner, NodeStatus.SOURCE)
    _interp_self = _make_self_method(_ComponentMethod.interp, _interp_inner, NodeStatus.INTERP)
    _track_self = _make_self_method(_ComponentMethod.track, _track_inner, NodeStatus.TRACKED)
    _untrack_self = _make_self_method(_ComponentMethod.untrack, _untrack_inner, NodeStatus.INTERP)
    _updated_self = _make_self_method(_ComponentMethod.updated, _updated_inner)
    _visit_self = _make_self_method(_ComponentMethod.visit, Struct._visit_inner)

    def _walk_rec(self) -> Iterable["Node"]:
        yield self
        if self.metatype in HAS_CHILD_NODE_TYPES:
            yield from self._root_graph.collect_descendants(self, recursive=True)

    _clear_rec = _make_rec_method(
        _ComponentMethod.clear, _clear_self, custom_kwargs=lambda n: dict(scope=n)
    )
    _interp_rec = _make_rec_method(
        _ComponentMethod.interp,
        _interp_self,
        custom_kwargs=lambda n: dict(scope=n, on_notice=n._on_notice),
    )
    _visit_rec = _make_rec_method(_ComponentMethod.visit, _visit_self)
    _validate_rec = _make_rec_method(
        _ComponentMethod.validate,
        Struct._validate_self,
        custom_kwargs=lambda n: dict(
            properties=n.__tracked_properties__.keys(), on_invalid=on_invalid_raise
        ),
    )
    _track_rec = _make_rec_method(_ComponentMethod.track, _track_self)
    _untrack_rec = _make_rec_method(_ComponentMethod.untrack, _untrack_self)


@_on_completing_setup
def _add_node_expression_base():
    from bench.language.expression import _NodeExpressionBase

    for name, attr in _NodeExpressionBase.__dict__.items():
        if name not in Property.__dict__ and name not in ("__annotations__", "__dict__"):
            setattr(Node, name, attr)


LINK_TARGET_NODE_TYPES: tuple[NodeType, ...] = tuple(
    nt
    for nt in NODE_TYPES
    if NodeType.PACKAGE.id < nt.id < NodeType.SESSION.id and nt != NodeType.LINK
)
LINK_PARENT_NODE_TYPES: tuple[NodeType, ...] = (NodeType.PACKAGE, NodeType.BLOCK)


@node(NodeType.LINK)
class Link(Node):
    """
    A reference to another node in some graph.
    The referenced subtree is inlined on access.
    The reference may be indirect through a value somewhere (which should point to a node).
    """

    parent: Node = p_parent(4, *LINK_PARENT_NODE_TYPES)
    reference: Optional[Node] = p_regular(
        30, array=False, references=LINK_TARGET_NODE_TYPES, require=False
    )
    computed_reference: Optional["ValueReference"] = p_regular(
        31, require=False, array=False, struct=StructType.VALUE_REFERENCE
    )
    order_key: Optional[str] = p_internal(32, default=None)


@node(NodeType.SKIP, stored=False)
class Skip(Node):
    """A reference to another node in some graph that wasn't available for some reason (usually permissions)."""

    parent: Node = p_parent(4, *LINK_PARENT_NODE_TYPES)
    reference: Optional[Node] = p_regular(
        30, array=False, references=LINK_TARGET_NODE_TYPES, require=True
    )
    order_key: Optional[str] = p_internal(31, default=None)

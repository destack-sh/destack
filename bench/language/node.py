import abc
import base64
import dataclasses
import enum
import functools
import inspect
import math
import secrets
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
    Generic,
    Iterable,
    Optional,
    Self,
    Type,
    TypeVar,
    Union,
    cast,
    dataclass_transform,
    final,
)
from uuid import UUID, uuid4

import structlog
from bitarray import bitarray
from cachetools import cached

from bench.language.const import (
    IN_BENCH_NODE_TYPES,
    IN_PACKAGE_NODE_TYPES,
    NODE_TYPES,
    SUB_BENCH_NODE_TYPES,
    SUB_PACKAGE_NODE_TYPES,
    TK_LENGTH_BYTES,
    UNSET,
    InterpStatus,
    NodeSource,
    NodeType,
    ReferenceKind,
    StructType,
    _active_session,
)
from bench.language.graph import DetachedNodeGraph, NodeDataGraph, NodeGraph, NodeList, ValueList
from bench.language.property import (
    _PROPERTY_SPECIFIERS,
    METATYPE_PROPERTY,
    Property,
    p_internal,
    p_node_ancestor,
    p_node_child,
    p_node_parent,
    p_regular,
    p_runtime,
    p_struct_parent,
    p_system,
)
from bench.language.setup import (
    HAS_CHILD_NODE_TYPES,
    NODE_CLASS_BY_TYPE,
    NODE_COMPONENT_CLASS_BY_NAME,
    STRUCT_CLASS_BY_TYPE,
)
from bench.language.validation import (
    PropertyValidationHandler,
    ValidationError,
    ValidationHandler,
    on_invalid_raise,
)
from bench.proto.wire import AnyNodeData, AnyStructData, NodeReferenceData, SomeNodeData
from bench.sql.core import Constraint, ConstraintType, Index, IndexType, PrimitiveType, Table
from bench.utils.casing import PYTHON_CASING, IdentifierType, to_casing
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import bytetuple, did_you_mean_str
from bench.utils.utils import frozendict

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Expression,
        Field,
        NodeReference,
        NoticeType,
        Package,
        PropertyReference,
        Run,
        Session,
        User,
        Value,
    )
    from bench.language.notice import NoticeHandler, NoticeOptions
    from bench.language.query import QueryBuilder

# pyright: reportIncompatibleVariableOverride=false,reportIncompatibleMethodOverride=false

logger = structlog.get_logger(__name__)


def new_struct_id() -> int:
    id = secrets.randbits(32)
    if id < 0:
        id = -id
    return id


new_node_id = uuid4


def get_tk_from_ck(ck: UUID) -> str:
    """Gets the stable across templates first 6 bytes of the ck."""
    return ck.bytes[:TK_LENGTH_BYTES].hex()


def get_tk_from_ptr(ptr: "NodeReference") -> str:
    if ptr.ck:
        return ptr.ck.bytes[:TK_LENGTH_BYTES].hex()
    elif ptr.id:
        return ptr.id.bytes[:TK_LENGTH_BYTES].hex()
    else:
        raise ValueError(f"invalid ptr: {ptr!r}")


def get_tk_from_ptr_maybe(ptr: Optional["NodeReference"]) -> Optional[str]:
    if ptr is None:
        return None
    elif ptr.ck:
        return ptr.ck.bytes[:TK_LENGTH_BYTES].hex()
    elif ptr.id:
        return ptr.id.bytes[:TK_LENGTH_BYTES].hex()
    else:
        raise ValueError(f"invalid ptr: {ptr!r}")


def get_tk_b64_from_ptr(ptr: "NodeReference") -> str:
    if ptr.ck:
        return base64.b64encode(ptr.ck.bytes[:TK_LENGTH_BYTES]).decode()
    elif ptr.id:
        return base64.b64encode(ptr.id.bytes[:TK_LENGTH_BYTES]).decode()
    else:
        raise ValueError(f"invalid ptr: {ptr!r}")


def pad_ck_from_tk_b64(sk_b64: str) -> UUID:
    """Pads the remainder with zeros"""
    bytes = base64.b64decode(sk_b64.encode()) + (16 - TK_LENGTH_BYTES) * b"\x00"
    return UUID(bytes=bytes)


def derive_source_node_id(package_id: UUID, ck: UUID):
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
        return f"_{self.value}_component"

    @property
    def self(self) -> str:
        return f"_{self.value}_self"

    @property
    def rec(self) -> str:
        return f"_{self.value}_rec"


# :NodeMethods
_FORBIDDEN_COMPONENT_METHODS = (
    tuple(m.self for m in _ComponentMethod)
    + tuple(m.rec for m in _ComponentMethod)
    + ("__post_init__", "__del__")
)
_COMPONENT_METHODS: dict[tuple[_ComponentMethod, type["Node"]], Any] = {}
_COMPONENT_CALL_ORDER: tuple[str, ...] = ("Node",)  # ... the rest


def _sort_components_in_call_order(
    components: Collection[type["Node"] | type["Struct"]],
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
    components: Collection[type["Node"] | type["Struct"]],
    method: _ComponentMethod,
    concrete_key: str,
) -> tuple[Callable, ...]:
    """Get the actually implemented methods in the given components in call order."""
    methods = []
    for component in _sort_components_in_call_order(components):
        if _COMPONENT_METHODS.get((method, component), None) is not None:
            methods.append(getattr(component, method.inner))
    return tuple(methods)


_StructT = TypeVar("_StructT", bound="Struct")


def _process_struct_base_cls(
    cls: type[_StructT],
    dynamic_components: tuple[type["Node"], ...] = (),
    reserved: set[str | int] | None = None,
    # for nodes only
    is_final: bool = False,
    is_sub_package: bool = False,
    is_sub_bench: bool = False,
    is_variable_root: bool = False,
    is_local: bool = False,
    no_ck: bool = False,
    # for structs only
    is_inlined: bool = False,
) -> tuple[type[_StructT], dict[str, "Property"]]:
    """Process a struct base class and return the processed class and its properties."""
    is_node_base = cls.__name__ in "Node"
    is_struct_base = cls.__name__ == "Struct"
    is_node = not is_struct_base and (is_node_base or issubclass(cls, Node))
    is_struct = not is_node
    metatype = METATYPE_PROPERTY.clone()
    metatype.component = cls
    properties_by_name: dict[str, "Property"] = {"metatype": metatype}

    # check that no forbidden methods are defined in non-base classes
    CORE_TYPES = ("Struct", "Node")
    if cls.__name__ not in CORE_TYPES:
        for name in _FORBIDDEN_COMPONENT_METHODS:
            meth = getattr(cls, name, None)
            good_meths = (getattr(cls, name, None) for cls in (Struct, Node, Node))
            if meth is not None and meth not in good_meths:
                raise ValueError(f"forbidden method {name} defined in {cls}")

    # collect static components from class hierarchy
    static_components: list[type["Node"] | type["Struct"]] = [cls]
    for base in cls.__bases__:
        if base.__name__ in ("ABC",):
            continue
        if hasattr(base, "__properties__"):
            base: type["Struct"]
            static_components.append(base)
            for grandparent in base.__static_components__:
                if grandparent not in static_components:
                    static_components.append(grandparent)

    # check components
    for component in chain(static_components[1:], dynamic_components):
        if component.__name__ in CORE_TYPES:
            continue  # ignore base classes
        if is_node and component.__is_struct_inlined__:
            raise ValueError(f"node {cls} has inlined struct {component}")
        if is_inlined and hasattr(component, "metatype") and not component.__is_struct_inlined__:
            raise ValueError(f"struct {cls} has non-inlined struct {component}")

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
    reserved_properties: set[str | int] = set(reserved or ())
    for component in chain(reversed(static_components), reversed(dynamic_components)):
        for name, prop in component.__own_properties__.items():
            # system struct identity is only for non-inlined structs :MagicProps
            #  (we remove it here because it conflicts with downstream props)
            if not is_struct and (prop.id == Struct.__properties__["order_key"].id):
                continue
            existing = properties_by_name.get(name, None)
            # override parent prop & id with more specific values
            if (
                existing is None
                or name == "id"
                or name.startswith("parent")
                or existing.id is UNSET
            ):
                if prop.is_ephemeral or component not in dynamic_components:
                    prop = prop.clone()
                    prop.component = cls
                    properties_by_name[name] = prop
                else:
                    pass  # ignore
            elif not prop._equals_type(existing):
                raise ValueError(f"property conflict '{name}': {prop!r}, {existing!r}")
            if not is_node and prop.is_tree_reference:
                raise ValueError(f"non-node {cls} has node-only relation {prop}")
        reserved_properties.update(component.__reserved_properties__)
    cls.__reserved_properties__ = frozenset(reserved_properties)

    if is_node and is_variable_root:
        # bench is optional in variable root types (since they can have other roots)
        properties_by_name["bench"].is_required = False

    # contribute extra properties
    for prop in tuple(properties_by_name.values()):
        prop: Property
        # collect any extra contributed properties
        if prop.reference_struct == StructType.PROPERTY_REFERENCE or prop.reference_kind in (
            ReferenceKind.NODE_PARENT,
            ReferenceKind.NODE_ANCESTOR_FIRST,
            ReferenceKind.NODE_ANCESTOR_ROOT,
            ReferenceKind.NODE_REGULAR,
            ReferenceKind.STRUCT_PARENT,
            ReferenceKind.PROPERTY,
        ):
            for p in prop._contribute_ptrs(is_inlined=is_inlined):
                if p.name in properties_by_name:
                    raise ValueError(
                        f"property conflict '{p.name}': {p!r}, {properties_by_name[prop.name]!r}"
                    )
                properties_by_name[p.name] = p
                if not p.is_computed:  # why is this needed?
                    setattr(cls, p.name, p)

    if is_final:
        # prune :MagicProps that shouldn't exist on this node type
        def _remove_magic_prop(name: str, delete_attr: bool = True):
            prop = properties_by_name.get(name)
            # property may be overwritten (like in PropertyReference.id)
            if prop is not None and prop.id < 30:
                del properties_by_name[name]
                if delete_attr:
                    try:
                        delattr(cls, name)
                    except AttributeError:
                        pass
                cls.__annotations__.pop(name, None)
                for contributed_prop in prop.contributed_props:
                    _remove_magic_prop(contributed_prop.name)

        if is_node and not is_sub_bench:
            if cls.__name__ == "Bench":
                cls.bench = _node_computed_ancestor_prop(properties_by_name["bench"])  # type: ignore
                _remove_magic_prop("bench", delete_attr=False)
            else:
                _remove_magic_prop("bench")
        if is_node and not is_sub_package:
            if cls.__name__ == "Package":
                cls.package = _node_computed_ancestor_prop(properties_by_name["package"])  # type: ignore
                _remove_magic_prop("package", delete_attr=False)
            else:
                _remove_magic_prop("package")
        if is_node and (no_ck or not is_sub_package):
            _remove_magic_prop("ck")
            setattr(cls, "ck", _node_ck_from_id_prop(properties_by_name["id"]))
        if is_struct and is_inlined:
            _remove_magic_prop("id")
            _remove_magic_prop("order_key")
            _remove_magic_prop("computed_properties")
            _remove_magic_prop("set_properties")

    # create class (map properties to dataclass fields)
    for name, prop in list(properties_by_name.items()):
        # only set attributes in final class to prevent conflicts
        if not is_final:
            attr = UNSET
        # map property to class attribute or dataclass field
        elif (
            prop.reference_kind == ReferenceKind.NODE_ANCESTOR_FIRST
            or prop.reference_kind == ReferenceKind.NODE_ANCESTOR_ROOT
        ) and not is_node_base:
            if prop.reference_source is None:  # actual ancestor property
                attr = _node_computed_ancestor_prop(prop)
            elif prop.name.endswith("_ptr"):  # wired pointer to ancestor property
                attr = _node_computed_ancestor_ptr_prop(prop)
            else:
                attr = UNSET  # will be set as extra computed property below
        elif prop.is_computed or not prop.is_runtime:
            attr = UNSET
        elif prop.default_factory is not None:
            attr = dataclasses.field(default_factory=prop.default_factory)
        elif prop.default is not UNSET:
            attr = dataclasses.field(default=prop.default)
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
        if prop.is_node_reference and not prop.reference_source:
            for postfix, ref_key, ptr_key in (
                ("id", "id", "id"),
                ("ck", "ck", "ck"),
                ("type", "metatype", "type"),
            ):
                if postfix == "type" and len(cast(tuple[NodeType, ...], prop.reference_nodes)) <= 1:
                    continue  # no need for type if only one possible node type
                computed_prop = _node_ref_computed_prop(
                    ref_key, ptr_key, prop, prop.reference_wired_ptr
                )
                setattr(cls, prop.name + "_" + postfix, computed_prop)

    # collect component methods implemented in this component
    for meth_type in _ComponentMethod:
        meth = getattr(cls, meth_type.inner, None)
        if meth is not None and not any(
            meth is getattr(base, meth_type.inner, None) for base in cls.__bases__
        ):
            _COMPONENT_METHODS[(meth_type, cls)] = meth  # type: ignore

    # register components and index properties
    cls.__static_components__ = tuple(static_components)  # type: ignore
    cls.__dynamic_components__ = tuple(dynamic_components or ())
    cls.__is_struct_inlined__ = is_inlined
    cls.__properties__ = frozendict(properties_by_name)
    properties_by_id: dict[int, Property] = {}
    for prop in properties_by_name.values():
        if prop.id is not None and prop.id is not UNSET and not prop.reference_source:
            existing = properties_by_id.get(prop.id, None)
            if existing is None:
                properties_by_id[prop.id] = prop
            elif not prop.reference_source:
                # contributed reference properties can share an id
                raise ValueError(f"property id conflict: {prop!r}, {existing!r}")
    props = properties_by_name.values()
    cls.__properties_by_id__ = frozendict(properties_by_id)
    cls.__properties_name_by_id__ = frozendict(
        {p.id: p.name for p in properties_by_id.values() if p.id is not None}
    )
    cls.__tracked_properties__ = frozendict(
        {
            p.name: p
            for p in props
            if not p.is_ephemeral
            and not p.is_autoset
            and not p.is_computed
            and not p.reference_source
        }
    )
    cls.__internal_properties__ = frozendict({p.name: p for p in props if p.is_internal})
    cls.__node_reference_properties__ = frozendict(
        {p.name: p for p in props if p.is_node_reference and not p.reference_source}
    )
    cls.__property_reference_properties__ = frozendict(
        {p.name: p for p in props if p.is_property_reference and not p.reference_source}
    )
    cls.__struct_reference_properties__ = frozendict(
        {p.name: p for p in props if p.is_struct_reference and not p.reference_source}
    )
    cls.__sensitive_properties__ = frozendict({p.name: p for p in props if p.is_sensitive})
    cls.__struct_properties__ = frozendict({p.name: p for p in props if p.is_struct})
    cls.__value_properties__ = frozendict({p.name: p for p in props if p.is_value_runtime})
    cls.__properties_in_order__ = tuple(sorted(properties_by_id.values(), key=lambda p: p.id))
    for i, prop in enumerate(cls.__properties_in_order__):
        prop.ord = i
        if prop.reference_wired_ptr:
            prop.reference_wired_ptr.ord = i
        for p in prop.reference_stored_props or ():
            p.ord = i
        if prop.reference_source:
            prop.reference_source.ord = i
    cls.__properties_id_in_order__ = tuple(p.id for p in cls.__properties_in_order__)
    cls.__max_property_ord__ = len(cls.__properties_in_order__) - 1
    cls.__properties_mask_set__ = bitarray(cls.__max_property_ord__ + 1)
    cls.__properties_mask_set__.setall(True)
    cls.__properties_mask_unset__ = bitarray(cls.__max_property_ord__ + 1)

    parent_property = cls.__properties__.get("parent", None)
    if (is_node or not is_inlined) and parent_property is None:
        raise ValueError(f"missing parent property for node {cls}")
    cls.__parent_property__ = parent_property

    # TODO :Performance!: use slots for Struct/Node and wire types (StructData/NodeData/...)
    #  Using slots everywhere is made trickier than it seems because
    #   1) some weird runtime errors
    #   2) we use dynamic props in Blocks (for now?)
    #   3) lack of betterproto support (unclear how challenging it would be to add)

    # transform class
    cls = dataclass(cls, slots=False, repr=False, eq=False)  # type: ignore
    for prop in props:  # update reference to 'new' class
        prop.component = cls
        # dataclass set the default value as a class attribute, but we don't want that
        #  (I can't figure out why they do that, the defaults are set in __init__ too?)
        if prop.default is not None and getattr(cls, prop.name, None) == prop.default:
            setattr(cls, prop.name, None)

    return cls, properties_by_name


_StructT = TypeVar("_StructT", bound="Struct")


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def struct_component(
    struct_type: StructType | None = None,
    reserved: set[str | int] | None = None,
    is_final: bool = False,
    is_inlined: bool = False,
):
    """
    Mark a class as a struct component (or concrete struct for a StructType).
    """

    def decorate(cls_in: Type[_StructT]) -> Type[_StructT]:
        cls, properties = _process_struct_base_cls(
            cls=cast(Any, cls_in), reserved=reserved, is_final=is_final, is_inlined=is_inlined
        )

        # register struct
        if struct_type:
            cls.metatype = struct_type
            if struct_type in STRUCT_CLASS_BY_TYPE:
                raise ValueError(
                    f"struct class conflict for {struct_type}: {cls}, {STRUCT_CLASS_BY_TYPE[struct_type]}"
                )
            STRUCT_CLASS_BY_TYPE[struct_type] = cls
        return cast(Type[_StructT], cls)

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def struct(
    struct_type: StructType,
    reserved: set[str | int] | None = None,
    inline: bool = False,
):
    def decorate(cls: Type[_StructT]) -> Type[_StructT]:
        cls = struct_component(
            struct_type=struct_type, reserved=reserved, is_final=True, is_inlined=inline
        )(cls)
        return cls

    return decorate


_NodeT = TypeVar("_NodeT", bound="Node")


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def node_component(
    node_type: NodeType | None = None,
    passthrough: tuple[tuple[str, "_Passthrough"], ...] = (),
    dynamic_components: tuple[type["Node"], ...] = (),
    reserved: set[str | int] | None = None,
    is_variable_root: bool = False,
    is_sub_package: bool = False,
    is_sub_bench: bool = False,
    is_final: bool = False,
    is_local: bool = False,
    no_ck: bool = False,
):
    """
    Mark a class as a node component (or concrete node for a NodeType).
    """

    def decorate(cls: Type[_NodeT]) -> Type[_NodeT]:
        cls, properties = _process_struct_base_cls(
            cls=cls,
            dynamic_components=dynamic_components,
            reserved=reserved,
            is_variable_root=is_variable_root,
            is_sub_bench=is_sub_bench,
            is_sub_package=is_sub_package,
            is_final=is_final,
            is_local=is_local,
            no_ck=no_ck,
        )
        cls.__passthrough_targets__ = passthrough

        # register node properties
        props = properties.values()
        list_properties: dict[str, Property] = {}
        list_properties_by_child: dict[NodeType, list[Property]] = defaultdict(list)
        for prop in properties.values():
            if prop.reference_kind == ReferenceKind.NODE_CHILD:
                if cls.__name__ != "Node" and not issubclass(cls, Node) and node_type is not None:
                    raise ValueError(f"{cls} is not a Node for {prop}")
                list_properties[prop.name] = prop
                for ref_t in prop.reference_nodes or ():
                    list_properties_by_child[ref_t].append(prop)
        cls.__node_list_properties__ = frozendict(list_properties)
        cls.__ancestor_properties__ = frozendict(
            {p.name: p for p in props if p.reference_kind == ReferenceKind.NODE_ANCESTOR_FIRST}
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

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def node(
    node_type: NodeType,
    passthrough: tuple[tuple[str, "_Passthrough"], ...] = (),
    dynamic_components: tuple[type["Node"], ...] = (),
    stored: bool = True,
    stored_custom: bool = False,
    index_in_search: bool = False,
    no_ck: bool = False,
    local: bool = False,
    roots: tuple[NodeType, ...] = (NodeType.BENCH,),
    reserved: set[str | int] | None = None,
    indexes: tuple[Index, ...] = (),
    constraints: tuple[Constraint, ...] = (),
    unique_together: tuple[tuple[str, ...], ...] = (),
    identifier: IdentifierType = IdentifierType.VARIABLE,
    id_factory: Callable[[], UUID] = new_node_id,
):
    """Register a class as a concrete node for the given node type."""

    in_package = node_type in IN_PACKAGE_NODE_TYPES
    sub_package = node_type in SUB_PACKAGE_NODE_TYPES
    in_bench = node_type in IN_BENCH_NODE_TYPES
    sub_bench = node_type in SUB_BENCH_NODE_TYPES

    def decorate(cls: Type[_NodeT]) -> Type[_NodeT]:
        cls = node_component(
            node_type=node_type,
            passthrough=passthrough,
            dynamic_components=dynamic_components,
            reserved=reserved,
            is_variable_root=len(roots) > 1,
            is_sub_bench=sub_bench,
            is_sub_package=sub_package,
            no_ck=no_ck,
            is_final=True,
            is_local=local,
        )(cls)
        cls.__is_stored__ = stored
        cls.__is_stored_custom__ = stored_custom
        cls.__is_indexed_in_search__ = index_in_search
        cls.__is_local__ = local
        cls.__identifier_type__ = identifier
        cls.__id_factory__ = id_factory

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

        cls.__roots__ = bytetuple(*roots, enum_cls=NodeType)
        cls.__is_in_package__ = in_package
        cls.__is_sub_package__ = sub_package
        cls.__is_in_bench__ = in_bench
        cls.__is_sub_bench__ = sub_bench

        return cls

    return decorate


NodeT = TypeVar("NodeT", bound="Node")


def _node_ck_from_id_prop(prop: Property) -> property:
    """Get ck from id (read-only)."""

    def get(self: Node) -> UUID:
        return self.id

    def set(self: Node, value: UUID):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get, set)


def _node_computed_ancestor_prop(prop: Property) -> property:
    """Computed ancestor property for Node instances."""

    if prop.reference_kind == ReferenceKind.NODE_ANCESTOR_FIRST:

        def get_ancestor_first(self: NodeT) -> Optional[NodeT]:
            parent = self
            while parent is not None:
                if prop.reference_nodes and parent.metatype in prop.reference_nodes:
                    return parent
                parent = parent.parent
            return None

        get = get_ancestor_first

    elif prop.reference_kind == ReferenceKind.NODE_ANCESTOR_ROOT:

        def get_ancestor_root(self: NodeT) -> Optional[NodeT]:
            parent = self.parent
            farthest = None
            while parent is not None:
                if prop.reference_nodes and parent.metatype in prop.reference_nodes:
                    farthest = parent
                parent = parent.parent
            return farthest

        get = get_ancestor_root

    else:
        raise ValueError(f"unexpected ancestor reference kind: {prop.reference_kind}")

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
    assert prop.reference_source is not None, f"no source for {prop!r}"

    def get_ancestor_ptr(self: "Node") -> Optional["NodeReference"]:
        from bench.language.expression import NodeReference

        ancestor = getattr(self, cast(Property, prop.reference_source).name)
        if ancestor is None:
            return None
        else:
            return NodeReference.from_node(ancestor)

    def set(self: Node, value: "NodeReference"):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get_ancestor_ptr, set)


def _node_ref_computed_prop(
    key_ref: str, key_ptr: str, ref_prop: Property, ptr_prop: Property | None
) -> property:
    """Computed value from another property. If prop is not set, use backup prop."""

    def get(self: "Node") -> Optional[Any]:
        ref = getattr(self, ref_prop.name)
        ptr = getattr(self, ptr_prop.name) if ptr_prop else None
        if ref_prop.is_list:
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

    def set(self: "Node", value: Any):
        raise NotImplementedError(f"cannot set computed property {ref_prop!r}: {value!r}")

    return property(get, set)


def _make_component_dunder_method(method: _ComponentMethod):
    """Creates method that proxies a builtin dunder method to the first _method_component"""

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


StructDataT = TypeVar("StructDataT", bound="Union[AnyStructData, AnyNodeData]")


@struct_component()
class Struct(abc.ABC, Generic[StructDataT]):
    """
    A non-node data structure, usually inside a node (which is the only way to store/retrieve it).
    Will track, track, etc. when we start using these in nodes.
    """

    metatype: ClassVar[StructType]  # type discriminator is field 0 if needed?
    __static_components__: ClassVar[tuple[type["Node"] | type["Struct"], ...]] = ()
    __dynamic_components__: ClassVar[tuple[type["Node"] | type["Struct"], ...]] = ()
    __passthrough_targets__: ClassVar[tuple[tuple[str, _Passthrough], ...]] = ()

    __parent_property__: ClassVar[Property] = UNSET

    __properties__: ClassVar[dict[str, Property]] = {}
    __own_properties__: ClassVar[dict[str, Property]] = {}
    __declared_properties__: ClassVar[dict[str, Property]] = {}
    __properties_by_id__: ClassVar[dict[int, Property]] = {}
    __properties_name_by_id__: ClassVar[dict[int, str]] = {}
    __tracked_properties__: ClassVar[dict[str, Property]] = {}
    __internal_properties__: ClassVar[dict[str, Property]] = {}
    __node_reference_properties__: ClassVar[dict[str, Property]] = {}
    __property_reference_properties__: ClassVar[dict[str, Property]] = {}
    __struct_reference_properties__: ClassVar[dict[str, Property]] = {}
    __sensitive_properties__: ClassVar[dict[str, Property]] = {}
    __struct_properties__: ClassVar[dict[str, Property]] = {}
    __value_properties__: ClassVar[dict[str, Property]] = {}
    __stored_properties__: ClassVar[dict[str, Property]] = {}
    __wired_properties__: ClassVar[dict[str, Property]] = {}
    __runtime_properties__: ClassVar[dict[str, Property]] = {}
    __reserved_properties__: ClassVar[frozenset[int | str]] = frozenset()
    __properties_in_order__: ClassVar[tuple[Property, ...]]
    __properties_id_in_order__: ClassVar[tuple[int, ...]]
    __max_property_ord__: ClassVar[int] = UNSET
    __properties_mask_set__: ClassVar[bitarray] = UNSET
    __properties_mask_unset__: ClassVar[bitarray] = UNSET

    __is_struct_only__: ClassVar[bool] = True
    __is_struct_inlined__: ClassVar[bool] = False
    __is_node__: ClassVar[bool] = False

    # TODO :Architecture :Cleanup: extract & type (Inline)Struct/Node parent/order_key/etc.
    #   Struct.parent/order_key/.. in a covariant and specialized way.
    #   Probably want a more general uber-class like Object, then InlineStruct/Struct/Node.
    #  (subclasses make it more specific like Field.parent:Block..
    #   see all the parent # type: ignore / reportIncompatibleVariableOverride errors)

    # NOTE: struct identity props (id/parent/....) only exist if not inlined & not node :MagicProps
    # (Struct.id is optional so that external clients don't need to generate ids for every struct,
    #  and also so that its field presence is tracked and we can validate that it is set when needed)
    id: int = p_system(2, default_factory=new_struct_id)
    parent: Union["Struct", "Node", "Value", None] = p_struct_parent(3)
    if TYPE_CHECKING:
        parent_type: NodeType | None = p_internal(4, default=None)
        parent_id: int | None = None
        parent_key: str | None = None
    order_key: str | None = p_internal(5, default=None)
    # for source nodes:
    # computed_properties: dict[int, ValueReference] | None = p_regular(21)
    # for template instances (= 'template' is set)
    set_properties: list[int] = p_regular(22, array=True)

    _status: InterpStatus = p_runtime(default=None)
    _updated_properties: bitarray | None = p_runtime(default=None)

    def __post_init__(self):
        if self._status is None:
            self._status = InterpStatus.INTERPED if _active_session.get() else InterpStatus.SOURCE
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
            return f"<{self.__class__.__name__} @ {self.id}>"

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
    def _mask_properties_ids(cls, properties: Collection[int]) -> bitarray:
        mask = bitarray(cls.__max_property_ord__ + 1)
        for prop_id in properties:
            prop = cls.__properties_by_id__[prop_id]
            mask[prop.ord] = True
        return mask

    @classmethod
    def _resolve_property(cls, ptr: "PropertyReference") -> Property:
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
                if (
                    prop.id == ptr.id
                    and prop.reference_nodes
                    and prop.reference_nodes[0] == ptr.references_type
                ):
                    return prop
            return None

    @property
    def _components(self) -> tuple[type["Node"] | type["Struct"], ...]:
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
            if prop.id < 5:
                continue  # ignore struct identity
            self_value = getattr(self, prop.name)
            other_value = getattr(other, prop.name)
            if self_value != other_value and (
                prop.py_type_stripped != float
                or not math.isclose(self_value, other_value, rel_tol=1e-5)
            ):
                return False
        return True

    def __eq__(self, other):
        if other is None:
            return False
        elif self.__is_struct_inlined__:
            return self.equals_content(other)
        else:
            return self.metatype == other.metatype and self.id == other.id

    if not TYPE_CHECKING:
        # NOTE: __setattr__/__getattr__ confuses type checking, so only use it in runtime
        #  (unfortunately this means we also don't get type-checking in here)
        def __setattr__(self, key, value):
            """Sets *any* attribute on this node (incl. slots)."""
            is_tracked = self._status == InterpStatus.TRACKED
            prop = self.__properties__.get(key)
            if prop is not None:
                if prop.is_ephemeral or prop.is_autoset:  # untracked
                    return object.__setattr__(self, key, value)
                elif prop.reference_kind == ReferenceKind.NODE_CHILD:
                    attr = object.__getattribute__(self, key)
                    if attr is None or type(attr) is Property:  # initial set
                        return object.__setattr__(self, key, value)
                    else:
                        return attr.set(value)  # has its own set
                elif (
                    prop.reference_kind == ReferenceKind.STRUCT_CHILD
                    and self._status is not None
                    and value is not None
                ):
                    # copy struct if needed (only after init since child struct needs our id)
                    if prop.is_list:
                        value = ValueList._lazy_copy_for(value, self, prop, prop)
                    else:
                        value = value._lazy_copy_to(self, prop)
                elif prop.is_computed:
                    raise AttributeError(f"cannot set computed property {prop!r}: {value!r}")

                # validate set
                if is_tracked:
                    prev = getattr(self, key)
                    object.__setattr__(self, key, value)
                    try:
                        self._validate_self((prop,), on_invalid=on_invalid_raise)
                    except ValidationError:  # reset on error
                        object.__setattr__(self, key, prev)
                        raise
                else:
                    object.__setattr__(self, key, value)

                # update reference pointers
                if self._status is not None and prop.reference_wired_ptr is not None:
                    object.__setattr__(
                        self, prop.reference_wired_ptr.name, prop.to_wired_ptr(value)
                    )

                # report edit
                if is_tracked:
                    self._updated_self((prop,))
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
            did_you_mean = did_you_mean_str(candidates, key)
            raise AttributeError(f"Cannot set '{key}' on {self!r}. {did_you_mean}")

        def __getattr__(self, item):
            # when using slots so this is not an instance attribute
            attr = UNSET
            # prefer components methods
            for component in self._components:
                attr = getattr(component, item, UNSET)
                if attr is not UNSET:
                    break
            # check passthrough targets if tracked in session
            if attr is UNSET and self._status == InterpStatus.TRACKED:
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
                    return attr.fget(self)  # type: ignore
                elif not isinstance(attr, Node) and callable(attr) and not inspect.ismethod(attr):
                    return functools.partial(attr, self)
                else:
                    return attr

            # report get error with additional info
            if self._status == InterpStatus.TRACKED:
                candidates = {
                    # own properties
                    **{
                        p.name: p
                        for p in self.__properties__.values()
                        if p.reference_kind or not p.is_ephemeral
                    },
                    # public methods
                    **{m: None for m in dir(self) if not m.startswith("_")},
                }
                did_you_mean = did_you_mean_str(candidates, item)
                raise AttributeError(f"{self!r} has no attribute '{item}'. {did_you_mean}")
            else:
                raise AttributeError(f"{self.__class__} has no attribute '{item}'")

    def _lazy_copy_to(
        self, parent: Union["Node", "Struct", "Value"], prop: Union[Property, "Field"]
    ) -> "Struct":
        """Create a copy of this struct for the given parent/prop if different."""
        assert self.__is_struct_only__, f"cannot copy non-struct {self!r}"
        prop_key = prop.id_as_str if isinstance(prop, Property) else prop.identity_key
        if self.parent is None:  # not assigned
            self.parent = parent
            self.parent_key = prop_key
            return self
        elif self.parent == parent and self.parent_key == prop_key:  # already the same
            return self
        else:
            copy = self._copy_to(parent, prop)
            return copy

    def _copy_to(self, parent: Union["Node", "Struct", "Value"], prop: Union[Property, "Field"]):
        """Create a copy of this struct for the given parent/prop."""
        kwargs = {
            p.name: getattr(self, p.name)
            for p in self.__properties__.values()
            if p.is_runtime and not p.is_ephemeral and not p.is_computed
        }
        kwargs["parent"] = parent
        kwargs["parent_key"] = prop.id_as_str if isinstance(prop, Property) else prop.identity_key
        copy = self.__class__(**kwargs)
        return copy

    def _walk_structs(self) -> Iterable["Struct"]:
        for prop in self.__struct_properties__.values():
            value = getattr(self, prop.name)
            if value is None:
                continue
            elif not prop.is_list:
                yield from value._walk_self()
            elif len(value) > 0:
                for item in value:
                    yield from item._walk_self()

    def _init_component(self):
        for prop in self.__struct_reference_properties__.values():
            # copy new structs if needed (now that we're init & have an id for sure)
            if prop.reference_kind == ReferenceKind.STRUCT_CHILD:
                existing = getattr(self, prop.name, None)
                if prop.is_list:
                    assert prop.reference_list_type is not None
                    self.__dict__[prop.name] = prop.reference_list_type(cast(Node, self), prop)
                    if existing:
                        # will auto copy if needed
                        self.__dict__[prop.name].extend(existing)
                elif existing is not None:
                    if prop.reference_kind == ReferenceKind.STRUCT_CHILD:
                        self.__dict__[prop.name] = existing._lazy_copy_to(self, prop)

            # init property reference pointers if references are set
            if prop.reference_kind == ReferenceKind.PROPERTY:
                assert prop.reference_wired_ptr is not None, f"no wired ptr for {prop!r}"
                existing = getattr(self, prop.name, None)
                if prop.is_list:
                    if existing:
                        self.__dict__[prop.reference_wired_ptr.name] = prop.to_wired_ptr(existing)
                elif existing is not None:
                    self.__dict__[prop.reference_wired_ptr.name] = prop.to_wired_ptr(existing)

        # init reference pointers if references are set
        for prop in self.__node_reference_properties__.values():
            if prop.reference_kind == ReferenceKind.NODE_ANCESTOR_FIRST:
                continue
            if prop.reference_wired_ptr is not None:
                value = getattr(self, prop.name)
                if value is not None:  # keep old value)
                    self.__dict__[prop.reference_wired_ptr.name] = prop.to_wired_ptr(value)

    def _interp_component(self, scope: Optional["Node"], on_notice: "NoticeHandler"):
        from bench.language.notice import NoticeType

        # TODO :Broken: turn all node references into computer propertied (against graph)
        # (using parent, so parent is still a proper reference?)
        if scope is not None:
            # resolve node references
            for prop in self.__node_reference_properties__.values():
                if (
                    prop.is_wired
                    or prop.is_stored
                    or prop.reference_kind != ReferenceKind.NODE_REGULAR
                ):
                    continue  # already resolved
                assert prop.reference_wired_ptr is not None, f"no wired ptr for {prop!r}"
                ptr = getattr(self, prop.reference_wired_ptr.name)
                if ptr is None:
                    continue
                if prop.is_list:
                    ptr = cast(list["NodeReference"], ptr)
                    resolved = []
                    for p in ptr:
                        r = scope._root_graph.get(cast(UUID, p.id or p.ck))
                        if r is None:
                            on_notice(self, NoticeType.MISSING_REFERENCE, {"properties": (prop,)})
                        resolved.append(r)
                    self.__dict__[prop.name] = resolved
                else:
                    ptr = cast("NodeReference", ptr)
                    resolved = scope._root_graph.get(cast(UUID, ptr.id or ptr.ck))
                    if resolved is None:
                        on_notice(self, NoticeType.MISSING_REFERENCE, {"properties": (prop,)})
                    self.__dict__[prop.name] = resolved

        # resolve property references
        for prop in self.__property_reference_properties__.values():
            if prop.is_wired or prop.is_stored or getattr(self, prop.name, None):
                continue  # already resolved
            assert prop.reference_wired_ptr is not None, f"no wired ptr for {prop!r}"
            ptr = getattr(self, prop.reference_wired_ptr.name)
            if ptr is None:
                continue
            if prop.is_list:
                ptr = cast(list["PropertyReference"], ptr)
                setattr(self, prop.name, [p.resolve() for p in ptr])
            else:
                ptr = cast("PropertyReference", ptr)
                setattr(self, prop.name, ptr.resolve())

    def _validate_component(
        self, properties: Collection[Property], on_invalid: "ValidationHandler"
    ) -> None:
        """Validate properties for illegal values that should not or cannot be stored."""

        if properties == ():  # validate all
            properties = self.__tracked_properties__.values()
        for prop in properties:
            value = getattr(self, prop.name)
            if value is None:
                # TODO :Robustness: track whether property was deferred
                #  (so we can validate it appropriately, and probably a bunch of other stuff)
                if prop.is_required and not (prop.is_sensitive or prop.is_deferred):
                    on_invalid(self, f"{prop.name} is required", [prop], None)
            elif prop.custom_validate is not None:
                handler = PropertyValidationHandler(self, prop, on_invalid)
                prop.custom_validate(prop, value, handler)

    def _flushed_self(self):
        """Called when this struct has been flushed to the store."""
        if self.__is_node__:
            (cast("Node", self))._is_new = False
        self._updated_properties = None

    def _updated_component(self, properties: Collection[Property]) -> None:
        """Called when properties in this struct have been updated successfully."""
        if self._status == InterpStatus.TRACKED:
            if self.__is_node__:
                if not cast("Node", self)._is_new:
                    session = cast("Node", self)._session
                    assert session
                    if self._updated_properties is None:
                        self._updated_properties = bitarray(self.__max_property_ord__ + 1)
                    for prop in properties:
                        self._updated_properties[prop.ord] = True
                        session.update(cast("Node", self), properties=properties)
            else:  # is struct
                # will need to deal with Value parents eventually...
                assert isinstance(self.parent, Struct), f"unexpected parent: {self.parent!r}"
                if self.parent is not None:
                    self.parent._updated_component(properties)

    @final
    def _init_self(self):
        for meth in _get_component_methods(
            self._components, _ComponentMethod.init, self._instance_cache_key
        ):
            meth(self)

    @final
    def _interp_self(self, scope: Optional["Node"], on_notice: "NoticeHandler"):
        for meth in _get_component_methods(
            self._components, _ComponentMethod.interp, self._instance_cache_key
        ):
            meth(self, scope, on_notice)
        self._status = InterpStatus.INTERPED

    @final
    def _validate_self(
        self, properties: Collection[Property], on_invalid: "ValidationHandler"
    ) -> None:
        for meth in _get_component_methods(
            self._components, _ComponentMethod.validate, self._instance_cache_key
        ):
            meth(self, properties, on_invalid)

    @final
    def _updated_self(self, properties: Collection[Property]) -> None:
        for meth in _get_component_methods(
            self._components, _ComponentMethod.updated, self._instance_cache_key
        ):
            meth(self, properties)

    @final
    def _walk_self(self) -> Iterable["Struct"]:
        yield self
        for prop in self.__struct_properties__.values():
            value = getattr(self, prop.name)
            if value:
                if prop.is_list:
                    for item in value:
                        yield from item._walk_self()
                else:
                    yield from value._walk_self()

    @final
    def _interp_rec(self, scope: Optional["Node"], on_notice: "NoticeHandler"):
        for inner_struct in self._walk_self():
            inner_struct._interp_self(scope, on_notice)

    @final
    def _validate_rec(
        self, properties: tuple[Property, ...], on_invalid: "ValidationHandler"
    ) -> None:
        for inner_struct in self._walk_self():
            inner_struct._validate_self(properties, on_invalid)

    @final
    def _to_data(self) -> StructDataT:
        """Convert to wire format"""
        from bench.proto.wiring import pack_struct

        return pack_struct(self)


NodeDataT = TypeVar("NodeDataT", bound="AnyNodeData")
FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type["Node"]]


@node_component()
class Node(Struct[NodeDataT], Generic[NodeDataT]):
    """
    A node in the Bench graph: a struct with a globally unique identity.
    Every node has a 'constant' key (ck) identifying its constant (id)entity across versions.
    The first part of the constant key is the template key (tk), which is constant in all instances of a template.
    For sub package nodes the 'id' is derived from the 'ck' per Package, else it's just the id.
    """

    metatype: ClassVar[NodeType]  # type: ignore
    __static_components__: ClassVar[tuple[type["Node"], ...]] = ()  # type: ignore
    __dynamic_components__: ClassVar[tuple[type["Node"], ...]] = ()  # type: ignore
    __identifier_type__: ClassVar[IdentifierType] = IdentifierType.VARIABLE
    __id_factory__: ClassVar[Callable[[], UUID]] = new_node_id

    __ancestor_properties__: ClassVar[dict[str, Property]] = {}
    __node_list_properties__: ClassVar[dict[str, Property]] = {}

    __roots__: ClassVar[bytetuple[NodeType]] = UNSET
    __is_struct_only__: ClassVar[bool] = False
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
    __table__: ClassVar[Table | None] = None  # if stored regularly, set after finalization

    # 1-9: reserved for node identity
    # NOTE: some node identity props (ck/package/bench/etc.) only exist sometimes :MagicProps
    id: UUID = p_system(2, default=None, require=True, autoset=True)
    ck: UUID = p_system(3, default=None, require=True, autoset=True)
    parent: Optional["Node"] = p_node_parent(4)  # type: ignore
    if TYPE_CHECKING:
        parent_id: Optional[UUID] = None
        parent_ptr: Optional[NodeReference] = None
    # template: Optional["Node"] = node_template(5)
    package: "Package" = p_node_ancestor(
        6, NodeType.PACKAGE, require=True, store=True, wire=True, is_bench_implicit=True
    )
    bench: "Bench" = p_node_ancestor(7, NodeType.BENCH, require=True, store=True, wire=True)
    if TYPE_CHECKING:
        package_id: Optional[UUID] = None
        bench_id: Optional[UUID] = None
    source: NodeSource = p_system(8, default=NodeSource.STORE, store=False, wire=True, require=True)

    # 10-29: reserved for node tracking
    revision: int = p_system(
        10, default=0, require=True, autoset=True, primitive_type=PrimitiveType.INT64
    )
    created_at: datetime = p_system(11, default=None, require=True, autoset=True)
    updated_at: datetime = p_system(12, default=None, require=True, autoset=True)
    deleted_at: Optional[datetime] = p_system(13, default=None, autoset=True)
    archived_at: Optional[datetime] = p_system(14, default=None, autoset=True)
    # not yet fully implemented:
    # changed_at (15), active_at (16), ....
    created_by: Union["User", "Run", None] = p_system(
        17,
        default=None,
        require=False,
        array=False,
        autoset=True,
        references=(NodeType.USER, NodeType.RUN),
        is_bench_implicit=True,
    )
    updated_by: Union["User", "Run", None] = p_system(
        18,
        default=None,
        require=False,
        array=False,
        autoset=True,
        references=(NodeType.USER, NodeType.RUN),
        is_bench_implicit=True,
    )
    if TYPE_CHECKING:
        created_by_id: Optional[UUID] = None
        created_by_type: NodeType | None = None
        updated_by_id: Optional[UUID] = None
        updated_by_type: NodeType | None = None
    # changed_by (19)?, active_by (20)?, ...
    # from Struct: computed_properties (21), set_properties (22)

    # 30+ for 'user' node/struct properties
    # <... defined in concrete type ...>

    links: NodeList["Link"] = p_node_child(NodeType.LINK)

    # the node graph is maintained at the highest root node
    _graph: Union["NodeGraph", "DetachedNodeGraph", None] = p_runtime(default=None)
    _data_graph: Optional["NodeDataGraph"] = p_runtime(default=None)
    _session: Optional["Session"] = p_runtime(default=None)
    _status: InterpStatus = p_runtime(default=None)
    _is_new: bool = p_runtime(default=False)

    def __post_init__(self):
        # init ck/id
        if self.__is_sub_package__:
            if self.ck is None:
                self.ck = self.__class__.__id_factory__()
                self._is_new = True
            if self.id is None and self.is_attached:
                self._assign_id(self.package.id)
        elif self.id is None:
            self.id = self.__class__.__id_factory__()
            self._is_new = True
        # init timestamps
        if self.created_at is None:
            now = utcnow_with_tz()
            self.created_at = now
            self.updated_at = now
        # init session context
        if self._session is None and self._session is not UNSET:
            self._session = _active_session.get()
        # init status
        if self._status is None:
            self._status = (
                InterpStatus.INTERPED if self._session is not None else InterpStatus.SOURCE
            )
        self._init_self()
        # track if in session
        if self._status == InterpStatus.INTERPED and self._session is not None:
            self._interp_self(self, on_notice=self._on_notice)
            self._track_self(self._session)

    def _assign_id(self, package_id: UUID):
        assert package_id, f"cannot assign id to {self!r} without a package id"
        assert self.id is None, f"cannot assign id to {self!r} twice"
        assert self.ck is not None, f"cannot assign id to {self!r} without ck"
        self.id = derive_source_node_id(package_id, self.ck)

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
    def _root_graph(self) -> Union["NodeGraph", "DetachedNodeGraph"]:
        root = self.root
        graph = root._graph
        assert graph is not None, f"no graph for root {root!r} (from {self!r})"
        return graph

    @final
    def __str__(self):  # type: ignore
        # override the default __str__ for nodes
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
    def __repr__(self):  # type: ignore
        # override the default __repr__ for nodes
        return f"<{self.__class__.__name__} {str(self)}>"

    @property
    def is_attached(self) -> bool:
        if self.__is_sub_package__:
            return self.parent is not None and self.package is not None
        elif self.__is_sub_bench__:
            return self.parent is not None and self.bench is not None
        elif self.__roots__:
            return self.parent is not None
        else:
            return True

    @property
    def tk(self) -> str:
        """The template key of this node lineage."""
        return get_tk_from_ck(self.ck)

    @property
    def identifier_type(self) -> IdentifierType:
        return self.__identifier_type__

    @property
    def bench_ident(self) -> Optional[str]:
        """The Bench identifier of this node (slug if exists, else name if exists)."""
        if self.metatype == NodeType.PACKAGE and self.parent is not None:
            return self.parent.bench_ident  # package shares its Bench's identifier
        if "slug" in self.__properties__:
            slug = getattr(self, "slug")
            if slug:  # prefer slug as ident
                return slug
        return getattr(self, "name", None)

    @property
    def bench_path_key(self) -> Optional[str]:
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
        if self.metatype == NodeType.PACKAGE and self.parent is not None:
            return self.parent.py_ident
        if "slug" in self.__properties__:
            slug = getattr(self, "slug", None)
            if slug:  # prefer slug as ident
                return slug
        name = getattr(self, "name", None)
        if name is None:
            return None
        return to_casing(name, PYTHON_CASING[self.identifier_type])

    @property
    def absolute_path(self) -> str:
        if self.__parent_property__ is None or not self.__parent_property__.reference_nodes:
            ident = self.bench_ident
            assert ident is not None, f"no bench ident for {self!r}"
            return ident
        elif self.parent is None:
            return f"<detached>/{self.bench_path_key}"
        else:
            path_segments: list[str] = []
            current = self
            while True:
                # skip bench (same path as pkg)
                path_key = current.bench_path_key
                assert path_key is not None, f"no path key for {current!r}"
                path_segments.append(path_key)
                next_parent = current.parent
                has_next = next_parent is not None and (
                    not self.__is_in_package__ or next_parent.metatype != NodeType.BENCH
                )
                if not has_next:
                    break
                elif current.metatype == NodeType.FIELD:
                    path_segments.append(".")
                elif (
                    current.metatype != NodeType.BLOCK
                    and cast(Node, next_parent).metatype == NodeType.BLOCK
                ):
                    path_segments.append(":")
                else:
                    path_segments.append("/")
                current = cast(Node, next_parent)

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

    def __eq__(self, other: Optional["Node"]):
        return (
            isinstance(other, Node)
            and self.metatype == other.metatype
            and (self.id is not None and self.id == other.id or self is other)
        )

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
        order_key: str | None = None,
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

    def _on_notice(
        self,
        subject: "Struct",
        type: "NoticeType",
        options: Optional["NoticeOptions"],
    ) -> None:
        pass  # TODO :Incomplete: Notices

    def _to_data_wrapped(self) -> SomeNodeData:
        """To wire format, wrapped in the generic any node container."""
        from bench.proto.wiring import pack_node, wrap_some_node

        return wrap_some_node(pack_node(self))

    #
    # The :ComponentMethods
    #

    def _init_component(self) -> None:
        if self.parent is None:
            # if we're not in a graph, start a new one
            if NodeType.BENCH in self.__roots__:
                self._graph = DetachedNodeGraph()
            else:
                self._graph = NodeGraph()
            self._graph.add(self)

    def _interp_component(self, scope: Optional["Node"], on_notice: "NoticeHandler"):
        """Interp this node."""
        # interp all structs recursive
        for s in self._walk_structs():
            s._interp_rec(scope, on_notice)

    def _track_component(self, session: "Session") -> None:
        """Track this object in the given session."""
        self._session = session
        self._status = InterpStatus.TRACKED

    def _untrack_component(self) -> None:
        """Stop tracking this object."""
        self._session = None

    def _attached_component(self) -> None:
        """Called when this node is attached to a root graph."""
        pass

    def _detached_component(self) -> None:
        """Called when this node is detached from a root graph."""
        pass

    _call_component = _make_component_dunder_method(_ComponentMethod.call)
    _iter_component = _make_component_dunder_method(_ComponentMethod.iter)
    _aiter_component = _make_component_dunder_method(_ComponentMethod.aiter)
    _len_component = _make_component_dunder_method(_ComponentMethod.len)
    _getitem_component = _make_component_dunder_method(_ComponentMethod.getitem)

    __call__ = _call_component
    __iter__ = _iter_component
    __aiter__ = _aiter_component
    __len__ = _len_component
    __getitem__ = _getitem_component

    def __bool__(self):
        return True  # allow truthy checks for nodes

    @final
    def _init_self(self):
        # init node lists
        existing_lists: dict[str, Any] | None = None
        for name, prop in self.__node_list_properties__.items():
            existing = getattr(self, name, None)
            assert prop.reference_list_type is not None
            node_list = prop.reference_list_type(self, prop)
            setattr(self, name, node_list)
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
            for name, existing in existing_lists.items():
                if existing and not isinstance(existing, NodeList):
                    getattr(self, name).extend(*existing)

        # validate if in session after all init are done
        if (
            self._status >= InterpStatus.INTERPED
            and self._is_new
            and self._session is not None
            and self._session is not UNSET
        ):
            self._validate_self(self.__tracked_properties__.values(), on_invalid=on_invalid_raise)

    @final
    def _interp_self(self, scope: Optional["Node"], on_notice: "NoticeHandler"):
        for meth in _get_component_methods(
            self._components, _ComponentMethod.interp, self._instance_cache_key
        ):
            meth(self, scope, on_notice)
        self._status = InterpStatus.INTERPED

    @final
    def _track_self(self, session: "Session"):
        for meth in _get_component_methods(
            self._components, _ComponentMethod.track, self._instance_cache_key
        ):
            meth(self, session)

    @final
    def _untrack_self(self):
        for meth in _get_component_methods(
            self._components, _ComponentMethod.untrack, self._instance_cache_key
        ):
            meth(self)
        self._status = InterpStatus.INTERPED

    @final
    def _walk_rec(self) -> Iterable["Node"]:
        yield self
        if self.metatype in HAS_CHILD_NODE_TYPES:
            yield from self._root_graph.collect_descendants(self, recursive=True)

    #
    # Querying
    #

    @classmethod
    def query(cls) -> "QueryBuilder[Self, NodeDataT]":
        from bench.language.query import QueryBuilder

        return QueryBuilder(node_type=cls.metatype)

    @classmethod
    async def get(cls, conditional: Optional["Expression"] = None, **kwargs) -> Self:
        return await cls.query().get(conditional, **kwargs)

    @classmethod
    def where(
        cls, filter: Optional["Expression"] = None, **kwargs
    ) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().where(filter, **kwargs)

    @classmethod
    def order_by(
        cls, sort: Optional["Expression"] = None, *args: str
    ) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().order_by(sort, *args)

    @classmethod
    def include(cls, *properties: FieldOrProperty) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().include(*properties)

    @classmethod
    def include_all(cls) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().include_all()

    @classmethod
    def exclude(cls, *properties: FieldOrProperty) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().exclude(*properties)

    @classmethod
    def related(cls, *properties: FieldOrProperty) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().related(*properties)

    @classmethod
    def include_ancestors(cls) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().include_ancestors()

    @classmethod
    def ancestors(cls, *node_types: NodeTypeOrClass) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().ancestors(*node_types)

    @classmethod
    def descendants(cls, *node_types: NodeTypeOrClass) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().descendants(*node_types)

    #
    # Fetch
    #

    @classmethod
    async def tolist(cls) -> list[Self]:
        return await cls.query().tolist()

    @classmethod
    def first(cls, count: int) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().first(count)

    @classmethod
    async def count(cls, filter: Optional["Expression"] = None, **kwargs) -> int:
        return await cls.query().count(filter, **kwargs)

    @classmethod
    async def exists(cls, filter: Optional["Expression"] = None, **kwargs) -> bool:
        return await cls.query().exists(filter, **kwargs)


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

    parent: Node = p_node_parent(4, *LINK_PARENT_NODE_TYPES)
    reference: Optional[Node] = p_regular(
        30, array=False, references=LINK_TARGET_NODE_TYPES, require=False
    )
    order_key: Optional[str] = p_internal(32, default=None)


@node(NodeType.SKIP, stored=False)
class Skip(Node):
    """A reference to another node in some graph that wasn't available for some reason (usually permissions)."""

    parent: Node = p_node_parent(4, *LINK_PARENT_NODE_TYPES)
    reference: Optional[Node] = p_regular(
        30, array=False, references=LINK_TARGET_NODE_TYPES, require=True
    )
    order_key: Optional[str] = p_internal(31, default=None)


@node_component()
class BasedNode(Node[NodeDataT], Generic[NodeDataT]):
    """A node that requires an explicit base (parent, type, whatever) in another node."""

    @property
    def base(self) -> Optional[Node]:
        raise NotImplementedError

    @property
    def base_ck(self) -> Optional[UUID]:
        return self.base.ck if self.base is not None else None

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        raise NotImplementedError

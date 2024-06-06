import abc
import base64
import contextlib
import dataclasses
import enum
import functools
import inspect
import math
import uuid
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime
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
from uuid import UUID

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
    NodeType,
    ObjectType,
    ReferenceKind,
    StructType,
    _active_session,
    get_active_session,
    new_node_id,
    new_struct_id,
)
from bench.language.graph import DetachedNodeGraph, NodeDataGraph, NodeGraph, NodeList
from bench.language.property import (
    _PROPERTY_SPECIFIERS,
    METATYPE_PROPERTY,
    Property,
    p_internal,
    p_node_ancestor_first,
    p_node_child,
    p_node_parent,
    p_node_template,
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
from bench.language.validation import ValidationError, ValidationHandler, on_invalid_raise
from bench.proto.wire import AnyNodeData, AnyStructData, NodeReferenceData, SomeNodeData
from bench.sql.core import Constraint, ConstraintType, Index, IndexType, PrimitiveType, Table
from bench.utils.casing import PYTHON_CASING, IdentifierType, to_casing
from bench.utils.dt import utcnow
from bench.utils.env import IS_DEV, IS_TEST
from bench.utils.func import bittuple, did_you_mean_str
from bench.utils.utils import frozendict
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Expression,
        Field,
        NodeReference,
        NoticeType,
        Object,
        Package,
        PropertyReference,
        ReadOptions,
        Run,
        Server,
        Session,
        User,
    )
    from bench.language.notice import NoticeHandler, NoticeIn
    from bench.language.query import QueryBuilder

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


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


def get_tk_b64_from_ck(ck: UUID) -> str:
    """Gets the stable across templates first 6 bytes of the ck."""
    return base64.b64encode(ck.bytes[:TK_LENGTH_BYTES]).decode()


def pad_ck_from_tk_b64(tk_b64: str) -> UUID:
    """Pads the remainder with zeros"""
    bytes = base64.b64decode(tk_b64.encode()) + (16 - TK_LENGTH_BYTES) * b"\x00"
    return UUID(bytes=bytes)


def derive_source_node_id(package_id: UUID, ck: UUID):
    """Derive the version-specific node id from its constant key"""
    return uuid.uuid5(package_id, str(ck))


class _ComponentMethod(enum.Enum):
    # lifecycle
    init = "init"
    interp = "interp"
    validate = "validate"
    updated = "updated"

    @property
    def component(self) -> str:
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
_COMPONENT_CALL_ORDER: tuple[str, ...] = ("Struct", "Node", "TypeInfoBase")  # ... the rest


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


@cached(cache={}, key=lambda cls, components, method: (cls, method))
def _get_component_methods(
    cls: type,
    components: Collection[type["Node"] | type["Struct"]],
    method: _ComponentMethod,
) -> tuple[Callable, ...]:
    """Get the actually implemented methods in the given components in call order."""
    methods = []
    for component in _sort_components_in_call_order(components):
        if _COMPONENT_METHODS.get((method, component), None) is not None:
            methods.append(getattr(component, method.component))
    return tuple(methods)


_StructT = TypeVar("_StructT", bound="Struct")
_CORE_TYPES = ("Struct", "Node")


def _process_struct_base_cls(
    cls: type[_StructT],
    object_type: ObjectType | None,
    # for nodes only
    is_final: bool = False,
    is_sub_package: bool = False,
    is_sub_bench: bool = False,
    is_root: bool = False,
    is_variable_root: bool = False,
    no_ck: bool = False,
    # for structs only
    is_inlined: bool = False,
) -> tuple[type[_StructT], dict[str, "Property"]]:
    """Process a struct base class and return the processed class and its properties."""
    assert isinstance(cls, type), f"expected type, got {cls} ({type(cls)})"

    is_node_base = cls.__name__ in "Node"
    is_struct_base = cls.__name__ == "Struct"
    is_node = not is_struct_base and (is_node_base or issubclass(cls, Node))
    is_struct = not is_node
    metatype = METATYPE_PROPERTY.clone()
    metatype.component = cls
    properties_by_name: dict[str, Property] = {"metatype": metatype}

    # collect static components from class hierarchy
    static_components: list[type[Node] | type[Struct]] = [cls]
    for base in cls.__bases__:
        if base.__name__ == "ABC":
            continue
        if hasattr(base, "__properties__"):
            base: type[Struct]
            static_components.append(base)
            for grandparent in base.__components__:
                if grandparent not in static_components:
                    static_components.append(grandparent)

    if IS_DEV or IS_TEST:
        # check that no forbidden methods are defined in non-base classes
        if cls.__name__ not in _CORE_TYPES:
            for name in _FORBIDDEN_COMPONENT_METHODS:
                meth = getattr(cls, name, None)
                good_meths = (getattr(cls, name, None) for cls in (Struct, Node, Node))
                if meth is not None and meth not in good_meths:
                    raise ValueError(f"forbidden method {name} defined in {cls}")

        # check components
        for component in static_components[1:]:
            if component.__name__ in _CORE_TYPES:
                continue  # ignore base classes
            if is_node and component.__is_struct_inlined__:
                raise ValueError(f"node {cls} has inlined struct {component}")
            if (
                is_inlined
                and hasattr(component, "metatype")
                and not component.__is_struct_inlined__
            ):
                raise ValueError(f"struct {cls} has non-inlined struct {component}")

    # collect properties from this
    own_properties: dict[str, Property] = {}
    for name, prop in list(cls.__dict__.items()):
        if (
            name.startswith("__")
            or type(prop).__name__.startswith("_")
            or inspect.ismethod(prop)
            or inspect.isfunction(prop)
            or isinstance(prop, (property, classmethod, staticmethod))
            or type(prop) == functools.cached_property
        ):
            continue  # ignore reserved names and non-fields
        if not isinstance(prop, Property):
            raise TypeError(f"{cls.__name__}.{name} is not a NodeProperty: {prop} ({type(prop)})")
        prop.name = intern(name)
        prop.component = cls
        prop.py_type_raw = cls.__annotations__.get(name, None)
        properties_by_name[name] = prop
        own_properties[name] = prop
    cls.__declared_properties__ = frozendict(own_properties)
    cls.__own_properties__ = frozendict(properties_by_name)  # remember 'own' properties

    # collect properties from all components
    for component in reversed(static_components):
        for name, prop in component.__own_properties__.items():
            # system struct identity is only for non-inlined structs :MagicProps
            #  (we remove it here because it conflicts with downstream props)
            if not is_struct and (prop.id == Struct.__properties__["order_key"].id):
                continue
            existing = properties_by_name.get(name)
            # override parent/id with more specific properties
            if (
                existing is None
                or name == "id"
                or name.startswith("parent")
                or existing.id is UNSET
            ):
                prop = prop.clone()
                prop.component = cls
                properties_by_name[name] = prop
            elif not prop._equals_type(existing):
                raise ValueError(f"property conflict '{name}': {prop!r}, {existing!r}")
            if not is_node and prop.is_tree_reference:
                raise ValueError(f"non-node {cls} has node-only relation {prop}")

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
            ReferenceKind.NODE_TEMPLATE,
            ReferenceKind.STRUCT_PARENT,
            ReferenceKind.PROPERTY,
        ):
            if prop.reference_kind == ReferenceKind.NODE_TEMPLATE:
                if not (is_final and is_node):
                    prop.is_stored = False
                    prop.is_wired = False
                    continue
                # template points to nodes of same type
                prop.reference_nodes = (cast(NodeType, object_type),)

            for p in prop._contribute_ptrs(is_root=is_root, is_inlined=is_inlined):
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
                    with contextlib.suppress(AttributeError):
                        delattr(cls, name)
                cls.__annotations__.pop(name, None)
                for contributed_prop in prop.contributed_props:
                    _remove_magic_prop(contributed_prop.name)

        if is_node and not is_sub_bench:
            if cls.__name__ == "Bench":
                cls.bench = _node_computed_ancestor_prop(properties_by_name["bench"])  # type: ignore
                _remove_magic_prop("bench", delete_attr=False)
            else:
                _remove_magic_prop("bench")
            _remove_magic_prop("created_epoch")
            _remove_magic_prop("updated_epoch")
        if is_node and not is_sub_package:
            if cls.__name__ == "Package":
                cls.package = _node_computed_ancestor_prop(properties_by_name["package"])  # type: ignore
                _remove_magic_prop("package", delete_attr=False)
            else:
                _remove_magic_prop("package")
        if is_node and (no_ck or not is_sub_package):
            _remove_magic_prop("ck")
            _remove_magic_prop("template")
            _remove_magic_prop("templated_epoch")
            _remove_magic_prop("computed_properties")
            # don't need to store set_properties if it's not a full source node
            # (but still want it at runtime/wired, e.g. for optimistic overrides)
            properties_by_name["set_properties"].is_stored = False
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
                if postfix == "type" and (
                    not prop.reference_nodes or len(prop.reference_nodes) <= 1
                ):
                    continue  # no need for type if only one possible node type
                computed_prop = _node_ref_computed_prop(
                    ref_key, ptr_key, prop, prop.reference_wired_ptr
                )
                computed_prop_name = prop.name + "_" + postfix
                existing = properties_by_name.get(computed_prop_name)
                if existing is not None and existing.reference_source is not prop:
                    raise ValueError(
                        f"computed property conflict '{computed_prop_name}': {computed_prop!r}, {existing!r}"
                    )
                setattr(cls, computed_prop_name, computed_prop)

    # collect component methods implemented in this component
    for meth_type in _ComponentMethod:
        meth = getattr(cls, meth_type.component, None)
        if meth is not None and not any(
            meth is getattr(base, meth_type.component, None) for base in cls.__bases__
        ):
            _COMPONENT_METHODS[(meth_type, cls)] = meth  # type: ignore

    # register components and index properties
    cls.__components__ = tuple(static_components)  # type: ignore
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
    #  Using slots everywhere is trickier than it seems because
    #   1) some weird runtime errors
    #   2) we use dynamic props in Blocks (for now?)
    #   3) lack of betterproto support for our *Data types
    #       (unclear how challenging it would be to add)

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
    is_final: bool = False,
    is_inlined: bool = False,
):
    """
    Mark a class as a struct component (or concrete struct for a StructType).
    """

    def decorate(cls_in: Type[_StructT]) -> Type[_StructT]:
        cls, _properties = _process_struct_base_cls(
            cls=cast(Any, cls_in), object_type=struct_type, is_final=is_final, is_inlined=is_inlined
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
def struct(struct_type: StructType, inline: bool = False):
    def decorate(cls: Type[_StructT]) -> Type[_StructT]:
        cls = struct_component(struct_type=struct_type, is_final=True, is_inlined=inline)(cls)
        return cls

    return decorate


_NodeT = TypeVar("_NodeT", bound="Node")


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def node_component(
    node_type: NodeType | None = None,
    passthrough: str | None = None,
    is_root: bool = False,
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
            object_type=node_type,
            is_root=is_root,
            is_variable_root=is_variable_root,
            is_sub_bench=is_sub_bench,
            is_sub_package=is_sub_package,
            is_final=is_final,
            no_ck=no_ck,
        )
        cls.__passthrough__ = passthrough

        # register node properties
        props = properties.values()
        list_properties: dict[str, Property] = {}
        list_properties_by_child: dict[NodeType, list[Property]] = defaultdict(list)
        for prop in properties.values():
            if prop.reference_kind == ReferenceKind.NODE_CHILDREN:
                if cls.__name__ != "Node" and not issubclass(cls, Node) and node_type is not None:
                    raise ValueError(f"{cls} is not a Node for {prop}")
                list_properties[prop.name] = prop
                for ref_t in prop.reference_nodes or ():
                    list_properties_by_child[ref_t].append(prop)
        cls.__node_child_properties__ = frozendict(list_properties)
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
    passthrough: str | None = None,
    stored: bool = True,
    stored_custom: bool = False,
    no_ck: bool = False,
    local: bool = False,
    roots: tuple[NodeType, ...] = (NodeType.BENCH,),
    constraints: tuple[Constraint, ...] = (),
    indexes: tuple[Index | tuple[str, ...], ...] = (),
    unique: tuple[tuple[str, ...], ...] = (),
    identifier: IdentifierType = IdentifierType.VARIABLE,
    id_factory: Callable[[], UUID] = new_node_id,
):
    """Register a class as a concrete node for the given node type."""

    in_package = node_type in IN_PACKAGE_NODE_TYPES
    sub_package = node_type in SUB_PACKAGE_NODE_TYPES
    in_bench = node_type in IN_BENCH_NODE_TYPES
    sub_bench = node_type in SUB_BENCH_NODE_TYPES

    if not no_ck and id_factory is not new_node_id:
        raise ValueError(f"cannot specify custom id_factory while keeping ck for {node_type}")

    def decorate(cls: Type[_NodeT]) -> Type[_NodeT]:
        cls = node_component(
            node_type=node_type,
            passthrough=passthrough,
            is_root=len(roots) == 0,
            is_variable_root=len(roots) > 1,
            is_sub_bench=sub_bench,
            is_sub_package=sub_package,
            no_ck=no_ck,
            is_final=True,
            is_local=local,
        )(cls)
        cls.__is_stored__ = stored
        cls.__is_stored_custom__ = stored_custom
        cls.__is_local__ = local
        cls.__identifier_type__ = identifier
        cls.__id_factory__ = id_factory

        extra_indexes: list[Index] = []
        extra_constraints: list[Constraint] = [*constraints]
        for columns in unique:
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
        for index in indexes:
            if isinstance(index, Index):
                extra_indexes.append(index)
            else:
                index_name = f"bench_idx_{'_'.join(index)}"
                extra_index = Index(
                    index_name, type=IndexType.BTREE, is_unique=False, columns=index
                )
                extra_indexes.append(extra_index)
        cls.__extra_indexes__ = tuple(extra_indexes)
        cls.__extra_constraints__ = tuple(extra_constraints)

        cls.__roots__ = bittuple(*roots, enum_cls=NodeType)
        cls.__is_in_package__ = in_package
        cls.__is_sub_package__ = sub_package
        cls.__is_in_bench__ = in_bench
        cls.__is_sub_bench__ = sub_bench

        return cls

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def timed_node(
    node_type: NodeType,
    passthrough: str | None = None,
    indexes: tuple[Index | tuple[str, ...], ...] = (),
):
    """Register a class as a concrete node for the given node type."""
    return node(
        node_type=node_type,
        passthrough=passthrough,
        local=True,
        no_ck=True,
        id_factory=UUIDT,
        indexes=(
            *indexes,
            ("created_at",),
            ("created_epoch",),
            ("package_id", "created_at"),
            ("package_id", "created_epoch"),
        ),
    )


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


def _required_prop(prop: Property) -> dataclasses.Field:
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


StructDataT = TypeVar("StructDataT", bound="Union[AnyStructData, AnyNodeData]")

# nocheckin :Architecture! :Cleanup!: extract & type (Inline)Struct/Node/PackageNode parent/order_key/etc.
#   Struct.parent/order_key/.. in a covariant and specialized way.
#   Probably want a more general uber-class like _Object, then InlineStruct/Struct/Node/PackageNode.
#  (subclasses make it more specific like Field.parent:Block..
#   see all the parent # type: ignore / reportIncompatibleVariableOverride errors)


@struct_component()
class Struct(abc.ABC, Generic[StructDataT]):
    """
    A non-node data structure, usually inside a node (which is the only way to store/retrieve it).
    Will track, track, etc. when we start using these in nodes.
    """

    metatype: ClassVar[StructType]  # type discriminator is field 0 if needed?
    __components__: ClassVar[tuple[type["Node"] | type["Struct"], ...]] = ()
    __passthrough__: ClassVar[str | None] = None

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
    __properties_in_order__: ClassVar[tuple[Property, ...]]
    __properties_id_in_order__: ClassVar[tuple[int, ...]]
    __max_property_ord__: ClassVar[int] = UNSET
    __properties_mask_set__: ClassVar[bitarray] = UNSET
    __properties_mask_unset__: ClassVar[bitarray] = UNSET

    __is_struct_only__: ClassVar[bool] = True
    __is_struct_inlined__: ClassVar[bool] = False
    __is_node__: ClassVar[bool] = False

    # NOTE :Architecture: struct identity props (id/parent/....) only exist sometimes :MagicProps
    #  (if not inlined & not node, Struct.id is optional so that external clients don't need to generate ids for every struct,
    #  and also so that its field presence is tracked and we can validate that it is set when needed)
    id: int = p_system(2, default_factory=new_struct_id)
    parent: Union["Struct", "Node", "Object", None] = p_struct_parent(3)
    if TYPE_CHECKING:
        parent_type: NodeType | None = p_internal(4, default=None)
        parent_id: int | None = None
        parent_key: str | None = None
    # struct-only order_key
    order_key: str | None = p_internal(9, default=None)
    # for source nodes:
    # computed_properties: dict[int, ValueReference] | None = p_regular(28)
    # for overlay branched/templated instances (the *additional* set properties)
    set_properties: list[int] = p_regular(29, array=True)

    _is_interped: bool = p_runtime(default=False)
    _session: "Session | None" = p_runtime(default=None)
    _updated_properties: bitarray | None = p_runtime(default=None)

    def __post_init__(self):
        if get_active_session() is not None:
            self._is_interped = True
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

    def equals_content(self, other: Any) -> bool:
        """Checks if all wired properties of the two structs are equal (recursively)."""
        if other is None or self.metatype != other.metatype:
            return False
        for prop in self.__wired_properties__.values():
            if prop.id < 30:
                continue  # ignore struct identity
            self_value = getattr(self, prop.name)
            other_value = getattr(other, prop.name)
            if self_value != other_value and (
                prop.py_type_stripped is not float
                or not math.isclose(self_value, other_value, rel_tol=1e-5)
            ):
                return False
        return True

    def __eq__(self, other):
        if other is self:
            return True
        elif other is None:
            return False
        else:
            return self.equals_content(other)

    def __getattr(self, item):
        attr = self.__dict__.get(item, UNSET)
        if attr is not UNSET:
            return attr
        else:
            # try components methods
            for component in self.__components__:
                attr = getattr(component, item, UNSET)
                if attr is not UNSET:
                    break
            else:
                # check passthrough if tracked in session
                if attr is UNSET and self._session is not None and self.__passthrough__ is not None:
                    target = getattr(self, self.__passthrough__)
                    attr = getattr(target, item, UNSET)
            # attribute could be property, method, or just plain value
            if attr is not UNSET:
                if type(attr) is property:
                    return attr.fget(self)  # type: ignore
                elif callable(attr) and not isinstance(attr, Node) and not inspect.ismethod(attr):
                    return functools.partial(attr, self)
                else:
                    return attr

        # report attribute error
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
        if did_you_mean:
            raise AttributeError(f"{self!r} has no attribute '{item}'. {did_you_mean}")
        else:
            raise AttributeError(f"{self!r} has no attribute '{item}'")

    def __setattr(self, key: str, value):
        """Sets *any* attribute on this node (incl. slots)."""
        session = self.__dict__.get("_session", None)
        is_tracked = session is not None and session is not UNSET
        prop = self.__properties__.get(key)
        if prop is not None:
            if (prop.is_ephemeral and not prop.is_value_runtime) or prop.is_autoset:  # untracked
                return object.__setattr__(self, key, value)
            elif prop.reference_kind == ReferenceKind.NODE_CHILDREN:
                attr = object.__getattribute__(self, key)
                if attr is None or type(attr) is Property:  # initial set
                    return object.__setattr__(self, key, value)
                else:
                    return attr.set(value)  # has its own set
            elif prop.is_computed:
                raise AttributeError(f"cannot set computed property {prop!r}: {value!r}")

            # validate/set
            old_value = self.__dict__.get(key)
            if is_tracked:
                # coerce & check type
                if prop.type_info is not None and prop.reference_source is None:
                    value = coerce_value(value, prop.type_info, self, prop, prop)
                    check_value(value, prop.type_info, invalid=on_invalid_raise)
                object.__setattr__(self, key, value)
                try:
                    self._validate_self((prop,), invalid=on_invalid_raise)
                except ValidationError:  # reset on error
                    object.__setattr__(self, key, old_value)
                    raise
            else:
                object.__setattr__(self, key, value)

            # update reference pointers :NodeRefs
            if prop.reference_wired_ptr is not None:
                wired_ptr = prop.to_wired_ptr(cast(Any, value))
                object.__setattr__(self, prop.reference_wired_ptr.name, wired_ptr)

            if is_tracked:
                # notify
                self._updated_self((prop,))
                if self.__is_node__:
                    node = cast("Node", self)
                    if not node._is_new:
                        session = node._session
                        assert session, f"no session for {node!r}"
                        if self._updated_properties is None:
                            self._updated_properties = bitarray(self.__max_property_ord__ + 1)
                        self._updated_properties[prop.ord] = True
                        session.update(node, properties=(prop,), old_values={prop.id: old_value})
                else:  # is struct
                    # will need to deal with Value parents eventually...
                    assert isinstance(self.parent, Struct), f"unexpected parent: {self.parent!r}"
                    if self.parent is not None:
                        self.parent._updated_self((prop,))
        elif is_tracked and self.__passthrough__ is not None:
            # try passthrough target (if any)
            target = getattr(self, self.__passthrough__)
            setattr(target, key, value)
        else:
            # report set error with additional info
            candidates = {p.name: p for p in self.__properties__.values() if not p.is_computed}
            did_you_mean = did_you_mean_str(candidates, key)
            if did_you_mean:
                raise AttributeError(f"Cannot set '{key}' on {self!r}. {did_you_mean}")
            else:
                raise AttributeError(f"Cannot set '{key}' on {self!r}")

    if not TYPE_CHECKING:
        # NOTE: __setattr__/__getattr__ confuses type checking, so only define it at runtime
        #  (we don't need it since dynamic access is meant for Values at runtime)
        __getattr__ = __getattr
        __setattr__ = __setattr

    def _move_to(
        self,
        parent: Union["Node", "Struct", "Object"],
        prop: Union[Property, "Field"],
        ancestor_prop: Property | None = None,
    ) -> "Struct":
        """Move or copy this struct into given parent/prop."""
        assert (
            self.__is_struct_only__
        ), f"cannot copy non-struct {self!r}"  # this is overriden by Node
        prop_key = prop.id_as_str if isinstance(prop, Property) else prop.identity_key
        if self.parent is None:  # not assigned
            self.parent = parent
            self.parent_key = prop_key
            return self
        elif self.parent is parent and self.parent_key == prop_key:
            # already there
            return self
        else:
            copy = self._copy_to(parent, prop)
            return copy

    def _copy_to(
        self, parent: Union["Node", "Struct", "Object"], prop: Union[Property, "Field"]
    ) -> Self:
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

    def _copy(self, **update) -> "Self":
        kwargs = {
            p.name: getattr(self, p.name)
            for p in self.__properties__.values()
            if p.is_runtime and not p.is_ephemeral and not p.is_computed
        }
        kwargs.update(update)
        copy = self.__class__(**kwargs)
        return copy

    def _walk_struct(self) -> Iterable["Struct"]:
        yield self
        for prop in self.__struct_properties__.values():
            value: Struct | list[Struct] | None = getattr(self, prop.name)
            if value is None:
                continue
            elif not prop.is_list:
                yield from (cast(Struct, value))._walk_struct()
            elif len(cast(list, value)) > 0:
                for item in cast(list[Struct], value):
                    yield from item._walk_struct()

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
                        self.__dict__[prop.name] = existing._move_to(self, prop)

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

    def _validate_component(self, properties: Collection[Property], invalid: "ValidationHandler"):
        if properties == ():
            properties = self.__tracked_properties__.values()
        for prop in properties:
            if prop.type_info is not None and prop.reference_source is None:
                value = self.__dict__.get(prop.name)
                check_value(value, prop.type_info, invalid=invalid)

    def _resolve_references(self, scope: Optional["Node"], notice: "NoticeHandler"):
        from bench.language.notice import NoticeType

        # NOTE :Robustness? :Architecture: turn regular node refs into computed properties? :NodeRefs
        #  Currently, we manually set wired ptrs on set and resolve on interp.
        #  If we had immediate (=fast) access to a graph in all Object/Struct/Nodes,
        #  we could skip having to resolve during interp and leaving stale refs until re-interp.
        #  I'm not sure how Nodes that aren't in our current graph should be treated then.
        if scope is not None:
            graph = scope._graph
            for prop in self.__node_reference_properties__.values():
                if (
                    prop.is_wired
                    or prop.is_stored
                    or prop.reference_kind != ReferenceKind.NODE_REGULAR
                ):
                    continue
                assert prop.reference_wired_ptr is not None, f"no wired ptr for {prop!r}"
                ptr = getattr(self, prop.reference_wired_ptr.name)
                if ptr is None:
                    continue
                if prop.is_list:
                    ptr = cast(list["NodeReference"], ptr)
                    resolved = []
                    for p in ptr:
                        r = graph.get(cast(UUID, p.id or p.ck))
                        if r is None:
                            notice(self, NoticeType.MISSING_REFERENCE, {"properties": (prop,)})
                        resolved.append(r)
                    self.__dict__[prop.name] = resolved
                else:
                    ptr = cast("NodeReference", ptr)
                    resolved = graph.get(cast(UUID, ptr.id or ptr.ck))
                    if resolved is None:
                        notice(self, NoticeType.MISSING_REFERENCE, {"properties": (prop,)})
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

    def _flushed_self(self):
        """Called when this struct has been flushed to the store."""
        if self.__is_node__:
            (cast("Node", self))._is_new = False
        self._updated_properties = None

    def _updated_component(self, properties: Collection[Property]) -> None:
        """Called when properties in this struct have been updated successfully, but before notifying the update."""
        pass

    def __bool__(self):
        return True  # allow truthy checks for structs

    @final
    def _init_self(self):
        for meth in _get_component_methods(
            self.__class__, self.__components__, _ComponentMethod.init
        ):
            meth(self)

    @final
    def _interp_self(self, scope: Optional["Node"], notice: "NoticeHandler"):
        # resolve references first
        self._resolve_references(scope, notice)
        # and then component interps
        for meth in _get_component_methods(
            self.__class__, self.__components__, _ComponentMethod.interp
        ):
            meth(self, scope, notice)
        self._is_interped = True

    @final
    def _validate_self(
        self, properties: Collection[Property], invalid: "ValidationHandler"
    ) -> None:
        for meth in _get_component_methods(
            self.__class__, self.__components__, _ComponentMethod.validate
        ):
            meth(self, properties, invalid)

    @final
    def _updated_self(self, properties: Collection[Property]) -> None:
        for meth in _get_component_methods(
            self.__class__, self.__components__, _ComponentMethod.updated
        ):
            meth(self, properties)

    @final
    def _interp_rec(self, scope: Optional["Node"], notice: "NoticeHandler"):
        for inner_struct in self._walk_struct():
            inner_struct._interp_self(scope, notice)

    @final
    def _validate_rec(self, properties: Collection[Property], invalid: "ValidationHandler") -> None:
        for inner_struct in self._walk_struct():
            inner_struct._validate_self(properties, invalid)

    @classmethod
    def _from_data(cls, data: StructDataT) -> Self:
        """Convert from wire format"""
        from bench.proto.wiring import unpack_struct

        return unpack_struct(cast(AnyStructData, data))

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
EditSubject = Union["User", "Server", "Run"]
EDIT_SUBJECT_TYPES = (NodeType.USER, NodeType.SERVER, NodeType.RUN)


@dataclass(slots=True)
class ReadInfo:
    options: "ReadOptions | None"
    epoch: int | None
    properties: bitarray | None = None
    graph: NodeDataGraph | None = None


def is_implicit_node_property(prop_id: int) -> bool:
    return prop_id < 30 and prop_id != 4  # parent is fine


@node_component()
class Node(Struct[NodeDataT], Generic[NodeDataT]):
    """
    A node in the Bench graph: a struct with a globally unique identity.
    Most nodes have a 'constant' key (ck) providing constant (id)entity across versions.
    The first part of the constant key is the template key (tk), which is constant in all instances of a template.
    For sub package nodes the 'id' is derived from the 'ck' per Package, otherwise it's just the id.
    """

    metatype: ClassVar[NodeType]  # type: ignore
    __components__: ClassVar[tuple[type["Node"], ...]] = ()  # type: ignore
    __identifier_type__: ClassVar[IdentifierType] = IdentifierType.VARIABLE
    __id_factory__: ClassVar[Callable[[], UUID]] = new_node_id

    __ancestor_properties__: ClassVar[dict[str, Property]] = {}
    __node_child_properties__: ClassVar[dict[str, Property]] = {}

    __roots__: ClassVar[bittuple[NodeType]] = UNSET
    __is_struct_only__: ClassVar[bool] = False
    __is_node__: ClassVar[bool] = True
    __is_in_bench__: ClassVar[bool] = UNSET  # part of a Bench
    __is_sub_bench__: ClassVar[bool] = UNSET  # part of a Bench (excludes Bench itself)
    __is_in_package__: ClassVar[bool] = UNSET  # part of a Package
    __is_sub_package__: ClassVar[bool] = UNSET  # part of a Package (excludes Package itself)
    __is_stored__: ClassVar[bool] = False  # stored in primary store (runtime or local)
    __is_stored_custom__: ClassVar[bool] = False  # custom storage logic (for records)
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
    package: "Package" = p_node_ancestor_first(
        5, NodeType.PACKAGE, require=True, store=True, wire=True, is_bench_implicit=True
    )
    bench: "Bench" = p_node_ancestor_first(6, NodeType.BENCH, require=True, store=True, wire=True)
    template: Optional["Node"] = p_node_template(7)
    templated_epoch: int | None = p_system(
        8, require=False, default=None, autoset=True, primitive_type=PrimitiveType.INT64
    )
    if TYPE_CHECKING:
        package_id: Optional[UUID] = None
        bench_id: Optional[UUID] = None
    # Struct only: order_key (9)

    # 10-29: reserved for node tracking
    revision: int = p_system(
        10,
        default=0,
        default_sql=None,
        require=True,
        autoset=True,
        primitive_type=PrimitiveType.INT64,
    )
    created_at: datetime = p_system(11, default=None, require=True, autoset=True)
    created_epoch: int = p_system(
        12, default=-1, default_sql=None, autoset=True, primitive_type=PrimitiveType.INT64
    )
    updated_at: datetime = p_system(13, default=None, require=True, autoset=True)
    updated_epoch: int = p_system(
        14, default=-1, default_sql=None, autoset=True, primitive_type=PrimitiveType.INT64
    )
    deleted_at: Optional[datetime] = p_system(15, default=None, autoset=True)
    archived_at: Optional[datetime] = p_system(16, default=None, autoset=True)
    # changed_at (for nested), active_at (for runnables), ...?
    created_by: EditSubject | None = p_system(
        21,
        default=None,
        require=False,
        array=False,
        autoset=True,
        references=EDIT_SUBJECT_TYPES,
        same_bench=True,
    )
    updated_by: EditSubject | None = p_system(
        22,
        default=None,
        require=False,
        array=False,
        autoset=True,
        references=EDIT_SUBJECT_TYPES,
        same_bench=True,
    )
    if TYPE_CHECKING:
        created_by_id: Optional[UUID] = None
        created_by_type: NodeType | None = None
        updated_by_id: Optional[UUID] = None
        updated_by_type: NodeType | None = None
    # from Struct: computed_properties (28), set_properties (29)

    # 30+ for 'user' node/struct properties
    # <... defined in concrete type ...>

    links: NodeList["Link"] = p_node_child(NodeType.LINK)

    _graph: Union["NodeGraph[Node]", "DetachedNodeGraph"] = p_runtime(default=None)
    _read: ReadInfo | None = p_runtime(default=None)
    _is_new: bool = p_runtime(default=False)

    def __post_init__(self):
        # init ck/id
        if self.__is_sub_package__ and "ck" in self.__properties__:
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
            now = utcnow()
            self.created_at = now
            self.updated_at = now
        # init graph
        if self.parent is None:
            # if we're not in a graph, start a new one
            if NodeType.BENCH in self.__roots__:
                self._graph = DetachedNodeGraph()
            else:
                self._graph = NodeGraph()
            self._graph.add(self)
        else:
            # we'll be added to the graph by our parent
            assert self.parent._graph is not None, f"no graph for {self.parent!r}"
            self._graph = self.parent._graph
        # init session context
        if self._session is None and self._session is not UNSET:
            self._session = _active_session.get()
            if self._session is not None:
                self._is_interped = True
        self._init_self()
        # track if in session
        if self._session is not None and self._session is not UNSET:
            self._interp_self(self, notice=self._on_notice)
            self._track_self(self._session)

    def _assign_id(self, package_id: UUID):
        assert package_id, f"cannot assign id to {self!r} without a package id"
        assert self.id is None, f"cannot assign id to {self!r} twice"
        assert self.ck is not None, f"cannot assign id to {self!r} without ck"
        self.id = derive_source_node_id(package_id, self.ck)

    def _find_root(self) -> "Node":
        """Current root of this node. May not be *the* "right" root if detached."""
        parent = self
        while parent.parent is not None:
            parent = parent.parent
        return parent

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
            status_str = " [archived, deleted]" if self.deleted_at is not None else " [archived]"
        elif self.deleted_at is not None:
            status_str = " [deleted]"
        else:
            status_str = ""
        if self.__parent_property__ is None:
            return f"'{ident_str}'{content_str}{status_str}"
        else:
            return f"'{self.absolute_path}'{content_str}{status_str}"

    @final
    def __repr__(self):  # type: ignore
        # override the default __repr__ for nodes
        return f"<{self.__class__.__name__} {self!s}>"

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
    def _data_graph(self) -> "NodeDataGraph":
        assert self._read is not None, f"no read info for {self!r}"
        assert self._read.graph is not None, f"no read data graph for {self!r}"
        return self._read.graph

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
        if "name" in self.__properties__:
            return getattr(self, "name")
        return None

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
            slug = getattr(self, "slug")
            if slug:  # prefer slug as ident
                return slug
        if "name" in self.__properties__:
            name = getattr(self, "name")
            if name is None:
                return None
            return to_casing(name, PYTHON_CASING[self.identifier_type])
        return None

    @property
    def absolute_path(self) -> str:
        if self.__parent_property__ is None or not self.__parent_property__.reference_nodes:
            ident = self.bench_ident
            assert ident is not None, f"no bench ident for {self!r}"
            return ident
        elif self.parent is None:
            return f"<detached>/{self.bench_path_key}"
        else:
            # NOTE :Broken: Node.absolute_path is a mess & incorrect
            path_segments: list[str] = []
            current = self
            while current is not None:
                path_key = current.bench_path_key
                assert path_key is not None, f"no path key for {current!r}"
                path_segments.append(path_key)
                next_parent = current.parent
                if next_parent is None:
                    break
                elif next_parent.metatype == NodeType.PACKAGE:
                    next_parent = next_parent.bench
                elif next_parent.metatype == NodeType.BRANCH:
                    next_parent = next_parent.parent
                # skip bench & branch (same path as pkg)
                elif current.metatype == NodeType.FIELD:
                    path_segments.append(".")
                elif (
                    current.metatype != NodeType.BLOCK
                    and cast(Node, next_parent).metatype == NodeType.BLOCK
                ):
                    path_segments.append(":")
                else:
                    path_segments.append("/")
                current = next_parent

            path_segments.reverse()
            path = "".join(path_segments)
            return path

    def to_ref(self) -> "NodeReference":
        """Gets a reference to this node."""
        from bench.language.expression import NodeReference

        return NodeReference.from_node(self)

    def _to_ref_data(self) -> NodeReferenceData:
        """Gets a data reference to this node."""
        from bench.language.expression import NodeReference

        return NodeReference.data_from_node(self)

    def __eq__(self, other: Any):
        return type(self) == type(other) and (
            (self.id is not None and self.id == other.id) or self is other
        )

    def __hash__(self):
        if "ck" in self.__properties__:
            # 'id' may not yet be assigned
            return hash((type(self), self.id, getattr(self, "ck")))
        else:
            return hash((type(self), self.id))

    @property
    def is_extant(self):
        return self.archived_at is None and self.deleted_at is None

    @property
    def is_archived(self) -> bool:
        return self.archived_at is not None or (self.parent is not None and self.parent.is_archived)

    @property
    def is_deleted(self) -> bool:
        return self.deleted_at is not None or (self.parent is not None and self.parent.is_deleted)

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
        assert not self.is_deleted, f"{self!r} is already deleted"
        raise NotImplementedError

    def restore(self):
        """Restore this node from soft deletion."""
        assert self.is_deleted, f"{self!r} is not deleted"
        raise NotImplementedError

    def erase(self):
        """Hard delete this node. Forever. Irreversibly."""
        raise NotImplementedError

    def _on_notice(
        self,
        subject: "Struct",
        type: "NoticeType",
        options: Optional["NoticeIn"],
    ) -> None:
        pass  # TODO :Incomplete: Notices

    def _to_data_wrapped(self) -> SomeNodeData:
        """To wire format, wrapped in the generic any node container."""
        from bench.proto.wiring import pack_node, wrap_some_node

        return wrap_some_node(pack_node(self))

    #
    # The :ComponentMethods
    #

    def __bool__(self):
        return True  # allow truthy checks for nodes

    @final
    def _init_self(self):  # type: ignore
        # init node lists
        existing_lists: dict[str, Any] | None = None
        for name, prop in self.__node_child_properties__.items():
            existing = getattr(self, name, None)
            assert prop.reference_list_type is not None
            node_list = prop.reference_list_type(self, prop)
            setattr(self, name, node_list)
            if existing and not isinstance(existing, NodeList):
                if existing_lists is None:
                    existing_lists = {}
                existing_lists[name] = existing

        # run component inits
        for meth in _get_component_methods(
            self.__class__, self.__components__, _ComponentMethod.init
        ):
            meth(self)

        # keep manually set node lists if passed in
        if existing_lists:
            for name, existing in existing_lists.items():
                if existing and not isinstance(existing, NodeList):
                    getattr(self, name).extend(*existing)

        # validate if in session
        if self._is_new and self._session is not None and self._session is not UNSET:
            self._validate_self((), invalid=on_invalid_raise)

    @final
    def _interp_self(self, scope: Optional["Node"], notice: "NoticeHandler"):  # type: ignore
        # interp contained structs
        for struct in self._walk_struct():
            if struct is not self:
                struct._interp_self(scope, notice)
        # and the component interps
        for meth in _get_component_methods(
            self.__class__, self.__components__, _ComponentMethod.interp
        ):
            meth(self, scope, notice)
        self._is_interped = True

    @final
    def _track_self(self, session: "Session"):
        """Track this object in the given session."""
        if (
            self._session is not None
            and self._session is not UNSET
            and self._session is not session
        ):
            raise RuntimeError(f"{self!r} is already in {self._session!r}, not {session!r}")
        self._session = session

    @final
    def _untrack_self(self) -> None:
        """Stop tracking this object."""
        self._session = None

    @final
    def _walk_descendants(self) -> Iterable["Node"]:
        yield self
        if self.metatype in HAS_CHILD_NODE_TYPES:
            yield from self._graph.collect_descendants(self, recursive=True)

    @final
    def _interp_rec(self, scope: Optional["Node"], notice: "NoticeHandler"):  # type: ignore
        # interp contained structs
        super()._interp_rec(scope, notice)
        # and descendants
        for inner_node in self._walk_descendants():
            inner_node._interp_self(scope, notice)

    @final
    def _validate_rec(self, properties: Collection[Property], invalid: "ValidationHandler") -> None:  # type: ignore
        # validate contained structs
        super()._validate_rec(properties, invalid)
        # and descendants
        for inner_node in self._walk_descendants():
            inner_node._validate_self(properties, invalid)

    @final
    def _track_rec(self, session: "Session"):
        for inner_node in self._walk_descendants():
            inner_node._track_self(session)

    @final
    def _untrack_rec(self):
        for inner_node in self._walk_descendants():
            inner_node._untrack_self()

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
    def select_all(cls) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().select_all()

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


# NOTE: import from .value later to avoid circular import
#  (but import at top level to avoid import in critical path)
from bench.language.value import check_value, coerce_value  # noqa: E402

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

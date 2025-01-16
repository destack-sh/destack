import abc
import base64
import contextvars
import functools
import inspect
from collections import defaultdict
from dataclasses import InitVar
from datetime import datetime
from enum import IntEnum
from itertools import chain
from sys import intern
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    ClassVar,
    Collection,
    Iterable,
    Literal,
    Mapping,
    NamedTuple,
    Optional,
    Self,
    Sequence,
    TypeGuard,
    Union,
    cast,
    dataclass_transform,
    final,
    overload,
    override,
)
from uuid import UUID, uuid4

import structlog
from bitarray import bitarray
from more_itertools import first
from opentelemetry import trace

from bench import pb2
from bench.language.registry import (
    BUILTIN_OBJECT_CLASS_BY_TYPE,
    DESCENDANT_NODE_TYPES,
    HAS_CHILD_NODE_TYPES,
    NODE_CLASS_BY_NAME,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)
from bench.pb2 import (
    AnyNodeData,
    AnyObjectData,
    AnyStructData,
    ClientOriginData,
    GraphScopeData,
    NodeReferenceData,
    lang_pb2,
)
from bench.pb2.lang_pb2 import EditOperationData
from bench.utils.env import IS_DEV, IS_TEST
from bench.utils.func import IdEnum, bittuple, dualmethod, is_close, stable_hash
from bench.utils.string import Casing, to_casing, to_code_name
from bench.utils.utils import frozendict
from bench.utils.uuidt import UUIDT

from .const import (
    BASED_NODE_TYPES,
    BENCH_NODE_TYPES,
    EMPTY_DICT,
    FLOAT_EPSILON,
    NODE_TYPES,
    PACKAGE_NODE_TYPES,
    TK_LENGTH_BYTES,
    UNSET,
    BlockType,
    ClientType,
    EditOperationType,
    FieldType,
    NodeArea,
    NodeMode,
    NodeType,
    ObjectType,
    QueryType,
    ReferenceKind,
    StructType,
    TypeKind,
    _active_session,
    active_session,
)
from .graph import NULL_SUPERGRAPH, NodeDataGraph, NodeGraph, NodeSuperGraph
from .list import LocalNodeList, RemoteNodeList, attach_node
from .property import (
    _PROPERTY_SPECIFIERS,
    METATYPE_PROPERTY,
    Property,
    PropertyReferenceType,
    p_internal,
    p_node_ancestor_with_self,
    p_node_parent,
    p_node_template,
    p_regular,
    p_runtime,
    p_struct_parent,
    p_subnode_packed,
    p_system,
)
from .validation import (
    ValidationError,
    ValidationHandler,
    constraint,
    on_invalid_raise,
)

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Bench,
        Block,
        Client,
        ComputedSourceIn,
        ComputedValue,
        ComputedValueMode,
        CustomObject,
        Expression,
        Field,
        GetConnection,
        Machine,
        NodeReference,
        Organization,
        Package,
        PathIn,
        Pipe,
        PropertyReference,
        Query,
        Run,
        SearchConnection,
        Session,
        User,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class _SetupStage(IntEnum):
    INITIALIZING = 1
    FINALIZING = 2
    COMPLETED = 3


_SETUP_STAGE = _SetupStage.INITIALIZING


def _is_setup_complete() -> bool:
    return _SETUP_STAGE == _SetupStage.COMPLETED


def _set_setup_finalizing():
    global _SETUP_STAGE
    _SETUP_STAGE = _SetupStage.FINALIZING


def _set_setup_complete():
    global _SETUP_STAGE
    _SETUP_STAGE = _SetupStage.COMPLETED


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


_BASE_OBJECT_NAMES = ("BuiltinObject", "Struct", "Struct", "Node")


def _process_object_cls[ObjectT: BuiltinObject](
    cls: type[ObjectT],
    object_type: ObjectType | None,
    is_final: bool = False,
    # for nodes only
    is_root: bool = False,
    is_variable_root: bool = False,
) -> tuple[type[ObjectT], dict[str, "Property"]]:
    """Process an object base class and return the processed class and its properties."""
    assert isinstance(cls, type), f"expected type, got {cls} ({type(cls)})"

    # NOTE :Performance :Architecture: we can't really use slots for our builtin objects
    #  (as it stands today, __slots__ always uses class-level descriptors, but we also
    #   want to use class level attributes for our own properties, like Block.type, ...
    #   - neglecting this conflict causes fun errors like 'X is a read-only attribute')

    is_object_base = cls.__name__ == "BuiltinObject"
    is_node_base = cls.__name__ == "Node"
    is_struct_base = cls.__name__ in ("Struct", "Struct")
    is_struct = not is_object_base and (is_struct_base or issubclass(cls, Struct))
    is_node = not is_object_base and not is_struct_base and (is_node_base or issubclass(cls, Node))
    metatype = METATYPE_PROPERTY.clone()
    metatype.component = cls
    properties_by_name: dict[str, Property] = {"metatype": metatype}

    # collect static components from class hierarchy
    static_components: list[type[BuiltinObject]] = [cls]
    for base in cls.__bases__:
        if base.__name__ == "ABC":
            continue
        if hasattr(base, "__properties__"):
            base: type[Struct]
            static_components.append(base)
            for grandparent in base.__components__:
                if grandparent not in static_components:
                    static_components.append(grandparent)

    # collect properties from this class
    declared_properties: dict[str, Property] = {}
    for name, prop in list(cls.__dict__.items()):
        if (
            name.startswith("__")
            or type(prop).__name__.startswith("_")
            or inspect.ismethod(prop)
            or inspect.isfunction(prop)
            or isinstance(prop, (property, classmethod, staticmethod, dualmethod))
            or type(prop) is functools.cached_property
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

    # collect properties from all parent components
    for component in reversed(static_components):
        for name, prop in component.__own_properties__.items():
            existing = properties_by_name.get(name)
            # allow overriding system properties with more specific properties
            if (
                existing is None
                or existing.id is UNSET
                or existing.id is None
                or ((prop.id is UNSET or prop.id is None or prop.id < 30) and existing.id < 30)
            ):
                prop = prop.clone()
                prop.component = cls
                properties_by_name[name] = prop
            elif not prop._equals_type(existing):
                raise ValueError(f"property conflict '{name}': {prop!r}, {existing!r}")
            if not is_node and prop.is_tree_reference:
                raise ValueError(f"non-node {cls} has node-only relation {prop}")

    # bench is optional in variable root types (since they can have other roots)
    if is_node and is_variable_root and "bench" in properties_by_name:
        properties_by_name["bench"].is_required = False

    # contribute extra properties
    for prop in tuple(properties_by_name.values()):
        # collect any extra contributed properties
        if prop.reference_kind in (
            ReferenceKind.NODE_PARENT,
            ReferenceKind.NODE_ANCESTOR_OR_SELF,
            ReferenceKind.NODE_ANCESTOR,
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

            for p in prop._contribute_ptrs(is_root=is_root):
                if p.name in properties_by_name:
                    raise ValueError(
                        f"property conflict '{p.name}': {p!r}, {properties_by_name[prop.name]!r}"
                    )
                properties_by_name[p.name] = p

    # add computed properties to final classes
    if is_final:

        def _set_computed(name: str, prop: property):
            """Sets a computed property that shouldn't conflict with any existing property."""
            existing = properties_by_name.get(name)
            if existing is not None and existing.is_runtime:
                raise ValueError(f"property conflict '{name}': {prop!r}, {existing!r}")
            setattr(cls, name, prop)

        for name, prop in properties_by_name.items():
            if prop.reference_source is None:  # not a contributed property
                # computed property.. property
                if prop.reference_kind == ReferenceKind.PROPERTY:
                    setattr(cls, name, _object_property_ref(prop))
                # computed node property
                elif prop.reference_kind in (
                    ReferenceKind.NODE_PARENT,
                    ReferenceKind.NODE_REGULAR,
                    ReferenceKind.NODE_TEMPLATE,
                ):
                    setattr(cls, name, _object_node_ref(prop))
                # computed node ancestor property
                elif prop.reference_kind in (
                    ReferenceKind.NODE_ANCESTOR,
                    ReferenceKind.NODE_ANCESTOR_OR_SELF,
                ):
                    setattr(cls, name, _node_ancestor_ref(prop))
                    setattr(cls, f"{name}_ptr", _node_ancestor_ptr_ref(prop))
                # computed _x node reference properties (e.g., parent_id, type_ck, node_type, ...)
                if prop.is_node_reference and prop.reference_kind != ReferenceKind.NODE_CHILDREN:
                    for obj_key, ptr_key in (("id", "id"), ("ck", "ck"), ("type", "node_type")):
                        if obj_key == "type" and (
                            not prop.reference_nodes or len(prop.reference_nodes) <= 1
                        ):
                            continue  # no need for *_type if only one possible node type
                        _set_computed(
                            f"{prop.name}_{obj_key}", _object_node_ref_attr(ptr_key, prop)
                        )

            # computed runtime value (with cache key)
            if prop.is_value_runtime or prop.is_subnode_packed:
                from .value import _object_value_runtime

                prop.cache_key = intern(f"_{prop.name}_cached")
                setattr(cls, prop.name, _object_value_runtime(prop))

    # finalize props & update reference to transformed class
    for prop in properties_by_name.values():
        prop.component = cls
        prop._finalize_meta()

    # register components and index properties
    cls.__components__ = tuple(static_components)  # type: ignore
    cls.__properties__ = frozendict(properties_by_name)
    cls.__original_properties__ = frozendict(
        {p.name: p for p in cls.__properties__.values() if p.reference_source is None}
    )
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
    cls.__value_runtime_properties__ = frozendict({p.name: p for p in props if p.is_value_runtime})
    cls.__stored_properties__ = frozendict(
        {p.name: p for p in cls.__properties__.values() if p.is_stored is True}
    )
    cls.__wired_properties__ = frozendict(
        {p.name: p for p in cls.__properties__.values() if p.is_wired is True}
    )
    cls.__runtime_properties__ = frozendict(
        {p.name: p for p in cls.__properties__.values() if p.is_runtime is True}
    )

    # assign ords
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
    if is_final and (is_node or is_struct) and parent_property is None:
        raise ValueError(f"missing parent property for node {cls}")
    cls.__parent_property__ = parent_property
    cls.__parent_types__ = parent_property.reference_nodes or () if parent_property else ()

    return cls, properties_by_name  # type: ignore


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def object_[_ObjectT: BuiltinObject](
    struct_type: StructType | None = None,
    is_final: bool = False,
):
    """
    Mark a class as an object component (or concrete struct for a StructType).
    """

    def decorate(cls_in: type[_ObjectT]) -> type[_ObjectT]:
        cls, _properties = _process_object_cls(
            cls=cast(Any, cls_in), object_type=struct_type, is_final=is_final
        )

        # register struct
        if struct_type:
            cls.metatype = struct_type
            if struct_type in STRUCT_CLASS_BY_TYPE:
                raise ValueError(
                    f"struct class conflict for {struct_type}: {cls}, {STRUCT_CLASS_BY_TYPE[struct_type]}"
                )
            STRUCT_CLASS_BY_TYPE[struct_type] = cls
        return cast(type[_ObjectT], cls)

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def struct_[_ObjectT: BuiltinObject](struct_type: StructType):
    """Register a class as a concrete struct for the given struct type."""

    def decorate(cls: type[_ObjectT]) -> type[_ObjectT]:
        cls = object_(struct_type=struct_type, is_final=True)(cls)
        if IS_DEV and cls.__name__ != "Struct" and cls.__name__ != "Struct":
            if not issubclass(cls, (Struct, Struct)):
                raise ValueError(f"{cls} is not a struct")
            if issubclass(cls, Node):
                raise ValueError(f"{cls} is a node for {struct_type}")

        return cast(type[_ObjectT], cls)

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def node_component(
    node_type: NodeType | None = None,
    passthrough_set: str | tuple[str, ...] | None = None,
    passthrough_get: str | tuple[str, ...] | None = None,
    is_root: bool = False,
    is_variable_root: bool = False,
    is_final: bool = False,
    is_subtype: bool = False,
):
    """Mark a class as a node component (or concrete node for a NodeType)."""
    if isinstance(passthrough_get, str):
        passthrough_get = (passthrough_get,)
    if isinstance(passthrough_set, str):
        passthrough_set = (passthrough_set,)

    def decorate(cls: type["Node"]) -> type["Node"]:
        cls, properties = _process_object_cls(
            cls=cls,
            object_type=node_type,
            is_root=is_root,
            is_variable_root=is_variable_root,
            is_final=is_final,
        )
        cls.__passthrough_get__ = passthrough_get
        cls.__passthrough_set__ = passthrough_set

        # register node properties
        list_properties: dict[str, Property] = {}
        list_properties_by_child: dict[NodeType, list[Property]] = defaultdict(list)
        for prop in properties.values():
            if prop.reference_kind == ReferenceKind.NODE_CHILDREN:
                if cls.__name__ != "Node" and not issubclass(cls, Node) and node_type is not None:
                    raise ValueError(f"{cls} is not a Node for {prop}")
                list_properties[prop.name] = prop
                assert isinstance(
                    prop.reference_nodes, tuple
                ), f"unexpected {prop.reference_nodes!r} for {prop!r}"
                list_properties_by_child[prop.reference_nodes[0]].append(prop)
        cls.__node_child_properties__ = frozendict(list_properties)

        # subtypes for every final node base
        if is_final and not is_subtype:
            cls.__subclass_by_subtype__ = {}
            cls.__subtype_by_subclass__ = {}

        # register as concrete node class for node_type
        if node_type:
            cls.metatype = node_type
        if node_type and not is_subtype:
            if node_type in NODE_CLASS_BY_TYPE:
                raise ValueError(
                    f"node class conflict for {node_type}: {cls}, {NODE_CLASS_BY_TYPE[node_type]}"
                )
            NODE_CLASS_BY_TYPE[node_type] = cls
            NODE_CLASS_BY_NAME[cls.__name__] = cls

        return cls

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def node_(
    node_type: NodeType,
    passthrough_get: str | tuple[str, ...] | None = None,
    passthrough_set: str | tuple[str, ...] | None = None,
    stored: bool = True,
    stored_value_unraveled: bool = False,
    local: bool = False,
    roots: tuple[NodeType, ...] = (NodeType.BENCH,),
    indexes: tuple[tuple[str, ...], ...] = (),
    unique: tuple[tuple[str, ...], ...] = (),
):
    """Register a class as a concrete node for the given node type."""

    in_package = node_type in PACKAGE_NODE_TYPES
    in_bench = node_type in BENCH_NODE_TYPES

    def decorate(cls: type["Node"]) -> type["Node"]:
        cls = node_component(
            node_type=node_type,
            passthrough_get=passthrough_get,
            passthrough_set=passthrough_set,
            is_root=len(roots) == 0,
            is_variable_root=len(roots) > 1,
            is_final=True,
        )(cls)
        cls.__is_stored__ = stored
        cls.__is_stored_value_unraveled__ = stored_value_unraveled

        if node_type.is_global:
            cls.__area__ = NodeArea.GLOBAL
        elif node_type.is_regional:
            cls.__area__ = NodeArea.REGIONAL
        elif node_type.is_local:
            cls.__area__ = NodeArea.LOCAL
        else:
            raise ValueError(f"unknown node store for {node_type}")

        cls.__extra_indexes__ = indexes
        cls.__extra_uniques__ = unique

        cls.__roots__ = bittuple(*roots, enum_cls=NodeType)
        cls.__is_in_package__ = in_package
        cls.__is_in_bench__ = in_bench

        if IS_DEV:
            if issubclass(cls, (Struct, Struct)):
                raise ValueError(f"{cls} is a struct")

        return cls

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def node_subtype_(
    subtype: IdEnum,
    passthrough_get: str | tuple[str, ...] | None = None,
    passthrough_set: str | tuple[str, ...] | None = None,
):
    """
    Mark a class as a subtype class of an ancestor node class.
    """

    def decorate(cls: type["Node"]) -> type["Node"]:
        for c in cls.__mro__[::-1]:
            if getattr(c, "metatype", None):
                base_cls = cast(type["Node"], c)
                assert issubclass(base_cls, Node), f"expected Node, got {base_cls} for {cls}"
                break
        else:
            raise RuntimeError(f"no base class found for {cls}")
        cls = node_component(
            node_type=base_cls.metatype,
            passthrough_get=passthrough_get,
            passthrough_set=passthrough_set,
            is_root=len(base_cls.__roots__) == 0,
            is_variable_root=len(base_cls.__roots__) > 1,
            is_final=True,
            is_subtype=True,
        )(cls)

        # cannot instantiate node subtypes directly (only through base class)
        def _fail_init(self, *args, **kwargs):
            raise TypeError(f"cannot instantiate node subtype {cls} directly")

        cls.__init__ = _fail_init

        # check
        if IS_DEV:
            # check name is good
            target_name = f"{to_casing(subtype.name, Casing.CAMEL)}{base_cls.__name__}"
            assert cls.__name__ == target_name, f"expected {target_name}, got {cls.__name__}"

            # check properties
            for prop in cls.__declared_properties__.values():
                if prop.reference_kind == ReferenceKind.NODE_CHILDREN:
                    raise ValueError(f"can't have children {prop!r} in subtype {cls}")
                if prop.id is not None and prop.id < 100:
                    raise ValueError(f"shouldn't have id < 100 {prop!r} in subtype {cls}")

        # register
        cls.__base_class__ = base_cls
        if subtype in base_cls.__subclass_by_subtype__:
            raise ValueError(f"subtype {subtype} already registered for {base_cls}")
        base_cls.__subclass_by_subtype__[subtype] = cls
        base_cls.__subtype_by_subclass__[cls] = subtype
        base_cls.__has_subtypes__ = True
        base_cls.__subtype_base_property__ = base_cls.__properties__["type"]
        cls.__subtype__ = subtype
        subtype_extra_properties = {
            p.name: p for p in cls.__properties__.values() if p.name not in base_cls.__properties__
        }
        cls.__subtype_extra_properties__ = frozendict(subtype_extra_properties)
        cls.__subtype_extra_original_properties__ = frozendict(
            {p.name: p for p in subtype_extra_properties.values() if p.reference_source is None}
        )

        # add subtype key to properties
        for prop in cls.__subtype_extra_properties__.values():
            if prop.subtype_key is None and type(prop.key) is str:
                prop.subtype_key = f"{subtype.value}.{prop.key}"

        return cls

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def timed_node_(
    node_type: NodeType,
    passthrough_get: str | tuple[str, ...] | None = None,
    passthrough_set: str | tuple[str, ...] | None = None,
    indexes: tuple[tuple[str, ...], ...] = (),
):
    """Register a class as a concrete node for the given node type."""
    return node_(
        node_type=node_type,
        passthrough_get=passthrough_get,
        passthrough_set=passthrough_set,
        local=True,
        indexes=(*indexes, ("created_at",)),
    )


# NOTE: we don't track the inner _do_set in these computed properties because they're called
#  via BuiltinObject._do_set already (it's called for every set), which applies the
#  track/validate level at the outer level if they are required.
#  (this is quite neat because it means we don't need to propagate the track/validate flags)


def _object_property_ref(prop: Property) -> property:
    """The computed get/set property for a property reference."""

    wired_prop = prop.reference_wired_ptr
    assert wired_prop is not None, f"no wired prop for {prop!r}"

    if not prop.is_list:

        def _get_property_scalar(self: BuiltinObject) -> Optional[Property]:
            value_ptr: PropertyReference | None = getattr(self, wired_prop.name)
            if value_ptr is not None:
                return value_ptr.resolve_or_error()
            else:
                return None

        def _set_property_scalar(self: BuiltinObject, value: Property | None):
            if value is None:
                self._do_set(wired_prop.name, None, track=False, validate=False)
            else:
                self._do_set(wired_prop.name, value.to_ref(), track=False, validate=False)

        return property(_get_property_scalar, _set_property_scalar)

    else:

        def _get_properties_many(self: BuiltinObject) -> tuple[Property, ...]:
            value_ptrs: list[PropertyReference] = getattr(self, wired_prop.name)
            assert type(value_ptrs) is list, f"invalid {prop!r}: {value_ptrs!r}"
            return tuple(p.resolve_or_error() for p in value_ptrs)

        def _set_properties_many(self: BuiltinObject, values: Collection[Property]):
            self._do_set(wired_prop.name, [p.to_ref() for p in values], track=False, validate=False)

        return property(_get_properties_many, _set_properties_many)


def _object_node_ref(prop: Property) -> property:
    """The computed get/set property for a node reference. Resolved against the active supergraph."""

    wired_prop = prop.reference_wired_ptr
    assert wired_prop is not None, f"no wired prop for {prop!r}"

    if not prop.is_list:

        def _get_node_scalar(self: BuiltinObject) -> Optional["Node | NodeReference"]:
            value_ptr: NodeReference | None = getattr(self, wired_prop.name)
            if value_ptr is None:
                return None
            value = self._supergraph.get(value_ptr)
            if value is None and self._session is not None:
                # node may already have been removed
                value = self._session._pending_nodes_by_id.get(cast(UUID, value_ptr.id))
            if value is not None:
                return value
            else:
                return None

        def _set_node_scalar(self: BuiltinObject, value: "Node | None"):
            if value is None:
                self._do_set(wired_prop.name, None, track=False, validate=False)
            else:
                self._do_set(wired_prop.name, value.to_ref(), track=False, validate=False)

        return property(_get_node_scalar, _set_node_scalar)

    else:

        def _get_node_many(self: BuiltinObject) -> Sequence["Node | NodeReference"]:
            value_ptrs: Collection[NodeReference] = getattr(self, wired_prop.name)
            if len(value_ptrs) == 0:
                return ()
            values = []
            for value_ptr in value_ptrs:
                value = self._supergraph.get(value_ptr)
                if value is None and self._session is not None:
                    # node may already have been removed
                    value = self._session._pending_nodes_by_id.get(cast(UUID, value_ptr.id))
                if value is not None:
                    values.append(value)
            return values

        def _set_node_many(self: BuiltinObject, values: Collection["Node"]):
            values = tuple(values)
            value_ptrs = [p.to_ref() for p in values]
            self._do_set(wired_prop.name, value_ptrs, track=False, validate=False)

        return property(_get_node_many, _set_node_many)


def _object_node_ref_attr(ptr_key: str, prop: Property) -> property:
    """The computed get property from a specific attribute of a node pointer."""

    wired_prop = prop.reference_wired_ptr
    assert wired_prop is not None, f"no wired prop for {prop!r}"

    if not prop.is_list:

        def _get_node_ref_attr_scalar(self):
            value_ptr = getattr(self, wired_prop.name)
            if value_ptr is not None:
                return getattr(value_ptr, ptr_key)
            else:
                return None

        def _set_node_ref_attr_scalar(self: BuiltinObject, value):
            raise RuntimeError(f"cannot set computed property attribute {prop!r}: {value!r}")

        return property(_get_node_ref_attr_scalar, _set_node_ref_attr_scalar)

    else:

        def _get_node_ref_attr_many(self: BuiltinObject):
            value_ptrs = getattr(self, wired_prop.name)
            assert type(value_ptrs) is list, f"invalid {prop}: {value_ptrs!r}"
            return tuple(getattr(p, ptr_key) for p in value_ptrs)

        def _set_node_ref_attr_many(self: BuiltinObject, values):
            raise RuntimeError(f"cannot set computed property attribute {prop!r}: {values!r}")

        return property(_get_node_ref_attr_many, _set_node_ref_attr_many)


def _node_ancestor_ref(prop: Property) -> property:
    """The computer get property for Node ancestors."""

    # NOTE :Performance: _node_ancestor_ref could just walk in the graph directly?

    assert prop.reference_nodes != "any", f"unexpected {prop.reference_nodes!r} for {prop!r}"

    if prop.reference_kind == ReferenceKind.NODE_ANCESTOR_OR_SELF:

        def get_ancestor_first_self(self: Node) -> Optional[Node]:
            parent = self
            while parent is not None:
                if prop.reference_nodes and parent.metatype in cast(
                    tuple[NodeType, ...], prop.reference_nodes
                ):
                    return parent
                parent = parent.parent
            return None

        get = get_ancestor_first_self

    elif prop.reference_kind == ReferenceKind.NODE_ANCESTOR:

        def get_ancestor_first_other(self: Node) -> Optional[Node]:
            parent = self.parent
            farthest = None
            while parent is not None:
                if prop.reference_nodes and parent.metatype in cast(
                    tuple[NodeType, ...], prop.reference_nodes
                ):
                    farthest = parent
                parent = parent.parent
            return farthest

        get = get_ancestor_first_other

    else:
        raise ValueError(f"unexpected ancestor reference kind: {prop.reference_kind}")

    def set(self: Node, value: Node):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get, set)


def _node_ancestor_ptr_ref(prop: Property) -> property:
    """The computed get property for Node ancestor pointers (computed because ancestors are computed)."""

    wired_prop = prop.reference_wired_ptr
    assert wired_prop is not None, f"no wired prop for {prop!r}"

    def get_ancestor_ptr(self: "Node") -> Optional["NodeReference"]:
        ancestor = getattr(self, cast(Property, wired_prop.reference_source).name)
        if ancestor is None:
            return None
        else:
            return ancestor.to_ref()

    def set(self: Node, value: "NodeReference"):
        raise NotImplementedError(f"cannot set computed property {wired_prop!r}: {value!r}")

    return property(get_ancestor_ptr, set)


def _trace_edit_operation(
    obj: "CustomObject | Struct | Node | None",
    key: Union["Property", "Field"],
    *,
    new_value: Any | None,
    old_value: Any | None,
    subtype: int | None,
):
    """Traces an edit operation to the given object."""
    # figure out if we're in a tracked node (before we start tracing the edit)
    if key.is_list and not (new_value is None or isinstance(new_value, list)):
        return  # ignore, not tracking edits to individual list items yet
    node = None
    o = obj
    while o is not None and type(o) is not Property:
        # (o may be a Property during setup when o.parent is not initialized)
        if isinstance(o, Node):
            node = cast("Node", o)
            break
        else:
            o = o.parent
    if node is None or node._is_new or node._session is None:
        return  # ignore, not tracked

    # trace path for edit
    path: list[str] = [key.key]
    k = key
    while obj is not None and not isinstance(obj, Node):
        parent = obj.parent
        if parent is not None:
            k = obj.parent_key
            assert k is not None, f"{obj!r} has no parent key"
            parent_key_str = k.key
            assert (
                type(parent_key_str) is str
            ), f"{obj!r} has non-str key {parent_key_str!r} in {parent!r}"
            path.insert(0, parent_key_str)
        obj = parent

    # figure out subtype if we're coming from a nested property
    if subtype is None and node.__has_subtypes__:
        assert type(k) is Property, f"expected Property, got {k!r} for {key!r} in {node!r}"
        subtype = cast(type[Node], k.component).__subtype__

    # add subtype to path
    if subtype is not None:
        path.insert(0, str(subtype))
        path.insert(0, Node.get_property("subnode_packed").key)

    # pack edit operation content
    operation_type = EditOperationType.CLEAR if new_value is None else EditOperationType.SET
    if type(key) is Property:
        typ = key._type_info
        assert typ is not None, f"{key!r} in {obj!r} has no type info"
    else:
        typ = cast("Field", key)
    old_value_packed = None if old_value is None else pack_value_data(old_value, typ)
    old_value_packed = pack_proto_json(old_value_packed)
    new_value_packed = None if new_value is None else pack_value_data(new_value, typ)
    new_value_packed = pack_proto_json(new_value_packed)

    try:
        operation = EditOperationData(
            metatype=lang_pb2.OBJECT_TYPE_EDIT_OPERATION,
            type=operation_type,  # type: ignore
            path=path,
            new_value_packed=new_value_packed,
            old_value_packed=old_value_packed,
        )
    except BaseException as e:
        raise ValueError(f"failed to pack edit operation {e!r} for {node!r} {key!r}") from e
    node._session._update(node, operation)


_HANDLING_ATTRIBUTE_ERROR = contextvars.ContextVar("handling_attribute_error", default=False)


@object_()
class BuiltinObject[ObjectDataT: AnyObjectData](abc.ABC):
    """The base for all intrinsic objects like Structs and Nodes and all their derivatives."""

    metatype: ClassVar[ObjectType]
    __components__: ClassVar[tuple[type["BuiltinObject"], ...]] = ()
    __passthrough_get__: ClassVar[tuple[str, ...] | None] = None
    __passthrough_set__: ClassVar[tuple[str, ...] | None] = None

    __is_struct__: ClassVar[bool] = False
    __is_node__: ClassVar[bool] = False

    __parent_property__: ClassVar[Property] = UNSET
    __parent_types__: ClassVar[tuple[NodeType, ...]] = ()
    __properties__: ClassVar[dict[str, Property]] = {}
    __properties_by_id__: ClassVar[dict[int, Property]] = {}
    __properties_name_by_id__: ClassVar[dict[int, str]] = {}
    __original_properties__: ClassVar[dict[str, Property]] = {}  # excl. contributed
    __own_properties__: ClassVar[dict[str, Property]] = {}
    __declared_properties__: ClassVar[dict[str, Property]] = {}
    __tracked_properties__: ClassVar[dict[str, Property]] = {}
    __internal_properties__: ClassVar[dict[str, Property]] = {}
    __node_reference_properties__: ClassVar[dict[str, Property]] = {}
    __property_reference_properties__: ClassVar[dict[str, Property]] = {}
    __struct_reference_properties__: ClassVar[dict[str, Property]] = {}
    __sensitive_properties__: ClassVar[dict[str, Property]] = {}
    __struct_properties__: ClassVar[dict[str, Property]] = {}
    __value_runtime_properties__: ClassVar[dict[str, Property]] = {}
    __stored_properties__: ClassVar[dict[str, Property]] = {}
    __wired_properties__: ClassVar[dict[str, Property]] = {}
    __runtime_properties__: ClassVar[dict[str, Property]] = {}
    __properties_in_order__: ClassVar[tuple[Property, ...]]
    __properties_id_in_order__: ClassVar[tuple[int, ...]]
    __max_property_ord__: ClassVar[int] = UNSET
    __properties_mask_set__: ClassVar[bitarray] = UNSET
    __properties_mask_unset__: ClassVar[bitarray] = UNSET

    if TYPE_CHECKING:
        parent: "BuiltinObject | CustomObject | None" = None
        _skip_validate_self: InitVar[bool] = False
        _skip_extra_kwargs: InitVar[bool] = False

    _session: "Session | None" = p_runtime(default=None)
    _supergraph: "NodeSuperGraph" = p_runtime(default=None)

    def __init__(
        self, *, _skip_validate_self: bool = False, _skip_extra_kwargs: bool = False, **kwargs
    ):
        assert (
            _SETUP_STAGE >= _SetupStage.FINALIZING
        ), f"{self.__class__.__name__} requires completed setup"

        self_dict = self.__dict__

        # init session / supergraph context (first)
        self_dict["_session"] = kwargs.pop("_session", None) or _active_session.get()
        supergraph: NodeSuperGraph
        if "_supergraph" in kwargs:
            supergraph = cast(NodeSuperGraph, kwargs.pop("_supergraph"))
        elif self_dict.get("_session") is not None:
            supergraph = cast("Session", self_dict["_session"])._supergraph
        else:
            supergraph = NULL_SUPERGRAPH
        self_dict["_supergraph"] = supergraph

        # init object (from kwargs & defaults)
        for prop in self.__runtime_properties__.values():
            if (
                prop.reference_source is not None
                or prop.name == "_session"
                or prop.name == "_supergraph"
                or prop.is_computed
            ):
                continue  # only handle top level properties
            wired_ptr_prop = prop.reference_wired_ptr
            prop_value = kwargs.get(prop.name, UNSET)

            # also get wired pointer if available
            if wired_ptr_prop is not None:
                wired_prop_value = kwargs.get(wired_ptr_prop.name, UNSET)
            else:
                wired_prop_value = UNSET

            # init node references if nodes are passed directly
            if prop.is_node_reference and prop_value is not UNSET:
                assert wired_ptr_prop is not None, f"no wired prop for {prop!r}"
                if wired_prop_value is not UNSET:
                    raise ValueError(
                        f"got both {prop} and {wired_ptr_prop}: {prop_value!r}, {wired_prop_value!r}"
                    )
                # init node references (must be in same)
                if prop_value is None:
                    wired_prop_value = None
                elif not prop.is_list:
                    wired_prop_value = cast(Any, prop_value).to_ref()
                else:
                    wired_prop_value = [p.to_ref() for p in prop_value]
                self_dict[wired_ptr_prop.name] = wired_prop_value
                continue
            # init property references if properties are passed directly
            elif prop.reference_kind == ReferenceKind.PROPERTY and prop_value is not UNSET:
                assert wired_ptr_prop is not None, f"no wired prop for {prop!r}"
                if wired_prop_value is not UNSET:
                    raise ValueError(
                        f"got both {prop!r} and {wired_ptr_prop!r}: {prop_value!r}, {wired_prop_value!r}"
                    )
                if prop_value is None:
                    wired_prop_value = None
                elif prop.is_list:
                    wired_prop_value = [p.to_ref() for p in prop_value]
                else:
                    wired_prop_value = cast(Any, prop_value).to_ref()
                self_dict[wired_ptr_prop.name] = wired_prop_value
                continue
            # move struct values into this object if passed
            elif prop.reference_kind == ReferenceKind.STRUCT_CHILD:
                if prop.is_list:
                    # and init value list
                    assert prop.reference_list_type is not None, f"no list type for {prop!r}"
                    value_list = prop.reference_list_type(cast(Node, self), prop)
                    if isinstance(prop_value, list):
                        value_list.extend(prop_value)  # will auto copy if needed
                    prop_value = value_list
                elif isinstance(prop_value, Struct):
                    prop_value = prop_value._move_to(cast("Node | Struct", self), prop)

            # default value
            if prop_value is UNSET:
                if prop.default_factory is not None:
                    prop_value = prop.default_factory()
                elif prop.default is not UNSET:
                    prop_value = prop.default
                elif not prop.is_required:
                    prop_value = None if not prop.is_list else []
                else:
                    raise ValueError(f"missing required value for {prop!r}")
            if wired_ptr_prop is not None and wired_prop_value is UNSET:
                if prop.is_list:
                    wired_prop_value = []
                elif not prop.is_required:
                    wired_prop_value = None
                else:
                    raise ValueError(f"missing required value for {wired_ptr_prop!r}")

            # and set it
            if prop_value is UNSET:
                raise ValueError(f"missing required value for {prop!r}")
            self_dict[prop.name] = prop_value
            if wired_ptr_prop is not None:
                self_dict[wired_ptr_prop.name] = wired_prop_value

        # init object values (from kwargs)
        for prop in self.__value_runtime_properties__.values():
            prop_value = kwargs.get(
                prop.name,
            )
            if prop_value is not None:
                object.__setattr__(self, prop.name, prop_value)

        # validate self
        if self._session is not None and not _skip_validate_self:
            self._validate_self((), invalid=on_invalid_raise)

        # check for extraneous kwargs
        if not _skip_extra_kwargs:
            self._init_extra_kwargs(kwargs)

    def _get_effective_cls(self) -> type["BuiltinObject"]:
        """Gets the class we're imitating (for subtypes)."""
        return type(self)

    def _init_extra_kwargs(self, kwargs: dict[str, Any]):
        """
        Initialize the object from any extraneous kwargs (try to stuff into values) if relevant.
        Otherwise just error.
        """
        cls = self._get_effective_cls()
        if self._session is not None and any(key not in cls.__properties__ for key in kwargs):
            if not self.__value_runtime_properties__:
                key = first(key for key in kwargs if key not in cls.__properties__)
                raise AttributeError(f"{self!r} has no attribute '{key}'")
            # try to stuff extra kwargs into first custom value property
            value_runtime_key = first(self.__value_runtime_properties__)
            value = getattr(self, value_runtime_key)
            if value is None:
                key = first(key for key in kwargs if key not in cls.__properties__)
                raise AttributeError(f"{self!r} has no attribute '{key}'")
            for key in kwargs:
                if key not in cls.__properties__:
                    setattr(value, key, kwargs[key])

    def __content_str__(self) -> str:
        return ""  # empty by default

    @final
    def __str__(self):
        return self.__content_str__()

    def __eq__(self, other):
        if other is self:
            return True
        elif other is None:
            return False
        else:
            return self.equals(other)

    def _prop_value_equals(
        self,
        prop: Property,
        self_value: Any,
        other_value: Any,
        identity_map: Mapping[UUID, "NodeReference"] = EMPTY_DICT,
    ) -> bool:
        """Checks whether two values for a given property are equal (recursively)."""
        if prop.is_struct:
            # apply identity map
            if prop.is_node_reference:
                if prop.is_list:
                    self_value = (
                        [identity_map.get(v.ck, v) for v in self_value] if self_value else ()
                    )
                    other_value = (
                        [identity_map.get(v.ck, v) for v in other_value] if other_value else ()
                    )
                else:
                    self_value = identity_map.get(self_value.ck, self_value) if self_value else None
                    other_value = (
                        identity_map.get(other_value.ck, other_value) if other_value else None
                    )

            # compare struct recursively
            if prop.is_list:
                if (
                    not isinstance(self_value, Sequence)
                    or not isinstance(other_value, Sequence)
                    or len(self_value) != len(other_value)
                ):
                    return False  # unequal list
                for i in range(len(self_value)):
                    if not self_value[i].equals(other_value[i], identity_map=identity_map):
                        return False  # unequal struct
            else:
                if type(self_value) is not type(other_value) or (
                    self_value is not None
                    and not cast(Struct, self_value).equals(other_value, identity_map=identity_map)
                ):
                    return False  # unequal struct
        else:
            # compare primitives
            if self_value != other_value and not is_close(self_value, other_value, FLOAT_EPSILON):
                return False  # unequal primitive

        return True

    def equals(
        self,
        other: Self | Any,
        identity_map: Mapping[UUID, "NodeReference"] = EMPTY_DICT,
    ) -> bool:
        """Checks if the content of the two objects is equal (recursively)."""
        if other is None or self.metatype != getattr(other, "metatype", None):
            return False
        for prop in self.__wired_properties__.values():
            if (
                prop.id < 30
                or prop.is_encrypted
                or prop.is_value_packed  # compared in runtime value
                or prop.name == "order_key"  # implicitly checked in lists
                or prop.name in HasTracingContext.__properties__
            ):
                continue  # ignore identity/tracking
            self_value = getattr(self, prop.name)
            other_value = getattr(other, prop.name)
            if not self._prop_value_equals(prop, self_value, other_value, identity_map):
                return False
        return True

    def _stable_hash(self) -> int:
        """Hash of content properties."""
        content_props = []
        for prop in self.__wired_properties__.values():
            if prop.id < 30:
                continue
            prop_value = getattr(self, prop.name)
            if prop.is_list and prop_value:
                for item in prop_value:
                    if isinstance(item, BuiltinObject):
                        content_props.append(item._stable_hash())
                    else:
                        content_props.append(item)
            else:
                if isinstance(prop_value, BuiltinObject):
                    content_props.append(prop_value._stable_hash())
                else:
                    content_props.append(prop_value)
        return stable_hash(content_props)

    def _patch_from(self, other: Self):
        """Patches this Node *in place* from another Node."""
        for prop in self.__wired_properties__.values():
            if prop.is_computed:
                continue  # ignore computed properties
            prop_value = getattr(other, prop.name)
            self._do_set(prop.name, prop_value, track=False, validate=False)

    def _do_get(self, key):
        """Called if an attribute doesn't exist in __dict__ / the usual places."""

        # check passthrough (if in a session)
        if self.__passthrough_get__ is not None and self._session is not None:
            for passthrough_key in self.__passthrough_get__:
                target = getattr(self, passthrough_key, UNSET)
                attr = getattr(target, key, UNSET)
                if attr is not UNSET:
                    return attr

        # attribute error
        # NOTE: we only try to repr once in a call chain to prevent recursive repr errors
        #  (this is rare, but can happen for instance when an init partially fails)
        self_str = self.__class__.__name__
        if not _HANDLING_ATTRIBUTE_ERROR.get():
            _handling_token = _HANDLING_ATTRIBUTE_ERROR.set(True)
            try:
                self_str = repr(self)
            finally:
                _HANDLING_ATTRIBUTE_ERROR.reset(_handling_token)
        raise AttributeError(f"{self_str} has no attribute '{key}'")

    def _do_set(
        self,
        key: str,
        new_value: Any,
        *,
        track: bool = True,
        coerce: bool = True,
        validate: bool = True,
    ):
        """Sets *any* attribute on this builtin object."""
        prop = self.__properties__.get(key)
        if prop is not None:
            # set regular property
            if prop.is_untracked:
                object.__setattr__(self, key, new_value)
                return

            # validate/set
            if track:
                if prop.is_value_runtime:
                    old_value = getattr(self, prop.value_packed_ptr.name)  # type: ignore
                else:
                    old_value = getattr(self, key)
                if validate:
                    # coerce & check type (if it's not a contributed property, which only we edit)
                    if prop._type_info is not None and prop.reference_source is None:
                        if coerce:
                            new_value = coerce_value(
                                new_value,
                                prop._type_info,
                                parent=cast("Struct | Node", self),
                                parent_key=prop,
                                supergraph=self._supergraph,
                            )
                        check_value(
                            new_value,
                            prop._type_info,
                            options=DEFAULT_CHECK_OPTIONS,
                            invalid=on_invalid_raise,
                        )
                    object.__setattr__(self, key, new_value)
                    try:
                        self._validate_self((prop,), invalid=on_invalid_raise)
                    except ValidationError:  # reset on error
                        object.__setattr__(self, key, old_value)
                        raise
                else:
                    object.__setattr__(self, key, new_value)
                if prop.is_value_runtime:
                    _trace_edit_operation(
                        cast("Struct | Node", self),
                        prop.value_packed_ptr,  # type: ignore
                        new_value=getattr(self, prop.value_packed_ptr.name),  # type: ignore
                        old_value=old_value,
                        subtype=None,
                    )
                else:
                    _trace_edit_operation(
                        cast("Struct | Node", self),
                        prop,
                        new_value=new_value,
                        old_value=old_value,
                        subtype=None,
                    )
            else:
                object.__setattr__(self, key, new_value)

            return
        elif track and self.__passthrough_set__ is not None:
            # try passthrough target (if any)
            for passthrough_key in self.__passthrough_set__:
                target = getattr(self, passthrough_key, UNSET)
                if target is not UNSET:
                    setattr(target, key, new_value)
                    return

        # attribute error
        try:
            self_str = repr(self)
        except Exception:
            self_str = self.__class__.__name__
        raise AttributeError(f"{self_str} has no attribute '{key}'")

    if not TYPE_CHECKING:
        # NOTE: __setattr__/__getattr__ confuses type checking, so only define it at runtime
        #  (we don't need it since dynamic access is meant for Values at runtime)
        __getattr__ = _do_get
        __setattr__ = _do_set

    def is_set(self, key: Property, value: Any = UNSET) -> bool:
        """Whether a key is set on this object."""
        if value is UNSET:
            value = getattr(self, key.name)
        if not key.is_list:
            if value is not None and (key.default is None or value != key.default):
                return True
        else:
            if type(value) is list and len(value) > 0:
                return True
        return False

    def clone(self, *, reset: bool = True) -> Self:
        """
        Create a clone of this object and its descendants (structs/nodes) with the same content.
        """
        copy_kwargs = {}
        for prop in self.__wired_properties__.values():
            prop_value = getattr(self, prop.name)
            if reset and prop.id < 30:
                continue  # ignore tracking/autoset properties
            if prop.is_struct:
                if prop.is_list:
                    if prop_value:
                        copy_kwargs[prop.name] = [item.clone() for item in prop_value]
                elif prop_value is not None:
                    copy_kwargs[prop.name] = prop_value.clone()
            else:
                if prop.is_list:
                    if prop_value:
                        copy_kwargs[prop.name] = list(prop_value)
                else:
                    copy_kwargs[prop.name] = prop_value
        return self.__class__(**copy_kwargs)

    def replace_references(self, new_node_by_id: Mapping[UUID, "Node"]):
        """Replaces Node references with new Nodes. Missing Nodes are kept as is."""
        for prop in self.__node_reference_properties__.values():
            if (
                prop.reference_kind == ReferenceKind.NODE_PARENT
                or prop.reference_kind == ReferenceKind.NODE_CHILDREN
            ):
                continue
            prop_value = getattr(self, prop.name)
            if not prop.is_list:
                if prop_value is None:
                    continue
                new_node = new_node_by_id.get(prop_value.id)
                if new_node is not None:
                    setattr(self, prop.name, new_node)
            else:
                if type(prop_value) is RemoteNodeList or len(prop_value) == 0:
                    continue
                new_nodes = [new_node_by_id.get(node.id) for node in prop_value]
                prop_value.set(new_nodes)

    @property
    def active_session(self) -> "Session":
        """The currently active session (errors if none)"""
        assert self._session is not None, f"no session for {self!r}"
        return self._session

    def _walk_struct(self) -> Iterable["BuiltinObject"]:
        yield self
        for prop in self.__struct_properties__.values():
            value: Struct | list[Struct] | None = getattr(self, prop.name)
            if value is None:
                continue
            elif not prop.is_list:
                yield from (cast(Struct, value))._walk_struct()
            elif len(cast(list, value)) > 0:
                for item in cast(list, value):
                    yield from cast(Struct, item)._walk_struct()

    @final
    def _validate_self(
        self, properties: tuple[Property, ...], invalid: "ValidationHandler"
    ) -> None:
        """Check the integrity of this object."""
        # NOTE :Architecture: BuiltinObject.validate does not validate custom objects, but .do_set does
        #  (this is somewhat inconsistent, but also useful because e.g. for Runs we don't want to error
        #   during validation when unpacking, only later when manually checking the inputs;
        #   however Run.inputs = ... directly errors if invalid, which is inconsistent but convenient.
        #   Maybe add a flag to .validate whether to validate values?)
        # check properties types
        for prop in properties or self.__tracked_properties__.values():
            # NOTE: references may be unloaded and there's not much to validate, so we don't
            if prop._type_info is not None and prop.reference_kind is None:
                value = getattr(self, prop.name)
                check_value(value, prop._type_info, options=DEFAULT_CHECK_OPTIONS, invalid=invalid)

    @final
    def _validate_rec(self, invalid: "ValidationHandler"):
        # check inner structs
        for inner_struct in self._walk_struct():
            inner_struct._validate_self((), invalid)

    def __bool__(self):
        return True  # support truthy checks for objects

    @final
    def _to_data(self) -> ObjectDataT:
        """Convert to wire format"""
        from bench.proto.wiring import pack_builtin_object

        return pack_builtin_object(self)  # type: ignore

    @classmethod
    def get_property(cls, key: str) -> Property:
        prop = cls.__properties__.get(key)
        if prop is None:
            raise ValueError(f"no property {key} in {cls}")
        return prop

    @classmethod
    def get_value_property(
        cls,
        field_type: FieldType,
        kind: Literal["runtime", "packed"] = "runtime",
    ) -> Property:
        """Gets the value property for the given object kind."""
        for prop in cls.__properties__.values():
            if prop.is_value_runtime and (
                prop.value_field_type is None or prop.value_field_type == field_type
            ):
                if kind == "runtime":
                    return prop
                elif kind == "packed":
                    assert type(prop.value_packed_ptr) is Property, f"no wired prop for {prop!r}"
                    return prop.value_packed_ptr
        else:
            raise ValueError(
                f"no value property for {field_type.bench_name} field type in {cls.__name__}"
            )

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


StructParent = Union["Struct", "Node", "CustomObject"]
StructParentKey = Union["Property", "Field"]


@object_()
class Struct[StructDataT: AnyStructData](BuiltinObject[StructDataT], abc.ABC):
    """A Struct is an ordered collection of Properties."""

    metatype: ClassVar[StructType]  # type: ignore

    __is_struct__: ClassVar[bool] = True

    parent: StructParent | None = p_struct_parent(3)
    parent_key: StructParentKey | None = p_runtime(default=None)

    def __content_str__(self) -> str:
        # default __content_str__ for Structs with all set properties
        value_strs = []
        for prop in self.__declared_properties__.values():
            prop_value = getattr(self, prop.name)
            if prop_value is not None and not (isinstance(prop_value, Sequence) and not prop_value):
                if prop.is_enum:
                    if prop.is_list:
                        prop_value_str = "|".join(p.bench_name for p in prop_value)
                    else:
                        prop_value_str = prop_value.bench_name  # type: ignore
                elif prop.reference_struct:
                    if prop.is_list:
                        prop_value_str = f"{prop.reference_struct.bench_name}[{len(prop_value)}]"
                    else:
                        prop_value_str = f"<{prop.reference_struct.bench_name} ...>"
                else:
                    prop_value_str = repr(prop_value)
                value_strs.append(f"{prop.name}={prop_value_str}")
        return ", ".join(value_strs)

    @final
    def __repr__(self):
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str}>"
        else:
            return f"<{self.__class__.__name__}>"

    def replace(self, **kwargs) -> Self:
        """Replaces specific properties in this struct (in a copy)."""
        copy = self.clone()
        for prop_name, prop_value in kwargs.items():
            setattr(copy, prop_name, prop_value)
        return copy

    def override(self, override: "Self | None" = None, copy: bool = True, **kwargs) -> Self:
        """Overrides this Struct with set properties from another struct (in a copy)."""
        if override is None and len(kwargs) == 0:
            return self
        clone = self.clone() if copy else self
        if override is not None:
            for prop in self.__declared_properties__.values():
                override_value = getattr(override, prop.name)
                if clone.is_set(prop, override_value):
                    setattr(clone, prop.name, override_value)
        for key, value in kwargs.items():
            setattr(clone, key, value)
        return clone

    def set_default(self, override: "Self", copy: bool = True, **kwargs) -> Self:
        """Sets default values from another Struct. Like override but only sets if unset."""
        clone = self.clone() if copy else self
        for prop in override.__declared_properties__.values():
            override_value = getattr(override, prop.name)
            if clone.is_set(prop, override_value) and not self.is_set(prop):
                setattr(clone, prop.name, override_value)
        return clone

    def _move_to(
        self,
        parent: StructParent,
        parent_key: StructParentKey,
    ) -> Self:
        """Move or copy this Struct into given parent/prop."""
        assert self.__is_struct__, f"cannot copy non-struct {self!r}"  # this is overriden by Node
        if self.parent is None:  # detached
            self._do_set("parent", parent, track=False)
            self._do_set("parent_key", parent_key, track=False)
            return self
        else:
            copy = self._copy_to(parent, parent_key)
            return copy

    def _copy_to(self, parent: StructParent, parent_key: StructParentKey) -> Self:
        """Create a copy of this Struct for the given parent/prop."""
        kwargs = {p.name: getattr(self, p.name) for p in self.__wired_properties__.values()}
        kwargs["parent"] = parent
        kwargs["parent_key"] = parent_key
        copy = self.__class__(**kwargs)
        return copy


FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type["Node"]]
EditSubject = Union["User", "Machine", "Block", "Action", "Run"]
EDIT_SUBJECT_TYPES = (
    NodeType.USER,
    NodeType.MACHINE,
    NodeType.BLOCK,
    NodeType.ACTION,
    NodeType.RUN,
)

Owner = Union["User", "Organization", "Block", "Action", "Run"]
OWNER_TYPES = (NodeType.USER, NodeType.ORGANIZATION, NodeType.BLOCK, NodeType.RUN)


def is_implicit_node_property(prop_id: int) -> bool:
    return prop_id < 30 and prop_id != 4  # parent is fine


@node_component()
class Node[NodeDataT: AnyNodeData](BuiltinObject[NodeDataT], abc.ABC):
    """
    A Node with properties (like a Struct) and a global identity in our graph.
    Conceptually, all Nodes live together happily in a single giant supergraph.
    In practice, there are multiple stores and we load smaller subgraphs at runtime.
    """

    # NOTE :Architecture!: we may need a better :NodeInheritance mechanism since some
    #  nodes have different subtypes with varying properties (e.g. Action or View).
    # Mapping their union into database columns is annoying since there may be many,
    #  so maybe this has to wait until we have custom storage engines.

    metatype: ClassVar[NodeType]  # type: ignore

    # NOTE :Test: make id factories deterministic (incl. UUIDT? somehow)
    __id_factory__: ClassVar[Callable[[], UUID]] = uuid4
    __ck_factory__: ClassVar[Callable[[], UUID]] = uuid4

    __node_child_properties__: ClassVar[dict[str, Property]] = frozendict()
    __subclass_by_subtype__: ClassVar[dict[IdEnum, type["Node"]]] = frozendict()
    __subtype_by_subclass__: ClassVar[dict[type["Node"], IdEnum]] = frozendict()
    __has_subtypes__: ClassVar[bool] = False
    __base_class__: ClassVar[type["Node"] | None] = None
    __subtype_base_property__: ClassVar["Property | None"] = None
    __subtype__: ClassVar[IdEnum | None] = None
    __subtype_extra_properties__: ClassVar[dict[str, Property]] = frozendict()
    __subtype_extra_original_properties__: ClassVar[dict[str, Property]] = frozendict()

    __roots__: ClassVar[bittuple[NodeType]] = UNSET
    __is_struct__: ClassVar[bool] = False
    __is_node__: ClassVar[bool] = True
    __is_in_bench__: ClassVar[bool] = UNSET  # part of a Bench
    __is_in_package__: ClassVar[bool] = UNSET  # part of a Package
    __is_stored__: ClassVar[bool] = False  # stored in primary store (runtime or local)
    __is_stored_value_unraveled__: ClassVar[bool] = False  # custom storage logic (for records)
    __area__: ClassVar[NodeArea]
    __extra_indexes__: ClassVar[tuple[tuple[str, ...], ...]] = ()  # extra indexes for PG
    __extra_uniques__: ClassVar[tuple[tuple[str, ...], ...]] = ()  # extra constraints for PG

    # 1-9: node identity
    # Node.metatype: 1
    id: UUID = p_system(2, default=None, require=True, autoset=True)
    # SourceNode.ck: 3
    parent: Optional["Node"] = p_node_parent(4)  # type: ignore
    if TYPE_CHECKING:
        parent_type: NodeType | None = None
        parent_id: Optional[UUID] = None
        parent_ck: Optional[UUID] = None
        parent_ptr: Optional[NodeReference] = None
    # BenchNode.bench: 5
    # PackageNode.package: 6
    # SourceNode.template: 7

    # 10-29: node tracking
    created_at: datetime = p_system(10, default=None, require=True, autoset=True)
    created_by: Optional[EditSubject] = p_system(  # type: ignore (pyright is wrong, EditSubject is a type)
        11,
        default=None,
        require=False,
        array=False,
        autoset=True,
        references=EDIT_SUBJECT_TYPES,
        same_bench=True,
        baseless=True,
    )
    updated_at: datetime = p_system(12, default=None, require=True, autoset=True)
    updated_by: Optional[EditSubject] = p_system(  # type: ignore (see above)
        13,
        default=None,
        require=False,
        array=False,
        autoset=True,
        references=EDIT_SUBJECT_TYPES,
        same_bench=True,
        baseless=True,
    )
    deleted_at: Optional[datetime] = p_system(14, default=None, autoset=True)
    # SourceNode.template_at: 15
    # ...managed_by? owned_by?
    if TYPE_CHECKING:
        created_by_id: Optional[UUID] = None
        created_by_type: NodeType | None = None
        updated_by_id: Optional[UUID] = None
        updated_by_type: NodeType | None = None
    # ...HasTracingContext[20-25]

    # NOTE :Architecture!: obviously, a better system would store subnodes directly
    #  (but we can't do that yet because we map top-level properties to Postgres columns,
    #   so we can't have crazy numbers of subtype-properties unless we pack them like this,
    #   similarly, out protobuf types would explode if we flatten out subnodes)
    subnode_packed: dict[str, dict[str, Any]] | None = p_subnode_packed(29)

    # 30-89 for general node/struct properties
    # ...

    _graph: "NodeGraph" = p_runtime(default=None)
    _connection: "GetConnection | SearchConnection" = p_runtime(default=None)
    _is_new: bool = p_runtime(default=False)

    def __init__(
        self, *, _skip_add_self: bool = False, _skip_validate_self: bool = False, **kwargs
    ):
        # inject :Tracing context
        cls = type(self)
        if isinstance(self, HasTracingContext):
            if not kwargs.get("mode"):
                tracing = get_tracing_context()
                kwargs["mode"] = tracing.mode

        # init object
        super().__init__(**kwargs, _skip_validate_self=True, _skip_extra_kwargs=True)

        # init node
        if isinstance(self, SourceNode):
            if self.ck is None:
                self.ck = cls.__ck_factory__()
                self.id = cls.__id_factory__()
                self._is_new = True
        elif self.id is None:
            self.id = cls.__id_factory__()
            self._is_new = True
        if self.created_at is None:
            if self._session is None and self.metatype == NodeType.SESSION:
                # 'bootstrap' session with itself
                self._session = cast("Session", self)
            assert self._session is not None, f"{self!r} is not in a session"
            now = self._session._oracle.utc()
            self.created_at = now
            self.updated_at = now

        # init graph (nodes must always be in a non-null supergraph & graph)
        assert self._supergraph is not NULL_SUPERGRAPH, f"no supergraph for {self!r}"
        if self._graph is not None:
            pass  # use given graph
        elif self.parent_ptr is not None:
            # use parents graph
            parent = self.parent
            assert (
                parent is not None
            ), f"parent for {type(self).__name__} not in {self._supergraph!r}: {self.parent_ptr!r}"
            self._graph = parent._graph
        else:
            # no parent, create our own graph
            # if we're not in a graph, start a new one
            # NOTE :Cleanup: NodeGraph definition "depends on itself", causing pyright errors
            graph = NodeGraph(  # type: ignore
                scope=EMPTY_SCOPE_DATA,
                node_types=(self.metatype, *DESCENDANT_NODE_TYPES[self.metatype].tuple),
                supergraph=self._supergraph,
            )
            self._graph = graph
            self._supergraph.add_graph(graph)
        if not _skip_add_self:
            self._graph.add(self)

        # init node lists (preserving existing)
        for name, prop in self.__node_child_properties__.items():
            assert prop.reference_list_type is not None
            node_list = prop.reference_list_type(self, prop)
            self.__dict__[name] = node_list
            existing = kwargs.get(name, UNSET)
            if existing is not UNSET:
                node_list.extend(*existing)

        # parse extraneous kwargs
        self._init_extra_kwargs(kwargs)

        # init session context
        if self._session is not None:
            if self._is_new and not _skip_validate_self:
                self._validate_self((), invalid=on_invalid_raise)
            self._track_self(self._session)

    @dualmethod
    def get_property(self, key: str) -> Property:  # type: ignore
        """Get a property by key from this instance."""
        prop = self._get_effective_cls().__properties__.get(key)
        if prop is None:
            raise ValueError(f"no property {key} in {self.__class__}")
        return prop

    @get_property.cls
    def get_property_cls(cls, key: str) -> Property:  # type: ignore  # noqa: N805
        """Get a property by key from the class."""
        prop = cls.__properties__.get(key)
        if prop is None:
            raise ValueError(f"no property {key} in {cls}")
        return prop

    @override
    def _get_effective_cls(self) -> type["Node"]:
        # extend BuiltinObject to point to subtype class if we have one
        if self.__has_subtypes__:
            subtype = self.__dict__["type"]
            subtype_cls = self.__subclass_by_subtype__.get(subtype)
            if subtype_cls is not None:
                return subtype_cls
        return type(self)

    @override
    def _init_extra_kwargs(self, kwargs: dict[str, Any]):
        # extend BuiltinObject to initialise subtype properties
        super()._init_extra_kwargs(kwargs)
        if self.__has_subtypes__:
            cls = self._get_effective_cls()
            subtype_key = str(self.__dict__["type"])
            if self.subnode_packed is None:
                subnode_packed = EMPTY_DICT
            else:
                subnode_packed = self.subnode_packed.get(subtype_key) or EMPTY_DICT
            for prop in cls.__subtype_extra_properties__.values():
                prop_value = kwargs.get(prop.name, UNSET)
                if prop_value is UNSET:
                    if prop.key in subnode_packed:
                        continue  # already set directly
                    if prop.default_factory is not None:
                        prop_value = prop.default_factory()
                    elif prop.default is not UNSET:
                        if prop.default is None:
                            continue  # skip optional None
                        prop_value = prop.default
                    elif not prop.is_required:
                        if prop_value.is_list:
                            prop_value = []
                        else:
                            continue  # skip optional None
                    else:
                        raise ValueError(f"missing required property: {prop!r}")
                self._do_set(prop.name, prop_value, track=False, validate=False)

    def __default_content_str__(self) -> str:
        """Default __content_str__ for Nodes with all set properties (incl. subtypes)."""
        value_strs = []
        properties = self.__declared_properties__.values()
        if self.__has_subtypes__:
            subtype = self.__dict__["type"]
            subtype_cls = self.__subclass_by_subtype__.get(subtype)
            if subtype_cls is not None:
                properties = chain(properties, subtype_cls.__subtype_extra_properties__.values())
        for prop in properties:
            if (
                prop.id is UNSET
                or prop.id is None
                or prop.reference_kind == ReferenceKind.NODE_CHILDREN
                or prop.is_sensitive
                or prop.id < 30
                or prop.name in ("type", "name", "order_key")
            ):
                continue
            prop_value = getattr(self, prop.name)
            if (
                prop_value is None
                or (isinstance(prop_value, Sequence) and not prop_value)
                or (
                    not isinstance(prop.default, BuiltinObject)
                    and type(prop_value) is type(prop.default)
                    and prop_value == prop.default
                )
            ):
                continue
            elif prop.is_enum:
                if prop.is_list:
                    prop_value_str = "|".join(p.bench_name for p in prop_value)
                else:
                    prop_value_str = prop_value.bench_name  # type: ignore
            elif isinstance(prop_value, Node):
                prop_value_str = f"<{prop_value._ident_key} ...>"
            elif (
                isinstance(prop_value, (list, tuple))
                and prop_value
                and isinstance(prop_value[0], Node)
            ):
                prop_value_str = f"[{', '.join(f'<{node.ident_str} ...>' for node in prop_value)}]"
            else:
                prop_value_str = repr(prop_value)
            value_strs.append(f"{prop.name}={prop_value_str}")
        return ", ".join(value_strs)

    @final
    def __str__(self):  # type: ignore
        # override the default __str__ for nodes
        content_str = self.__content_str__()
        if content_str:
            content_str = f" ({content_str})"
        status_str = " [deleted]" if self.deleted_at is not None else ""
        return f"{self._ident_key}{content_str}{status_str}"

    @final
    def __repr__(self):  # type: ignore
        # override the default __repr__ for nodes
        if self.__has_subtypes__:
            subtype = self.__dict__["type"]
            return f"<{subtype.bench_name}{self.__class__.__name__} {self!s}>"
        else:
            return f"<{self.__class__.__name__} {self!s}>"

    @property
    def ck(self):
        return self.id

    @property
    def is_attached(self) -> bool:
        return True

    def iter_descendants(self, recursive: bool = False):
        """Iterate over all descendants of this node."""
        for child_prop in self.__node_child_properties__.values():
            child_list = getattr(self, child_prop.name)
            if type(child_list) is not LocalNodeList:
                continue  # only iterate over local lists
            for child in child_list:
                yield child
                if recursive:
                    yield from child.iter_descendants(recursive=True)

    @override
    def clone(
        self,
        *,
        reset: bool = True,
        recursive: bool = True,
        detach: bool = False,
        map: bool | dict[UUID, "Node"] = True,
    ) -> Self:
        # clone self
        clone = super().clone(reset=reset)

        # clone children and append to self (recursive)
        if map is True:
            map = {self.id: clone}
        if recursive:
            for child_prop in self.__node_child_properties__.values():
                if child_prop.reference_list_type is not LocalNodeList:
                    continue  # only clone local lists
                child_list = getattr(self, child_prop.name)
                clone_list = getattr(clone, child_prop.name)
                for child in child_list:
                    child_clone = child.clone(recursive=True, detach=True, map=map)
                    clone_list.append(child_clone)  # re-attach
                    if type(map) is dict:
                        map[child.id] = child_clone

        # map new identities
        if type(map) is dict:
            for node in map.values():
                node.replace_references(map)

        # append to our parent to re-attach
        parent = self.parent
        if detach:
            clone.parent_ptr = None
        elif parent:
            parent.append(clone)
        return clone

    def __eq__(self, other: Any):
        """Equals node identity."""
        return self is other or (type(self) is type(other) and (self.id == other.id))

    def _stable_hash(self):
        """Hash node identity."""
        return stable_hash((self.metatype, self.id))

    # only define __hash__ for nodes since their id is constant
    __hash__ = _stable_hash  # type: ignore

    @override
    def equals(
        self, other: Self | Any, identity_map: Mapping[UUID, "NodeReference"] = EMPTY_DICT
    ) -> bool:
        if not super().equals(other, identity_map):
            return False
        if self.__has_subtypes__:
            # compare node subtype
            subtype = self.__dict__["type"]
            subtype_cls = self.__subclass_by_subtype__.get(subtype)
            if subtype_cls is not None:
                self_subnode_packed = self.__dict__.get("subnode_packed") or EMPTY_DICT
                self_subnode = self_subnode_packed.get(str(subtype)) or EMPTY_DICT
                other_subnode_packed = other.__dict__.get("subnode_packed") or EMPTY_DICT
                other_subnode = other_subnode_packed.get(str(subtype)) or EMPTY_DICT
                for prop in subtype_cls.__subtype_extra_original_properties__.values():
                    self_value = self_subnode.get(prop.name)
                    other_value = other_subnode.get(prop.name)
                    if not self._prop_value_equals(prop, self_value, other_value):
                        return False
        return True

    @override
    def _do_get(self, key: str):
        """Called if an attribute doesn't exist in __dict__ / the usual places."""

        if self.__has_subtypes__:
            subtype = self.__dict__["type"]
            subtype_cls = self.__subclass_by_subtype__.get(subtype)
            if subtype_cls is not None:
                # short-circuit to subclass property or method if it exists
                subtype_attr = getattr(subtype_cls, key, None)
                if subtype_attr is not None:
                    if type(subtype_attr) is property and subtype_attr.fget is not None:
                        # it's a (Python) property
                        return subtype_attr.fget(self)
                    elif type(subtype_attr) is not Property and callable(subtype_attr):
                        # it's a method
                        return functools.partial(subtype_attr, self)

                # regular subtype property
                prop = subtype_cls.__properties__.get(key)
                if prop is not None:
                    subtype_key = str(subtype)
                    subnode_packed = self.__dict__.get("subnode_packed", EMPTY_DICT)
                    if subnode_packed is not None:
                        subnode_packed = subnode_packed.get(subtype_key)
                        if subnode_packed is not None:
                            subnode_value = subnode_packed.get(prop.key)
                            if subnode_value is not None:
                                return subnode_value
                    if prop.is_list:
                        return ()
                    else:
                        return None
                else:
                    # check subtype passthrough
                    if subtype_cls.__passthrough_get__ is not None and self._session is not None:
                        for passthrough_key in subtype_cls.__passthrough_get__:
                            target = getattr(self, passthrough_key, UNSET)
                            attr = getattr(target, key, UNSET)
                            if attr is not UNSET:
                                return attr

        # check passthrough (if in a session)
        if self.__passthrough_get__ is not None and self._session is not None:
            for passthrough_key in self.__passthrough_get__:
                target = getattr(self, passthrough_key, UNSET)
                attr = getattr(target, key, UNSET)
                if attr is not UNSET:
                    return attr

        # attribute error
        try:
            self_str = repr(self)
        except Exception:
            self_str = self.__class__.__name__
        raise AttributeError(f"{self_str} has no attribute '{key}'")

    @override
    def _do_set(
        self,
        key: str,
        new_value: Any,
        *,
        track: bool = True,
        coerce: bool = True,
        validate: bool = True,
    ):
        """Sets *any* attribute on this builtin object."""
        if self.__has_subtypes__ and key not in self.__properties__:
            # set subtype property
            subtype = self.__dict__["type"]
            subtype_cls = self.__subclass_by_subtype__.get(subtype)
            if subtype_cls is not None:
                subtype_prop = getattr(subtype_cls, key, None)

                # set regular subtype property
                prop = subtype_cls.__properties__.get(key)
                if prop is not None:
                    if validate:
                        # coerce & check type
                        if prop._type_info is not None and prop.reference_source is None:
                            new_value = coerce_value(
                                new_value,
                                prop._type_info,
                                parent=self,
                                parent_key=prop,
                            )
                            check_value(
                                new_value,
                                prop._type_info,
                                options=DEFAULT_CHECK_OPTIONS,
                                invalid=on_invalid_raise,
                            )

                    # set
                    # short-circuit to subclass 'property' if it exists
                    subtype_key = str(subtype)
                    if type(subtype_prop) is property and subtype_prop.fset is not None:
                        assert subtype_prop.fget is not None
                        if prop.is_value_runtime:
                            old_value = getattr(self, prop.value_packed_ptr.name)  # type: ignore
                        else:
                            old_value = subtype_prop.fget(self)
                        subtype_prop.fset(self, new_value)
                    elif self.subnode_packed is None:
                        old_value = None
                        # set directly
                        self.__dict__["subnode_packed"] = {subtype_key: {prop.key: new_value}}
                    elif subtype_key not in self.subnode_packed:
                        old_value = None
                        self.subnode_packed[subtype_key] = {prop.key: new_value}
                    else:
                        old_value = getattr(self, key)
                        self.subnode_packed[subtype_key][prop.key] = new_value

                    # track
                    if track:
                        if prop.is_value_runtime:
                            _trace_edit_operation(
                                self,
                                prop.value_packed_ptr,  # type: ignore
                                new_value=getattr(self, prop.value_packed_ptr.name),  # type: ignore
                                old_value=old_value,
                                subtype=subtype,
                            )
                        else:
                            _trace_edit_operation(
                                self,
                                prop,
                                new_value=new_value,
                                old_value=old_value,
                                subtype=subtype,
                            )

                    return  # success

        super()._do_set(key, new_value, track=track, coerce=coerce, validate=validate)

    if not TYPE_CHECKING:  # (see above in BuiltinObject)
        __getattr__ = _do_get
        __setattr__ = _do_set

    @property
    def connection(self):
        """The currently active connection (errors if none)"""
        assert self._connection is not None, f"no connection for {self!r}"
        return self._connection  # type: ignore (class definition "depends on itself" for some reason)

    @property
    def _is_live(self) -> bool:
        """Whether this node is live."""
        return (
            self._connection is not None
            and self._connection.is_live
            and not self._connection._is_closed
        )

    @property
    def _data_graph(self) -> "NodeDataGraph":
        assert self._connection is not None, f"no connection for {self!r}"
        assert self._connection.result_data is not None, f"no data graph for {self!r}"
        return self._connection.result_data.graph

    def _unload_rec(self):
        """Unloads this node from the graph (and supergraph). Only meant for testing."""
        assert IS_TEST, f"cannot unload {self!r} (not in test mode)"
        if len(self._graph) == 1:
            # remove entire graph from supergraph if it was just this node (and its descendants)
            self._supergraph.remove_graph(self._graph)
        else:
            self._graph.remove(self)

    @final
    def _track_self(self, session: "Session"):
        """Track this object in the given session."""
        if self._session is not None and self._session is not session:
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
            yield from self._graph.get_descendants(self, recursive=True)

    @final
    def _track_rec(self, session: "Session"):
        for inner_node in self._walk_descendants():
            inner_node._track_self(session)

    @final
    def _untrack_rec(self):
        for inner_node in self._walk_descendants():
            inner_node._untrack_self()

    @property
    def tk(self) -> str:
        """The template key of this node lineage."""
        return get_tk_from_ck(self.ck)

    @property
    def _ident(self) -> Optional[str]:
        """The Bench identifier of this node (slug if exists, else name if exists)."""
        if self.metatype == NodeType.PACKAGE and self.parent is not None:
            return self.parent._ident  # package shares its Bench's identifier
        if "slug" in self.__properties__:
            slug = getattr(self, "slug")
            if slug:  # prefer slug as ident
                return slug
        if "name" in self.__properties__:
            return getattr(self, "name")
        return None

    @property
    def _path_key(self) -> str:
        """The Bench *path* identifier of this node (prefers bench_ident, ck/id filter otherwise)"""
        bench_ident = self._ident
        if bench_ident is not None:
            if self.metatype == NodeType.BENCH:
                return f"@{bench_ident}"
            else:
                return bench_ident
        elif "ck" in self.__properties__:
            return f"{self.metatype.bench_name}[ck={self.ck}]"
        else:
            return f"{self.metatype.bench_name}[id={self.id}]"

    @property
    def _ident_key(self) -> str:
        """Get the identifier string for this node."""
        ident_str = self.code_name
        if ident_str is None:
            ident_str = str(self.id)
        if self.__parent_property__ is not None:
            return self.absolute_path
        return ident_str

    @property
    def code_name(self) -> Optional[str]:
        """The python identifier-compatible name of this node."""
        if "slug" in self.__properties__:
            slug = getattr(self, "slug")
            if slug:  # prefer slug as ident
                return slug
        if "name" in self.__properties__:
            name = getattr(self, "name")
            if name:
                return to_code_name(name)
        return None

    @property
    def absolute_path(self) -> str:
        if not self.__parent_types__:
            # this is a root node
            ident = self._ident
            assert ident is not None, f"no bench ident for {self!r}"
            return ident
        else:
            # assemble path (like in Path.render)
            path_parts: list[str] = []
            current = self
            while current is not None:
                if current.metatype == NodeType.PACKAGE:
                    bench = cast("Bench | Package", current).bench
                    if bench is not None:
                        if bench.main_package_id == current.id:
                            path_parts.append(bench._path_key)
                        else:
                            path_parts.append(f"{bench._path_key}:{current._path_key}")
                        break

                path_parts.append(current._path_key)
                next_parent = current.parent
                if next_parent is None and current.metatype in self.__roots__:
                    break  # reached the root
                current = next_parent
            else:
                path_parts.append("<detached>")
            return "/".join(reversed(path_parts))

    def to_ref(self) -> "NodeReference":
        """Gets a reference to this node. May be rich in subclasses."""
        return NodeReference._ref_from_node(self)

    def _to_ref_data(self) -> "NodeReferenceData":
        """Gets a data reference to this node. May be rich in subclasses."""
        return NodeReference._ref_from_node(self)._to_data()

    #
    # Lifecycle
    #

    @property
    def is_extant(self):
        return self.deleted_at is None

    @property
    def is_deleted(self) -> bool:
        parent = self.parent
        return self.deleted_at is not None or (parent is not None and parent.is_deleted)

    def delete(self):
        """Delete this Node."""
        assert not self.is_deleted, f"{self!r} is already deleted"
        self.active_session._delete(self)

    def restore(self):
        """Restore this deleted Node from the trash."""
        assert self.is_deleted, f"{self!r} is not deleted"
        self.active_session._restore(self)

    def erase(self):
        """Wipe this Node from this universe forever."""
        self.active_session._erase(self)

    def move(self, to: "Node"):
        """Move this Node to a new parent."""
        to.append(self, move=True)

    def append(self, child: "Node", move: bool = False):
        """Append a Node as a child of this Node."""
        child_prop = self.get_child_property(child.metatype)
        if child_prop is not None:
            child_list = getattr(self, child_prop.name)
            child_list.append(child, move=move)
        elif isinstance(child, HasNodeBase):
            base = child.base
            assert base is not None, f"no base for {child!r}"
            child_prop = base.get_child_property(child.metatype)
            if child_prop is not None:
                child_list = getattr(base, child_prop.name)
                child_list.append(child, move=move)
            else:
                raise ValueError(f"no child property for {child.metatype.bench_name} in {base!r}")
        elif self.metatype in child.__parent_types__:
            attach_node(child, self, move=move)
        else:
            raise ValueError(f"cannot append {child!r} to {self!r}")

    def _move_to_graph(self, graph: NodeGraph, force: bool = False):
        """Moves this Node and its descendants to a new graph."""
        moved = self._graph.get_descendants(self, recursive=True)  # type: ignore
        moved = (self, *moved)
        for n in moved:
            if force and graph.get(n.id) is not None:
                graph.update(n)
            else:
                graph.add(n)
            n._graph = graph
        return moved

    async def wait_until(self, condition: Callable[[Self], bool]):
        """Wait until the given condition is true."""
        runtime = active_session().runtime
        await runtime._wait_for(nodes=[self], condition=lambda: condition(self))

    @classmethod
    def get_child_property_or_error(cls, node_type: NodeType) -> Property:
        """Gets the child property for the given Node type."""
        prop = cls.get_child_property(node_type)
        if prop is None:
            raise ValueError(f"no child property for {node_type.bench_name} in {cls.__name__}")
        return prop

    @classmethod
    def get_child_property(cls, node_type: NodeType) -> Property | None:
        """Gets the child property for the given Node type, or None if not found."""
        for prop in cls.__node_child_properties__.values():
            if (
                prop.reference_nodes
                and prop.reference_nodes != "any"
                and node_type in prop.reference_nodes
            ):
                return prop
        return None

    @classmethod
    def partial(
        cls, *, type: int | None = None, block: "Block | None" = None, **kwargs: Any
    ) -> "CustomObject":
        """Creates a new partial Node of this type."""
        from bench.language.source.field import Type, TypeConstraint

        from .value import coerce_custom_object_scalar

        if cls is not Node:
            kwargs["metatype"] = cls.metatype
        if type is not None:
            kwargs["type"] = type
            constraint = TypeConstraint(node_subtypes=[type])
        else:
            constraint = None
        if block is not None:
            kwargs["block"] = block
            typ = Type(
                kind=TypeKind.PARTIAL_OBJECT,
                base_type=block,
                base_field_types=[FieldType.MEMBER],
                bench_type=cls.metatype,
                constraint=constraint,
            )
        else:
            typ = Type(kind=TypeKind.PARTIAL_OBJECT, bench_type=cls.metatype, constraint=constraint)
        return coerce_custom_object_scalar(kwargs, typ)

    @classmethod
    def from_partial(cls, partial: "CustomObject", **kwargs) -> Self:
        """Creates a new full Node from a partial Node."""
        from .value import make_node_from_partial

        node = make_node_from_partial(partial, **kwargs)
        assert isinstance(node, cls), f"unexpected node {node!r} from partial {partial!r}"
        return node

    #
    # Querying
    #

    @classmethod
    def _query(cls) -> "Query[Self, NodeDataT]":
        from bench.language import Query

        return Query(type=QueryType.SEARCH, node_type=cls.metatype)

    @classmethod
    def where(cls, filter: Optional["Expression"] = None, **kwargs) -> "Query[Self, NodeDataT]":
        return cls._query().where(filter, **kwargs)

    @classmethod
    def order_by(
        cls, sort: "Optional[Expression] | str | Field | Property" = None, *args: str
    ) -> "Query[Self, NodeDataT]":
        return cls._query().order_by(sort, *args)

    @classmethod
    def include(cls, *properties: Property) -> "Query[Self, NodeDataT]":
        return cls._query().include(*properties)

    @classmethod
    def select(cls, *keys: FieldOrProperty) -> "Query[Self, NodeDataT]":
        return cls._query().select(*keys)

    @classmethod
    def select_all(cls) -> "Query[Self, NodeDataT]":
        return cls._query().select_all()

    @classmethod
    def deselect(cls, *properties: FieldOrProperty) -> "Query[Self, NodeDataT]":
        return cls._query().deselect(*properties)

    @classmethod
    def include_ancestors(cls, *node_types: NodeTypeOrClass) -> "Query[Self, NodeDataT]":
        return cls._query().include_ancestors(*node_types)

    @classmethod
    def include_descendants(cls, *node_types: NodeTypeOrClass) -> "Query[Self, NodeDataT]":
        return cls._query().include_descendants(*node_types)

    @overload
    @classmethod
    async def get(
        cls,
        filter: Optional["Expression | NodeReference | None"] = None,
        live: bool = False,
        **kwargs,
    ) -> Self: ...
    @overload
    @classmethod
    async def get(
        cls, filter: Sequence["NodeReference"], live: bool = False, **kwargs
    ) -> list[Self]: ...
    @classmethod
    async def get(
        cls,
        filter: Optional["Expression | NodeReference | Sequence[NodeReference] | None"] = None,
        live: bool = False,
        **kwargs,
    ) -> Self | list[Self]:
        return await cls._query().get(filter, live=live, **kwargs)

    @classmethod
    async def search(cls, filter: Optional["Expression"] = None, **kwargs) -> list[Self]:
        return await cls._query().search(filter, **kwargs)

    @classmethod
    def first(cls, count: int) -> "Query[Self, NodeDataT]":
        return cls._query().first(count)

    @classmethod
    async def count(cls, filter: Optional["Expression"] = None, **kwargs) -> int:
        return await cls._query().count(filter, **kwargs)

    @classmethod
    async def exists(cls, filter: Optional["Expression"] = None, **kwargs) -> bool:
        return await cls._query().exists(filter, **kwargs)


class NodeSubtypeStub[NodeT: Node]:
    """
    The stub for the virtual subclass of a Node for a specific subtype.
    Basically, we use this so we can have instantiate & instance check with subnodes,
     like with DatabaseBlock or TreeView (even when the actual subtype-class doesn't exist).
    """

    __slots__ = ("_name", "_node_cls", "_node_subtype", "_node_type")

    def __init__(self, cls: type[NodeT], type: NodeType, subtype: IdEnum):
        self._node_cls = cls
        self._node_type = type
        self._node_subtype = subtype
        self._name = f"{to_casing(subtype.name, Casing.CAMEL)}{cls.__name__}"

    def __call__(self, **kwargs: Any) -> NodeT:
        return self._node_cls(type=self._node_subtype, **kwargs)


@object_()
class HasTracingContext(BuiltinObject):
    """Context for a Node in some Session."""

    # tracing :Tracing
    mode: NodeMode = p_internal(20, default=None, default_sql=str(NodeMode.PRODUCTION.value))
    # ... more tracing context


class TracingContext(NamedTuple):  # :Tracing
    mode: NodeMode


def get_tracing_context() -> TracingContext:
    """Gather the current runtime tracing context."""
    session = _active_session.get(None)
    if session is not None:
        mode = session.active_mode
        return TracingContext(mode=mode)
    else:
        return TracingContext(mode=NodeMode.PRODUCTION)


@node_component()
class BenchNode[NodeDataT: AnyNodeData](Node[NodeDataT], abc.ABC):
    """A Node inside a Bench."""

    bench: "Bench | None" = p_node_ancestor_with_self(
        5, NodeType.BENCH, require=True, store=True, wire=True
    )
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None

    @property
    def is_attached(self) -> bool:
        return self.parent_ptr is not None and self.bench is not None


@node_component()
class PackageNode[NodeDataT: AnyNodeData](BenchNode[NodeDataT], HasTracingContext, abc.ABC):
    """A Node inside a Package."""

    package: "Package | None" = p_node_ancestor_with_self(
        6, NodeType.PACKAGE, require=True, store=True, wire=True, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        package_id: Optional[UUID] = None
        package_ptr: Optional[NodeReference] = None

    @property
    def is_attached(self) -> bool:
        return self.parent_ptr is not None and self.package is not None


@node_component()
class SourceNode[NodeDataT: AnyNodeData](PackageNode[NodeDataT], abc.ABC):
    """A Node in a Package with a persistent identity that can be instanced (with computed values)."""

    ck: UUID = p_system(3, default=None, require=True, autoset=True)  # type: ignore
    template: Optional["Node"] = p_node_template(7)
    if TYPE_CHECKING:
        template_id: Optional[UUID] = None
        template_ptr: Optional[NodeReference] = None
    template_at: datetime | None = p_system(16, default=None, autoset=True)
    computed_values: list["ComputedValue"] = p_internal(
        28, require=False, array=True, struct=StructType.COMPUTED_VALUE
    )

    def set_computed(
        self,
        target: "PathIn",
        source: "ComputedSourceIn",
        *,
        mode: "ComputedValueMode | None" = None,
        is_active: bool = True,
    ):
        """Sets and overrides the computed value for the target path."""
        from .expression import ComputedValue, ComputedValueMode

        computed_value = ComputedValue.new(
            target=target, source=source, mode=mode or ComputedValueMode.ALWAYS, is_active=is_active
        )
        self.computed_values = [
            *(cv for cv in self.computed_values if cv.target_path != target),
            computed_value,
        ]

    def clear_computed(self, target: "PathIn"):
        """Clears the computed value for the target path."""
        from .path import to_path

        target = to_path(target)
        self.computed_values = [
            *(cv for cv in self.computed_values if cv.target_path != target),
        ]


@node_component()
class StateNode[NodeDataT: AnyNodeData](BenchNode[NodeDataT], abc.ABC):
    """A Node in a Bench with a persistent cross-Package identity."""

    parent: Optional["Bench"] = p_node_parent(4, NodeType.BENCH)


@node_component()
class HasTimeIdentity(BuiltinObject, abc.ABC):
    """A Node with a time-based identity."""

    __id_factory__: ClassVar[Callable[[], UUID]] = UUIDT
    __ck_factory__: ClassVar[Callable[[], UUID]] = UUIDT


@object_()
class HasContext(BuiltinObject):
    """Context for a Node sonewhere."""

    # NOTE :Security: session context properties are p_internal (not p_system) so we can update
    #   them in all Clients. But this also means Users could mess with them if they really want to.
    session: Optional["Session"] = p_internal(
        80, require=False, array=False, references=NodeType.SESSION, same_bench=True
    )
    run: Optional["Run"] = p_internal(
        81, require=False, array=False, references=NodeType.RUN, same_bench=True
    )
    run_root: Optional["Run"] = p_internal(
        82, require=False, array=False, references=NodeType.RUN, same_bench=True
    )
    client: Optional["Client"] = p_internal(
        83, require=False, array=False, references=NodeType.CLIENT, same_bench=True
    )
    machine: Optional["Machine"] = p_internal(
        84, require=False, array=False, references=NodeType.MACHINE, same_bench=True
    )
    user: Optional["User"] = p_internal(86, require=False, array=False, references=NodeType.USER)
    identity: Optional["Block"] = p_internal(
        87,
        require=False,
        array=False,
        references=NodeType.BLOCK,
        constraint=constraint(node_subtypes=[BlockType.IDENTITY]),
    )
    if TYPE_CHECKING:
        session_ptr: Optional[NodeReference] = None
        session_id: Optional[UUID] = None
        run_ptr: Optional[NodeReference] = None
        run_id: Optional[UUID] = None
        run_root_ptr: Optional[NodeReference] = None
        run_root_id: Optional[UUID] = None
        client_ptr: Optional[NodeReference] = None
        client_id: Optional[UUID] = None
        machine_ptr: Optional[NodeReference] = None
        machine_id: Optional[UUID] = None
        user_ptr: Optional[NodeReference] = None
        user_id: Optional[UUID] = None
        identity_ptr: Optional[NodeReference] = None
        identity_id: Optional[UUID] = None

    @property
    def runtime(self):
        """The Runtime associated with this context (if any)."""
        return self.session._runtime if self.session is not None else None

    @property
    def thread(self):
        """The RuntimeThread associated with this context (if any)."""
        return self.runtime.thread if self.runtime is not None else None


@node_component()
class RuntimeNode[NodeDataT: AnyNodeData](
    HasTimeIdentity, BenchNode[NodeDataT], HasContext, HasTracingContext, abc.ABC
):
    """A Node that exists only (conceptually) at/in a Runtime."""


RunnableNode = Union["Block", "Action", "Pipe"]


@node_component()
class HasNodeBase(BuiltinObject, abc.ABC):
    """A Node which may have a 'base' in another Node (e.g., its type definition)."""

    @property
    @abc.abstractmethod
    def base(self) -> Optional[BenchNode]: ...

    @property
    def base_ck(self) -> Optional[UUID]:
        return self.base.ck if self.base is not None else None

    @staticmethod
    @abc.abstractmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]: ...


def is_node[T: Node](obj: Any, node_cls: type[T]) -> TypeGuard[T]:
    return isinstance(obj, Node) and obj.metatype == node_cls.metatype


def is_struct[T: Struct | Struct](obj: Any, struct_cls: type[T]) -> TypeGuard[T]:
    return isinstance(obj, (Struct, Struct)) and obj.metatype == struct_cls.metatype


#
# Utility types
#


@struct_(StructType.GRAPH_SCOPE)
class GraphScope(Struct[GraphScopeData]):
    """The scope for an operation on the Bench graph."""

    bench_id: Optional[UUID] = p_internal(30, default=None)
    package_id: Optional[UUID] = p_internal(31, default=None)

    def __content_str__(self) -> str:
        return repr_scope(self)


def repr_scope(scope: GraphScope | GraphScopeData) -> str:
    if scope.package_id:
        return f"[bench_id={scope.bench_id}, package_id={scope.package_id}]"
    elif scope.bench_id:
        return f"[bench_id={scope.bench_id}]"
    else:
        return "[*]"


EMPTY_SCOPE_DATA = GraphScopeData(metatype=lang_pb2.OBJECT_TYPE_GRAPH_SCOPE)


@struct_(StructType.CLIENT_ORIGIN)
class ClientOrigin(Struct[ClientOriginData]):
    """Information to identify a Client."""

    type: ClientType = p_internal(30, require=True)
    id: Optional[UUID] = p_internal(31, default=None)
    nonce: Optional[UUID] = p_internal(32, default=None)


@struct_(StructType.NODE_REFERENCE)
class NodeReference(Struct[NodeReferenceData]):
    """
    A plain reference to a Node.
    We include the Bench and 'ck' where available.
    Base = the Node is 'based' on (as in HasNodeBase).
    """

    node_type: NodeType = p_internal(30, require=True)
    id: Optional[UUID] = p_internal(31, default=None)
    ck: Optional[UUID] = p_internal(32, default=None)
    bench_id: Optional[UUID] = p_internal(33, default=None)
    base_ck: Optional[UUID] = p_internal(34, default=None)
    # base could be in a different Bench (e.g. a Signal in Bench A with a type from Bench B)
    base_bench_id: Optional[UUID] = p_internal(35, default=None)

    @staticmethod
    def _clone_ref[T: NodeReference | Any](
        ref_cls: type[T], ref: "NodeReference | Any", **kwargs
    ) -> T:
        return ref_cls(
            id=ref.id,
            ck=ref.ck,
            node_type=ref.node_type,
            bench_id=ref.bench_id,
            base_ck=ref.base_ck,
            base_bench_id=ref.base_bench_id,
            **kwargs,
        )

    def to_ref(self) -> "Self":
        """Gets the reference (noop for compatibility with Node.to_ref)."""
        return self

    def __content_str__(self):
        content_parts = []
        if self.id is not None:
            content_parts.append(f"id={self.id}")
        if self.ck is not None:
            if self.ck == self.id:
                content_parts.append("ck=id")
            else:
                content_parts.append(f"ck={self.ck}")
        if self.bench_id is not None:
            content_parts.append(f"bench_id={self.bench_id}")
        if self.base_ck is not None:
            content_parts.append(f"base_ck={self.base_ck}")
        if self.base_bench_id is not None:
            if self.base_bench_id == self.bench_id:
                content_parts.append("base_bench_id=bench_id")
            else:
                content_parts.append(f"base_bench_id={self.base_bench_id}")
        selector_str = ", ".join(content_parts)
        return f"{self.node_type.bench_name}:[{selector_str}]"

    @staticmethod
    def _ref_from_node(node: Node) -> "NodeReference":
        assert isinstance(node, Node), f"expected Node, got {node!r}"

        # bench
        bench_id: UUID | None = None
        if node.metatype == NodeType.BENCH:
            bench_id = node.id
        elif isinstance(node, BenchNode):
            bench_id = node.bench_id
            if bench_id is None:
                # maybe just creating, try from context
                session = _active_session.get()
                if session is not None and session.bench_id is not None:
                    bench_id = session.bench_id

        # base
        base_ck: UUID | None = None
        base_bench_id: UUID | None = None
        if node.metatype in BASED_NODE_TYPES:
            base = cast(HasNodeBase, node).base
            if base is not None:
                base_ck = base.ck
                base_bench_id = base.bench_id
                if base_bench_id is None:
                    # maybe also just creating, try from context
                    session = _active_session.get()
                    if session is not None and session.bench_id is not None:
                        base_bench_id = session.bench_id
                base_bench_id = base_bench_id

        reference = NodeReference(
            node_type=node.metatype,
            id=node.id,
            ck=node.ck,
            bench_id=bench_id,
            base_ck=base_ck,
            base_bench_id=base_bench_id,
            _skip_validate_self=True,
        )
        return reference

    @staticmethod
    def _ref_data_from_node_data(node_data: AnyNodeData) -> "NodeReferenceData":
        node_cls = BUILTIN_OBJECT_CLASS_BY_TYPE[cast(ObjectType, node_data.metatype)]
        reference = NodeReferenceData(
            metatype=pb2.OBJECT_TYPE_NODE_REFERENCE,
            node_type=cast(pb2.NodeType, node_data.metatype),
            id=node_data.id,
            ck=getattr(node_data, "ck", node_data.id),
        )

        # bench
        if node_data.metatype == NodeType.BENCH:
            if node_data.id:
                reference.bench_id = node_data.id
        elif "bench" in node_cls.__properties__ and node_data.parent_ptr.bench_id:
            reference.bench_id = node_data.parent_ptr.bench_id
        # base
        if NodeType(node_data.metatype) in BASED_NODE_TYPES:
            base = cast(HasNodeBase, node_cls).get_base_from_data(node_data)
            if base is not None:
                if base.ck:
                    reference.base_ck = base.ck
                if base.bench_id:
                    reference.base_bench_id = base.bench_id

        return reference


@struct_(StructType.PROPERTY_REFERENCE)
class PropertyReference(Struct):
    """
    A reference to a builtin object's Property.
    If type is unset, this refers to a base property in one of the base BuiltinObject types.
    """

    object_type: ObjectType | None = p_regular(30)
    id: int = p_regular(31)
    node_subtype: Optional[int] = p_internal(32, default=None)
    references_node_type: Optional[NodeType] = p_internal(35)  # disambiguate reference properties
    references_meta: Optional[PropertyReferenceType] = p_internal(36, default=None)

    def __content_str__(self):
        object_cls = (
            Node if self.object_type is None else BUILTIN_OBJECT_CLASS_BY_TYPE.get(self.object_type)
        )
        if object_cls is None:
            if self.object_type is None:
                return f"Node.??? [id={self.id}]"
            else:
                return f"{self.object_type.name}.??? [id={self.id}]"
        else:
            prop = object_cls.__properties_by_id__.get(self.id)
            if prop is None:
                return f"{object_cls.__name__}.??? [id={self.id}]"
            else:
                return f"{object_cls.__name__}.{prop.name} [id={self.id}]"

    @property
    def object_cls(self) -> type[BuiltinObject] | None:
        if self.object_type is None:
            return Node
        else:
            return BUILTIN_OBJECT_CLASS_BY_TYPE.get(self.object_type)

    def resolve_or_error(self) -> Property:
        """Resolves the property reference to a Property."""
        resolved = self.resolve()
        if resolved is None:
            raise ValueError(f"could not resolve {self!r}")
        return resolved

    def resolve(self) -> Property | None:
        """Resolves the property reference to a Property."""
        object_cls = self.object_cls
        prop = (object_cls or Node).__properties_by_id__.get(self.id)
        if (
            prop is None
            and object_cls is not None
            and issubclass(object_cls, Node)
            and self.node_subtype
        ):
            subtype_cls = object_cls.__subclass_by_subtype__.get(cast(Any, self.node_subtype))
            if subtype_cls is not None:
                prop = subtype_cls.__properties_by_id__.get(self.id)
        if prop is not None:
            if self.references_meta is not None:
                assert prop.reference_stored_metas is not None, f"{prop!r} has no stored metas"
                meta_prop = prop.reference_stored_metas.get(self.references_meta)  # type: ignore
                if meta_prop is not None:
                    return meta_prop
            if self.references_node_type is not None:
                assert prop.reference_stored_ids_by_type is not None, f"{prop!r} has no stored ids"
                id_prop = prop.reference_stored_ids_by_type.get(self.references_node_type)
                if id_prop is not None:
                    return id_prop
        return prop


@node_(NodeType.SKIP, stored=False)
class Skip(Node):
    """A reference to another node in some graph that wasn't available for some reason (usually permissions)."""

    parent: Node = p_node_parent(4, *NODE_TYPES.tuple)
    reference: Optional[Node] = p_regular(30, array=False, references="any", require=True)
    order_key: str | None = p_internal(31, default=None)


@node_(NodeType.EMPTY, stored=False)
class Empty(Node):
    """An empty node."""

    parent: Node = p_node_parent(4, *NODE_TYPES.tuple)


#
# Utilities
#


# NOTE: import from .value later to avoid circular import
#  (but import at top level to avoid import in critical path)


from .value import (  # noqa: E402
    DEFAULT_CHECK_OPTIONS,
    check_value,
    coerce_value,
    pack_proto_json,
    pack_value_data,
)


def extract_name_id(name: str) -> Optional[int]:
    """Extracts the last (potentially multi-digit) characters as an integer."""
    for i in range(len(name), 0, -1):
        if not name[i - 1].isdigit():
            return None if i == len(name) else int(name[i:])
    return int(name)


def generate_node_name(
    metatype: NodeType, type: Optional[Any], siblings: Collection["Node"]
) -> str:
    """Generates a new name for the given node based on its siblings. :AutoNaming"""
    if metatype == NodeType.BLOCK or metatype == NodeType.VIEW or metatype == NodeType.ACTION:
        assert isinstance(type, IdEnum), f"expected type for {metatype!r}, got {type!r}"
        base_name = to_casing(type.name, Casing.CAMEL)
        type_siblings = tuple(n for n in siblings if getattr(n, "type") == type)
    else:
        base_name = to_casing(metatype.name, Casing.CAMEL)
        type_siblings = tuple(n for n in siblings if n.metatype == metatype)

    if len(type_siblings) == 0:
        max_id = 0
    else:
        max_id = max((extract_name_id(getattr(n, "name")) or 0) for n in type_siblings)
    return f"{base_name}{max_id + 1}"


def patch_graph(*, old_graph: NodeGraph, new_graph: NodeGraph) -> None:
    """Patches the old graph *in place* from the new graph."""
    for existing_node in tuple(old_graph.nodes):
        if existing_node.id not in new_graph:
            # node removed: leave as is, remove from existing graph
            old_graph.remove(existing_node)
            continue
        else:
            # node updated: patch in place
            patch_node = new_graph[existing_node.id]
            for prop in existing_node.__wired_properties__.values():
                if prop.is_computed:
                    continue  # ignore computed properties
                prop_value = getattr(existing_node, prop.name)
                existing_node._do_set(prop.name, prop_value, track=False, validate=False)
    for patch_node in tuple(new_graph.nodes):
        if patch_node.id not in old_graph:
            # node added: add to existing graph
            old_graph.add(patch_node)


def sync_node(*, parent: Node, old_root: SourceNode | None, new_root: SourceNode) -> None:
    """Patches the old node *in place* from the new node (recursively)."""

    def _copy(node: SourceNode, *, detach: bool) -> SourceNode:
        """Copy the node (exact non-recursive clone with preserved identity)."""
        return node.clone(recursive=False, reset=False, detach=detach)

    def _sync(old: SourceNode, new: SourceNode) -> None:
        """Sync the old node *in place* from the new node."""
        for prop in old.__wired_properties__.values():
            if prop.id < 30 or prop.is_computed or prop.name == "order_key":
                continue  # ignore internal properties
            old_value = getattr(old, prop.name)
            new_value = getattr(new, prop.name)
            if old_value != new_value:
                old._do_set(prop.name, new_value, track=True, validate=False)

    if old_root is None:
        old_root = _copy(new_root, detach=False)
        parent.append(old_root)

    # create/update new nodes
    _sync(old_root, new_root)
    for new in new_root.iter_descendants(recursive=True):
        old = old_root._graph.get(new.id)
        if old is None:
            new_copy = _copy(new, detach=True)
            if new.parent_ptr is None:
                old_parent = new_copy
            else:
                old_parent = old_root._graph.get(new.parent_ptr.id)
                assert isinstance(old_parent, SourceNode), f"unexpected {old_parent!r} for {new!r}"
            old_parent.append(new_copy)
        else:
            assert isinstance(old, SourceNode), f"unexpected {old!r} for {new!r}"
            _sync(old, new)

    # remove old nodes
    for old in parent.iter_descendants(recursive=True):
        if old.id not in new_root._graph:
            old.delete()

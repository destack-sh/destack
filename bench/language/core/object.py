import abc
import base64
import contextvars
import functools
import inspect
from dataclasses import InitVar
from enum import IntEnum
from sys import intern
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Collection,
    Iterable,
    Literal,
    Mapping,
    Optional,
    Self,
    Sequence,
    TypeGuard,
    Union,
    cast,
    dataclass_transform,
    final,
)
from uuid import UUID

import structlog
from bitarray import bitarray
from more_itertools import first
from opentelemetry import trace

from bench.language.registry import BUILTIN_OBJECT_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE
from bench.pb2 import AnyObjectData, AnyStructData, GraphScopeData, lang_pb2
from bench.pb2.lang_pb2 import EditOperationData
from bench.utils.func import dualmethod, stable_hash
from bench.utils.utils import frozendict

from .const import (
    ACTIVE_SESSION,
    EMPTY_DICT,
    IS_IN_USER_CODE,
    TK_LENGTH_BYTES,
    UNSET,
    EditOperationType,
    FieldType,
    NodeType,
    ObjectType,
    ReferenceKind,
    StructType,
)
from .graph import NULL_SUPERGRAPH, NodeSuperGraph
from .list import RemoteNodeList
from .property import (
    _PROPERTY_SPECIFIERS,
    METATYPE_PROPERTY,
    Property,
    PropertyReferenceType,
    p_internal,
    p_regular,
    p_runtime,
    p_struct_parent,
)
from .validation import ValidationHandler, on_invalid_raise

if TYPE_CHECKING:
    from bench.language import (
        Action,
        CustomObject,
        Field,
        Flow,
        Machine,
        Node,
        NodeReference,
        Organization,
        PropertyReference,
        Run,
        Session,
        User,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type["Node"]]
EditSubject = Union["User", "Machine", "Flow", "Action", "Run"]
EDIT_SUBJECT_TYPES = (
    NodeType.USER,
    NodeType.MACHINE,
    NodeType.FLOW,
    NodeType.ACTION,
    NodeType.RUN,
)

Owner = Union["User", "Organization", "Flow", "Action", "Run", "Machine"]
OWNER_TYPES = (
    NodeType.USER,
    NodeType.ORGANIZATION,
    NodeType.FLOW,
    NodeType.ACTION,
    NodeType.RUN,
    NodeType.MACHINE,
)


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
    is_struct: bool = False,
    is_node: bool = False,
    # for nodes only
    is_root: bool = False,
    is_variable_root: bool = False,
) -> tuple[type[ObjectT], dict[str, "Property"]]:
    """Process an object base class and return the processed class and its properties."""
    assert isinstance(cls, type), f"expected type, got {cls} ({type(cls)})"

    # NOTE :Performance :Architecture: we can't really use slots for our builtin objects
    #  (as of Python 3.13, __slots__ always uses class-level descriptors, but we also
    #   want to use class level attributes for our own properties, like Block.type, ...
    #   - neglecting this conflict causes fun errors like 'X is a read-only attribute')

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
    cls.__raw_properties__ = frozendict(properties_by_name)  # remember 'own' properties

    # collect properties from all parent components
    for component in reversed(static_components):
        for name, prop in component.__raw_properties__.items():
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
                if name in declared_properties:
                    declared_properties[name] = prop
            elif not prop.equals_type(existing):
                raise ValueError(f"property conflict '{name}': {prop!r}, {existing!r}")
            if not is_node and prop.is_tree_reference:
                raise ValueError(f"non-node {cls} has node-only relation {prop}")
    cls.__declared_properties__ = frozendict(declared_properties)

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
    is_struct: bool = False,
    is_node: bool = False,
):
    """
    Mark a class as an object component (or concrete struct for a StructType).
    """

    def decorate(cls_in: type[_ObjectT]) -> type[_ObjectT]:
        cls, _properties = _process_object_cls(
            cls=cast(Any, cls_in),
            object_type=struct_type,
            is_final=is_final,
            is_struct=is_struct,
            is_node=is_node,
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
        return cast(type[_ObjectT], cls)

    return decorate


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
                self._do_set(wired_prop.name, None, track=False)
            else:
                self._do_set(wired_prop.name, value.to_ref(), track=False)

        return property(_get_property_scalar, _set_property_scalar)

    else:

        def _get_properties_many(self: BuiltinObject) -> tuple[Property, ...]:
            value_ptrs: list[PropertyReference] = getattr(self, wired_prop.name)
            assert type(value_ptrs) is list, f"invalid {prop!r}: {value_ptrs!r}"
            return tuple(p.resolve_or_error() for p in value_ptrs)

        def _set_properties_many(self: BuiltinObject, values: Collection[Property]):
            self._do_set(wired_prop.name, [p.to_ref() for p in values], track=False)

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
                self._do_set(wired_prop.name, None, track=False)
            else:
                self._do_set(wired_prop.name, value.to_ref(), track=False)

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
            self._do_set(wired_prop.name, value_ptrs, track=False)

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

        def get_ancestor_first_self(self: "Node") -> Optional["Node"]:
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

        def get_ancestor_first_other(self: "Node") -> Optional["Node"]:
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

    def set(self: "Node", value: "Node"):
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

    def set(self: "Node", value: "NodeReference"):
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
        if getattr(type(o), "__is_node__", False):
            node = cast("Node", o)
            break
        else:
            o = o.parent
    if node is None or node._is_new or node._session is None:
        return  # ignore, not tracked

    from .node import Node

    # trace path for edit
    path: list[str] = [key.key]
    k = key
    while obj is not None and not getattr(type(obj), "__is_node__", False):
        parent = obj.parent
        if parent is not None:
            k = cast("Struct", obj).parent_key
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
    __raw_properties__: ClassVar[dict[str, Property]] = {}
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
        self_dict["_session"] = kwargs.pop("_session", None) or ACTIVE_SESSION.get()
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
                    value_list = prop.reference_list_type(cast("Node", self), prop)
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
            prop_value = kwargs.get(prop.name)
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
                or prop._type_info is None
            ):
                continue  # ignore identity/tracking
            self_value = getattr(self, prop.name)
            other_value = getattr(other, prop.name)
            if not value_equals(prop._type_info, self_value, other_value, identity_map):
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
            self._do_set(prop.name, prop_value, track=False)

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
        validate: bool = False,
    ):
        """Sets *any* attribute on this builtin object."""
        prop = self.__properties__.get(key)
        if prop is not None:
            # set regular property
            if prop.is_untracked:
                object.__setattr__(self, key, new_value)
                return

            # check (if it's not a contributed property, which are system-only)
            if (typ := prop._type_info) is not None and prop.reference_source is None:
                # move
                if prop.reference_kind == ReferenceKind.STRUCT_CHILD:
                    if not prop.is_list:
                        if new_value is not None:
                            new_value = new_value._move_to(self, prop)
                    else:
                        new_value = [v._move_to(self, prop) for v in new_value]

                # coerce value
                if IS_IN_USER_CODE.get():
                    new_value = coerce_value(new_value, typ, supergraph=self._supergraph)
                    check_value(
                        new_value, typ, options=DEFAULT_CHECK_OPTIONS, invalid=on_invalid_raise
                    )
                elif validate:
                    check_value(
                        new_value, typ, options=DEFAULT_CHECK_OPTIONS, invalid=on_invalid_raise
                    )

            # set
            if track:
                if prop.is_value_runtime:
                    old_value = getattr(self, prop.value_packed_ptr.name)  # type: ignore
                else:
                    old_value = getattr(self, key)
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
            raise ValueError(f"no property '{key}' in {cls.__name__}")
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
        if self.object_type is None:
            from .node import Node

            object_cls = Node
        else:
            object_cls = BUILTIN_OBJECT_CLASS_BY_TYPE.get(self.object_type)
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
            from .node import Node

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
        from .node import Node

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
    value_equals,
)

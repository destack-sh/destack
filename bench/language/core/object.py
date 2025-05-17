import abc
import base64
import contextvars
import functools
import inspect
from enum import IntEnum
from sys import intern
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Collection,
    Iterable,
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
from opentelemetry import trace

from bench.language.registry import (
    BUILTIN_OBJECT_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)
from bench.pb2 import AnyObjectData, AnyStructData, GraphScopeData, lang_pb2
from bench.utils.func import dualmethod, hash_stable, is_close
from bench.utils.utils import frozendict

from .const import (
    EMPTY_DICT,
    FLOAT_EPSILON,
    UNSET,
    NodeReferenceKind,
    NodeType,
    ObjectType,
    StructType,
    TypeKind,
)
from .graph import NodeSuperGraph
from .list import RemoteNodeList
from .property import (
    _PROPERTY_SPECIFIERS,
    METATYPE_PROPERTY,
    Property,
    p_internal,
    p_regular,
    p_runtime,
)

if TYPE_CHECKING:
    from bench.language import Field, Node, NodeReference, PropertyReference, Session, TypeBase

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type["Node"]]


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


def get_tk_b64_from_ck(ck: UUID) -> str:
    """Gets the stable across templates first 6 bytes of the ck."""
    return base64.b64encode(ck.bytes).decode()


_BASE_OBJECT_NAMES = ("BuiltinObject", "Struct", "Struct", "Node")


def _process_object_cls[ObjectT: BuiltinObject](
    cls: type[ObjectT],
    object_type: ObjectType | None,
    is_final: bool = False,
    is_struct: bool = False,
    is_node: bool = False,
) -> tuple[type[ObjectT], dict[str, "Property"]]:
    """Process an object base class and return the processed class and its properties."""
    assert isinstance(cls, type), f"expected type, got {cls} ({type(cls)})"

    # nocheckin: :Performance! :Architecture: use slots or something for our builtin objects
    #  (unfortunately, as of Python 3.13, __slots__ always uses class-level descriptors, but we also
    #   want to use class level attributes for our own properties, like Block.type, ...
    #   - neglecting this conflict causes fun errors like 'X is a read-only attribute'
    #  .. unless we just use Block.get_property('type') instead of Block.type everywhere?
    #  also we could maybe just use NamedTuple for simpler Structs like NodeReference?)

    metatype = METATYPE_PROPERTY.clone()
    metatype.component = cls

    # collect static components from class hierarchy
    components: list[type[BuiltinObject]] = [cls]
    for base in cls.__bases__:
        if base.__name__ == "ABC":
            continue
        if hasattr(base, "__properties__"):
            base: type[Struct]
            components.append(base)
            for grandparent in base.__components__:
                if grandparent not in components:
                    components.append(grandparent)

    # collect properties from this class definition
    properties_by_name: dict[str, Property] = {"metatype": metatype}
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
    cls.__declared_properties__ = frozendict(properties_by_name)

    # collect properties from ancestor components
    for component in reversed(components):
        for name, prop in component.__declared_properties__.items():
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
            else:
                raise ValueError(f"property conflict '{name}': {prop!r}, {existing!r}")
            if not is_node and prop.is_tree_reference:
                raise ValueError(f"non-node {cls} has node-only relation {prop}")

    # contribute extra properties
    for prop in tuple(properties_by_name.values()):
        # collect any extra contributed properties
        if prop.node_kind in (
            NodeReferenceKind.NODE_PARENT,
            NodeReferenceKind.NODE_ANCESTOR_OR_SELF,
            NodeReferenceKind.NODE_ANCESTOR,
            NodeReferenceKind.NODE_REGULAR,
            NodeReferenceKind.NODE_TEMPLATE,
        ):
            if prop.node_kind == NodeReferenceKind.NODE_TEMPLATE:
                if not (is_final and is_node):
                    prop.is_stored = False
                    prop.is_proto = False
                    continue
                # template points to nodes of same type
                prop.node_types = (cast(NodeType, object_type),)

            prop.ptr_prop = prop._to_ptr_prop()
            if prop.ptr_prop is not None:
                prop.is_runtime = True
                properties_by_name[prop.ptr_prop.name] = prop.ptr_prop

    # add computed properties to final classes
    if is_final:

        def _set_computed(name: str, prop: property):
            """Sets a computed property that shouldn't conflict with any existing property."""
            existing = properties_by_name.get(name)
            if existing is not None and existing.is_runtime:
                raise ValueError(f"property conflict '{name}': {prop!r}, {existing!r}")
            setattr(cls, name, prop)

        for name, prop in properties_by_name.items():
            if prop.runtime_prop is None:  # not a contributed property
                # computed property.. property
                if prop.is_property_reference:
                    setattr(cls, name, _object_property_ref(prop))
                # computed node property
                elif prop.node_kind in (
                    NodeReferenceKind.NODE_PARENT,
                    NodeReferenceKind.NODE_REGULAR,
                    NodeReferenceKind.NODE_TEMPLATE,
                ):
                    setattr(cls, name, _object_node_ref(prop))
                # computed node ancestor property
                elif prop.node_kind in (
                    NodeReferenceKind.NODE_ANCESTOR,
                    NodeReferenceKind.NODE_ANCESTOR_OR_SELF,
                ):
                    setattr(cls, name, _node_ancestor_ref(prop))
                    setattr(cls, f"{name}_ptr", _node_ancestor_ptr_ref(prop))
                # computed _x node reference properties (e.g., parent_id, type_ck, node_type, ...)
                if prop.is_node_reference:
                    for obj_key, ptr_key in (("id", "id"), ("ck", "ck"), ("type", "node_type")):
                        if obj_key == "type" and (not prop.node_types or len(prop.node_types) <= 1):
                            continue  # no need for *_type if only one possible node type
                        _set_computed(
                            f"{prop.name}_{obj_key}", _object_node_ref_attr(ptr_key, prop)
                        )

    # finalize props
    for prop in properties_by_name.values():
        prop.component = cls
        prop._finalize_meta()
        if is_final:
            prop._finalize_type()

    # register components and index properties
    cls.__components__ = tuple(components)  # type: ignore
    cls.__properties__ = frozendict(properties_by_name)
    properties_by_id: dict[int, Property] = {}
    for prop in properties_by_name.values():
        if prop.id is not None and prop.id is not UNSET and not prop.runtime_prop:
            existing = properties_by_id.get(prop.id, None)
            if existing is None:
                properties_by_id[prop.id] = prop
            elif not prop.runtime_prop:
                # contributed reference properties can share an id
                raise ValueError(f"property id conflict: {prop!r}, {existing!r}")
    props = properties_by_name.values()
    cls.__properties_by_id__ = frozendict(properties_by_id)
    cls.__properties_name_by_id__ = frozendict(
        {p.id: p.name for p in properties_by_id.values() if p.id is not None}
    )
    cls.__node_properties__ = frozendict(
        {p.name: p for p in props if p.is_node_reference and not p.runtime_prop}
    )
    cls.__struct_properties__ = frozendict({p.name: p for p in props if p.is_struct})
    cls.__stored_properties__ = frozendict(
        {p.name: p for p in cls.__properties__.values() if p.is_stored is True}
    )
    cls.__proto_properties__ = frozendict(
        {p.name: p for p in cls.__properties__.values() if p.is_proto is True}
    )

    # assign ords
    cls.__properties_in_order__ = tuple(sorted(properties_by_id.values(), key=lambda p: p.id))
    for i, prop in enumerate(cls.__properties_in_order__):
        prop.ord = i
        if prop.ptr_prop:
            prop.ptr_prop.ord = i
    cls.__properties_id_in_order__ = tuple(p.id for p in cls.__properties_in_order__)
    cls.__max_property_ord__ = len(cls.__properties_in_order__) - 1
    cls.__properties_mask_set__ = bitarray(cls.__max_property_ord__ + 1)
    cls.__properties_mask_set__.setall(True)
    cls.__properties_mask_unset__ = bitarray(cls.__max_property_ord__ + 1)

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

    wired_prop = prop.ptr_prop
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

    wired_prop = prop.ptr_prop
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

    wired_prop = prop.ptr_prop
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

    assert prop.node_types != "any", f"unexpected {prop.node_types!r} for {prop!r}"

    if prop.node_kind == NodeReferenceKind.NODE_ANCESTOR_OR_SELF:

        def get_ancestor_first_self(self: "Node") -> Optional["Node"]:
            parent = self
            while parent is not None:
                if prop.node_types and parent.metatype in cast(
                    tuple[NodeType, ...], prop.node_types
                ):
                    return parent
                parent = parent.parent
            return None

        get = get_ancestor_first_self

    elif prop.node_kind == NodeReferenceKind.NODE_ANCESTOR:

        def get_ancestor_first_other(self: "Node") -> Optional["Node"]:
            parent = self.parent
            farthest = None
            while parent is not None:
                if prop.node_types and parent.metatype in cast(
                    tuple[NodeType, ...], prop.node_types
                ):
                    farthest = parent
                parent = parent.parent
            return farthest

        get = get_ancestor_first_other

    else:
        raise ValueError(f"unexpected ancestor reference kind: {prop.node_kind}")

    def set(self: "Node", value: "Node"):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get, set)


def _node_ancestor_ptr_ref(prop: Property) -> property:
    """The computed get property for Node ancestor pointers (computed because ancestors are computed)."""

    wired_prop = prop.ptr_prop
    assert wired_prop is not None, f"no wired prop for {prop!r}"

    def get_ancestor_ptr(self: "Node") -> Optional["NodeReference"]:
        ancestor = getattr(self, cast(Property, wired_prop.runtime_prop).name)
        if ancestor is None:
            return None
        else:
            return ancestor.to_ref()

    def set(self: "Node", value: "NodeReference"):
        raise NotImplementedError(f"cannot set computed property {wired_prop!r}: {value!r}")

    return property(get_ancestor_ptr, set)


_HANDLING_ATTRIBUTE_ERROR = contextvars.ContextVar("handling_attribute_error", default=False)


@object_()
class BuiltinObject[ObjectDataT: AnyObjectData](abc.ABC):
    """The base for all intrinsic objects like Structs and Nodes and all their derivatives."""

    metatype: ClassVar[ObjectType]
    __components__: ClassVar[tuple[type["BuiltinObject"], ...]] = ()

    __is_struct__: ClassVar[bool] = False
    __is_node__: ClassVar[bool] = False

    __properties__: ClassVar[dict[str, Property]] = {}
    __properties_by_id__: ClassVar[dict[int, Property]] = {}
    __properties_name_by_id__: ClassVar[dict[int, str]] = {}

    __declared_properties__: ClassVar[dict[str, Property]] = {}
    __node_properties__: ClassVar[dict[str, Property]] = {}
    __struct_properties__: ClassVar[dict[str, Property]] = {}
    __stored_properties__: ClassVar[dict[str, Property]] = {}
    __proto_properties__: ClassVar[dict[str, Property]] = {}

    __properties_in_order__: ClassVar[tuple[Property, ...]]
    __properties_id_in_order__: ClassVar[tuple[int, ...]]
    __max_property_ord__: ClassVar[int] = UNSET
    __properties_mask_set__: ClassVar[bitarray] = UNSET
    __properties_mask_unset__: ClassVar[bitarray] = UNSET

    _session: "Session | None" = p_runtime(default=None)
    _supergraph: "NodeSuperGraph" = p_runtime(default=None)

    def __init__(self, **kwargs):
        raise NotImplementedError("nocheckin: generate __init__")

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
        for prop in self.__proto_properties__.values():
            if (
                prop.id < 30
                or prop.name == "order_key"  # implicitly checked in lists
                or prop._type is None
            ):
                continue  # ignore identity/tracking
            self_value = getattr(self, prop.name)
            other_value = getattr(other, prop.name)
            if not value_equals(prop._type, self_value, other_value, identity_map):
                return False
        return True

    def _stable_hash(self) -> int:
        """Hash of content properties."""
        content_props = []
        for prop in self.__proto_properties__.values():
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
        return hash_stable(content_props)

    def _patch_from(self, other: Self):
        """Patches this Node *in place* from another Node."""
        for prop in self.__proto_properties__.values():
            if prop.is_computed:
                continue  # ignore computed properties
            prop_value = getattr(other, prop.name)
            self._do_set(prop.name, prop_value, track=False)

    def _do_get(self, key):
        """Called if an attribute doesn't exist in __dict__ / the usual places."""

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
        if prop is None:
            # attribute error
            try:
                self_str = repr(self)
            except Exception:
                self_str = self.__class__.__name__
            raise AttributeError(f"{self_str} has no attribute '{key}'")

        # set regular property
        if prop.is_untracked:
            object.__setattr__(self, key, new_value)
            return
        raise NotImplementedError("nocheckin: flat edits")

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

    def _clone_kwargs(self, reset: bool = True):
        """Clone kwargs for a new instance."""
        copy_kwargs = {}
        for prop in self.__proto_properties__.values():
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
        return copy_kwargs

    def clone(self, *, reset: bool = True, **kwargs) -> Self:
        """
        Create a clone of this object and its descendants (structs/nodes) with the same content.
        """
        copy_kwargs = self._clone_kwargs(reset=reset)
        copy_kwargs.update(kwargs)
        return self.__class__(**copy_kwargs)

    def replace_references(
        self,
        new_node_by_id: Mapping[UUID, "Node"],
        exclude: Collection[NodeReferenceKind],
    ):
        """Replaces Node references with new Nodes. Missing Nodes are kept as is."""
        for prop in self.__node_properties__.values():
            if prop.node_kind in exclude:
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


@object_()
class Struct[StructDataT: AnyStructData](BuiltinObject[StructDataT], abc.ABC):
    """A Struct is an ordered collection of Properties."""

    metatype: ClassVar[StructType]  # type: ignore

    __is_struct__: ClassVar[bool] = True

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
                elif prop.struct_type:
                    if prop.is_list:
                        prop_value_str = f"{prop.struct_type.bench_name}[{len(prop_value)}]"
                    else:
                        prop_value_str = f"<{prop.struct_type.bench_name} ...>"
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


def is_node[T: Node](obj: Any, node_cls: type[T]) -> TypeGuard[T]:
    return isinstance(obj, Node) and obj.metatype == node_cls.metatype


def is_struct[T: Struct | Struct](obj: Any, struct_cls: type[T]) -> TypeGuard[T]:
    return isinstance(obj, (Struct, Struct)) and obj.metatype == struct_cls.metatype


#
# Utility types
#


@struct_(StructType.SCOPE)
class Scope(Struct[GraphScopeData]):
    """The scope for an operation on the Bench graph."""

    bench_id: Optional[UUID] = p_internal(30)
    package_ids: list[UUID] = p_internal(31)

    def __content_str__(self) -> str:
        return repr_scope(self)


def repr_scope(scope: Scope | GraphScopeData) -> str:
    if scope.package_ids:
        return f"[bench_id={scope.bench_id}, package_ids={', '.join(str(id) for id in scope.package_ids)}]"
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
        return prop


def value_equals(
    typ: "TypeBase",
    self_value: Any,
    other_value: Any,
    identity_map: Mapping[UUID, "NodeReference"] = EMPTY_DICT,
) -> bool:
    """Checks whether two values for a given type are equal (recursively)."""
    if self_value is None or other_value is None:
        return self_value is other_value
    elif typ.kind == TypeKind.PRIMITIVE:
        # compare primitives directly with is_close
        is_float = typ.primitive_type is not None and typ.primitive_type.is_float
        if not typ.is_list:
            return self_value == other_value or (
                is_float and is_close(self_value, other_value, FLOAT_EPSILON)
            )
        else:
            if len(self_value) != len(other_value):
                return False  # unequal list
            for i in range(len(self_value)):
                self_el = self_value[i]
                other_el = other_value[i]
                if self_el != other_el or (
                    is_float and not is_close(self_el, other_el, FLOAT_EPSILON)
                ):
                    return False  # unequal list
            return True
    elif typ.kind == TypeKind.ENUM:
        # compare enums directly
        return self_value == other_value
    elif typ.kind == TypeKind.NODE or typ.bench_type == StructType.NODE_REFERENCE:
        if not typ.is_list:
            self_value = identity_map.get(self_value.ck, self_value)
            other_value = identity_map.get(other_value.ck, other_value)
            return self_value.id == other_value.id
        else:
            if len(self_value) != len(other_value):
                return False  # unequal list
            for i in range(len(self_value)):
                self_element = identity_map.get(self_value[i].ck, self_value[i])
                other_element = identity_map.get(other_value[i].ck, other_value[i])
                if self_element.id != other_element.id:
                    return False  # unequal list
            return True
    elif typ.kind == TypeKind.STRUCT:
        # compare struct recursively
        if not typ.is_list:
            if type(self_value) is not type(other_value) or (
                self_value is not None
                and not cast(Struct, self_value).equals(other_value, identity_map=identity_map)
            ):
                return False  # unequal struct
        else:
            if (
                not isinstance(self_value, Sequence)
                or not isinstance(other_value, Sequence)
                or len(self_value) != len(other_value)
            ):
                return False  # unequal list
            for i in range(len(self_value)):
                if not self_value[i].equals(other_value[i], identity_map=identity_map):
                    return False  # unequal struct

    return True

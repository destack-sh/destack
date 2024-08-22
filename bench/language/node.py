import abc
import base64
import functools
import inspect
from collections import defaultdict
from datetime import datetime
from sys import intern
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    ClassVar,
    Collection,
    Iterable,
    Optional,
    Self,
    Sequence,
    Type,
    TypeGuard,
    TypeVar,
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

from bench.language.const import (
    BASED_NODE_TYPES,
    FLOAT_EPSILON,
    IN_BENCH_NODE_TYPES,
    IN_PACKAGE_NODE_TYPES,
    NODE_TYPES,
    TK_LENGTH_BYTES,
    UNSET,
    ClientType,
    NodeType,
    ObjectType,
    PrimitiveType,
    ReadType,
    ReferenceKind,
    StructType,
    _active_session,
)
from bench.language.graph import NULL_SUPERGRAPH, NodeDataGraph, NodeGraph, NodeSuperGraph
from bench.language.property import (
    _PROPERTY_SPECIFIERS,
    METATYPE_PROPERTY,
    Property,
    p_internal,
    p_node_ancestor_with_self,
    p_node_parent,
    p_node_template,
    p_regular,
    p_runtime,
    p_struct_parent,
    p_system,
)
from bench.language.setup import (
    DESCENDANT_NODE_TYPES,
    HAS_CHILD_NODE_TYPES,
    NODE_CLASS_BY_TYPE,
    NODE_COMPONENT_CLASS_BY_NAME,
    OBJECT_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)
from bench.language.validation import ValidationError, ValidationHandler, on_invalid_raise
from bench.proto.wire import (
    AnyNodeData,
    AnyStructData,
    ClientOriginData,
    FileReferenceData,
    GraphScopeData,
    NodeReferenceData,
    SecretReferenceData,
)
from bench.utils.env import IS_DEV, IS_TEST
from bench.utils.func import bittuple, is_close, stable_hash
from bench.utils.string import to_py_name
from bench.utils.utils import frozendict
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Block,
        Branch,
        Expression,
        Field,
        FileReference,
        GetConnection,
        NodeReference,
        Package,
        PropertyReference,
        QueryBuilder,
        Run,
        SearchConnection,
        SecretReference,
        Server,
        Session,
        Step,
        User,
        ValueObject,
    )

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
                    # NOTE :Performance: maybe Node.parent shouldn't be computed?
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
                    for key in ("id", "ck", "type"):
                        if key == "type" and (
                            not prop.reference_nodes or len(prop.reference_nodes) <= 1
                        ):
                            continue  # no need for *_type if only one possible node type
                        _set_computed(f"{prop.name}_{key}", _object_node_ref_attr(key, prop))

            # computed runtime value
            if prop.is_value_runtime:
                from bench.language.value import _object_value_runtime

                setattr(cls, prop.name, _object_value_runtime(prop))

    # finalize props & update reference to transformed class
    for prop in properties_by_name.values():
        prop.component = cls
        prop._finalize_meta()

    # register components and index properties
    cls.__components__ = tuple(static_components)  # type: ignore
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

    return cls, properties_by_name  # type: ignore


_ObjectT = TypeVar("_ObjectT", bound="BuiltinObject")


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def object_component(
    struct_type: StructType | None = None,
    is_final: bool = False,
    is_inlined: bool = False,
):
    """
    Mark a class as an object component (or concrete struct for a StructType).
    """

    def decorate(cls_in: Type[_ObjectT]) -> Type[_ObjectT]:
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
        return cast(Type[_ObjectT], cls)

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def struct_(struct_type: StructType, inline: bool = False):
    """Register a class as a concrete struct for the given struct type."""

    def decorate(cls: Type[_ObjectT]) -> Type[_ObjectT]:
        cls = object_component(struct_type=struct_type, is_final=True, is_inlined=inline)(cls)
        if IS_DEV and cls.__name__ != "Struct" and cls.__name__ != "Struct":
            if not issubclass(cls, (Struct, Struct)):
                raise ValueError(f"{cls} is not a struct")
            if issubclass(cls, Node):
                raise ValueError(f"{cls} is a node for {struct_type}")

            # check that inline matches inheriting Struct
            is_cls_inlined = not issubclass(cls, Struct)
            if is_cls_inlined != inline:
                raise ValueError(f"inline mismatch for {cls}: ={is_cls_inlined}, inline={inline}")

        return cls

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def node_component(
    node_type: NodeType | None = None,
    passthrough: str | tuple[str, ...] | None = None,
    is_root: bool = False,
    is_variable_root: bool = False,
    is_final: bool = False,
):
    """Mark a class as a node component (or concrete node for a NodeType)."""
    if isinstance(passthrough, str):
        passthrough = (passthrough,)

    def decorate(cls: Type["Node"]) -> Type["Node"]:
        cls, properties = _process_object_cls(
            cls=cls,
            object_type=node_type,
            is_root=is_root,
            is_variable_root=is_variable_root,
            is_final=is_final,
        )
        cls.__passthrough__ = passthrough

        # register node properties
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
def node_(
    node_type: NodeType,
    passthrough: str | tuple[str, ...] | None = None,
    stored: bool = True,
    stored_custom: bool = False,
    local: bool = False,
    roots: tuple[NodeType, ...] = (NodeType.BENCH,),
    indexes: tuple[tuple[str, ...], ...] = (),
    unique: tuple[tuple[str, ...], ...] = (),
):
    """Register a class as a concrete node for the given node type."""

    in_package = node_type in IN_PACKAGE_NODE_TYPES
    in_bench = node_type in IN_BENCH_NODE_TYPES

    def decorate(cls: Type["Node"]) -> Type["Node"]:
        cls = node_component(
            node_type=node_type,
            passthrough=passthrough,
            is_root=len(roots) == 0,
            is_variable_root=len(roots) > 1,
            is_final=True,
        )(cls)
        cls.__is_stored__ = stored
        cls.__is_stored_custom__ = stored_custom
        cls.__is_local__ = local

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


local_node_ = functools.partial(node_, local=True)
if TYPE_CHECKING:
    local_node_ = node_


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def timed_node_(
    node_type: NodeType,
    passthrough: str | None = None,
    indexes: tuple[tuple[str, ...], ...] = (),
):
    """Register a class as a concrete node for the given node type."""
    return local_node_(
        node_type=node_type,
        passthrough=passthrough,
        local=True,
        indexes=(
            *indexes,
            ("created_at",),
            ("created_epoch",),
            ("package_id", "created_at"),
            ("package_id", "created_epoch"),
        ),
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
                return value_ptr.resolve()
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
            return tuple(p.resolve() for p in value_ptrs)

        def _set_properties_many(self: BuiltinObject, values: Collection[Property]):
            self._do_set(wired_prop.name, [p.to_ref() for p in values], track=False, validate=False)

        return property(_get_properties_many, _set_properties_many)


def _object_node_ref(prop: Property) -> property:
    """The computed get/set property for a node reference. Resolved against the active supergraph."""

    wired_prop = prop.reference_wired_ptr
    assert wired_prop is not None, f"no wired prop for {prop!r}"

    if not prop.is_list:

        def _get_node_scalar(self: BuiltinObject) -> Optional["Node | SomeNodeReference"]:
            value_ptr: NodeReference | None = getattr(self, wired_prop.name)
            if value_ptr is None:
                return None
            value = self._supergraph.get(value_ptr)
            if value is not None:
                return value
            elif value_ptr.type in RICH_REFERENCE_TYPES_BY_NODE_TYPE:
                return value_ptr  # :RichReferences
            else:
                return None

        def _set_node_scalar(self: BuiltinObject, value: "Node | None"):
            if value is None:
                self._do_set(wired_prop.name, None, track=False, validate=False)
            else:
                assert self._supergraph.has(
                    value._supergraph
                ), f"{prop}: {value!r} is from {value._supergraph!r} not {self._supergraph!r}"
                self._do_set(wired_prop.name, value.to_ref(), track=False, validate=False)

        return property(_get_node_scalar, _set_node_scalar)

    else:

        def _get_node_many(self: BuiltinObject) -> Sequence["Node | SomeNodeReference"]:
            value_ptrs: Collection[NodeReferenceBase] = getattr(self, wired_prop.name)
            assert type(value_ptrs) is list, f"invalid {prop}: {value_ptrs!r}"
            if len(value_ptrs) == 0:
                return ()
            values = []
            for value_ptr in value_ptrs:
                value = self._supergraph.get(value_ptr)
                if value is not None:
                    values.append(value)
                elif value_ptr.type in RICH_REFERENCE_TYPES_BY_NODE_TYPE:
                    values.append(value_ptr)  # :RichReferences
            return values

        def _set_node_many(self: BuiltinObject, values: Collection["Node"]):
            values = tuple(values)
            assert all(
                self._supergraph.has(v._supergraph) for v in values
            ), f"{prop}: {values!r} is from {values[0]._supergraph} not {self._supergraph!r}"
            value_ptrs = [p.to_ref() for p in values]
            self._do_set(wired_prop.name, value_ptrs, track=False, validate=False)

        return property(_get_node_many, _set_node_many)


def _object_node_ref_attr(key: str, prop: Property) -> property:
    """The computed get property from a specific attribute of a node pointer."""

    wired_prop = prop.reference_wired_ptr
    assert wired_prop is not None, f"no wired prop for {prop!r}"

    if not prop.is_list:

        def _get_node_ref_attr_scalar(self):
            value_ptr = getattr(self, wired_prop.name)
            if value_ptr is not None:
                return getattr(value_ptr, key)
            else:
                return None

        def _set_node_ref_attr_scalar(self: BuiltinObject, value):
            raise RuntimeError(f"cannot set computed property attribute {prop!r}: {value!r}")

        return property(_get_node_ref_attr_scalar, _set_node_ref_attr_scalar)

    else:

        def _get_node_ref_attr_many(self: BuiltinObject):
            value_ptrs = getattr(self, wired_prop.name)
            assert type(value_ptrs) is list, f"invalid {prop}: {value_ptrs!r}"
            return tuple(getattr(p, key) for p in value_ptrs)

        def _set_node_ref_attr_many(self: BuiltinObject, values):
            raise RuntimeError(f"cannot set computed property attribute {prop!r}: {values!r}")

        return property(_get_node_ref_attr_many, _set_node_ref_attr_many)


def _node_ancestor_ref(prop: Property) -> property:
    """The computer get property for Node ancestors."""

    # NOTE :Performance: _node_ancestor_ref could just walk in the graph directly?

    if prop.reference_kind == ReferenceKind.NODE_ANCESTOR_OR_SELF:

        def get_ancestor_first_self(self: Node) -> Optional[Node]:
            parent = self
            while parent is not None:
                if prop.reference_nodes and parent.metatype in prop.reference_nodes:
                    return parent
                parent = parent.parent
            return None

        get = get_ancestor_first_self

    elif prop.reference_kind == ReferenceKind.NODE_ANCESTOR:

        def get_ancestor_first_other(self: Node) -> Optional[Node]:
            parent = self.parent
            farthest = None
            while parent is not None:
                if prop.reference_nodes and parent.metatype in prop.reference_nodes:
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


@object_component()
class BuiltinObject[ObjectDataT: AnyNodeData | AnyStructData](abc.ABC):
    """The base for all intrinsic objects like Structs and Nodes and all their derivatives."""

    metatype: ClassVar[ObjectType]
    __components__: ClassVar[tuple[type["BuiltinObject"], ...]] = ()
    __passthrough__: ClassVar[tuple[str, ...] | None] = None

    __is_struct__: ClassVar[bool] = False
    __is_node__: ClassVar[bool] = False

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
        parent: "BuiltinObject | ValueObject | None" = None

    _session: "Session | None" = p_runtime(default=None)
    _supergraph: "NodeSuperGraph" = p_runtime(default=None)
    _updated_properties: bitarray | None = p_runtime(default=None)

    def __init__(
        self, *, _skip_init_self: bool = False, _skip_validate_self: bool = False, **kwargs
    ):
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
                    assert supergraph.has(
                        prop_value._supergraph
                    ), f"{prop}: {prop_value!r} is from {prop_value._supergraph!r} not {supergraph!r}"
                    wired_prop_value = cast(Any, prop_value).to_ref()
                else:
                    assert all(
                        supergraph.has(v._supergraph) for v in prop_value
                    ), f"{prop}: {prop_value!r} is from {prop_value[0]._supergraph!r} not {supergraph!r}"
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
                    prop_value = prop_value._move_to(self, prop)

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

        # and init components
        if not _skip_init_self:
            self._init_self()

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
            return self._equals_content(other)

    def _equals_content(self, other: Any, ignore: tuple[Property | Any, ...] = ()) -> bool:
        """Checks if all wired properties of the two structs are equal (recursively)."""
        if other is None or self.metatype != getattr(other, "metatype", None):
            return False
        for prop in self.__wired_properties__.values():
            if (
                prop.id < 30
                or prop.is_encrypted
                or prop.is_value_packed  # compared in runtime value
                or prop.name == "order_key"
            ):
                continue  # ignore identity/tracking
            self_value = getattr(self, prop.name)
            other_value = getattr(other, prop.name)
            if (
                self_value != other_value
                and not is_close(self_value, other_value, FLOAT_EPSILON)
                and prop not in ignore
            ):
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

    def _do_get(self, key):
        """Called if an attribute doesn't exist in __dict__ / the usual places."""

        # check passthrough (if in a session)
        if self.__passthrough__ is not None and self._session is not None:
            for passthrough_key in self.__passthrough__:
                target = getattr(self, passthrough_key, UNSET)
                attr = getattr(target, key, UNSET)
                if attr is not UNSET:
                    return attr

        try:
            self_str = repr(self)
        except Exception:
            self_str = self.__class__.__name__
        raise AttributeError(f"{self_str} has no attribute '{key}'")

    def _do_set(self, key: str, value, *, track: bool = True, validate: bool = True):
        """Sets *any* attribute on this node."""
        is_tracked = track and self._session is not None
        prop = self.__properties__.get(key)
        if prop is not None:
            if (prop.is_ephemeral and not prop.is_value_runtime) or prop.is_autoset:  # untracked
                object.__setattr__(self, key, value)
                return
            elif prop.reference_kind == ReferenceKind.NODE_CHILDREN:
                existing = getattr(self, key)
                existing.set(value)
                return

            # validate/set
            old_value = getattr(self, key)
            if is_tracked and validate:
                # coerce & check type (if it's not a contributed property, which only we edit)
                if prop._type_info is not None and prop.reference_source is None:
                    value = coerce_value(
                        value,
                        prop._type_info,
                        as_packed=False,
                        parent=self,
                        parent_prop=prop,
                        ancestor_prop=prop,
                    )
                    check_value(value, prop._type_info, invalid=on_invalid_raise)
                object.__setattr__(self, key, value)
                try:
                    self._validate_self((prop,), invalid=on_invalid_raise)
                except ValidationError:  # reset on error
                    object.__setattr__(self, key, old_value)
                    raise
            else:
                object.__setattr__(self, key, value)

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
                        if prop.is_value_runtime:
                            # put value packed update into _packed property :ComputedValueProp
                            value_packed_ptr = cast(Property, prop.value_packed_ptr)
                            old_value = getattr(self, value_packed_ptr.name)
                            self._updated_properties[value_packed_ptr.ord] = True
                            session._update(
                                node,
                                properties=(value_packed_ptr,),
                                old_values={value_packed_ptr.id: old_value},
                            )
                        else:
                            self._updated_properties[prop.ord] = True
                            session._update(
                                node, properties=(prop,), old_values={prop.id: old_value}
                            )
                else:
                    pass  # TODO :Broken: handle in struct updates
            return
        elif is_tracked and self.__passthrough__ is not None:
            # try passthrough target (if any)
            for passthrough_key in self.__passthrough__:
                target = getattr(self, passthrough_key, UNSET)
                if target is not UNSET:
                    setattr(target, key, value)
                    return

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

    def clone(self) -> Self:
        """
        Create a clone of this object and its descendants (structs/nodes) with the same content.
        NOTE :Broken: node clone should keep inner references consistent :CloneNodeReferences
        """
        copy_kwargs = {}
        for prop in self.__wired_properties__.values():
            prop_value = getattr(self, prop.name)
            if prop.id < 30:
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

    def _init_component(self):  # noqa: B027
        """Called after post init is done."""
        pass  # do nothing by default

    def _init_self(self):
        """Called after post init is done."""
        for component in self.__components__:
            component._init_component(self)

    def _validate_component(self, properties: tuple[Property, ...], invalid: "ValidationHandler"):  # noqa: B027
        """Check the integrity of the component."""
        pass  # do nothing by default

    @final
    def _validate_self(
        self, properties: tuple[Property, ...], invalid: "ValidationHandler"
    ) -> None:
        """Check the integrity of this object."""
        # NOTE :Architecture: BuiltinObject.validate does not validate value objects, but .do_set does
        #  (this is somewhat inconsistent, but also useful because e.g. for Runs we don't want to error
        #   during validation when unpacking, only later when manually checking the inputs;
        #   however Run.inputs = ... directly errors if invalid, which is inconsistent but convenient.
        #   Maybe add a flag to .validate whether to validate values?)
        # check properties types
        for prop in properties or self.__tracked_properties__.values():
            # NOTE: references may be unloaded and there's not much to validate, so we don't
            if prop._type_info is not None and prop.reference_kind is None:
                value = getattr(self, prop.name)
                check_value(value, prop._type_info, invalid=invalid)
        # check components
        for component in self.__components__:
            component._validate_component(self, properties, invalid)

    @final
    def _validate_rec(self, invalid: "ValidationHandler"):
        # check inner structs
        for inner_struct in self._walk_struct():
            inner_struct._validate_self((), invalid)

    def _updated_component(self, properties: tuple[Property, ...]):  # noqa: B027
        """Called when properties have been updated."""
        pass  # do nothing by default

    @final
    def _updated_self(self, properties: tuple[Property, ...]) -> None:
        """Called when properties have been updated."""
        for component in self.__components__:
            component._updated_component(self, properties)

    def _flushed_self(self):
        """Called when this struct has been flushed to the store."""
        if self.__is_node__:
            (cast("Node", self))._is_new = False
        self._updated_properties = None

    def __bool__(self):
        return True  # support truthy checks for objects

    @final
    def _to_data(self) -> ObjectDataT:
        """Convert to wire format"""
        from bench.proto.wiring import pack_object

        return pack_object(self)  # type: ignore

    @classmethod
    def get_property(cls, key: str | Property) -> Property:
        if isinstance(key, str):
            prop = cls.__properties__.get(key)
            if prop is None:
                raise ValueError(f"no property {key} in {cls}")
            return prop
        elif isinstance(key, Property):
            if key.component == cls:
                return key
            else:
                prop = cls.__properties_by_id__.get(key.id)
                if prop is None:
                    raise ValueError(f"no property {key} in {cls}")
                return prop
        else:
            raise ValueError(f"invalid prop key {key}")

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


@object_component()
class Struct[StructDataT: AnyStructData](BuiltinObject[StructDataT], abc.ABC):
    """A base for structs with properties."""

    metatype: ClassVar[StructType]  # type: ignore

    __is_struct__: ClassVar[bool] = True

    parent: Union["BuiltinObject", "ValueObject", None] = p_struct_parent(3)

    def __content_str__(self) -> str:
        # default __content_str__ for structs where we're too lazy to define one
        value_strs = []
        for prop in self.__declared_properties__.values():
            prop_value = getattr(self, prop.name)
            if prop_value is not None and not (isinstance(prop_value, list) and not prop_value):
                value_strs.append(f"{prop.name}={prop_value!r}")
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

    def override(self, override: "Self | None") -> Self:
        """Overrides this struct with set properties from another struct (in a copy)."""
        if override is None:
            return self
        copy = self.clone()
        for prop in self.__declared_properties__.values():
            override_value = getattr(override, prop.name)
            if (not prop.is_list and override_value is not None) or (
                prop.is_list and override_value
            ):
                setattr(copy, prop.name, override_value)
        return copy

    def _move_to(
        self,
        parent: Union["BuiltinObject", "ValueObject"],
        prop: Union[Property, "Field"],
        ancestor_prop: Property | None = None,
    ) -> Self:
        """Move or copy this struct into given parent/prop."""
        assert self.__is_struct__, f"cannot copy non-struct {self!r}"  # this is overriden by Node
        if self.parent is None:  # not assigned
            self.parent = parent
            return self
        else:
            copy = self._copy_to(parent, prop)
            return copy

    def _copy_to(
        self, parent: Union["BuiltinObject", "ValueObject"], prop: Union[Property, "Field"]
    ) -> Self:
        """Create a copy of this struct for the given parent/prop."""
        kwargs = {p.name: getattr(self, p.name) for p in self.__wired_properties__.values()}
        kwargs["parent"] = parent
        copy = self.__class__(**kwargs)
        return copy


FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type["Node"]]
EditSubject = Union["User", "Server", "Block", "Step", "Run"]
EDIT_SUBJECT_TYPES = (NodeType.USER, NodeType.SERVER, NodeType.BLOCK, NodeType.STEP, NodeType.RUN)


def is_implicit_node_property(prop_id: int) -> bool:
    return prop_id < 30 and prop_id != 4  # parent is fine


@node_component()
class Node[NodeDataT: AnyNodeData](BuiltinObject[NodeDataT], abc.ABC):
    """
    A Node with properties like a Struct and a global identity in our graph.
    Conceptually, all nodes live together happily in a single giant supergraph.
    In practice, there are multiple stores and we load smaller subgraphs at runtime.
    Nodes resolve references to each through a super graph composed of *currently loaded* NodeGraphs.
    """

    # NOTE :Architecture!: we may need a better :NodeInheritance mechanism since some
    #  nodes have different subtypes with varying properties (e.g. Step or View).
    # Mapping their union into database columns is annoying since there may be many,
    #  so maybe this has to wait until we have custom storage engines.

    metatype: ClassVar[NodeType]  # type: ignore

    __is_node__: ClassVar[bool] = True
    # NOTE :Test: make id factories deterministic (incl. UUIDT? somehow)
    __id_factory__: ClassVar[Callable[[], UUID]] = uuid4
    __ck_factory__: ClassVar[Callable[[], UUID]] = uuid4

    __node_child_properties__: ClassVar[dict[str, Property]] = {}

    __roots__: ClassVar[bittuple[NodeType]] = UNSET
    __is_struct__: ClassVar[bool] = False
    __is_node__: ClassVar[bool] = True
    __is_in_bench__: ClassVar[bool] = UNSET  # part of a Bench
    __is_in_package__: ClassVar[bool] = UNSET  # part of a Package
    __is_stored__: ClassVar[bool] = False  # stored in primary store (runtime or local)
    __is_stored_custom__: ClassVar[bool] = False  # custom storage logic (for records)
    __is_local__: ClassVar[bool] = False  # stored in Bench-local DB (instead of global Bench DB)
    __extra_indexes__: ClassVar[tuple[tuple[str, ...], ...]] = ()  # extra indexes for PG
    __extra_uniques__: ClassVar[tuple[tuple[str, ...], ...]] = ()  # extra constraints for PG

    # 1-9: reserved for node identity
    id: UUID = p_system(2, default=None, require=True, autoset=True)
    parent: Optional["Node"] = p_node_parent(4)  # type: ignore
    if TYPE_CHECKING:
        parent_type: NodeType | None = None
        parent_id: Optional[UUID] = None
        parent_ptr: Optional[NodeReference] = None

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
    updated_at: datetime = p_system(13, default=None, require=True, autoset=True)
    deleted_at: Optional[datetime] = p_system(15, default=None, autoset=True)
    archived_at: Optional[datetime] = p_system(16, default=None, autoset=True)
    # NOTE: created_by/updated_by are 'baseless' because Run can only be a subject if it's a lambda
    #  (and otherwise we attribute the change to the Run's block/step/identity)
    created_by: EditSubject | None = p_system(
        21,
        default=None,
        require=False,
        array=False,
        autoset=True,
        references=EDIT_SUBJECT_TYPES,
        same_bench=True,
        baseless=True,
    )
    updated_by: EditSubject | None = p_system(
        22,
        default=None,
        require=False,
        array=False,
        autoset=True,
        references=EDIT_SUBJECT_TYPES,
        same_bench=True,
        baseless=True,
    )
    if TYPE_CHECKING:
        created_by_id: Optional[UUID] = None
        created_by_type: NodeType | None = None
        updated_by_id: Optional[UUID] = None
        updated_by_type: NodeType | None = None

    # computed_properties: dict[int, "ComputedValue"] = p_internal(28, array=True, store=False)
    set_properties: list[int] = p_internal(29, array=True, store=False)

    # 30+ for 'user' node/struct properties
    # <... defined in concrete type ...>

    _graph: "NodeGraph" = p_runtime(default=None)
    _connection: "GetConnection | SearchConnection" = p_runtime(default=None)
    _is_new: bool = p_runtime(default=False)

    def __init__(
        self, *, _skip_add_self: bool = False, _skip_validate_self: bool = False, **kwargs
    ):
        super().__init__(**kwargs, _skip_init_self=True, _skip_validate_self=True)

        # init ck/id
        if isinstance(self, HasPersistentIdentity):
            if self.ck is None:
                self.ck = self.__class__.__ck_factory__()
                self.id = self.__class__.__id_factory__()
                self._is_new = True
        elif self.id is None:
            self.id = self.__class__.__id_factory__()
            self._is_new = True

        # init timestamps
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
                scope=GraphScope()._to_data(),
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

        # init session context
        if self._session is not None:
            if self._is_new and not _skip_validate_self:
                self._validate_self((), invalid=on_invalid_raise)
            self._track_self(self._session)

        # init components
        self._init_self()

    @final
    def __str__(self):  # type: ignore
        # override the default __str__ for nodes
        content_str = self.__content_str__()
        if content_str:
            content_str = f" ({content_str})"
        ident_str = self.py_name
        if ident_str is None:
            ident_str = str(self.id)
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
    def ck(self):
        return self.id

    @property
    def _is_attached(self) -> bool:
        return True

    @override
    def clone(self, detach: bool = False) -> Self:
        # clone self
        clone = super().clone()

        # clone children and append to self (recursive)
        for child_prop in self.__node_child_properties__.values():
            child_list = getattr(self, child_prop.name)
            clone_list = getattr(clone, child_prop.name)
            for child in child_list:
                child_clone = child.clone(detach=True)
                clone_list.append(child_clone)  # re-attach

        # append to our parent to re-attach
        parent = self.parent
        if not detach and parent:
            parent_child_prop = parent.get_node_child_property(self.metatype)
            parent_list = getattr(parent, parent_child_prop.name)
            parent_list.append(clone)
        return clone

    def __eq__(self, other: Any):
        """Equals node identity."""
        return self is other or (type(self) == type(other) and (self.id == other.id))

    def _stable_hash(self):
        """Hash node identity."""
        return stable_hash((self.metatype, self.id))

    # only define __hash__ for nodes since their id is constant
    __hash__ = _stable_hash  # type: ignore

    @property
    def connection(self):
        """The currently active connection (errors if none)"""
        assert self._connection is not None, f"no connection for {self!r}"
        return self._connection  # type: ignore (class definition "depends on itself" for some reason)

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
            return f"[ck={self.ck}]"
        else:
            return f"[id={self.id}]"

    @property
    def py_name(self) -> Optional[str]:
        """The python identifier-compatible name of this node."""
        if self.metatype == NodeType.PACKAGE and self.parent is not None:
            return self.parent.py_name
        if "slug" in self.__properties__:
            slug = getattr(self, "slug")
            if slug:  # prefer slug as ident
                return slug
        if "name" in self.__properties__:
            name = getattr(self, "name")
            if name is None:
                return None
            return to_py_name(name)
        return None

    @property
    def absolute_path(self) -> str:
        if self.__parent_property__ is None or not self.__parent_property__.reference_nodes:
            ident = self._ident
            assert ident is not None, f"no bench ident for {self!r}"
            return ident
        elif self._supergraph is None or self.parent is None:
            return f"<detached>/{self._path_key}"
        else:
            # this is a mini version of Path.render
            path_parts: list[str] = [self._path_key]
            parent = self.parent
            while parent is not None:
                if parent.metatype == NodeType.BRANCH or parent.metatype == NodeType.PACKAGE:
                    parent = cast("Branch | Package", parent).bench
                path_parts.append(parent._path_key)
                parent = parent.parent
            return "/".join(reversed(path_parts))

    def _to_plain_ref(self) -> "NodeReference":
        """Gets a plain reference to this node."""
        return NodeReference._ref_from_node(self)

    def _to_plain_ref_data(self) -> NodeReferenceData:
        """Gets a plain data reference to this node."""
        return NodeReference._ref_from_node(self)._to_data()

    def to_ref(self) -> "SomeNodeReference":
        """Gets a reference to this node. May be rich in subclasses."""
        return NodeReference._ref_from_node(self)

    def _to_ref_data(self) -> "SomeNodeReferenceData":
        """Gets a data reference to this node. May be rich in subclasses."""
        return NodeReference._ref_from_node(self)._to_data()

    #
    # Lifecycle
    #

    @property
    def is_extant(self):
        return self.archived_at is None and self.deleted_at is None

    @property
    def is_archived(self) -> bool:
        parent = self.parent
        return self.archived_at is not None or (parent is not None and parent.is_archived)

    @property
    def is_deleted(self) -> bool:
        parent = self.parent
        return self.deleted_at is not None or (parent is not None and parent.is_deleted)

    def move(
        self,
        to_parent: "Node",
        after: Optional["Node"],
        before: Optional["Node"],
    ):
        """Move this node to a new parent."""
        from_parent = self.parent
        from_parent_ptr = self.parent_ptr
        assert from_parent_ptr and from_parent, f"{self!r} has no parent"
        assert to_parent._graph is from_parent._graph, f"{self!r} not in graph of {to_parent!r}"
        raise NotImplementedError("move not yet supported")

    def archive(self):
        """Archive this node."""
        assert not self.is_archived, f"{self!r} is already archived"
        self.active_session._archive(self)

    def unarchive(self):
        """Unarchive this node."""
        assert self.is_archived, f"{self!r} is not archived"
        self.active_session._unarchive(self)

    def delete(self):
        """Delete this node (move to trash)."""
        assert not self.is_deleted, f"{self!r} is already deleted"
        self.active_session._delete(self)

    def restore(self):
        """Restore this deleted node from the trash."""
        assert self.is_deleted, f"{self!r} is not deleted"
        self.active_session._restore(self)

    def erase(self):
        """Wipe this node from this universe forever."""
        self.active_session._erase(self)

    @classmethod
    def get_node_child_property(cls, node_type: NodeType) -> Property:
        for prop in cls.__node_child_properties__.values():
            if prop.reference_nodes and node_type in prop.reference_nodes:
                return prop
        else:
            raise ValueError(f"no child property for {node_type} in {cls}")

    #
    # Querying
    #

    @classmethod
    def query(cls) -> "QueryBuilder[Self, NodeDataT]":
        from bench.language.query import QueryBuilder

        return QueryBuilder(read_type=ReadType.SEARCH, node_type=cls.metatype)

    @classmethod
    def where(
        cls, filter: Optional["Expression"] = None, **kwargs
    ) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().where(filter, **kwargs)

    @classmethod
    def order_by(
        cls, sort: Optional["Expression"] | str = None, *args: str
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
    def include_ancestors(cls) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().include_ancestors()

    @classmethod
    def ancestors(cls, *node_types: NodeTypeOrClass) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().ancestors(*node_types)

    @classmethod
    def descendants(cls, *node_types: NodeTypeOrClass) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().descendants(*node_types)

    @overload
    @classmethod
    async def get(
        cls,
        filter: Optional["Expression | SomeNodeReference | None"] = None,
        live: bool = False,
        **kwargs,
    ) -> Self: ...
    @overload
    @classmethod
    async def get(
        cls, filter: Sequence["SomeNodeReference"], live: bool = False, **kwargs
    ) -> list[Self]: ...
    @classmethod
    async def get(
        cls,
        filter: Optional[
            "Expression | SomeNodeReference | Sequence[SomeNodeReference] | None"
        ] = None,
        live: bool = False,
        **kwargs,
    ) -> Self | list[Self]:
        return await cls.query().get(filter, live=live, **kwargs)

    @classmethod
    async def search(cls, filter: Optional["Expression"] = None, **kwargs) -> list[Self]:
        return await cls.query().search(filter, **kwargs)

    @classmethod
    def first(cls, count: int) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().first(count)

    @classmethod
    async def count(cls, filter: Optional["Expression"] = None, **kwargs) -> int:
        return await cls.query().count(filter, **kwargs)

    @classmethod
    async def exists(cls, filter: Optional["Expression"] = None, **kwargs) -> bool:
        return await cls.query().exists(filter, **kwargs)


@node_component()
class BenchNode[NodeDataT: AnyNodeData](Node[NodeDataT], abc.ABC):
    """A node that exists inside a Bench."""

    bench: "Bench" = p_node_ancestor_with_self(
        6, NodeType.BENCH, require=True, store=True, wire=True
    )
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None
    created_epoch: int = p_system(
        12, default=-1, default_sql=None, autoset=True, primitive_type=PrimitiveType.INT64
    )
    updated_epoch: int = p_system(
        14, default=-1, default_sql=None, autoset=True, primitive_type=PrimitiveType.INT64
    )

    @property
    def _is_attached(self) -> bool:
        return self.parent is not None and self.bench is not None


@node_component()
class PackageNode[NodeDataT: AnyNodeData](BenchNode[NodeDataT], abc.ABC):
    """A node that exists inside a Package."""

    package: "Package" = p_node_ancestor_with_self(
        5, NodeType.PACKAGE, require=True, store=True, wire=True, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        package_id: Optional[UUID] = None
        package_ptr: Optional[NodeReference] = None

    @property
    def _is_attached(self) -> bool:
        return self.parent is not None and self.package is not None


@node_component()
class HasPersistentIdentity(Node, abc.ABC):
    """A node with a persistent identity across versions."""

    ck: UUID = p_system(3, default=None, require=True, autoset=True)  # type: ignore


@node_component()
class SourceNode[NodeDataT: AnyNodeData](PackageNode[NodeDataT], HasPersistentIdentity, abc.ABC):
    """A package node with a persistent identity that can be instanced."""

    template: Optional["Node"] = p_node_template(7)
    templated_epoch: int | None = p_system(
        8, require=False, default=None, autoset=True, primitive_type=PrimitiveType.INT64
    )
    if TYPE_CHECKING:
        template_id: Optional[UUID] = None
        template_ptr: Optional[NodeReference] = None


@node_component()
class RemoteNode[NodeDataT: AnyNodeData](PackageNode[NodeDataT], abc.ABC):
    """A package node with a persistent identity that can be instanced."""

    pass


@node_component()
class HasTimeIdentity(BuiltinObject, abc.ABC):
    """A node whose identity is tied to a specific point in time."""

    __id_factory__: ClassVar[Callable[[], UUID]] = UUIDT
    __ck_factory__: ClassVar[Callable[[], UUID]] = UUIDT


@node_component()
class HasNodeBase(BuiltinObject, abc.ABC):
    """A node which may have a 'base' in another node (e.g., its type definition)."""

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


# :NodeSubtype
NODE_SUBTYPE_PROPERTY_BY_TYPE: dict[NodeType, str] = {
    NodeType.BLOCK: "type",
    NodeType.STEP: "type",
    NodeType.FIELD: "zone",
    NodeType.VIEW: "type",
    NodeType.FILE: "coarse_type",
}
NODE_SUBSUBTYPE_PROPERTY_BY_TYPE: dict[NodeType, str] = {
    NodeType.FIELD: "kind",
    NodeType.FILE: "format",
}

#
# Utility types
#


@struct_(StructType.GRAPH_SCOPE)
class GraphScope(Struct[GraphScopeData]):
    """The scope for an operation on the Bench graph."""

    bench_id: Optional[UUID] = p_internal(30, default=None)
    package_id: Optional[UUID] = p_internal(31, default=None)


EMPTY_SCOPE = GraphScope()


@struct_(StructType.CLIENT_ORIGIN)
class ClientOrigin(Struct[ClientOriginData]):
    """Information to identify a client."""

    type: ClientType = p_internal(30, require=True)
    id: Optional[UUID] = p_internal(31, default=None)
    nonce: Optional[UUID] = p_internal(32, default=None)


@object_component()
class NodeReferenceBase[NT: Node, ND: AnyNodeData, RT: NodeReferenceBase, RD: AnyStructData](
    BuiltinObject
):
    """A base for node references for extension in richer references (Files, Secrets, ...)"""

    type: NodeType = p_internal(30, require=True)
    id: Optional[UUID] = p_internal(31, default=None)
    ck: Optional[UUID] = p_internal(32, default=None)
    bench_id: Optional[UUID] = p_internal(33, default=None)
    base_ck: Optional[UUID] = p_internal(34, default=None)
    # base could be in a different Bench (e.g. a Signal in Bench A with a type from Bench B)
    base_bench_id: Optional[UUID] = p_internal(35, default=None)

    @staticmethod
    def _clone_ref[T: NodeReferenceBase | Any](
        ref_cls: Type[T], ref: "NodeReferenceBase | Any", **kwargs
    ) -> T:
        return ref_cls(
            id=ref.id,
            ck=ref.ck,
            type=ref.type,
            bench_id=ref.bench_id,
            base_ck=ref.base_ck,
            base_bench_id=ref.base_bench_id,
            **kwargs,
        )

    def to_ref(self) -> "Self":
        """Gets the reference (noop for compatibility with Node.to_ref)."""
        return self

    def _to_plain_ref(self) -> "NodeReference":
        """Gets a plain reference from this reference."""
        return NodeReference(
            type=self.type,
            id=self.id,
            ck=self.ck,
            bench_id=self.bench_id,
            base_ck=self.base_ck,
            base_bench_id=self.base_bench_id,
        )

    @staticmethod
    @abc.abstractmethod
    def _ref_from_node(node: NT) -> RT:
        """Turn a node into a reference to that node."""
        raise NotImplementedError

    @staticmethod
    @abc.abstractmethod
    def _ref_data_from_node_data(node_data: ND) -> RD:
        """Turn a data node into a data node reference to that node."""
        raise NotImplementedError


@struct_(StructType.NODE_REFERENCE)
class NodeReference(Struct[NodeReferenceData], NodeReferenceBase):
    """
    A plain reference to a Node.
    We include the Bench and 'ck' where available.
    Base = the node is 'based' on (like Record.parent->Block, Signal.type->Block).
    """

    # ...NodeReferenceBase[30-39]

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
            content_parts.append(f"base_bench_id={self.base_bench_id}")
        selector_str = ", ".join(content_parts)
        return f"{self.type.bench_name}:[{selector_str}]"

    def _validate_component(
        self, properties: Collection[Property], invalid: "ValidationHandler"
    ) -> None:
        pass  # NOTE :Robustness: we used to require bench_id for sub-bench types here

    @override
    def _to_plain_ref(self) -> "NodeReference":
        return self

    @override
    @staticmethod
    def _ref_from_node(node: Node) -> "NodeReference":
        assert isinstance(node, Node), f"expected Node, got {node!r}"
        reference = NodeReference(type=node.metatype, id=node.id, ck=node.ck)

        # bench
        if node.metatype == NodeType.BENCH:
            reference.bench_id = node.id
        elif isinstance(node, BenchNode):
            bench_id = node.bench_id
            if bench_id is None:
                # maybe just creating, try from context
                session = _active_session.get()
                if session is not None and session.bench_id is not None:
                    bench_id = session.bench_id
            reference.bench_id = bench_id
        # base
        if node.metatype in BASED_NODE_TYPES:
            base = cast(HasNodeBase, node).base
            if base is not None:
                reference.base_ck = base.ck
                base_bench_id = base.bench_id
                if base_bench_id is None:
                    # maybe also just creating, try from context
                    session = _active_session.get()
                    if session is not None and session.bench_id is not None:
                        base_bench_id = session.bench_id
                reference.base_bench_id = base_bench_id

        return reference

    @override
    @staticmethod
    def _ref_data_from_node_data(node_data: AnyNodeData) -> "NodeReferenceData":
        from bench.proto import wire

        node_cls = OBJECT_CLASS_BY_TYPE[cast(ObjectType, node_data.metatype)]
        reference = NodeReferenceData(
            metatype=wire.ObjectType.NODE_REFERENCE,
            type=cast(wire.NodeType, node_data.metatype),
            id=node_data.id,
            ck=getattr(node_data, "ck", node_data.id),
        )

        # bench
        if node_data.metatype == NodeType.BENCH:
            reference.bench_id = node_data.id
        elif "bench" in node_cls.__properties__ and node_data.parent_ptr is not None:
            reference.bench_id = node_data.parent_ptr.bench_id
        # base
        if NodeType(node_data.metatype) in BASED_NODE_TYPES:
            base = cast(HasNodeBase, node_cls).get_base_from_data(node_data)
            if base is not None:
                reference.base_ck = base.ck
                reference.base_bench_id = base.bench_id

        return reference


# some node types have richer representations in references :RichReferences
#  (we only store those in Values or when the property explicitly has reference_is_rich,
#   since otherwise every single ptr to a potential rich type would carry a lot of metadata)
RICH_REFERENCE_TYPES_BY_NODE_TYPE: dict[NodeType, StructType] = {
    NodeType.FILE: StructType.FILE_REFERENCE,
    NodeType.SECRET: StructType.SECRET_REFERENCE,
}
NODE_REFERENCE_TYPES = (StructType.NODE_REFERENCE, *RICH_REFERENCE_TYPES_BY_NODE_TYPE.values())
SomeNodeReference = Union[NodeReference, "FileReference", "SecretReference"]
SomeNodeReferenceData = Union[NodeReferenceData, FileReferenceData, SecretReferenceData]


@struct_(StructType.PROPERTY_REFERENCE)
class PropertyReference(Struct):
    """
    A reference to a builtin object's Property.
    If type is unset, this refers to a base property in one of the base BuiltinObject types.
    """

    type: ObjectType | None = p_regular(30)
    id: int = p_regular(31)
    references_node: Optional[NodeType] = p_internal(32)  # disambiguate reference properties

    def __content_str__(self):
        object_cls = Node if self.type is None else OBJECT_CLASS_BY_TYPE.get(self.type)
        if object_cls is None:
            if self.type is None:
                return f"Node.??? [id={self.id}]"
            else:
                return f"{self.type.name}.??? [id={self.id}]"
        else:
            prop = object_cls.__properties_by_id__.get(self.id)
            if prop is None:
                return f"{object_cls.__name__}.??? [id={self.id}]"
            else:
                return f"{object_cls.__name__}.{prop.name} [id={self.id}]"

    @property
    def object_cls(self) -> Type[BuiltinObject] | None:
        if self.type is None:
            return Node
        else:
            return OBJECT_CLASS_BY_TYPE.get(self.type)

    @override
    def _validate_component(self, properties: tuple[Property, ...], invalid: "ValidationHandler"):
        object_cls = self.object_cls
        if object_cls is None:
            invalid(self, "invalid type", (PropertyReference.type,))
        else:
            prop = object_cls.__properties_by_id__.get(self.id)
            if prop is None:
                invalid(self, "invalid prop id", (PropertyReference.id,))

    def resolve(self) -> Property:
        resolved = self.resolve_maybe()
        if resolved is None:
            raise ValueError(f"could not resolve {self!r}")
        return resolved

    def resolve_maybe(self) -> "Property | None":
        object_cls = self.object_cls
        prop = (object_cls or Node).__properties_by_id__.get(self.id)
        if prop is None:
            return None
        if self.references_node is not None:
            assert prop.reference_stored_ids_by_type is not None, f"{prop!r} has no stored ids"
            prop = prop.reference_stored_ids_by_type.get(self.references_node)
            if prop is None:
                return None
        return prop


# NOTE: import from .value later to avoid circular import
#  (but import at top level to avoid import in critical path)


from bench.language.value import check_value, coerce_value  # noqa: E402


@local_node_(NodeType.SKIP, stored=False)
class Skip(Node):
    """A reference to another node in some graph that wasn't available for some reason (usually permissions)."""

    parent: Node = p_node_parent(4, *NODE_TYPES.tuple)
    reference: Optional[Node] = p_regular(
        30, array=False, references=NODE_TYPES.tuple, require=True
    )
    order_key: Optional[str] = p_internal(31, default=None)

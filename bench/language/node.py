import abc
import base64
import functools
import inspect
import math
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
    Iterable,
    Optional,
    Self,
    Type,
    TypeGuard,
    TypeVar,
    Union,
    cast,
    dataclass_transform,
    final,
)
from uuid import UUID, uuid4

import structlog
from bitarray import bitarray

from bench.language.const import (
    IN_BENCH_NODE_TYPES,
    IN_PACKAGE_NODE_TYPES,
    NODE_TYPES,
    TK_LENGTH_BYTES,
    UNSET,
    NodeType,
    ObjectType,
    PrimitiveType,
    ReadType,
    ReferenceKind,
    StructType,
    _active_session,
    new_struct_id,
)
from bench.language.graph import NodeDataGraph, NodeGraph
from bench.language.property import (
    _PROPERTY_SPECIFIERS,
    METATYPE_PROPERTY,
    Property,
    p_internal,
    p_node_ancestor_first,
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
    STRUCT_CLASS_BY_TYPE,
)
from bench.language.validation import ValidationError, ValidationHandler, on_invalid_raise
from bench.proto.wire import AnyNodeData, AnyStructData, GraphScope, NodeReferenceData
from bench.sql.core import Constraint, ConstraintType, Index, IndexType, Table, stable_hash
from bench.utils.casing import PYTHON_CASING, IdentifierType, to_casing
from bench.utils.dt import utcnow
from bench.utils.env import IS_DEV, IS_TEST
from bench.utils.func import bittuple
from bench.utils.utils import frozendict
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Expression,
        Field,
        GetConnection,
        NodeReference,
        NodeSuperGraph,
        Package,
        PropertyReference,
        QueryBuilder,
        ReadOptions,
        Run,
        SearchConnection,
        Server,
        Session,
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


_BASE_OBJECT_NAMES = ("BuiltinObject", "InlineStruct", "Struct", "Node")


def _process_object_cls[ObjectT: BuiltinObject](
    cls: type[ObjectT],
    object_type: ObjectType | None,
    is_final: bool = False,
    # for nodes only
    is_root: bool = False,
    is_variable_root: bool = False,
    # for structs only
    is_inlined: bool = False,
) -> tuple[type[ObjectT], dict[str, "Property"]]:
    """Process an object base class and return the processed class and its properties."""
    assert isinstance(cls, type), f"expected type, got {cls} ({type(cls)})"

    # NOTE :Performance :Architecture: we can't really use slots for our builtin objects
    #  (as it stands today, __slots__ always uses class-level descriptors, but we also
    #   want to use class level attributes for our own properties, like Block.type, ...
    #   - neglecting this conflict causes fun errors like 'X is a read-only attribute')

    is_object_base = cls.__name__ == "BuiltinObject"
    is_node_base = cls.__name__ == "Node"
    is_struct_base = cls.__name__ in ("InlineStruct", "Struct")
    is_struct = not is_object_base and (is_struct_base or issubclass(cls, InlineStruct))
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

    if IS_DEV or IS_TEST:
        # check components
        for component in static_components[1:]:
            if component.__name__ in _BASE_OBJECT_NAMES:
                continue  # ignore base classes
            if is_node and component.__is_struct_inlined__:
                raise ValueError(f"node {cls} has inlined struct {component}")
            if (
                is_inlined
                and hasattr(component, "metatype")
                and not component.__is_struct_inlined__
            ):
                raise ValueError(f"struct {cls} has non-inlined struct {component}")

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

    # collect properties from all components
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

    # add copmuted properties to final classes
    if is_final:

        def _set_computed(name: str, prop: property):
            """Sets a computed property that shouldn't conflict with any existing property."""
            existing = properties_by_name.get(name)
            if existing is not None and existing.is_runtime:
                raise ValueError(f"property conflict '{name}': {prop!r}, {existing!r}")
            setattr(cls, name, prop)

        for name, prop in properties_by_name.items():
            if prop.reference_source is not None:
                continue  # ignore contributed property

            # computed property.. property
            if prop.reference_kind == ReferenceKind.PROPERTY:
                setattr(cls, name, _object_computed_property_ref(prop))
            # computed node property
            elif prop.reference_kind in (
                ReferenceKind.NODE_PARENT,
                ReferenceKind.NODE_REGULAR,
                ReferenceKind.NODE_TEMPLATE,
            ):
                setattr(cls, name, _object_computed_node_ref(prop))
            # computed node ancestor property
            elif prop.reference_kind in (
                ReferenceKind.NODE_ANCESTOR_FIRST,
                ReferenceKind.NODE_ANCESTOR_ROOT,
            ):
                setattr(cls, name, _node_computed_ancestor_prop(prop))
                setattr(cls, f"{name}_ptr", _node_computed_ancestor_ptr_prop(prop))

            # computed _x node reference properties (e.g., parent_id, type_ck, node_type, ...)
            if prop.is_node_reference and prop.reference_kind != ReferenceKind.NODE_CHILDREN:
                for key in ("id", "ck", "type"):
                    if key == "type" and (
                        not prop.reference_nodes or len(prop.reference_nodes) <= 1
                    ):
                        continue  # no need for *_type if only one possible node type
                    _set_computed(f"{prop.name}_{key}", _object_computed_node_ref_attr(key, prop))

    # update reference to transformed class
    for prop in properties_by_name.values():
        prop.component = cls

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
    if is_final and (is_node or (is_struct and not is_inlined)) and parent_property is None:
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
        return cast(Type[_ObjectT], cls)

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def struct_(struct_type: StructType, inline: bool = False):
    """Register a class as a concrete struct for the given struct type."""

    def decorate(cls: Type[_ObjectT]) -> Type[_ObjectT]:
        cls = object_component(struct_type=struct_type, is_final=True, is_inlined=inline)(cls)
        if IS_DEV and cls.__name__ != "Struct" and cls.__name__ != "InlineStruct":
            if not issubclass(cls, (InlineStruct, Struct)):
                raise ValueError(f"{cls} is not a struct")
            if issubclass(cls, Node):
                raise ValueError(f"{cls} is a node for {struct_type}")

            # check that inline matches inheriting InlineStruct
            is_cls_inlined = not issubclass(cls, Struct)
            if is_cls_inlined != inline:
                raise ValueError(f"inline mismatch for {cls}: ={is_cls_inlined}, inline={inline}")

        return cls

    return decorate


_NodeT = TypeVar("_NodeT", bound="Node")


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def node_component(
    node_type: NodeType | None = None,
    passthrough: str | None = None,
    is_root: bool = False,
    is_variable_root: bool = False,
    is_final: bool = False,
):
    """Mark a class as a node component (or concrete node for a NodeType)."""

    def decorate(cls: Type[_NodeT]) -> Type[_NodeT]:
        cls, properties = _process_object_cls(
            cls=cls,
            object_type=node_type,
            is_root=is_root,
            is_variable_root=is_variable_root,
            is_final=is_final,
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
def node_(
    node_type: NodeType,
    passthrough: str | None = None,
    stored: bool = True,
    stored_custom: bool = False,
    local: bool = False,
    roots: tuple[NodeType, ...] = (NodeType.BENCH,),
    constraints: tuple[Constraint, ...] = (),
    indexes: tuple[Index | tuple[str, ...], ...] = (),
    unique: tuple[tuple[str, ...], ...] = (),
    identifier: IdentifierType = IdentifierType.VARIABLE,
):
    """Register a class as a concrete node for the given node type."""

    in_package = node_type in IN_PACKAGE_NODE_TYPES
    in_bench = node_type in IN_BENCH_NODE_TYPES

    def decorate(cls: Type[_NodeT]) -> Type[_NodeT]:
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
        cls.__identifier_type__ = identifier

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
        cls.__is_in_bench__ = in_bench

        if IS_DEV:
            if issubclass(cls, (InlineStruct, Struct)):
                raise ValueError(f"{cls} is a struct")

        return cls

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def timed_node(
    node_type: NodeType,
    passthrough: str | None = None,
    indexes: tuple[Index | tuple[str, ...], ...] = (),
):
    """Register a class as a concrete node for the given node type."""
    return node_(
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


def _object_computed_property_ref(prop: Property) -> property:
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
                self._do_set(wired_prop.name, None)
            else:
                self._do_set(wired_prop.name, value.to_ref())

        return property(_get_property_scalar, _set_property_scalar)

    else:

        def _get_properties_many(self: BuiltinObject) -> tuple[Property, ...]:
            value_ptrs: list[PropertyReference] = getattr(self, wired_prop.name)
            return tuple(p.resolve() for p in value_ptrs)

        def _set_properties_many(self: BuiltinObject, values: Collection[Property]):
            self._do_set(wired_prop.name, [p.to_ref() for p in values])

        return property(_get_properties_many, _set_properties_many)


def _object_computed_node_ref(prop: Property) -> property:
    """The computed get/set property for a node reference."""
    wired_prop = prop.reference_wired_ptr
    assert wired_prop is not None, f"no wired prop for {prop!r}"

    if not prop.is_list:

        def _get_node_scalar(self: BuiltinObject) -> Optional["Node"]:
            value_ptr: NodeReference | None = getattr(self, wired_prop.name)
            if value_ptr is not None:
                raise NotImplementedError("nocheckin: resolve node!")
            else:
                return None

        def _set_node_scalar(self: BuiltinObject, value: "Node | None"):
            if value is None:
                self._do_set(wired_prop.name, None)
            else:
                self._do_set(wired_prop.name, value.to_ref())

        return property(_get_node_scalar, _set_node_scalar)

    else:

        def _get_node_many(self: BuiltinObject) -> tuple["Node", ...]:
            raise NotImplementedError(f"list nodes not yet supported {prop!r}")

        def _set_node_many(self: BuiltinObject, values: Collection["Node"]):
            raise NotImplementedError(f"list nodes not yet supported {prop!r}")

        return property(_get_node_many, _set_node_many)


def _object_computed_node_ref_attr(key: str, prop: Property) -> property:
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
            return tuple(getattr(p, key) for p in value_ptrs)

        def _set_node_ref_attr_many(self: BuiltinObject, values):
            raise RuntimeError(f"cannot set computed property attribute {prop!r}: {values!r}")

        return property(_get_node_ref_attr_many, _set_node_ref_attr_many)


def _node_computed_ancestor_prop(prop: Property) -> property:
    """The computer get property for Node ancestors."""

    if prop.reference_kind == ReferenceKind.NODE_ANCESTOR_FIRST:

        def get_ancestor_first(self: Node) -> Optional[Node]:
            parent = self
            while parent is not None:
                if prop.reference_nodes and parent.metatype in prop.reference_nodes:
                    return parent
                parent = parent.parent
            return None

        get = get_ancestor_first

    elif prop.reference_kind == ReferenceKind.NODE_ANCESTOR_ROOT:

        def get_ancestor_root(self: Node) -> Optional[Node]:
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

    def set(self: Node, value: Node):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get, set)


def _node_computed_ancestor_ptr_prop(prop: Property) -> property:
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
    __passthrough__: ClassVar[str | None] = None

    __is_struct__: ClassVar[bool] = False
    __is_struct_inlined__: ClassVar[bool] = False
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
    __value_properties__: ClassVar[dict[str, Property]] = {}
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
    _updated_properties: bitarray | None = p_runtime(default=None)

    def __init__(self, **kwargs):
        self._init_pointers()
        if self._session is not UNSET and self._session is None:
            self._session = _active_session.get()
        self._init_self()

    def _init_pointers(self):
        for prop in self.__struct_reference_properties__.values():
            # copy new structs if needed
            if prop.reference_kind == ReferenceKind.STRUCT_CHILD:
                existing = getattr(self, prop.name, None)
                if prop.is_list:
                    assert prop.reference_list_type is not None
                    value_list = prop.reference_list_type(cast(Node, self), prop)
                    self._do_set(
                        prop.name, value_list, track=False
                    )
                    if existing:
                        # will auto copy if needed
                        value_list.extend(existing)
                elif existing is not None:
                    if prop.reference_kind == ReferenceKind.STRUCT_CHILD:
                        self._do_set(prop.name, existing._move_to(self, prop), track=False)

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

    def _equals_content(self, other: Any) -> bool:
        """Checks if all wired properties of the two structs are equal (recursively)."""
        if other is None or self.metatype != other.metatype:
            return False
        for prop in self.__wired_properties__.values():
            if prop.id < 30:
                continue  # ignore identity/tracking
            self_value = getattr(self, prop.name)
            other_value = getattr(other, prop.name)
            if self_value != other_value and (
                prop.py_type_stripped is not float
                or not math.isclose(self_value, other_value, rel_tol=1e-5)
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

    def _do_get(self, item):
        """Called if an attribute doesn't exist in __dict__ or the usual places."""

        # check passthrough (if 'live' in session)
        if self.__passthrough__ is not None and self._session is not None:
            target = getattr(self, self.__passthrough__)
            attr = getattr(target, item, UNSET)
            if attr is not UNSET:
                return attr

        raise AttributeError(item)

    def _do_set(self, key: str, value, *, track: bool = True, validate: bool = True):
        """Sets *any* attribute on this node."""
        session = getattr(self, "_session", None)
        is_tracked = track and session is not None and session is not UNSET
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
                raise AttributeError(f"cannot set computed property: '{key}'")

            # validate/set
            old_value = getattr(self, key, UNSET)
            if (
                is_tracked
                and validate
                and old_value is not UNSET
                and old_value is not getattr(self.__class__, key, UNSET)
            ):
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
                else:
                    pass  # TODO :Broken: handle in struct updates!
            return
        elif is_tracked and self.__passthrough__ is not None:
            # try passthrough target (if any)
            target = getattr(self, self.__passthrough__, UNSET)
            if target is not UNSET:
                setattr(target, key, value)
                return

        raise AttributeError(key)

    if not TYPE_CHECKING:
        # NOTE: __setattr__/__getattr__ confuses type checking, so only define it at runtime
        #  (we don't need it since dynamic access is meant for Values at runtime)
        __getattr__ = _do_get
        __setattr__ = _do_set

    @property
    def _is_tracked(self) -> bool:
        return self._session is not None and self._session is not UNSET

    @property
    def active_session(self) -> "Session":
        """The currently active session (errors if none)"""
        assert self._session is not None, f"no session for {self!r}"
        return self._session

    def _copy(self, **update) -> "Self":
        kwargs = {
            p.name: getattr(self, p.name)
            for p in self.__properties__.values()
            if p.is_runtime and not p.is_ephemeral and not p.is_computed
        }
        kwargs.update(update)
        copy = self.__class__(**kwargs)
        return copy

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
                    yield from cast(InlineStruct, item)._walk_struct()

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
        # check properties types
        for prop in properties or self.__tracked_properties__.values():
            if prop.type_info is not None and prop.reference_source is None:
                value = getattr(self, prop.name)
                check_value(value, prop.type_info, invalid=invalid)
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
        return True  # allow truthy checks for objects

    @classmethod
    def _from_data(cls, data: ObjectDataT) -> Self:
        """Convert from wire format"""
        from bench.proto.wiring import unpack_object

        return unpack_object(data, expect=cls)

    @final
    def _to_data(self) -> ObjectDataT:
        """Convert to wire format"""
        from bench.proto.wiring import pack_object

        return pack_object(self)  # type: ignore

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
class InlineStruct[StructDataT: AnyStructData](BuiltinObject[StructDataT], abc.ABC):
    """A base for structs with properties."""

    metatype: ClassVar[StructType]  # type: ignore

    __is_struct_inlined__: ClassVar[bool] = True
    __is_struct__: ClassVar[bool] = True

    parent: Union["BuiltinObject", "ValueObject", None] = p_struct_parent(3, wire=False)
    if TYPE_CHECKING:
        parent_id: int | None = None
        parent_key: str | None = None

    @final
    def __repr__(self):
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str}>"
        else:
            return f"<{self.__class__.__name__}>"

    def _move_to(
        self,
        parent: Union["BuiltinObject", "ValueObject"],
        prop: Union[Property, "Field"],
        ancestor_prop: Property | None = None,
    ) -> Self:
        """Move or copy this struct into given parent/prop."""
        assert self.__is_struct__, f"cannot copy non-struct {self!r}"  # this is overriden by Node
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
        self, parent: Union["BuiltinObject", "ValueObject"], prop: Union[Property, "Field"]
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


@object_component()
class Struct[StructDataT: AnyStructData](InlineStruct[StructDataT], abc.ABC):
    """A base for structs with properties and a local identity (within a node or some other object)."""

    __is_struct_inlined__: ClassVar[bool] = False
    __is_struct__: ClassVar[bool] = True

    id: int = p_system(2, default_factory=new_struct_id)
    parent: Union["BuiltinObject", "ValueObject", None] = p_struct_parent(3, wire=True)
    order_key: str | None = p_internal(9, default=None)

    @final
    def __repr__(self):  # type: ignore
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str}>"
        else:
            return f"<{self.__class__.__name__} @ {self.id}>"


FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type["Node"]]
EditSubject = Union["User", "Server", "Run"]
EDIT_SUBJECT_TYPES = (NodeType.USER, NodeType.SERVER, NodeType.RUN)


@dataclass(slots=True)
class ReadInfo:
    # NOTE :Architecture: ReadInfo is a clumsy way of passing around epoch/connection_token?
    options: "ReadOptions | None"
    epoch: int | None
    connection_token: str | None
    graph: NodeDataGraph | None = None


def is_implicit_node_property(prop_id: int) -> bool:
    return prop_id < 30 and prop_id != 4  # parent is fine


@node_component()
class Node[NodeDataT: AnyNodeData](BuiltinObject[NodeDataT], abc.ABC):
    """A basic node with properties like a Struct and a global identity in our graph."""

    metatype: ClassVar[NodeType]  # type: ignore

    __is_node__: ClassVar[bool] = True
    __identifier_type__: ClassVar[IdentifierType] = IdentifierType.VARIABLE
    __id_factory__: ClassVar[Callable[[], UUID]] = uuid4
    __ck_factory__: ClassVar[Callable[[], UUID]] = uuid4

    __ancestor_properties__: ClassVar[dict[str, Property]] = {}
    __node_child_properties__: ClassVar[dict[str, Property]] = {}

    __roots__: ClassVar[bittuple[NodeType]] = UNSET
    __is_struct__: ClassVar[bool] = False
    __is_node__: ClassVar[bool] = True
    __is_in_bench__: ClassVar[bool] = UNSET  # part of a Bench
    __is_in_package__: ClassVar[bool] = UNSET  # part of a Package
    __is_stored__: ClassVar[bool] = False  # stored in primary store (runtime or local)
    __is_stored_custom__: ClassVar[bool] = False  # custom storage logic (for records)
    __is_local__: ClassVar[bool] = False  # stored in Bench-local DB (instead of global Bench DB)
    __extra_indexes__: ClassVar[tuple[Index, ...]] = ()  # extra indexes for PG
    __extra_constraints__: ClassVar[tuple[Constraint, ...]] = ()  # extra constraints for PG
    __table__: ClassVar[Table | None] = None  # if stored regularly, set after finalization

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
    set_properties: list[int] = p_regular(29, array=True, store=False)

    # 30+ for 'user' node/struct properties
    # <... defined in concrete type ...>

    _graph: "NodeGraph" = p_runtime(default=None)
    _supergraph: "NodeSuperGraph" = p_runtime(default=None)
    _connection: "GetConnection | SearchConnection" = p_runtime(default=None)
    _is_new: bool = p_runtime(default=False)

    def __init__(self, **kwargs):
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
            now = utcnow()
            self.created_at = now
            self.updated_at = now
        # init graph
        if self.parent is None:
            # if we're not in a graph, start a new one
            #  (not sure which scope/node_types to use here?)
            # NOTE: cleanup NodeGraph definition "depends on itself", causing pyright errors
            graph = NodeGraph(  # type: ignore
                scope=GraphScope(),
                node_types=(self.metatype, *DESCENDANT_NODE_TYPES[self.metatype].tuple),
            )
            graph.add(self)  # type: ignore
            self._graph = graph
        else:
            # we'll be added to the graph by our parent
            self._graph = self.parent._graph

        # init super (builtin object)
        self._init_pointers()

        # init node lists (preserving existing lists)
        for name, prop in self.__node_child_properties__.items():
            assert prop.reference_list_type is not None
            node_list = prop.reference_list_type(self, prop)
            setattr(self, name, node_list)

        # init session context
        if self._session is not UNSET:
            if self._session is None:
                self._session = _active_session.get()
            if self._session is not None:
                if self._is_new:
                    self._validate_self((), invalid=on_invalid_raise)
                self._track_self(self._session)

        # init components
        self._init_self()

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
    def ck(self):
        return self.id

    @property
    def _is_attached(self) -> bool:
        return True

    @property
    def connection(self):
        """The currently active connection (errors if none)"""
        assert self._connection is not None, f"no connection for {self!r}"
        return self._connection

    @property
    def _data_graph(self) -> "NodeDataGraph":
        assert self._connection is not None, f"no connection for {self!r}"
        assert self._connection.result_data is not None, f"no data graph for {self!r}"
        return self._connection.result_data.graph

    def __eq__(self, other: Any):
        """Equals node identity."""
        return type(self) == type(other) and (self.id == other.id or self is other)

    def _stable_hash(self):
        """Hash node identity."""
        return stable_hash((self.metatype, self.id))

    # only allow __hash__ for nodes since their id is constant
    __hash__ = _stable_hash  # type: ignore

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
                    next_parent = cast("Package", next_parent).bench
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

    #
    # Lifecycle
    #

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

    def archive(self):
        """Archive this node."""
        assert not self.is_archived, f"{self!r} is already archived"
        self.active_session.archive(self)

    def unarchive(self):
        """Unarchive this node."""
        assert self.is_archived, f"{self!r} is not archived"
        self.active_session.unarchive(self)

    def delete(self):
        """Soft delete this node."""
        assert not self.is_deleted, f"{self!r} is already deleted"
        self.active_session.delete(self)

    def restore(self):
        """Restore this node from soft deletion."""
        assert self.is_deleted, f"{self!r} is not deleted"
        self.active_session.restore(self)

    def erase(self):
        """Hard delete this node. Forever. Irreversibly."""
        self.active_session.erase(self)

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
    def include_ancestors(cls) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().include_ancestors()

    @classmethod
    def ancestors(cls, *node_types: NodeTypeOrClass) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().ancestors(*node_types)

    @classmethod
    def descendants(cls, *node_types: NodeTypeOrClass) -> "QueryBuilder[Self, NodeDataT]":
        return cls.query().descendants(*node_types)

    #
    # Read
    #

    @classmethod
    async def get(cls, filter: Optional["Expression"] = None, live: bool = False, **kwargs) -> Self:
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

    bench: "Bench" = p_node_ancestor_first(6, NodeType.BENCH, require=True, store=True, wire=True)
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

    package: "Package" = p_node_ancestor_first(
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


# NOTE :Architecture: to get proper branching for Messages/Records/Notifications/...
#  we'll have to swap use ck as primary key (swapping ck/id, but only in spirit, not literally)
#  (same for any local nodes with persistent identity)


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
    set_properties: list[int] = p_regular(29, array=True, store=True)


@node_component()
class HasTimeIdentity(BuiltinObject, abc.ABC):
    """A node whose identity is tied to a specific point in time."""

    __id_factory__: ClassVar[Callable[[], UUID]] = UUIDT
    __ck_factory__: ClassVar[Callable[[], UUID]] = UUIDT


@node_component()
class HasNodeBase(BuiltinObject, abc.ABC):
    """A node which may have a 'base' in another node (e.g., its type definition)."""

    @property
    def base(self) -> Optional[BenchNode]:
        raise NotImplementedError

    @property
    def base_ck(self) -> Optional[UUID]:
        return self.base.ck if self.base is not None else None

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        raise NotImplementedError


def is_node[T: Node](obj: Any, node_cls: type[T]) -> TypeGuard[T]:
    return isinstance(obj, Node) and obj.metatype == node_cls.metatype


def is_struct[T: InlineStruct | Struct](obj: Any, struct_cls: type[T]) -> TypeGuard[T]:
    return isinstance(obj, (InlineStruct, Struct)) and obj.metatype == struct_cls.metatype


# NOTE: import from .value later to avoid circular import
#  (but import at top level to avoid import in critical path)


from bench.language.value import check_value, coerce_value  # noqa: E402

LINK_TARGET_NODE_TYPES: tuple[NodeType, ...] = tuple(
    nt
    for nt in NODE_TYPES
    if NodeType.PACKAGE.id < nt.id < NodeType.SESSION.id and nt != NodeType.LINK
)
LINK_PARENT_NODE_TYPES: tuple[NodeType, ...] = (NodeType.PACKAGE, NodeType.BLOCK)


@node_(NodeType.LINK)
class Link(SourceNode):
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


@node_(NodeType.SKIP, stored=False)
class Skip(Node):
    """A reference to another node in some graph that wasn't available for some reason (usually permissions)."""

    parent: Node = p_node_parent(4, *LINK_PARENT_NODE_TYPES)
    reference: Optional[Node] = p_regular(
        30, array=False, references=LINK_TARGET_NODE_TYPES, require=True
    )
    order_key: Optional[str] = p_internal(31, default=None)

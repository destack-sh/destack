import base64
from collections.abc import Mapping
from datetime import date, datetime, time, timedelta
from itertools import chain
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Collection,
    Iterable,
    Literal,
    NamedTuple,
    Optional,
    Sequence,
    TypeGuard,
    Union,
    cast,
)
from uuid import UUID

import pytz
import regex
import structlog
from google.protobuf.duration_pb2 import Duration
from google.protobuf.json_format import MessageToDict
from google.protobuf.struct_pb2 import NULL_VALUE as PROTO_NULL_VALUE
from google.protobuf.struct_pb2 import ListValue as ProtoList
from google.protobuf.struct_pb2 import Struct as ProtoStruct
from google.protobuf.struct_pb2 import Value as ProtoValue
from google.protobuf.timestamp_pb2 import Timestamp
from opentelemetry import trace

from bench.language.const import (
    FLOAT_EPSILON,
    PY_TYPE_BY_PRIMITIVE_TYPE,
    UNSET,
    EnumType,
    NodeType,
    ObjectKind,
    ObjectType,
    PartialNodeScope,
    PrimitiveType,
    PrimitiveValue,
    StructType,
    TypeKind,
)
from bench.language.graph import NULL_SUPERGRAPH, NodeSuperGraph
from bench.language.property import (
    Property,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.language.registry import (
    BENCH_TYPE_BY_CLASS,
    BUILTIN_OBJECT_CLASS_BY_TYPE,
    ENUM_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
)
from bench.language.validation import NAME_CONSTRAINT, TYPE_CONSTRAINT_BY_FORMAT, on_invalid_raise
from bench.proto.wire import AnyNodeData, AnyStructData, Date, TimeOfDay
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import IdEnum
from bench.utils.time import timedelta_from_isoformat, timedelta_to_isoformat

if TYPE_CHECKING:
    from bench.language import (
        BuiltinObject,
        Call,
        Continue,
        Field,
        Node,
        Session,
        Text,
        TypeBase,
        TypeConstraint,
        TypeConstraintIn,
        TypeInfo,
    )
    from bench.language.validation import ValidationHandler


# pyright: reportIncompatibleVariableOverride=false


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

ScalarValue = Union["CustomObject", PrimitiveValue, "BuiltinObject"]
ScalarValueData = Union[
    AnyNodeData,
    AnyStructData,
    PrimitiveValue,
    dict[str, "ScalarValueData"],
    list["ScalarValueData"],
    ProtoStruct,
    ProtoValue,
    Timestamp,
    Date,
    TimeOfDay,
    Duration,
]
SomeValue = Union[ScalarValue, Collection[ScalarValue], None]
SomeValueData = Union[ScalarValueData, Collection[ScalarValueData], None]
JsonPrimitive = Union[str, int, float, bool, None]
JsonValue = Union[JsonPrimitive, dict[str, "JsonValue"], list["JsonValue"]]

ValueParent = Union["CustomObject", "Struct", "Node"]
ValueParentKey = Union["Property", "Field"]


class CustomObject(Mapping[str, Any]):
    """
    A custom Object; generally the user defined equivalent of our built-in Objects (Structs/Nodes).
    Basic Objects  include values for a specific subset of Fields and the common builtin Object ones
      (e.g., Block variable, Run inputs, Class instance).
    Partial Nodes are a CustomObject + a partial Node, they include the Node, its subtype (if any),
      and values for a specific subset of its value Fields (if any).
      (e.g., CreateStep input, Step inputs generally)
    """

    OWN_PROPERTIES: ClassVar[set[str]] = {
        "_kind",
        "_supergraph",
        "_type",
        "_value",
        "ancestor_prop",
        "parent",
        "parent_id",
        "parent_key",
    }

    def __init__(
        self,
        kind: ObjectKind,
        typ: "TypeBase",
        value: dict[str, SomeValue],
        parent: ValueParent | None = None,
        parent_key: ValueParentKey | None = None,
        supergraph: NodeSuperGraph | None = None,
    ):
        self._kind = kind
        self._type = typ
        self._value = value  # unpacked value
        self.parent = parent
        self.parent_key = parent_key
        self._supergraph = supergraph or typ._supergraph
        assert self._supergraph != NULL_SUPERGRAPH, f"missing supergraph for {self!r}"

    def __str__(self) -> str:
        set_fields: list[str] = []
        for prop in _get_custom_object_properties(self._kind, self._type, self._value):
            if prop.id is None or prop.id < 30:
                continue  # ignore tracking properties
            prop_value = self._do_get(prop)
            if prop_value:
                if type(prop_value) is list:
                    set_fields.append(f"{prop.name}[{len(prop_value)}]")
                elif type(prop_value) is CustomObject:
                    set_fields.append(f"{prop.name}=<{prop_value._type_name} (...)>")
                else:
                    set_fields.append(f"{prop.name}={prop_value!r}")
        for field in self._type._fields:
            field_value = self._do_get(field)
            if field_value:
                if type(field_value) is list:
                    set_fields.append(f"{field.name}[{len(field_value)}]")
                elif type(field_value) is CustomObject:
                    set_fields.append(f"{field.name}=<{field_value._type_name} (...)>")
                else:
                    set_fields.append(f"{field.name}={field_value!r}")
        return ", ".join(set_fields)

    def __repr__(self) -> str:
        return f"<{self._type_name} ({self})>"

    @property
    def _type_name(self) -> str:
        if self._type.kind == TypeKind.PARTIAL_NODE and self._type.bench_type is not None:
            return f"Partial{self._type.bench_type.bench_name}"
        elif self._type.base_type is not None:
            return self._type.base_type.absolute_path
        else:
            return self._type.kind.bench_name

    def equals(self, other: Any) -> bool:
        """Checks if all fields of the two Values are equal (recursively)."""
        if other is None or type(other) is not CustomObject:
            return False
        for prop in _get_custom_object_properties(self._kind, self._type, self._value):
            if self._value.get(prop.key) != other._value.get(prop.key):
                return False
        for field in self._type._base_fields:
            if self._value.get(field.storage_key) != other._value.get(field.storage_key):
                return False
        return True

    __eq__ = equals

    def __bool__(self) -> bool:
        return True  # always True

    def _get_key(self, item: str) -> "Field | Property | None":
        prop = _get_custom_object_property(self._kind, self._type, self._value, item)
        if prop is not None:
            return prop
        else:
            return self._type._get_field(item)

    def __getitem__(self, item: "str | Field | Property") -> SomeValue:
        # NOTE: __getattr__ is called only when ident is not in the slots, so this is a value lookup
        # get field
        if type(item) is str:
            key = self._get_key(item)
            if key is None:
                raise AttributeError(f"{self!r} has no Field or Property with identifier '{item}'")
        else:
            key = item
        return self._do_get(key)  # type: ignore

    __getattr__ = __getitem__

    def _do_get(self, key: "Field | Property") -> SomeValue:
        if type(key) is Property:
            if key.is_value_packed:
                return None
            if key.is_value_runtime:
                assert isinstance(
                    key.value_packed_ptr, Property
                ), f"unexpected {key.value_packed_ptr!r} for {key!r} in {self!r}"
                key = key.value_packed_ptr
        value = self._value.get(key.key)
        if value is None:
            default = key.default
            if default is not UNSET:  # may be unset in Property.default
                return default
            else:
                return None
        elif type(value) is NodeReference:
            # auto resolve references
            resolved_value = self._supergraph.get(value)
            if resolved_value is not None:
                return resolved_value
            else:
                return None  # couldn't resolve
        else:
            return value

    def _do_set(
        self,
        item: "str | Field | Property",
        new_value: SomeValue,
        track: bool = True,
        validate: bool = True,
    ) -> None:
        if type(item) is str and item in self.OWN_PROPERTIES:
            return object.__setattr__(self, item, new_value)

        # figure out key
        if isinstance(item, str):
            key = self._get_key(item)
            if key is None:
                raise AttributeError(f"{self!r} has no Field or Property with identifier '{item}'")
        else:
            key = item

        # check/coerce
        coerce = True
        if isinstance(key, Property):
            assert not key.is_value_packed, f"cannot set {key!r} directly in {self!r}"
            if key.is_value_runtime:
                assert (
                    type(key.value_packed_ptr) is Property
                ), f"unexpected {key.value_packed_ptr!r} for {key!r} in {self!r}"
                key = key.value_packed_ptr
                coerce = False
        if coerce:
            key_typ = key.type_info if isinstance(key, Property) else key
            new_value = coerce_value(
                new_value, key_typ, as_packed=True, parent=self, parent_key=key
            )
            if validate:
                check_value(
                    new_value, key_typ, options=DEFAULT_CHECK_OPTIONS, invalid=on_invalid_raise
                )

        # set/track
        storage_key = key.key
        if track:
            from bench.language.node import _trace_edit_operation

            old_value = self._value.get(storage_key)
            self._value[storage_key] = new_value
            _trace_edit_operation(self, key, new_value=new_value, old_value=old_value, subtype=None)
        else:
            self._value[storage_key] = new_value

    __setitem__ = _do_set

    def __setattr__(self, item: str, value: SomeValue) -> None:
        # NOTE: __setattr__ is also called for slots so we have to bypass those
        if item in self.OWN_PROPERTIES:
            object.__setattr__(self, item, value)
        else:
            self.__setitem__(item, value)

    def __delitem__(self, item: str) -> None:
        # delete field value if it's not required
        field = self._type._get_field(item)
        if field is None:
            raise AttributeError(f"{self!r} has no Field or Property with identifier '{item}'")
        if field.is_required:
            raise AttributeError(f"{field!r} is required")
        if self._value is not None:
            self._value.pop(field.storage_key, None)

    @property
    def fields(self):
        for field in self._type._base_fields:
            if self._type.base_field_type is None or field.type == self._type.base_field_type:
                yield field

    def __iter__(self):
        for field in self._type._base_fields:
            if self._type.base_field_type is None or field.type == self._type.base_field_type:
                yield field.name

    def __len__(self) -> int:
        return len(self._type._base_fields)

    def __contains__(self, item: object) -> bool:
        for field in self._type._base_fields:
            if self._type.base_field_type is not None and field.type != self._type.base_field_type:
                continue
            if field.code_name == item or field.name == item:
                return True
        return False

    def _move_to(self, parent: ValueParent, parent_key: ValueParentKey) -> "CustomObject":
        """Move or copy this object into the given parent/key."""
        if self.parent is None:
            # not yet assigned
            self.parent = parent
            self.parent_key = parent_key
            return self
        else:
            copy = self._copy_to(parent, parent_key)
            return copy

    def _copy_to(
        self,
        parent: ValueParent,
        parent_key: ValueParentKey,
    ) -> "CustomObject":
        """Copy this object into the given parent/prop."""
        value_packed = pack_custom_object(self, self._type)
        copy = unpack_custom_object(
            self._kind,
            value_packed,
            typ=self._type,
            supergraph=self._supergraph,
            parent=parent,
            parent_key=parent_key,
        )
        return copy

    def clone(self) -> "CustomObject":
        """Clones this object."""
        return CustomObject.new(
            self._kind,
            {**self._value} if self._value is not None else None,
            self._type,
            supergraph=self._supergraph,
        )

    def update(
        self,
        values: Mapping["str | Field", Any]
        | Mapping[str, Any]
        | Mapping["Field", Any]
        | None = None,
        **kwargs,
    ):
        """Updates this object with the given value."""
        if values is not None:
            for key, v in values.items():
                self._do_set(key, v, validate=True)
        for key, v in kwargs.items():
            self._do_set(key, v, validate=True)

    @staticmethod
    def new(
        kind: ObjectKind,
        value: dict[str, SomeValue] | None,
        typ: "TypeBase",
        parent: ValueParent | None = None,
        parent_property: ValueParentKey | None = None,
        supergraph: NodeSuperGraph | None = None,
    ) -> "CustomObject":
        """Creates a new Object of the given Object type, coercing the given value."""
        assert (
            typ.kind == TypeKind.CUSTOM_OBJECT or typ.kind == TypeKind.PARTIAL_NODE
        ), f"{typ!r} is not an Object type"
        return CustomObject(
            kind=kind,
            typ=typ,
            value=value if value is not None else {},
            parent=parent,
            parent_key=parent_property,
            supergraph=supergraph,
        )


def make_node_from_partial(partial_node: "CustomObject", **kwargs) -> "Node":
    """Converts the partial Node into a full Node."""
    assert (
        partial_node._kind == ObjectKind.BUILTIN
        and partial_node._type.kind == TypeKind.PARTIAL_NODE
    ), f"cannot convert non-partial {partial_node!r} to Node"

    # figure out node type
    if partial_node._type.bench_type is not None:
        bench_type = cast(NodeType, partial_node._type.bench_type)
    elif "1" in partial_node._value:
        bench_type = cast(NodeType, int(partial_node._value["1"]))  # type: ignore
    else:
        raise ValueError(f"no concrete set node type for {partial_node!r}")
    node_cls = NODE_CLASS_BY_TYPE[bench_type]

    # assemble kwargs
    node_kwargs: dict[str, Any] = {}
    for prop in _get_custom_object_properties(
        partial_node._kind, partial_node._type, partial_node._value
    ):
        prop_value = partial_node._do_get(prop)
        if prop_value is not None:
            node_kwargs[prop.name] = prop_value
    node_kwargs.update(kwargs)  # override with given kwargs

    # assemble value
    value: dict[str, SomeValue] = {}
    for field in partial_node._type._base_fields:
        key = field.storage_key
        field_value = partial_node._value.get(key)
        if field_value is not None:
            value[key] = field_value
    if value:
        value_prop = node_cls.get_value_property(
            cast(ObjectKind | None, partial_node._type.base_field_type), "packed"
        )
        node_kwargs[value_prop.name] = value

    # make node
    node = node_cls(**node_kwargs)
    return node


def patch_node_from_partial(node: "Node", partial_node: "CustomObject"):
    """Applies the set Properties/Fields from the partial Node to the Node."""
    assert (
        partial_node._kind == ObjectKind.BUILTIN
        and partial_node._type.kind == TypeKind.PARTIAL_NODE
    ), f"cannot convert non-partial {partial_node!r} to Node"

    # apply properties
    for prop in _get_custom_object_properties(
        partial_node._kind, partial_node._type, partial_node._value
    ):
        if (
            prop.id is None
            or prop.id < 30
            or prop.is_computed
            or prop.name == "order_key"
            or prop.is_system
        ):
            continue  # ignore internal
        new_prop_value = partial_node._do_get(prop)
        if new_prop_value is not None:
            node._do_set(prop.name, new_prop_value, track=True, validate=False)

    # apply value
    # NOTE :nocheckin: patching partial node value only works for passthrough values
    #  (so this works for Record.value or Message.value but not for Block.variables or Run.inputs
    #    .. should fix this, see make_node_from_partial above)
    for field in partial_node._type._base_fields:
        new_field_value = partial_node._do_get(field)
        if new_field_value is not None:
            node._do_set(field.name, new_field_value, track=True, validate=False)


def _get_custom_object_properties(
    kind: ObjectKind, typ: "TypeBase", value_packed: Mapping[str, JsonValue | SomeValue]
) -> "Iterable[Property]":
    """Gets all the custom object properties available in this value."""
    custom_object_cls = CUSTOM_OBJECT_CLASS_BY_KIND.get(kind)
    custom_properties = (
        custom_object_cls.__original_properties__.values() if custom_object_cls is not None else ()
    )
    if typ.kind == TypeKind.PARTIAL_NODE:
        # figure out actual node type
        if typ.bench_type is not None:
            bench_type = cast(NodeType, typ.bench_type)
            node_cls = NODE_CLASS_BY_TYPE[bench_type]
        elif "1" in value_packed:  # generic partial
            bench_type = cast(NodeType, int(value_packed["1"]))  # type: ignore
            node_cls = NODE_CLASS_BY_TYPE[bench_type]
        else:
            # no known node type, so we can't resolve subtype properties
            node_cls = Node

        # check subtype
        node_properties = node_cls.__original_properties__.values()
        subtype_properties = ()
        if node_cls.__subtype_base_property__ is not None:
            if typ.constraint is not None and typ.constraint.node_subtypes:
                subtype = typ.constraint.node_subtypes[0]
            else:
                subtype = value_packed.get(node_cls.__subtype_base_property__.key)
            if subtype is not None:
                subtype_cls = node_cls.__subclass_by_subtype__.get(cast(IdEnum, subtype))
                if subtype_cls is not None:
                    subtype_properties = subtype_cls.__subtype_extra_original_properties__.values()

        # assemble properties
        if typ.partial_scope == PartialNodeScope.BASE:
            return chain(node_properties, custom_properties)
        elif typ.partial_scope == PartialNodeScope.SUBTYPE:
            return chain(subtype_properties, custom_properties)
        else:
            return chain(subtype_properties, node_properties, custom_properties)
    else:
        # just the base properties
        return custom_properties


def _get_custom_object_property(
    kind: ObjectKind,
    typ: "TypeBase",
    value_packed: Mapping[str, JsonValue | SomeValue],
    name: str,
) -> "Property | None":
    """Gets the property with the given name from the custom object."""
    if typ.kind == TypeKind.PARTIAL_NODE:
        # figure out actual node type
        if typ.bench_type is not None:
            bench_type = cast(NodeType, typ.bench_type)
            node_cls = NODE_CLASS_BY_TYPE[bench_type]
        elif "1" in value_packed:  # generic partial
            bench_type = cast(NodeType, int(value_packed["1"]))  # type: ignore
            node_cls = NODE_CLASS_BY_TYPE[bench_type]
        else:
            node_cls = Node  # just base node properties

        # check node
        prop = node_cls.__original_properties__.get(name)
        if prop is not None:
            return prop
        elif node_cls.__subtype_base_property__ is not None:
            if typ.constraint is not None and typ.constraint.node_subtypes:
                subtype = typ.constraint.node_subtypes[0]
            else:
                subtype = value_packed.get(node_cls.__subtype_base_property__.key)
            if subtype is not None:
                subtype_cls = node_cls.__subclass_by_subtype__[cast(IdEnum, subtype)]
                return subtype_cls.__subtype_extra_original_properties__.get(name)

    custom_object_cls = CUSTOM_OBJECT_CLASS_BY_KIND.get(kind)
    if custom_object_cls is not None:
        return custom_object_cls.__original_properties__.get(name)
    else:
        return None


def _do_get_value_runtime(obj: "Struct | Node", prop: Property):
    """Computes the runtime value for the given property."""
    wired_prop = prop.value_packed_ptr
    assert type(wired_prop) is Property, f"no wired prop for {prop!r}"
    value_packed = getattr(obj, wired_prop.name)
    value_type = prop.value_type_info_getter(obj) if prop.value_type_info_getter else None
    if value_type is not None and (
        value_type.kind == TypeKind.CUSTOM_OBJECT or value_type.kind == TypeKind.PARTIAL_NODE
    ):
        if value_packed is None:
            # init custom objects to empty value object instead of None
            #  (so we can track modifications properly)
            object_kind = prop.value_object_kind
            assert object_kind is not None, f"no object kind for {prop!r}"
            value_packed = {}
            obj._do_set(wired_prop.name, value_packed, track=False, validate=False)
            value = CustomObject(
                object_kind,
                value_type,
                value_packed,
                parent=obj,
                parent_key=wired_prop,
                supergraph=obj._supergraph,
            )
        else:
            value = unpack_value(
                value_packed,
                value_type,
                parent=obj,
                parent_key=wired_prop,
                wrap_scalar=False,
                supergraph=obj._supergraph,
            )
    elif (
        value_packed is None
        or (prop.is_list and len(value_packed) == 0 and not prop.is_required)
        or value_type is None
    ):
        value = None
    else:
        value = unpack_value(
            value_packed,
            value_type,
            parent=obj,
            parent_key=wired_prop,
            wrap_scalar=True,
            supergraph=obj._supergraph,
        )
    return value_type, value


def _object_value_runtime(prop: Property) -> property:
    """The computed get/set property for a runtime value property."""

    assert not prop.is_list, f"runtime value cannot be list {prop!r}"

    cache_key = f"_{prop.name}_cached"

    def _get_value_runtime(self: "Struct | Node") -> Optional[SomeValue]:
        """Gets the runtime value for this property (with auto resolving)."""
        if cache_key in self.__dict__:
            value_type = prop.value_type_info_getter(self) if prop.value_type_info_getter else None
            value = self.__dict__[cache_key]
        else:
            value_type, value = _do_get_value_runtime(self, prop)
            self.__dict__[cache_key] = value

        # imitate CustomObject._do_get
        if (
            value is None
            and value_type is not None
            and self is not value_type
            and value_type.default_packed
        ):
            return value_type.default
        elif isinstance(value, NodeReference):
            # auto resolve references
            resolved_value = self._supergraph.get(value)
            if resolved_value is not None:
                return resolved_value
            else:
                return None  # couldn't resolve
        else:
            return value

    def _set_value_runtime(self: "Struct | Node", value: SomeValue):
        wired_prop = prop.value_packed_ptr
        assert type(wired_prop) is Property, f"no wired prop for {prop!r}"
        value_type = prop.value_type_info_getter(self) if prop.value_type_info_getter else None
        if value is not None and value_type is not None:
            if type(value) is CustomObject:
                value = value._move_to(self, wired_prop)
            elif isinstance(value, list) and value and type(value[0]) is CustomObject:
                value = [v._move_to(self, wired_prop) for v in value]  # type: ignore
            value_packed = pack_value(value, value_type, wrap_scalar=True)
            self._do_set(wired_prop.name, value_packed, track=False)
        else:
            self._do_set(wired_prop.name, {} if wired_prop.is_required else None, track=False)
        # NOTE :Robustness: is it correct to cache runtime value immediately on set?
        self.__dict__[cache_key] = value

    return property(_get_value_runtime, _set_value_runtime)


#
# Value coercion
#


def _coerce_value_scalar(
    value: ScalarValue,
    typ: "TypeBase",
    as_packed: bool = False,
    *,
    parent: ValueParent | None = None,
    parent_key: ValueParentKey | None = None,
    supergraph: NodeSuperGraph | None = None,
) -> ScalarValue:
    """Coerces a scalar value (primitive, node, struct)"""
    try:
        if typ.kind == TypeKind.PRIMITIVE:
            assert typ.primitive_type is not None, f"missing primitive type for {typ!r}"
            if typ.primitive_type.is_numeric:
                if typ.primitive_type.is_float:
                    value = float(cast(Any, value))
                elif typ.primitive_type.is_int:
                    value = int(cast(Any, value))
        elif typ.kind == TypeKind.STRUCT and parent is not None and isinstance(value, Struct):
            assert parent_key is not None, f"{typ!r} got parent {parent!r} but no parent_prop"
            value = cast("Struct", value)._move_to(parent, parent_key)
        elif as_packed and (typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE):
            assert isinstance(value, (Node, NodeReference)), f"expected Node, got {value!r}"
            value = value.to_ref()
        return value
    except (AssertionError, AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not coerce {value!r} ({type(value)}) as {typ!r}") from e


def coerce_custom_object_scalar(
    kind: ObjectKind,
    value: Mapping[str, Any] | CustomObject | None,
    typ: "TypeBase",
    as_packed: bool = False,
    *,
    parent: ValueParent | None = None,
    parent_prop: ValueParentKey | None = None,
    supergraph: NodeSuperGraph | None = None,
) -> CustomObject:
    """Coerces a single object from its dict representation or existing CustomObject."""
    if type(value) is CustomObject:
        # NOTE :Robustness: not sure if _coerce_object_scalar is correct if given an existing object
        if parent is not None:
            assert parent_prop is not None, f"{typ!r} got parent {parent!r} but no parent_prop"
            return value._move_to(parent, parent_prop)
        else:
            return value
    else:
        if value is None:
            value = {}
        if not isinstance(value, Mapping):
            raise TypeError(f"{value!r} is a {type(value).__name__}, expected {typ!r}")

    # coerce
    obj = CustomObject.new(
        kind=kind,
        value={},
        typ=typ,
        parent=parent,
        parent_property=parent_prop,
        supergraph=supergraph,
    )
    properties = _get_custom_object_properties(kind, typ, value)
    for prop in properties:
        prop_value = value.get(prop.name)
        if prop_value is not None:
            obj._do_set(prop, prop_value, track=False)
    for field in typ._base_fields:
        # try getting value by storage key, name and ident
        field_value = value.get(field.storage_key)
        if field_value is None:
            field_value = value.get(field.name)
        if field_value is None:
            code_name = field.code_name
            if code_name is not None:
                field_value = value.get(code_name)
        if field_value is not None:
            obj._do_set(field, field_value, track=False)

    return obj


def coerce_value(
    value: Any,
    typ: "TypeBase",
    as_packed: bool = False,
    *,
    parent: ValueParent | None = None,
    parent_key: ValueParentKey | None = None,
    supergraph: NodeSuperGraph | None = None,
) -> SomeValue:
    """
    Coerces the given value to the expected type (recursively).
    Returns value as is if already of correct type, raises TypeError if coercion is not possible.
    NOTE :Performance: we re-create and copy lists during coercion even if the type was already good
    """
    if typ.kind == TypeKind.CUSTOM_OBJECT or typ.kind == TypeKind.PARTIAL_NODE:
        assert typ.base_field_type is not None, f"missing base field type for {typ!r}"
        object_kind = ObjectKind(typ.base_field_type)
        if not typ.is_list:
            return coerce_custom_object_scalar(
                object_kind,
                cast(dict, value),
                typ,
                as_packed=as_packed,
                parent=parent,
                parent_prop=parent_key,
                supergraph=supergraph,
            )
        else:
            if not isinstance(value, Sequence):
                raise TypeError(
                    f"{value!r} ({type(value).__name__}) is not a sequence, expected {typ!r}"
                )
            return [
                coerce_custom_object_scalar(
                    object_kind,
                    cast(dict, element),
                    typ,
                    as_packed=as_packed,
                    parent=parent,
                    parent_prop=parent_key,
                    supergraph=supergraph,
                )
                for element in value
            ]
    else:
        if value is None:
            return None
        elif not typ.is_list:
            return _coerce_value_scalar(
                value,
                typ,
                as_packed=as_packed,
                parent=parent,
                parent_key=parent_key,
                supergraph=supergraph,
            )
        else:
            if not isinstance(value, Sequence):
                raise TypeError(
                    f"{value!r} ({type(value).__name__}) is not a sequence, expected {typ!r}"
                )
            return [
                _coerce_value_scalar(
                    element, typ, as_packed=as_packed, parent=parent, parent_key=parent_key
                )
                for element in value
            ]


#
# Type checking :TypeChecking
#

# bounds checking
MIN_VALUE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, Any] = {
    PrimitiveType.INT16: -(2**15),
    PrimitiveType.INT32: -(2**31),
    PrimitiveType.INT64: -(2**63),
    PrimitiveType.FLOAT32: -3.4028235e38,
    PrimitiveType.FLOAT64: -1.7976931348623157e308,
    PrimitiveType.INTERVAL: timedelta(days=-(1000 * 365)),
}
MAX_VALUE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, Any] = {
    PrimitiveType.INT16: 2**15 - 1,
    PrimitiveType.INT32: 2**31 - 1,
    PrimitiveType.INT64: 2**63 - 1,
    PrimitiveType.FLOAT32: 3.4028235e38,
    PrimitiveType.FLOAT64: 1.7976931348623157e308,
    PrimitiveType.INTERVAL: timedelta(days=(1000 * 365)),
}


class CheckOptions(NamedTuple):
    """Options for checking a value."""

    detached_is: Literal["none", "invalid"] = "none"


DEFAULT_CHECK_OPTIONS = CheckOptions()


def check_value_scalar_constraint(
    value: SomeValue,
    typ: "TypeBase",
    constraint: "TypeConstraint | TypeConstraintIn",
    *,
    options: CheckOptions,
    invalid: "ValidationHandler",
) -> None:
    """Checks whether the given value satisfies the given scalar constraint."""  # :TypeChecking
    if type(value) is int or type(value) is float:
        if constraint.min_value is not None and value < constraint.min_value:
            invalid(value, "too small", typ)
        if constraint.max_value is not None and value > constraint.max_value:
            invalid(value, "too large", typ)
        if constraint.step_value is not None:
            delta = abs(value % constraint.step_value)
            if delta > FLOAT_EPSILON and abs(delta - constraint.step_value) > FLOAT_EPSILON:
                invalid(value, f"not a multiple of {constraint.step_value}", typ)
    elif type(value) is str:
        if constraint.min_length is not None and len(value) < constraint.min_length:
            invalid(value, "too short", typ)
        if constraint.max_length is not None and len(value) > constraint.max_length:
            invalid(value, "too long", typ)
        if constraint.regex is not None and not regex.match(constraint.regex, value):
            invalid(value, "does not match regex", typ)
        if constraint.starts_with is not None and not value.startswith(constraint.starts_with):
            invalid(value, f"does not start with {constraint.starts_with}", typ)
        if constraint.ends_with is not None and not value.endswith(constraint.ends_with):
            invalid(value, f"does not end with {constraint.ends_with}", typ)


def check_value_scalar(
    value: SomeValue, typ: "TypeBase", *, options: CheckOptions, invalid: "ValidationHandler"
) -> None:
    """Checks whether the given scalar value has the expected type."""  # :TypeChecking
    if typ.kind == TypeKind.PRIMITIVE:
        expected_type = PY_TYPE_BY_PRIMITIVE_TYPE.get(cast(PrimitiveType, typ.primitive_type))
        if expected_type is None:
            return  # nothing to check?
        elif type(value) is not expected_type and not isinstance(value, expected_type):
            invalid(value, f"not of type, is '{type(value).__name__}'", typ)
            return  # also nothing to do
        # check constraint
        if typ.constraint is not None:
            check_value_scalar_constraint(
                value, typ, typ.constraint, options=options, invalid=invalid
            )
        if typ.format is not None:
            check_value_scalar_constraint(
                value, typ, TYPE_CONSTRAINT_BY_FORMAT[typ.format], options=options, invalid=invalid
            )
        # strings cannot be empty (because of protobuf we must disambiguate unset from empty)
        if type(value) is str and len(value) == 0:
            invalid(value, "empty string", typ)
        # check bounds
        min_value = MIN_VALUE_BY_PRIMITIVE_TYPE.get(cast(PrimitiveType, typ.primitive_type))
        max_value = MAX_VALUE_BY_PRIMITIVE_TYPE.get(cast(PrimitiveType, typ.primitive_type))
        if min_value is not None and value < min_value:
            invalid(value, "too small", typ)
        if max_value is not None and value > max_value:
            invalid(value, "too large", typ)
    elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
        from bench.language.node import Node, NodeReference

        if typ.bench_type is not None:
            allowed_types = (typ.bench_type,)
        elif typ.constraint is not None and typ.constraint.node_types:
            allowed_types = typ.constraint.node_types
        else:
            allowed_types = None

        if isinstance(value, Node):
            if options.detached_is == "invalid" and not value.is_attached:
                invalid(value, "detached node", typ)
            elif allowed_types and value.metatype not in allowed_types:
                invalid(value, f"not of type, is {value.metatype}", typ)
        elif isinstance(value, NodeReference):
            if allowed_types and value.node_type not in allowed_types:
                invalid(value, f"not of type, is {value.node_type}", typ)
        else:
            invalid(value, "not a Node", typ)
    elif typ.kind == TypeKind.STRUCT:
        if typ.bench_type == StructType.PROPERTY_REFERENCE:
            if not isinstance(value, Property):
                invalid(value, "not a Property", typ)
        else:
            if not getattr(cast("Struct", value), "__is_struct__", False):
                invalid(value, "not a Struct", typ)
            elif cast("Struct", value).metatype != typ.bench_type:
                invalid(value, f"not of type, is {cast('Struct', value).metatype}", typ)
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        try:
            enum_cls(cast(int, value))
        except ValueError:
            invalid(value, f"not a valid {enum_cls}", typ)
    else:
        raise RuntimeError(f"unexpected type {typ!r}")


def _check_is_list(
    value: SomeValue, typ: "TypeBase", *, options: CheckOptions, invalid: "ValidationHandler"
) -> TypeGuard[list]:
    """Checks whether the given value is a list of the expected dimensions."""
    if not isinstance(value, (list, tuple)):
        invalid(value, "not a list", typ)
        return False
    if typ.constraint is not None:
        if typ.constraint.min_length is not None and len(value) < typ.constraint.min_length:
            invalid(value, "too short", typ)
        if typ.constraint.max_length is not None and len(value) > typ.constraint.max_length:
            invalid(value, "too long", typ)
    return True


def _check_is_object(
    value: SomeValue, typ: "TypeBase", *, options: CheckOptions, invalid: "ValidationHandler"
) -> TypeGuard[CustomObject]:
    if not isinstance(value, CustomObject):
        invalid(value, "not an object", typ)
        return False
    return True


def check_custom_object_scalar(
    value: SomeValue, typ: "TypeBase", *, options: CheckOptions, invalid: "ValidationHandler"
) -> None:
    """Checks whether the given object value has the expected type (recursively)."""
    if _check_is_object(value, typ, options=options, invalid=invalid):
        for field in typ._fields:
            field_value = value._do_get(field)
            check_value(field_value, field, options=options, invalid=invalid)


def check_value(
    value: Any, typ: "TypeBase", *, options: CheckOptions, invalid: "ValidationHandler"
) -> None:
    """
    Checks whether the given value has the expected type (recursively).
    """
    if value is None:
        if typ.is_required:
            invalid(value, "missing required value", typ)
        else:
            return
    if typ.kind == TypeKind.CUSTOM_OBJECT or typ.kind == TypeKind.PARTIAL_NODE:
        if not typ.is_list:
            check_custom_object_scalar(value, typ, options=options, invalid=invalid)
        elif _check_is_list(value, typ, options=options, invalid=invalid):
            for element in value:
                check_custom_object_scalar(element, typ, options=options, invalid=invalid)
    else:
        if not typ.is_list:
            check_value_scalar(value, typ, options=options, invalid=invalid)
        elif _check_is_list(value, typ, options=options, invalid=invalid):
            for element in value:
                check_value_scalar(element, typ, options=options, invalid=invalid)


#
# Value sampling
# NOTE :Incomplete: we'll probably want value sampling as a more general feature
#  (also, value sampling is suspiciously similar to the strategy-based sampling we do
#   during testing? It's *just* different enough as this is sparse, low-volume & user-facing)
#

SAMPLE_VALUE_BY_PROPERTY: dict[str, SomeValue] = {
    "emoji": "😀",
    "order_key": INTEGER_ZERO,
    "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
}


def sample_scalar_value(typ: "TypeBase") -> ScalarValue | None:
    """Samples a representative (not necessarily random) scalar value for the given type."""
    if typ.kind == TypeKind.PRIMITIVE:
        assert typ.primitive_type is not None, f"missing primitive type for {typ!r}"
        if typ.primitive_type == PrimitiveType.BOOLEAN:
            return True
        elif typ.primitive_type.is_numeric:
            primitive_cls = PY_TYPE_BY_PRIMITIVE_TYPE[cast(PrimitiveType, typ.primitive_type)]
            # sample with constraint
            if typ.constraint is not None:
                if typ.constraint.min_value is not None:
                    return primitive_cls(typ.constraint.min_value)
                if typ.constraint.max_value is not None:
                    return primitive_cls(typ.constraint.max_value)
                if typ.constraint.step_value is not None:
                    return primitive_cls(typ.constraint.step_value)
            # sample without constraint
            if typ.primitive_type.is_float:
                return 17.0
            elif typ.primitive_type.is_int:
                return 42
        elif typ.primitive_type == PrimitiveType.STRING:
            return "string"
        elif typ.primitive_type == PrimitiveType.BYTES:
            return b"bytes"
        elif typ.primitive_type == PrimitiveType.UUID:
            return UUID("00000000-0000-0000-0000-000000000000")
        elif typ.primitive_type == PrimitiveType.JSON:
            return None
        elif typ.primitive_type == PrimitiveType.DATETIME:
            return datetime(2024, 6, 12, tzinfo=pytz.utc)
        elif typ.primitive_type == PrimitiveType.DATE:
            return date(2024, 6, 12)
        elif typ.primitive_type == PrimitiveType.TIME:
            return time(12, 34, 56)
        elif typ.primitive_type == PrimitiveType.INTERVAL:
            return timedelta(seconds=42)
        else:
            raise RuntimeError(f"unexpected primitive type {typ.primitive_type!r}")
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        for enum_value in enum_cls:
            return enum_value
        else:
            return None  # no enum values?
    elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
        if typ.bench_type == NodeType.FIELD:
            assert typ.base_type is not None, f"missing base type for {typ!r}"
            if len(typ.base_type.fields) > 0:
                return typ.base_type.fields[0]
        return None  # TODO :Incomplete: :SampleNodeValues
    elif typ.kind == TypeKind.STRUCT:
        return sample_builtin_object_scalar(typ)
    else:
        raise RuntimeError(f"unexpected type {typ!r}")


def sample_builtin_object_scalar(typ: "TypeBase") -> "BuiltinObject":
    """Samples a representative object value for the given type (recursively)."""
    builtin_object_cls = BUILTIN_OBJECT_CLASS_BY_TYPE.get(cast(ObjectType, typ.bench_type))
    assert builtin_object_cls is not None, f"missing object class for {typ!r}"
    object_kwargs = {}
    for prop in builtin_object_cls.__runtime_properties__.values():
        if (
            # ignore runtime-only properties
            prop.id is None
            # ignore identity/tracking properties
            or (prop.id < 30 and prop.reference_kind is not None)
            # ignore contributed wired properties (they're derived from the generated one)
            or (prop.reference_source is not None)
            # ignore autoset properties (ids, timestamps)
            or prop.is_autoset
        ):
            continue  # :IgnoredGeneratedProperties
        elif prop.name in SAMPLE_VALUE_BY_PROPERTY:
            prop_value = SAMPLE_VALUE_BY_PROPERTY[prop.name]
            object_kwargs[prop.name] = [prop_value] if prop.is_list else prop_value
        elif prop.is_node_reference:
            ...  # TODO :Incomplete: :SampleNodeValues
        elif prop.is_struct_reference and not prop.is_required:
            object_kwargs[prop.name] = [] if prop.is_list else None  # don't recurse
        elif prop.is_property_reference:
            object_kwargs[prop.name] = [prop] if prop.is_list else prop
        elif prop.reference_is_node_data or prop.is_value_runtime or prop.is_value_packed:
            object_kwargs[prop.name] = None
        else:
            object_kwargs[prop.name] = sample_value(prop.type_info)
    if typ.bench_type == StructType.TYPE_INFO:
        # many types don't support lists, so just make it a scalar
        object_kwargs["is_list"] = False
    return builtin_object_cls(**object_kwargs)


def sample_custom_object_scalar(
    kind: ObjectKind, typ: "TypeBase", recurse_objects: bool = True
) -> CustomObject:
    """Samples a representative object value for the given type (recursively)."""
    assert (
        typ.kind == TypeKind.CUSTOM_OBJECT or typ.kind == TypeKind.PARTIAL_NODE
    ), f"expected object type, got {typ!r}"
    value = {}
    for field in typ._base_fields:
        if field.is_list:
            value[field.storage_key] = [sample_value(field, recurse_objects) for _ in range(1)]
        else:
            value[field.storage_key] = sample_value(field, recurse_objects)
    return CustomObject.new(kind, value, typ)


@tracer.start_as_current_span(name="value.sample")
def sample_value(typ: "TypeBase", recurse_objects: bool = True) -> SomeValue:
    """Samples a representative value for the given type (recursively)."""
    if typ.kind == TypeKind.CUSTOM_OBJECT or typ.kind == TypeKind.PARTIAL_NODE:
        if typ.base_field_type is not None:
            object_kind = ObjectKind(typ.base_field_type)
        else:
            object_kind = ObjectKind.BUILTIN
        if not typ.is_list:
            return sample_custom_object_scalar(object_kind, typ, recurse_objects)
        else:
            return [
                sample_custom_object_scalar(object_kind, typ, recurse_objects) for _ in range(2)
            ]
    else:
        if not typ.is_list:
            return sample_scalar_value(typ)
        else:
            scalar_values = []
            for _ in range(1):
                scalar_value = sample_scalar_value(typ)
                if scalar_value is not None:
                    scalar_values.append(scalar_value)
            return scalar_values


#
# Value packing
#


def pack_value_scalar(value: ScalarValue | ScalarValueData, typ: "TypeBase") -> JsonValue:
    """
    Packs the given scalar runtime or data value into a JSON-able representation.
    """
    if typ.kind == TypeKind.PRIMITIVE:
        if typ.primitive_type == PrimitiveType.BYTES:
            return base64.b64encode(cast(bytes, value)).decode()
        elif typ.primitive_type == PrimitiveType.UUID:
            return str(cast(UUID, value))
        elif typ.primitive_type == PrimitiveType.JSON:
            if type(value) is ProtoValue:
                return cast(JsonValue, unpack_proto_json_struct(value))
            elif type(value) is ProtoStruct:
                return cast(JsonValue, MessageToDict(value))
            else:
                return cast(JsonValue, value)
        elif typ.primitive_type == PrimitiveType.DATETIME:
            if type(value) is Timestamp:
                return value.ToDatetime(tzinfo=pytz.utc).isoformat()
            else:
                return cast(datetime, value).isoformat()
        elif typ.primitive_type == PrimitiveType.DATE:
            if type(value) is Date:
                return unpack_proto_date(value).isoformat()
            else:
                return cast(date, value).isoformat()
        elif typ.primitive_type == PrimitiveType.TIME:
            if type(value) is TimeOfDay:
                return unpack_proto_time(value).isoformat()
            else:
                return cast(time, value).isoformat()
        elif typ.primitive_type == PrimitiveType.INTERVAL:
            if type(value) is Duration:
                return timedelta_to_isoformat(value.ToTimedelta())
            else:
                return timedelta_to_isoformat(cast(timedelta, value))
        else:
            return cast(JsonValue, value)
    elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
        if cast("Struct | AnyStructData", value).metatype != StructType.NODE_REFERENCE:
            ref = cast("Node", value).to_ref()
        else:
            ref = cast("NodeReference | NodeReferenceData", value)
        if isinstance(ref, BuiltinObject):
            ref = ref._to_data()
        return pack_builtin_object_data(ref)
    elif typ.kind == TypeKind.ENUM:
        return int(cast(Any, value))
    elif typ.kind == TypeKind.STRUCT:
        if isinstance(value, BuiltinObject):
            return pack_builtin_object(value)
        else:
            assert hasattr(
                value, "metatype"
            ), f"unexpected value {value!r} ({type(value).__name__ }) for {typ!r}"
            return pack_builtin_object_data(cast(AnyStructData | AnyNodeData, value))
    else:
        raise TypeError(f"cannot pack value of type {typ!r}")


def unpack_value_scalar(
    value_packed: JsonValue, typ: "TypeBase", *, supergraph: NodeSuperGraph | None
) -> ScalarValue:
    """
    Unpacks the given scalar value into its runtime representation.
    """
    if typ.kind == TypeKind.PRIMITIVE:
        if typ.primitive_type == PrimitiveType.BYTES:
            return base64.b64decode(cast(str, value_packed))
        elif typ.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return int(cast(int, value_packed))
        elif typ.primitive_type == PrimitiveType.UUID:
            return UUID(cast(str, value_packed))
        elif typ.primitive_type == PrimitiveType.DATETIME:
            return datetime.fromisoformat(cast(str, value_packed))
        elif typ.primitive_type == PrimitiveType.DATE:
            return date.fromisoformat(cast(str, value_packed))
        elif typ.primitive_type == PrimitiveType.TIME:
            return time.fromisoformat(cast(str, value_packed))
        elif typ.primitive_type == PrimitiveType.INTERVAL:
            return timedelta_from_isoformat(cast(str, value_packed))
        else:
            return cast(PrimitiveValue, value_packed)
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        return enum_cls(cast(int, value_packed))
    elif typ.kind in (TypeKind.NODE, TypeKind.BASED_NODE, TypeKind.STRUCT):
        assert isinstance(value_packed, dict), f"{value_packed!r} is not a dict, expected {typ!r}"
        return unpack_builtin_object(value_packed, supergraph=supergraph)
    else:
        raise TypeError(f"cannot unpack value of type {typ!r}")


def unpack_value_scalar_data(value_packed: JsonValue, typ: "TypeBase") -> ScalarValueData:
    """
    Unpacks the given scalar value into its proto data representation. See above.
    """
    if typ.kind == TypeKind.PRIMITIVE:
        if typ.primitive_type == PrimitiveType.BYTES:
            return base64.b64decode(cast(str, value_packed))
        elif typ.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return int(cast(int, value_packed))
        elif typ.primitive_type == PrimitiveType.UUID:
            return cast(str, value_packed)  # leave as string
        elif typ.primitive_type == PrimitiveType.JSON:
            return pack_proto_json(cast(Any, value_packed))
        elif typ.primitive_type == PrimitiveType.DATETIME:
            ts = Timestamp()
            ts.FromDatetime(datetime.fromisoformat(cast(str, value_packed)))
            return ts
        elif typ.primitive_type == PrimitiveType.DATE:
            dt = date.fromisoformat(cast(str, value_packed))
            return pack_proto_date(dt)
        elif typ.primitive_type == PrimitiveType.TIME:
            dt = time.fromisoformat(cast(str, value_packed))
            return pack_proto_time(dt)
        elif typ.primitive_type == PrimitiveType.INTERVAL:
            dur = Duration()
            dur.FromTimedelta(timedelta_from_isoformat(cast(str, value_packed)))
            return dur
        else:
            return cast(PrimitiveValue, value_packed)
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        return enum_cls(cast(int, value_packed))
    elif typ.kind in (TypeKind.NODE, TypeKind.BASED_NODE, TypeKind.STRUCT):
        assert isinstance(value_packed, dict), f"{value_packed!r} is not a dict, expected {typ!r}"
        return unpack_builtin_object_data(value_packed)
    else:
        raise TypeError(f"cannot unpack value of type {typ!r}")


def pack_builtin_object(
    value: "BuiltinObject", only: Collection[Property] | None = None
) -> dict[str, JsonValue]:
    """Packs a BuiltinObject into a JSON representation."""
    value_packed: dict[str, JsonValue] = {}
    object_cls = BUILTIN_OBJECT_CLASS_BY_TYPE[value.metatype]
    for prop in only if only is not None else object_cls.__wired_properties__.values():
        if prop.reference_wired_ptr is not None:
            prop = prop.reference_wired_ptr
        prop_name = prop.name
        prop_value = getattr(value, prop_name)
        if prop_value is None or (prop.is_list and len(prop_value) == 0):
            continue
        elif prop.is_list:
            prop_type = prop.type_info
            prop_value_packed = [pack_value_scalar(e, prop_type) for e in prop_value]
        else:
            prop_value_packed = pack_value_scalar(prop_value, prop.type_info)
        value_packed[prop.key] = prop_value_packed
    return value_packed


def unpack_builtin_object[T: BuiltinObject = BuiltinObject](
    value_packed: dict[str, Any],
    *,
    supergraph: NodeSuperGraph | None,
    expect: type[T] | None = None,
    session: "Session | None" = None,
) -> T:
    """Unpacks a BuiltinObject from a JSON representation."""

    if expect is None:
        object_type = value_packed.get("1")
        assert object_type is not None, f"{value_packed!r} has no object type and none given"
        object_type = cast(ObjectType, int(object_type))  # type: ignore
    else:
        object_type = BENCH_TYPE_BY_CLASS[expect]
    object_cls = BUILTIN_OBJECT_CLASS_BY_TYPE[cast(ObjectType, object_type)]
    assert not object_cls.__is_node__, f"cannot unpack {object_cls.__name__} from value"

    object_kwargs = {}
    for prop in object_cls.__wired_properties__.values():
        if prop.reference_wired_ptr is not None:
            prop = prop.reference_wired_ptr
        prop_value_packed = value_packed.get(prop.key)
        if prop_value_packed is None:
            continue
        elif prop.is_list:
            prop_type = prop.type_info
            object_kwargs[prop.name] = [
                unpack_value_scalar(e, prop_type, supergraph=supergraph) for e in prop_value_packed
            ]
        else:
            object_kwargs[prop.name] = unpack_value_scalar(
                prop_value_packed, prop.type_info, supergraph=supergraph
            )
    if session is not None:
        object_kwargs["_session"] = session

    obj = object_cls(**object_kwargs)
    return cast(T, obj)


def pack_builtin_object_data(
    value: AnyStructData | AnyNodeData,
    only: Collection[Property] | None = None,
) -> dict[str, JsonValue]:
    """Packs a single struct/node data value into a JSON representation."""
    value_packed: dict[str, JsonValue] = {}
    builtin_object_cls = BUILTIN_OBJECT_CLASS_BY_TYPE[value.metatype]  # type: ignore
    for prop in only if only is not None else builtin_object_cls.__wired_properties__.values():
        if prop.reference_wired_ptr is not None:
            prop = prop.reference_wired_ptr
        prop_name = prop.name
        if prop.is_optional_scalar and not value.HasField(prop_name):
            continue
        prop_value = getattr(value, prop_name)
        if prop_value is None or (prop.is_list and len(prop_value) == 0):
            continue
        elif prop.is_list:
            prop_type = prop.type_info
            prop_value_packed = [pack_value_scalar(element, prop_type) for element in prop_value]
        else:
            prop_value_packed = pack_value_scalar(prop_value, prop.type_info)
        value_packed[prop.key] = prop_value_packed
    return value_packed


def unpack_builtin_object_data[T: AnyStructData | AnyNodeData](
    value_packed: dict[str, Any],
    expect: type[T] | None = None,
    into: T | None = None,
) -> AnyStructData | AnyNodeData:
    """Unpacks a single struct/node data value from a JSON representation."""
    from bench.proto import wiring

    if expect is None:
        object_type = value_packed.get("1")
        assert object_type is not None, f"{value_packed!r} has no object type and none given"
        object_type = cast(ObjectType, int(object_type))
    else:
        object_type = wiring.OBJECT_TYPE_BY_PROTO_CLASS[expect]
    object_cls = BUILTIN_OBJECT_CLASS_BY_TYPE[object_type]
    proto_cls = wiring.PROTO_CLASS_BY_TYPE[object_type]

    value = into if into is not None else proto_cls(metatype=object_type)  # type: ignore
    for prop in object_cls.__wired_properties__.values():
        if prop.reference_wired_ptr is not None:
            prop = prop.reference_wired_ptr
        prop_value_packed = value_packed.get(prop.key)
        if prop_value_packed is None or (prop.is_list and len(prop_value_packed) == 0):
            continue
        elif prop.is_list:
            prop_type = prop.type_info
            prop_value = [
                unpack_value_scalar_data(element, prop_type) for element in prop_value_packed
            ]
        else:
            prop_value = unpack_value_scalar_data(prop_value_packed, prop.type_info)
        wiring.set_builtin_object_prop(value, prop, prop_value)
    return value


def pack_custom_object(value: CustomObject, typ: "TypeBase") -> dict[str, JsonValue]:
    """
    Packs an object value into a JSON representation.
    """
    value_packed: dict[str, JsonValue] = {}
    _value = value._value if isinstance(value, CustomObject) else value
    if not _value:
        return value_packed  # empty value

    # fields
    for field in typ._base_fields:
        storage_key = field.storage_key
        field_value = cast(SomeValue, _value.get(storage_key))
        if field_value is None:
            continue
        elif field.kind == TypeKind.CUSTOM_OBJECT or field.kind == TypeKind.PARTIAL_NODE:
            value_packed[storage_key] = pack_value(field_value, field, wrap_scalar=False)
        elif not field.is_list:
            value_packed[storage_key] = pack_value_scalar(cast(ScalarValue, field_value), field)
        else:  # scalar list
            assert isinstance(
                field_value, list
            ), f"{field_value!r} is not a list, expected {field!r}"
            value_packed[storage_key] = [
                pack_value_scalar(element, field) for element in field_value
            ]

    # properties
    for prop in _get_custom_object_properties(value._kind, typ, _value):
        storage_key = prop.key
        prop_value = cast(SomeValue, _value.get(storage_key))
        if prop_value is None:
            continue
        elif prop.is_value_packed:
            assert (
                type(prop_value) is CustomObject
            ), f"unexpected {prop_value!r} for {prop!r} in {value!r}"
            value_packed[storage_key] = pack_custom_object(prop_value, prop_value._type)
        elif not prop.is_list:
            value_packed[storage_key] = pack_value_scalar(
                cast(ScalarValue, prop_value), prop.type_info
            )
        else:  # scalar list
            assert isinstance(prop_value, list), f"{prop_value!r} is not a list, expected {prop!r}"
            prop_typ = prop.type_info
            value_packed[storage_key] = [
                pack_value_scalar(element, prop_typ) for element in prop_value
            ]

    return value_packed


def unpack_custom_object(
    kind: ObjectKind,
    value_packed: dict[str, JsonValue],
    typ: "TypeBase",
    *,
    parent: ValueParent | None = None,
    parent_key: ValueParentKey | None = None,
    supergraph: NodeSuperGraph | None,
) -> CustomObject:
    """
    Unpacks an object value from a JSON packed representation.
    """
    value: dict[str, SomeValue] = {}

    # fields
    for field in typ._base_fields:
        storage_key = field.storage_key
        field_value_packed = value_packed.get(storage_key)
        if field_value_packed is None:
            continue
        elif field.kind == TypeKind.CUSTOM_OBJECT or field.kind == TypeKind.PARTIAL_NODE:
            field_value = unpack_value(field_value_packed, field, wrap_scalar=False)
            if field_value is None:
                continue
        elif not field.is_list:
            field_value = unpack_value_scalar(field_value_packed, field, supergraph=supergraph)
        else:  # scalar list
            assert isinstance(
                field_value_packed, list
            ), f"{field_value_packed!r} is not a list, expected {field!r}"
            field_value = [
                unpack_value_scalar(element, field, supergraph=supergraph)
                for element in field_value_packed
            ]
        value[storage_key] = field_value

    # properties
    for prop in _get_custom_object_properties(kind, typ, value_packed):
        storage_key = prop.key
        prop_value_packed = value_packed.get(storage_key)
        if prop_value_packed is None:
            continue
        elif prop.is_value_packed:
            # nested value_packed
            assert prop.value_runtime_ptr is not None, f"unexpected {prop!r} in {value!r}"
            prop = prop.value_runtime_ptr
            assert prop.value_object_kind is not None, f"unexpected {prop!r} in {value!r}"
            assert prop.value_type_info_getter is not None, f"unexpected {prop!r} in {value!r}"
            prop_type = prop.value_type_info_getter(None)  # type: ignore
            assert prop_type is not None, f"missing type from {prop!r} in {value!r}"
            prop_value = unpack_custom_object(
                kind=prop.value_object_kind,
                value_packed=cast(dict[str, JsonValue], prop_value_packed),
                typ=prop_type,
                supergraph=supergraph,
            )
        elif not prop.is_list:
            prop_value = unpack_value_scalar(
                prop_value_packed, prop.type_info, supergraph=supergraph
            )
        else:
            assert isinstance(
                prop_value_packed, list
            ), f"{prop_value_packed!r} is not a list, expected {prop!r}"
            prop_typ = prop.type_info
            prop_value = [
                unpack_value_scalar(element, prop_typ, supergraph=supergraph)
                for element in prop_value_packed
            ]
        value[storage_key] = prop_value

    return CustomObject.new(
        kind=kind,
        value=value,
        typ=typ,
        parent=parent,
        parent_property=parent_key,
        supergraph=supergraph,
    )


def pack_value(value: SomeValue | None, typ: "TypeBase", *, wrap_scalar: bool) -> JsonValue:
    """
    Packs a value into a JSON representation.
    """

    if typ.kind == TypeKind.CUSTOM_OBJECT or typ.kind == TypeKind.PARTIAL_NODE:
        # nested object
        if not typ.is_list:
            if type(value) is not CustomObject:
                raise TypeError(f"{value!r} is not an Object, expected {typ!r}")
            return pack_custom_object(value, typ)
        else:
            value_packed: JsonValue = []
            for element in cast(Collection[SomeValue], value):
                if type(element) is not CustomObject:
                    raise TypeError(f"{element!r} is not an Object, expected {typ!r}")
                inner_value_packed = pack_custom_object(element, typ)
                value_packed.append(inner_value_packed)
            return value_packed
    else:
        # wrap scalar
        value_packed: JsonValue
        if value is None:
            value_packed = None  # no value
        elif not typ.is_list:
            value_packed = pack_value_scalar(cast(ScalarValue, value), typ)
        else:
            value_packed = [pack_value_scalar(element, typ) for element in cast(list, value)]
        if wrap_scalar:
            value_packed = {typ.identity_key: value_packed}
        return value_packed


def pack_value_data(value: SomeValueData, typ: "TypeBase", wrap_scalar: bool = True) -> JsonValue:
    """Packs a data value into a JSON representation. See above."""
    assert typ.kind not in (
        TypeKind.CUSTOM_OBJECT,
        TypeKind.PARTIAL_NODE,
    ), f"cannot pack data for {typ!r}"
    # wrap scalar
    value_packed: JsonValue
    if value is None:
        value_packed = None
    elif not typ.is_list:
        value_packed = pack_value_scalar(cast(ScalarValueData, value), typ)
    else:
        value_packed = [pack_value_scalar(element, typ) for element in cast(list, value)]
    if wrap_scalar:
        value_packed = {typ.identity_key: value_packed}
    return value_packed


def unpack_value(
    value_packed: JsonValue,
    typ: "TypeBase",
    *,
    parent: ValueParent | None = None,
    parent_key: ValueParentKey | None = None,
    supergraph: NodeSuperGraph | None = None,
    wrap_scalar: bool,
) -> SomeValue | None:
    """
    Unpacks a value from its JSON representation.
    """
    if typ.kind == TypeKind.CUSTOM_OBJECT or typ.kind == TypeKind.PARTIAL_NODE:
        # nested object
        if typ.base_field_type is not None:
            kind = ObjectKind(typ.base_field_type)
        else:
            kind = ObjectKind.BUILTIN
        if not typ.is_list:
            if not isinstance(value_packed, dict):
                raise TypeError(f"{value_packed!r} is not a dict, expected {typ!r}")
            return unpack_custom_object(
                kind=kind,
                value_packed=value_packed,
                typ=typ,
                supergraph=supergraph,
                parent=parent,
                parent_key=parent_key,
            )
        else:
            if not isinstance(value_packed, list):
                raise TypeError(f"{value_packed!r} is not a list, expected {typ!r}")
            return [
                unpack_custom_object(
                    kind=kind,
                    value_packed=cast(dict[str, JsonValue], element),
                    typ=typ,
                    supergraph=supergraph,
                    parent=parent,
                    parent_key=parent_key,
                )
                for element in value_packed
            ]
    else:
        # unwrap scalar
        if wrap_scalar and isinstance(value_packed, dict):
            value_packed = value_packed.get(typ.identity_key)
        if value_packed is None:
            return None
        elif not typ.is_list:
            return unpack_value_scalar(value_packed, typ, supergraph=supergraph)
        else:
            if not isinstance(value_packed, list):
                raise TypeError(f"{value_packed!r} is not a list, expected {typ!r}")
            return [
                unpack_value_scalar(element, typ, supergraph=supergraph) for element in value_packed
            ]


def unpack_value_data(
    value_packed: JsonValue, typ: "TypeBase", wrap_scalar: bool
) -> SomeValueData | JsonValue | None:
    """
    Unpacks a value from its JSON representation. Return nested objects as JSON (as is).
    """
    if typ.kind == TypeKind.CUSTOM_OBJECT or typ.kind == TypeKind.PARTIAL_NODE:
        # nested object
        return value_packed
    else:
        # scalar
        if wrap_scalar and isinstance(value_packed, dict):
            value_packed = value_packed.get(typ.identity_key)
        if value_packed is None:
            return None
        elif not typ.is_list:
            return unpack_value_scalar_data(value_packed, typ)
        else:
            if not isinstance(value_packed, list):
                raise TypeError(f"expected list for {typ!r}, got {value_packed!r}")
            return [unpack_value_scalar_data(v, typ) for v in value_packed]


#
# Common proto stuff
#

# NOTE :Performance: packing/unpacking proto JSON could probably be much more efficient


def pack_proto_date(value: date) -> Date:
    return Date(year=value.year, month=value.month, day=value.day)


def unpack_proto_date(value: Date) -> date:
    return date(year=value.year, month=value.month, day=value.day)


def pack_proto_time(value: time) -> TimeOfDay:
    return TimeOfDay(hours=value.hour, minutes=value.minute, seconds=value.second)


def unpack_proto_time(value: TimeOfDay) -> time:
    return time(hour=value.hours, minute=value.minutes, second=value.seconds)


def pack_proto_json_struct(value: dict[str, Any]) -> ProtoValue:
    struct = ProtoStruct()
    struct.update(value)
    proto_value = ProtoValue(struct_value=struct)
    return proto_value


def unpack_proto_json_struct(value: ProtoValue | ProtoStruct) -> dict[str, Any]:
    json = MessageToDict(value)
    return json


def pack_proto_json(value: JsonValue) -> ProtoValue:
    t = type(value)
    if value is None:
        return ProtoValue(null_value=PROTO_NULL_VALUE)
    elif t is bool:
        return ProtoValue(bool_value=value)  # type: ignore
    elif t is int or t is float:
        return ProtoValue(number_value=float(value))  # type: ignore
    elif t is str:
        return ProtoValue(string_value=value)  # type: ignore
    elif t is list:
        list_value = ProtoList(values=[pack_proto_json(item) for item in value])  # type: ignore
        return ProtoValue(list_value=list_value)
    elif t is dict:
        struct_value = ProtoStruct()
        struct_value.update(value)  # type: ignore
        return ProtoValue(struct_value=struct_value)
    elif t is ProtoList:
        return ProtoValue(list_value=value)  # type: ignore
    elif t is ProtoStruct:
        return ProtoValue(struct_value=value)  # type: ignore
    else:
        raise ValueError(f"unsupported JSON value {value} ({type(value)!r})")


def unpack_proto_json(value: ProtoValue) -> JsonValue:
    if value.HasField("bool_value"):
        return value.bool_value
    elif value.HasField("number_value"):
        return value.number_value
    elif value.HasField("string_value"):
        return value.string_value
    elif value.HasField("list_value"):
        return [unpack_proto_json(item) for item in value.list_value.values]
    elif value.HasField("struct_value"):
        return MessageToDict(value.struct_value)
    else:
        return None


# import later to avoid circular imports (Object is used in node.py)
from bench.language.node import (  # noqa: E402
    BuiltinObject,
    Node,
    NodeReference,
    NodeReferenceData,
    Struct,
    object_,
    struct_,
)

# NOTE :Architecture: custom objects properties start at 900 to avoid interference
#  (when used for partial nodes, especially when they have subtypes)


@object_()
class CustomObjectBase(BuiltinObject):
    """
    Common properties for custom objects.
    The actual values are field-encoded, this is just a base to type the builtin properties.
    (That is, these object bases are never directly instantiated.)
    """

    # TODO :Incomplete: 'free' values in custom objects
    # TODO :Incomplete :Tracing: sources, references, ...
    pass


@struct_(StructType.VARIABLE_OBJECT)
class VariableObject(Struct, CustomObjectBase):
    """A variable object."""

    pass


@struct_(StructType.MEMBER_OBJECT)
class MemberObject(Struct, CustomObjectBase):
    """A member object."""

    pass


@struct_(StructType.INPUT_OBJECT)
class InputObject(Struct, CustomObjectBase):
    """An input object."""

    text: Optional["Text"] = p_regular(
        900, default=None, require=False, array=False, struct=StructType.TEXT
    )


@struct_(StructType.OUTPUT_OBJECT)
class OutputObject(Struct, CustomObjectBase):
    """An output object."""

    text: Optional["Text"] = p_regular(
        900, default=None, require=False, array=False, struct=StructType.TEXT
    )
    call: "Call | None" = p_regular(910, require=False, struct=StructType.CALL)
    continuations: list["Continue"] = p_regular(
        911, default=[], array=True, struct=StructType.CONTINUE
    )


CUSTOM_OBJECT_CLASS_BY_KIND = {
    ObjectKind.MEMBER: MemberObject,
    ObjectKind.VARIABLE: VariableObject,
    ObjectKind.INPUT: InputObject,
    ObjectKind.OUTPUT: OutputObject,
}


@struct_(StructType.VALUE)
class Value(Struct):
    """A generic typed 'freeform' value."""

    type: "TypeInfo" = p_regular(31, struct=StructType.TYPE_INFO)
    name: str | None = p_regular(32, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.TEXT
    )
    value_packed: Any = p_value_packed(35)
    value: Any = p_value_runtime(
        35, kind=ObjectKind.MEMBER, typ=lambda self: cast("Value", self).type
    )

import base64
import re
from collections.abc import Mapping
from datetime import datetime, timedelta
from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    Optional,
    Sequence,
    TypeGuard,
    Union,
    cast,
    override,
)
from uuid import UUID

import pytz
import structlog
from betterproto.lib.google.protobuf import Struct as ProtoStruct
from opentelemetry import trace

from bench.language.const import (
    EMPTY_DICT,
    FLOAT_EPSILON,
    PY_TYPE_BY_PRIMITIVE_TYPE,
    UNSET,
    EnumType,
    NodeType,
    ObjectType,
    PrimitiveType,
    PrimitiveValue,
    StructType,
    TypeKind,
)
from bench.language.property import (
    Property,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.language.setup import ENUM_CLASS_BY_TYPE, OBJECT_CLASS_BY_TYPE
from bench.language.validation import NAME_CONSTRAINT, on_invalid_raise
from bench.proto.wire import AnyNodeData, AnyStructData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import (
        BuiltinObject,
        Field,
        Node,
        NodeReference,
        Struct,
        Text,
        TypeInfo,
        TypeInfoBase,
    )
    from bench.language.validation import ValidationHandler


# pyright: reportIncompatibleVariableOverride=false


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

ScalarValue = Union["ValueObject", PrimitiveValue, "BuiltinObject"]
ScalarValueData = Union[
    AnyNodeData,
    AnyStructData,
    PrimitiveValue,
    dict[str, "ScalarValueData"],
    list["ScalarValueData"],
    ProtoStruct,
]
SomeValue = Union[ScalarValue, Collection[ScalarValue], None]
SomeValueData = Union[ScalarValueData, Collection[ScalarValueData], None]
JsonPrimitive = Union[str, int, float, bool, None]
JsonValue = Union[JsonPrimitive, dict[str, "JsonValue"], list["JsonValue"]]
ValueParent = Union["ValueObject", "BuiltinObject"]
ValueProperty = Union["Property", "Field"]


class ValueObject(Mapping[str, Any]):
    """
    An Object-like Value with fields, the user defined equivalent of our built-in Objects (Structs/Nodes).
    Objects can be 'partial' (e.g., Block variable, Run inputs, Class instance).
    """

    __slots__ = (
        "_type",
        "_value",
        "ancestor_prop",
        "id",
        "order_key",
        "parent",
        "parent_id",
        "parent_key",
        "parent_prop",
    )

    def __init__(
        self,
        type: "TypeInfoBase",
        value: dict[str, SomeValue] | None = None,
        parent: ValueParent | None = None,
        parent_prop: ValueProperty | None = None,
        ancestor_prop: Optional["Property"] = None,
        parent_key: str | None = None,
    ):
        self._type = type
        self._value = value
        self.parent = parent
        self.parent_prop = parent_prop
        self.ancestor_prop = ancestor_prop
        self.parent_key = parent_key
        if self.parent is not None and self.ancestor_prop is None:
            assert isinstance(self.parent_prop, Property), f"{self.parent_prop!r} is not a Property"
            self.ancestor_prop = self.parent_prop

    def __str__(self) -> str:
        if self._value is None:
            return ""
        set_fields: list[str] = []
        for field in self._type._base_fields:
            field_value = self._do_get(field)
            if field_value:
                if type(field_value) is list:
                    set_fields.append(f"{field.name}[{len(field_value)}]")
                elif type(field_value) is ValueObject:
                    set_fields.append(f"{field.name}=<{field_value._type_name} (...)>")
                else:
                    set_fields.append(f"{field.name}={field_value!r}")
        return ", ".join(set_fields)

    def __repr__(self) -> str:
        return f"<{self._type_name} ({self})>"

    def _equals_content(self, other: Any) -> bool:
        """Checks if all fields of the two Values are equal (recursively)."""
        if other is None or type(other) is not ValueObject:
            return False
        elif self._value is None:
            return other._value is None
        elif other._value is None:
            return False
        for field in self._type._base_fields:
            if self._value.get(field.storage_key) != other._value.get(field.storage_key):
                return False
        return True

    __eq__ = _equals_content

    def __getitem__(self, item: str) -> SomeValue:
        # NOTE: __getattr__ is called only when ident is not in the slots, so this is a value lookup
        # get field value
        field = self._type._get_field(item)
        if field is None:
            raise AttributeError(f"{self._type!r} has no field named '{item}'")
        if self._type.base_field_zone is not None and field.zone != self._type.base_field_zone:
            raise AttributeError(f"{field!r} is not in the same zone as {self._type!r}")
        return self._do_get(field)

    __getattr__ = __getitem__

    def _do_get(self, field: "Field") -> SomeValue:
        if self._value is None:
            return field.default
        value = self._value.get(field.storage_key)
        if value is None:
            return field.default
        elif isinstance(value, NodeReferenceBase):
            # auto resolve references
            resolved_value = self._type._supergraph.get(value)
            if resolved_value is not None:
                return resolved_value
            elif value.type in NODE_REFERENCE_TYPES_BY_NODE_TYPE:
                return value  # :RichReferences
            else:
                return None  # couldn't resolve
        else:
            return value

    def __setitem__(self, item: str, value: SomeValue) -> None:
        # set field value
        field: Field | None = self._type._get_field(item)
        if field is None:
            raise AttributeError(f"{self._type!r} has no field with identifier {item}")
        if self._type.base_field_zone is not None and field.zone != self._type.base_field_zone:
            raise AttributeError(f"{field!r} is not in the same zone as {self._type!r}")
        # coerce & copy if needed
        value = coerce_value(
            value, field, parent=self, parent_prop=field, ancestor_prop=self.ancestor_prop
        )
        check_value(value, field, invalid=on_invalid_raise)
        if self._value is None:
            self._value = {}
        if (field.kind == TypeKind.NODE or field.kind == TypeKind.BASED_NODE) and value:
            # turn nodes into reference
            if field.is_list:
                assert isinstance(value, list), f"{value!r} is not a list"
                value = [cast(Node, n).to_ref() for n in value]
            else:
                assert isinstance(value, Node), f"{value!r} is not a Node"
                value = value.to_ref()
        self._value[field.storage_key] = value

        # notify
        self._updated_self((field,))

    def __setattr__(self, item: str, value: SomeValue) -> None:
        # NOTE: __setattr__ is also called for slots so we have to bypass those
        if item in ValueObject.__slots__:
            return object.__setattr__(self, item, value)
        self.__setitem__(item, value)

    def __delitem__(self, item: str) -> None:
        # delete field value if it's not required
        field = self._type._get_field(item)
        if field is None:
            raise AttributeError(f"{self._type!r} has no field with identifier {item}")
        if field.is_required:
            raise AttributeError(f"{field!r} is required")
        if self._value is not None:
            self._value.pop(field.storage_key, None)
        # notify
        self._updated_self((field,))

    @property
    def fields(self):
        for field in self._type._base_fields:
            if self._type.base_field_zone is None or field.zone == self._type.base_field_zone:
                yield field

    def __iter__(self):
        for field in self._type._base_fields:
            if self._type.base_field_zone is None or field.zone == self._type.base_field_zone:
                yield field.name

    def __len__(self) -> int:
        return len(self._type._base_fields)

    def __contains__(self, item: object) -> bool:
        for field in self._type._base_fields:
            if self._type.base_field_zone is not None and field.zone != self._type.base_field_zone:
                continue
            if field.py_ident == item or field.name == item:
                return True
        return False

    def _move_to(
        self, parent: ValueParent, prop: ValueProperty, ancestor_prop: Optional["Property"]
    ) -> "ValueObject":
        """Move or copy this object into the given parent/prop."""
        prop_key = prop.id_as_str if isinstance(prop, Property) else prop.identity_key
        if self.parent is None:
            # not yet assigned
            self.parent = parent
            self.parent_prop = prop
            self.ancestor_prop = ancestor_prop
            self.parent_key = prop_key
            return self
        elif self.parent is parent and self.parent_key == prop_key:
            # already there
            return self
        else:
            copy = self._copy_to(parent, prop, ancestor_prop)
            return copy

    def _copy_to(
        self, parent: ValueParent, prop: ValueProperty, ancestor_prop: Optional["Property"]
    ) -> "ValueObject":
        """Copy this object into the given parent/prop."""
        value_packed = pack_value_object(self, self._type)
        copy = unpack_value_object(value_packed, self._type, parent, prop, ancestor_prop)
        copy.parent_key = prop.id_as_str if isinstance(prop, Property) else prop.identity_key
        return copy

    def _updated_self(self, properties: tuple[Union["Property", "Field", Any], ...]) -> None:
        if self.parent is not None:
            prop = self.ancestor_prop if self.ancestor_prop is not None else self.parent_prop
            assert type(prop) is Property, f"{prop!r} is not a Property"
            self.parent._updated_self((prop,))

    @staticmethod
    def new(
        value: dict[str, SomeValue] | None,
        typ: "TypeInfoBase",
        parent: ValueParent | None = None,
        parent_property: ValueProperty | None = None,
        ancestor_property: Optional["Property"] = None,
    ) -> "ValueObject":
        """Creates a new Object of the given Object type, coercing the given value."""
        assert typ.kind == TypeKind.OBJECT, f"{typ!r} is not an Object type"
        return ValueObject(
            type=typ,
            value=value,
            parent=parent,
            parent_prop=parent_property,
            ancestor_prop=ancestor_property,
        )

    @property
    def _type_name(self) -> str:
        if self._type.kind != TypeKind.OBJECT or self._type.base_type is None:
            if self._type._resolved_type is not None:
                return cast(TypeKind, self._type._resolved_type.kind).bench_name
            elif self._type.kind is not None:
                return self._type.kind.bench_name
            else:
                return "?Object"
        else:
            return self._type.base_type.absolute_path


#
# Value coercion
#


def _coerce_value_scalar(
    value: ScalarValue,
    typ: "TypeInfoBase",
    parent: ValueParent | None = None,
    parent_prop: ValueProperty | None = None,
    ancestor_prop: "Property | None" = None,
) -> ScalarValue:
    """Coerces a scalar value (primitive, node, struct)"""
    if typ.kind == TypeKind.PRIMITIVE:
        assert typ.primitive_type is not None, f"missing primitive type for {typ!r}"
        if typ.primitive_type.is_numeric:
            if typ.primitive_type.is_float:
                value = float(cast(Any, value))
            elif typ.primitive_type.is_int:
                value = int(cast(Any, value))
    elif typ.kind == TypeKind.STRUCT and parent is not None and isinstance(value, Struct):
        assert parent_prop is not None, f"{typ!r} got parent {parent!r} but no parent_prop"
        value = cast("Struct", value)._move_to(parent, parent_prop, ancestor_prop)
    return value


def coerce_object_scalar(
    value: dict | ValueObject,
    typ: "TypeInfoBase",
    parent: ValueParent | None = None,
    parent_prop: ValueProperty | None = None,
    ancestor_prop: "Property | None" = None,
) -> ValueObject:
    """Coerces a single object from a dict representation or existing Object (recursively)."""
    if type(value) is ValueObject:
        # NOTE :Robustness: not sure if _coerce_object_scalar is correct if given an existing object
        if parent is not None:
            assert parent_prop is not None, f"{typ!r} got parent {parent!r} but no parent_prop"
            return value._move_to(parent, parent_prop, ancestor_prop)
        else:
            return value
    else:
        # coerce
        assert isinstance(value, dict), f"{value!r} is not a dict (expected {typ!r})"
        value_coerced = {}
        for field in typ._base_fields:
            field_type = field._to_resolved()
            # try getting value by storage key, name and ident
            field_value = value.get(field.storage_key)
            if field_value is None:
                field_value = value.get(field.name)
            if field_value is None:
                field_value = value.get(field.py_ident)
            if field_value is None:
                continue
            value_coerced[field.storage_key] = coerce_value(
                field_value, field_type, parent, parent_prop, ancestor_prop
            )
        return ValueObject.new(
            value=value_coerced,
            typ=typ,
            parent=parent,
            parent_property=parent_prop,
            ancestor_property=ancestor_prop,
        )


def coerce_value(
    value: Any,
    typ: "TypeInfoBase",
    parent: ValueParent | None = None,
    parent_prop: ValueProperty | None = None,
    ancestor_prop: "Property | None" = None,
) -> SomeValue:
    """
    Coerces the given value to the expected type (recursively). Returns value as is if already of correct type.
    To maintain clarity, we try tdio coerce as little as possible outside the typical python cases.
    Raises TypeError if not possible.
    NOTE :Performance: we re-create and copy lists during coercion even if the type was already good
    """
    if typ.kind == TypeKind.OBJECT:
        if not typ.is_list:
            return coerce_object_scalar(cast(dict, value), typ, parent, parent_prop, ancestor_prop)
        else:
            if isinstance(value, Sequence):
                raise TypeError(f"{value!r} is not a sequence (expected {typ!r})")
            return [
                coerce_object_scalar(cast(dict, element), typ, parent, parent_prop, ancestor_prop)
                for element in value
            ]
    else:
        if value is None:
            return None
        elif not typ.is_list:
            return _coerce_value_scalar(value, typ, parent, parent_prop, ancestor_prop)
        else:
            if not isinstance(value, Sequence):
                raise TypeError(f"{value!r} is not a sequence (expected {typ!r})")
            return [
                _coerce_value_scalar(element, typ, parent, parent_prop, ancestor_prop)
                for element in value
            ]


#
# Type checking
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


def check_value_scalar(value: SomeValue, typ: "TypeInfoBase", invalid: "ValidationHandler") -> None:
    """Checks whether the given scalar value has the expected type."""
    if typ.kind == TypeKind.PRIMITIVE:
        expected_type = PY_TYPE_BY_PRIMITIVE_TYPE.get(cast(PrimitiveType, typ.primitive_type))
        if expected_type is None:
            return  # nothing to check?
        elif type(value) is not expected_type and not isinstance(value, expected_type):
            invalid(value, "not of type", typ)
            return  # also nothing to do
        # check constraint
        if typ.constraint is not None:
            if type(value) is int or type(value) is float:
                if typ.constraint.min_value is not None and value < typ.constraint.min_value:
                    invalid(value, "too small", typ)
                if typ.constraint.max_value is not None and value > typ.constraint.max_value:
                    invalid(value, "too large", typ)
                if (
                    typ.constraint.step_value is not None
                    and abs(value % typ.constraint.step_value) > FLOAT_EPSILON
                ):
                    invalid(value, f"not a multiple of {typ.constraint.step_value}", typ)
            if type(value) is str:
                if typ.constraint.min_length is not None and len(value) < typ.constraint.min_length:
                    invalid(value, "too short", typ)
                if typ.constraint.max_length is not None and len(value) > typ.constraint.max_length:
                    invalid(value, "too long", typ)
                if typ.constraint.regex is not None and not re.match(typ.constraint.regex, value):
                    raise TypeError(f"{value!r} does not match {typ.constraint.regex!r}", typ)
                if typ.constraint.starts_with is not None and not value.startswith(
                    typ.constraint.starts_with
                ):
                    invalid(value, f"does not start with {typ.constraint.starts_with}", typ)
                if typ.constraint.ends_with is not None and not value.endswith(
                    typ.constraint.ends_with
                ):
                    invalid(value, f"does not end with {typ.constraint.ends_with}", typ)
        # strings cannot be empty (because we use protobuf and have to disambiguate unset from empty)
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
        if not getattr(type(cast("Node", value)), "__is_node__", False):
            invalid(value, "not a Node", typ)
        elif cast("Node", value).metatype != typ.bench_type and (
            typ._from_property is None
            # special case :FakeNodePropertyUnion for reference properties
            or cast("Node", value).metatype not in (typ._from_property.reference_nodes or ())
        ):
            invalid(value, "not of type", typ)
        if typ.kind == TypeKind.BASED_NODE:
            if typ.base_type is not None and cast(HasNodeBase, value).base != typ.base_type:
                invalid(value, f"not based on {typ.base_type}", typ)
    elif typ.kind == TypeKind.STRUCT:
        if typ.bench_type == StructType.PROPERTY_REFERENCE:
            if not isinstance(value, Property):
                invalid(value, "not a Property", typ)
        else:
            if not getattr(cast("Struct", value), "__is_struct__", False):
                invalid(value, "not a Struct", typ)
            elif cast("Struct", value).metatype != typ.bench_type:
                invalid(value, "not of type", typ)
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        try:
            enum_cls(cast(int, value))
        except ValueError:
            invalid(value, f"not a valid {enum_cls}", typ)
    else:
        raise RuntimeError(f"unexpected type {typ!r}")


def _check_is_list(
    value: SomeValue, typ: "TypeInfoBase", invalid: "ValidationHandler"
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
    value: SomeValue, typ: "TypeInfoBase", invalid: "ValidationHandler"
) -> TypeGuard[ValueObject]:
    if not isinstance(value, ValueObject):
        invalid(value, "not an object", typ)
        return False
    return True


def check_value_object_scalar(
    value: SomeValue, typ: "TypeInfoBase", invalid: "ValidationHandler"
) -> None:
    """Checks whether the given object value has the expected type (recursively)."""
    if _check_is_object(value, typ, invalid):
        for field in typ._base_fields:
            field_type = field._to_resolved()
            field_value = cast(SomeValue, getattr(value, field.name, None))
            check_value(field_value, field_type, invalid)


def check_value(value: Any, typ: "TypeInfoBase", invalid: "ValidationHandler") -> None:
    """
    Checks whether the given value has the expected type (recursively).
    """
    typ = typ._to_resolved()
    assert typ.kind != TypeKind.ALIAS, f"unresolved type {typ!r}"
    if typ.kind == TypeKind.OBJECT:
        if not typ.is_list:
            if value is None:
                if typ.is_required:
                    invalid(value, "missing required value", typ)
                else:
                    return
            check_value_object_scalar(value, typ, invalid)
        elif _check_is_list(value, typ, invalid):
            for element in value:
                check_value_object_scalar(element, typ, invalid)
    else:
        if not typ.is_list:
            if value is None:
                if typ.is_required:
                    invalid(value, "missing required value", typ)
                else:
                    return
            check_value_scalar(value, typ, invalid)
        elif _check_is_list(value, typ, invalid):
            for element in value:
                check_value_scalar(element, typ, invalid)


#
# Value sampling
# NOTE :Incomplete :Feature: we'll probably want value sampling as a more general feature
#  (also, value sampling is suspiciously similar to the strategy-based sampling we do
#   during testing, but just different enough because this is sparse, low-volume & user-facing)
#

SAMPLE_VALUE_BY_PROPERTY: dict[str, SomeValue] = {
    "order_key": INTEGER_ZERO,
}


def sample_scalar_value(typ: "TypeInfoBase") -> ScalarValue | None:
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
        return None  # NOTE :Incomplete: :SampleNodeValues
    elif typ.kind == TypeKind.STRUCT:
        return sample_builtin_object(typ)
    else:
        raise RuntimeError(f"unexpected type {typ!r}")


def sample_builtin_object(typ: "TypeInfoBase") -> "BuiltinObject":
    """Samples a representative object value for the given type (recursively)."""
    object_cls = OBJECT_CLASS_BY_TYPE.get(cast(ObjectType, typ.bench_type))
    assert object_cls is not None, f"missing object class for {typ!r}"
    object_kwargs = {}
    for prop in object_cls.__runtime_properties__.values():
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
            ...  # NOTE :Incomplete: :SampleNodeValues
        elif prop.is_struct_reference and not prop.is_required:
            object_kwargs[prop.name] = [] if prop.is_list else None  # don't recurse
        elif prop.is_property_reference:
            object_kwargs[prop.name] = [prop] if prop.is_list else prop
        elif prop.reference_is_node_data or prop.is_value_runtime or prop.is_value_packed:
            object_kwargs[prop.name] = None
        else:
            object_kwargs[prop.name] = sample_value(prop.type_info)
    return object_cls(**object_kwargs)


def sample_value_object_scalar(typ: "TypeInfoBase", recurse_objects: bool = True) -> ValueObject:
    """Samples a representative object value for the given type (recursively)."""
    assert typ.kind == TypeKind.OBJECT, f"expected object type, got {typ!r}"
    value = {}
    for field in typ._base_fields:
        field_type = field._to_resolved()
        if field.is_list:
            value[field.storage_key] = [sample_value(field_type, recurse_objects) for _ in range(1)]
        else:
            value[field.storage_key] = sample_value(field_type, recurse_objects)
    return ValueObject.new(value, typ)


@tracer.start_as_current_span(name="value.sample")
def sample_value(typ: "TypeInfoBase", recurse_objects: bool = True) -> SomeValue:
    """Samples a representative value for the given type (recursively)."""
    typ = typ._to_resolved()
    assert typ.kind != TypeKind.ALIAS, f"unresolved type {typ!r}"
    if typ.kind == TypeKind.OBJECT:
        if not typ.is_list:
            return sample_value_object_scalar(typ, recurse_objects)
        else:
            return [sample_value_object_scalar(typ, recurse_objects) for _ in range(1)]
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
# TODO :Incomplete: support freeform values (incl. alongside typed values)
#


def pack_value_scalar(value: ScalarValue | ScalarValueData, typ: "TypeInfoBase") -> JsonValue:
    """
    Packs the given scalar runtime or data value into a JSON-able representation.
    """
    if typ.kind == TypeKind.PRIMITIVE:
        if typ.primitive_type == PrimitiveType.BYTES:
            return base64.b64encode(cast(bytes, value)).decode()
        elif typ.primitive_type == PrimitiveType.UUID:
            return str(cast(UUID, value))
        elif typ.primitive_type == PrimitiveType.JSON:
            if isinstance(value, ProtoStruct):
                from bench.proto import wiring

                return cast(JsonValue, wiring.unpack_proto_json(value))  # :ProtoStructMapping
            else:
                return cast(JsonValue, value)
        elif typ.primitive_type == PrimitiveType.DATETIME:
            return cast(datetime, value).isoformat()
        elif typ.primitive_type == PrimitiveType.INTERVAL:
            return cast(timedelta, value).total_seconds()
        else:
            return cast(JsonValue, value)
    elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
        if cast("Struct", value).metatype != StructType.NODE_REFERENCE:
            ref = cast("Node", value).to_ref()
        else:
            ref = cast("NodeReference", value)
        if isinstance(ref, BuiltinObject):
            ref = ref._to_data()
        return pack_builtin_object_data(ref)
    elif typ.kind == TypeKind.ENUM:
        return int(cast(int, value))
    elif typ.kind == TypeKind.STRUCT:
        if isinstance(value, BuiltinObject):
            value = value._to_data()
        return pack_builtin_object_data(cast(AnyStructData | AnyNodeData, value))
    else:
        raise TypeError(f"cannot pack value of type {typ!r}")


def unpack_value_scalar(value_packed: JsonValue, typ: "TypeInfoBase") -> ScalarValue:
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
        elif typ.primitive_type == PrimitiveType.INTERVAL:
            return timedelta(seconds=cast(int, value_packed))
        else:
            return cast(PrimitiveValue, value_packed)
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        return enum_cls(cast(int, value_packed))
    elif typ.kind in (TypeKind.NODE, TypeKind.BASED_NODE, TypeKind.STRUCT):
        from bench.proto import wiring

        assert isinstance(value_packed, dict), f"{value_packed!r} is not a dict (expected {typ!r})"
        value_struct = unpack_builtin_object_data(value_packed)
        # TODO :Broken: pass in proper supergraph to values (and structs in values)
        return wiring.unpack_object(cast(AnyStructData, value_struct), supergraph=None)
    else:
        raise TypeError(f"cannot unpack value of type {typ!r}")


def unpack_value_scalar_data(value_packed: JsonValue, typ: "TypeInfoBase") -> ScalarValueData:
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
            from bench.proto import wiring

            return wiring.pack_proto_json(cast(dict, value_packed))
        elif typ.primitive_type == PrimitiveType.DATETIME:
            return datetime.fromisoformat(cast(str, value_packed))
        elif typ.primitive_type == PrimitiveType.INTERVAL:
            return timedelta(seconds=cast(int, value_packed))
        else:
            return cast(PrimitiveValue, value_packed)
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        return enum_cls(cast(int, value_packed))
    elif typ.kind in (TypeKind.NODE, TypeKind.BASED_NODE, TypeKind.STRUCT):
        assert isinstance(value_packed, dict), f"{value_packed!r} is not a dict (expected {typ!r})"
        return unpack_builtin_object_data(value_packed)
    else:
        raise TypeError(f"cannot unpack value of type {typ!r}")


def pack_builtin_object_data(
    value: AnyStructData | AnyNodeData,
    only: Sequence[Property] | None = None,
) -> dict[str, JsonValue]:
    """Packs a single struct/node data value using typed proto ids as keys and enum values."""
    value_packed: dict[str, JsonValue] = {}
    object_cls = OBJECT_CLASS_BY_TYPE[cast(ObjectType, value.metatype)]
    for prop in only if only is not None else object_cls.__wired_properties__.values():
        if prop.reference_wired_ptr is not None:
            prop = prop.reference_wired_ptr
        prop_value = getattr(value, prop.name)
        if prop_value is None or (prop.is_list and len(prop_value) == 0):
            continue
        elif prop.is_list:
            prop_value_packed = [
                pack_value_scalar(element, prop.type_info) for element in prop_value
            ]
        else:
            prop_value_packed = pack_value_scalar(prop_value, prop.type_info)
        value_packed[prop.id_as_str] = prop_value_packed
    return value_packed


def unpack_builtin_object_data[T: AnyStructData | AnyNodeData](
    value_packed: dict[str, Any],
    expect: type[T] | None = None,
    only: Collection[Property | Any] | None = None,
) -> AnyStructData | AnyNodeData:
    """Unpacks a single struct/node data value using typed proto ids as keys and enum values."""
    from bench.proto import wire, wiring

    if expect is None:
        object_type = value_packed.get("1")
        assert object_type is not None, f"{value_packed!r} has no object type and none given"
        object_type = wire.ObjectType(object_type)
    else:
        object_type = wiring.OBJECT_TYPE_BY_PROTO_CLASS[expect]
        object_type = wire.ObjectType(object_type)
    object_cls = OBJECT_CLASS_BY_TYPE[cast(ObjectType, object_type)]
    proto_cls = wiring.PROTO_CLASS_BY_TYPE[cast(ObjectType, object_type)]

    value = proto_cls(metatype=object_type)  # type: ignore
    for prop in only if only is not None else object_cls.__wired_properties__.values():
        if prop.reference_wired_ptr is not None:
            prop = prop.reference_wired_ptr
        prop_value_packed = value_packed.get(prop.id_as_str)
        if prop_value_packed is None or (prop.is_list and len(prop_value_packed) == 0):
            continue
        elif prop.is_list:
            prop_value = [
                unpack_value_scalar_data(element, prop.type_info) for element in prop_value_packed
            ]
        else:
            prop_value = unpack_value_scalar_data(prop_value_packed, prop.type_info)
        setattr(value, prop.name, prop_value)
    return value


def pack_value_object(value: ValueObject, typ: "TypeInfoBase") -> dict[str, JsonValue]:
    """
    Packs an object value into a packed value & secret packed value.
    The secret split applies only to nested values within the type, not the type itself.
    """
    value_packed: dict[str, JsonValue] = {}
    _value = value._value
    assert _value is not None, f"{value!r} has no value"

    for field in typ._base_fields:
        field_type = field._to_resolved()
        field_value = cast(SomeValue, _value.get(field.storage_key))
        if field_value is None:
            continue
        elif field_type.kind == TypeKind.OBJECT:
            value_packed[field.storage_key] = pack_value(field_value, field_type)
        elif not field_type.is_list:
            value_packed[field.storage_key] = pack_value_scalar(
                cast(ScalarValue, field_value), field_type
            )
        else:  # scalar list
            assert isinstance(
                field_value, list
            ), f"{field_value!r} is not a list (expected {field!r})"
            value_packed[field.storage_key] = [
                pack_value_scalar(element, field_type) for element in field_value
            ]
    return value_packed


def unpack_value_object(
    value_packed: dict[str, JsonValue],
    typ: "TypeInfoBase",
    parent: ValueParent | None = None,
    parent_prop: ValueProperty | None = None,
    ancestor_prop: Optional["Property"] = None,
) -> ValueObject:
    """
    Unpacks an object value from a packed value & secret packed value.
    """
    value: dict[str, SomeValue] = {}
    for field in typ._base_fields:
        field_type = field._to_resolved()
        field_value_packed = value_packed.get(field.storage_key)
        if field_value_packed is None:
            continue
        elif field_type.kind == TypeKind.OBJECT:
            field_value = unpack_value(field_value_packed, field_type)
            if field_value is None:
                continue
        elif not field_type.is_list:
            field_value = unpack_value_scalar(field_value_packed, field_type)
        else:  # scalar list
            assert isinstance(
                field_value_packed, list
            ), f"{field_value_packed!r} is not a list (expected {field!r})"
            field_value = [
                unpack_value_scalar(element, field_type) for element in field_value_packed
            ]
        value[field.storage_key] = field_value
    return ValueObject.new(
        value=value,
        typ=typ,
        parent=parent,
        parent_property=parent_prop,
        ancestor_property=ancestor_prop,
    )


@tracer.start_as_current_span(name="value.pack")
def pack_value(
    value: SomeValue | None, typ: "TypeInfoBase", wrap_primitive: bool = True
) -> JsonValue:
    """
    Packs a value into JSON-able parts (packed value & secret packed value).
    Only minimal type checks are performed, invalid values will error in various ways.
    """
    typ = typ._to_resolved()
    assert typ.kind != TypeKind.ALIAS, f"unresolved type {typ!r}"
    if typ.kind == TypeKind.OBJECT:
        # nested object
        if not typ.is_list:
            if type(value) is not ValueObject:
                raise TypeError(f"{value!r} is not an Object (expected {typ!r})")
            return pack_value_object(value, typ)
        else:
            value_packed: JsonValue = []
            for element in cast(Collection[SomeValue], value):
                if type(element) is not ValueObject:
                    raise TypeError(f"{element!r} is not an Object (expected {typ!r})")
                inner_value_packed = pack_value_object(element, typ)
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
        if wrap_primitive:
            value_packed = {typ.identity_key: value_packed}
        return value_packed


def pack_value_data(
    value: SomeValueData, typ: "TypeInfoBase", wrap_primitive: bool = True
) -> JsonValue:
    """Packs a data value into JSON-able parts. See above."""
    typ = typ._to_resolved()
    assert typ.kind != TypeKind.ALIAS, f"unresolved type {typ!r}"
    assert typ.kind != TypeKind.OBJECT, f"cannot pack data for {typ!r}"
    # wrap scalar
    value_packed: JsonValue
    if value is None:
        value_packed = None
    elif not typ.is_list:
        value_packed = pack_value_scalar(cast(ScalarValueData, value), typ)
    else:
        value_packed = [pack_value_scalar(element, typ) for element in cast(list, value)]
    if wrap_primitive:
        value_packed = {typ.identity_key: value_packed}
    return value_packed


@tracer.start_as_current_span(name="value.unpack")
def unpack_value(
    value_packed: JsonValue,
    typ: "TypeInfoBase",
    parent: ValueParent | None = None,
    parent_prop: ValueProperty | None = None,
    ancestor_prop: Optional["Property"] = None,
) -> SomeValue | None:
    """
    Unpacks a value from its constituent JSON-able parts (packed value & secret packed value).
    Only minimal type checks are performed, invalid values will error in various ways.
    """
    typ = typ._to_resolved()
    assert typ.kind != TypeKind.ALIAS, f"unresolved type {typ!r}"
    if typ.kind == TypeKind.OBJECT:
        # nested object
        if not typ.is_list:
            if not isinstance(value_packed, dict):
                raise TypeError(f"{value_packed!r} is not a dict (expected {typ!r})")
            return unpack_value_object(
                value_packed=value_packed,
                typ=typ,
                parent=parent,
                parent_prop=parent_prop,
                ancestor_prop=ancestor_prop,
            )
        else:
            if not isinstance(value_packed, list):
                raise TypeError(f"{value_packed!r} is not a list (expected {typ!r})")
            return [
                unpack_value_object(
                    value_packed=cast(dict[str, JsonValue], element),
                    typ=typ,
                    parent=parent,
                    parent_prop=parent_prop,
                    ancestor_prop=ancestor_prop,
                )
                for element in value_packed
            ]
    else:
        # unwrap scalar
        if isinstance(value_packed, dict):
            value_packed = value_packed.get(typ.identity_key)
        if value_packed is None:
            return None
        elif not typ.is_list:
            return unpack_value_scalar(value_packed, typ)
        else:
            if not isinstance(value_packed, list):
                raise TypeError(f"{value_packed!r} is not a list (expected {typ!r})")
            return [unpack_value_scalar(element, typ) for element in value_packed]


# import later to avoid circular imports (Object is used in node.py)
from bench.language.node import (  # noqa: E402
    NODE_REFERENCE_TYPES_BY_NODE_TYPE,
    BuiltinObject,
    HasNodeBase,
    Node,
    NodeReferenceBase,
    Struct,
    object_component,
    struct_,
)


@object_component()
class HasValues(BuiltinObject):
    # NOTE :Robustness :Architecture: turn value into computed property? :ComputedValueProp

    @override
    def _init_component(self):
        # if unpacked is set, pack in place, otherwise vice versa
        # (this is a bit unwieldy and means we don't get value if the object is created
        #  outside a session, but we'll likely change this soon anyway - see above;
        #  also, it feels like this should be done in the main BuiltinObject.__init__ loop?)
        if self._session is None:
            return
        for prop in self.__value_properties__.values():
            assert type(prop.value_packed_ptr) is Property, f"{prop!r} has no value_packed_ptr"
            value = getattr(self, prop.name)
            value_packed = getattr(self, prop.value_packed_ptr.name)
            if (value is None and value_packed is None) or value_packed is UNSET:
                continue
            value_type = prop.value_type_info_getter(self) if prop.value_type_info_getter else None
            if value_type is None:
                continue  # not ready yet or :FreeformValues
            if value is not None:
                if getattr(self, prop.value_packed_ptr.name) is not None:
                    continue  # skip if already set
                value_packed = pack_value(value, value_type)
                self._do_set(prop.value_packed_ptr.name, value_packed, track=False)
            else:
                value = unpack_value(value_packed, value_type)
                self._do_set(prop.name, value, track=False)

    @override
    def _updated_component(self, properties: Collection[Property]) -> None:
        # update packed properties  :ComputedValueProp
        if any(prop.is_value_runtime for prop in properties):
            # NOTE :Performance: only update packed values prior to serialization? (see above)
            #  (but note that we would still need the packed data for the Edit)
            self._pack_values_inplace(properties)

    def _unpack_values_inplace(self, properties: Collection[Property] = ()) -> None:
        # also a bit crummy, see above :ComputedValueProp
        if len(properties) == 0:
            properties = self.__value_properties__.values()
        for prop in properties:
            if not prop.is_value_runtime:
                continue
            assert type(prop.value_packed_ptr) is Property, f"{prop!r} has no value_packed_ptr"
            value_packed = getattr(self, prop.value_packed_ptr.name)
            value_type = prop.value_type_info_getter(self) if prop.value_type_info_getter else None
            if value_type is not None:
                if value_packed is None:
                    value_packed = EMPTY_DICT
                value = unpack_value(value_packed, value_type)
                self._do_set(prop.name, value, track=False)

    def _pack_values_inplace(
        self, properties: Collection[Property] = (), skip_already_set: bool = False
    ) -> None:
        # more ugh here  :ComputedValueProp
        for prop in properties:
            if not prop.is_value_runtime:
                continue
            assert type(prop.value_packed_ptr) is Property, f"{prop!r} has no value_packed_ptr"
            # allow us to bail if we want to force set a temporary value in the constructor
            if skip_already_set and getattr(self, prop.value_packed_ptr.name) is not None:
                continue
            value = getattr(self, prop.name)
            if value is not None:
                value_type = (
                    prop.value_type_info_getter(self) if prop.value_type_info_getter else None
                )
                if value_type is not None:
                    value_packed = pack_value(value, value_type)
                    self._do_set(prop.value_packed_ptr.name, value_packed, track=False)
            else:
                self._do_set(prop.value_packed_ptr.name, None, track=False)


@struct_(StructType.VALUE)
class Value(Struct, HasValues):
    """A generic 'freeform' value."""

    type: "TypeInfo" = p_regular(31, struct=StructType.TYPE_INFO)
    name: str | None = p_regular(32, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.TEXT
    )
    value_packed: Any = p_value_packed(35)
    value: Any = p_value_runtime(35, typ=lambda self: cast("Value", self).type)


def coerce_value_object(typ: "TypeInfoBase", value_raw: Any) -> ValueObject:
    """
    Tries to coerce a value object from a given raw value.
    We support 4 coercions:
     1. Tuple of return values if there are multiple fields (with same length)
     2. Dict of return values with { FieldName: Value }
     3. ValueObject of same shape
     4. Single return value if there is one field.

    If this doesn't work, we raise ValueError/TypeError accordingly.
    """

    typ = typ._to_resolved()
    assert typ.kind == TypeKind.OBJECT, f"{typ!r} is not an Object"

    fields = typ._fields
    if isinstance(value_raw, tuple):
        coerced = ValueObject(typ, value={})
        if len(value_raw) != len(fields):
            raise ValueError(
                f"got {len(value_raw)} values for {typ!r}, expected {len(fields)}: {', '.join(f.name for f in fields)}"
            )
        for i, field in enumerate(fields):
            setattr(coerced, field.name, value_raw[i])
    elif isinstance(value_raw, dict):
        coerced = ValueObject(typ, value={})
        for field in fields:
            setattr(coerced, field.name, value_raw.get(field.name))
    elif isinstance(value_raw, ValueObject) and value_raw._type == typ:
        coerced = value_raw
    else:
        coerced = ValueObject(typ, value={})
        if len(fields) == 0:
            if value_raw is not None:
                raise ValueError(f"got value for {typ!r}, expected None")
        else:
            if len(fields) > 1:
                raise ValueError(
                    f"got single value for {typ!r}, need {len(fields)}: {', '.join(f.name for f in fields)}"
                )
            check_value(value_raw, fields[0], invalid=on_invalid_raise)
            setattr(coerced, fields[0].name, value_raw)

    return coerced

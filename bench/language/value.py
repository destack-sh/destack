import base64
import dataclasses
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Collection, Optional, Union, cast
from uuid import UUID

import structlog

from bench.language.const import (
    EnumType,
    PrimitiveType,
    PrimitiveValue,
    StructType,
    TypeKind,
    is_node_type,
    new_struct_id,
)
from bench.language.property import Property
from bench.language.setup import ENUM_CLASS_BY_TYPE
from bench.proto.monkey import _PatchedMessage
from bench.proto.wire import AnyStructData

if TYPE_CHECKING:
    from bench.language import Field, Node, NodeReference
    from bench.language.field import TypeInfoBase

logger = structlog.get_logger(__name__)

ScalarValue = Union["Object", PrimitiveValue, "Struct", "Node"]
SomeValue = Union[ScalarValue, Collection[ScalarValue]]
JsonPrimitive = Union[str, int, float, bool, None]
JsonValue = Union[JsonPrimitive, dict[str, "JsonValue"], list["JsonValue"]]

ValueParent = Union["Object", "Struct", "Node"]
ValueProperty = Union["Property", "Field"]


@dataclass(slots=True)
class Object:
    """
    Any user-defined Value that's not builtin (has nested fields), can also be partial.
    (e.g., the variable value of a Block, inputs to a Run, a Class instance).
    TODO :Incomplete: handle :SecretValues
    """

    # content
    _type: "TypeInfoBase"
    _value: dict[str, SomeValue] | None = None  # in unpacked form
    _is_revealed: bool = False

    # local identity (conforms with Struct protocol)
    id: int = dataclasses.field(default_factory=new_struct_id)
    parent: ValueParent | None = None
    parent_id: int | UUID | None = None
    parent_prop: ValueProperty | None = None
    ancestor_prop: Optional["Property"] = None
    parent_key: str | None = None
    order_key: str | None = None

    def __post_init__(self):
        if self.parent is not None and self.ancestor_prop is None:
            if type(self.parent_prop) is not Property:
                raise ValueError(f"{self.parent_prop!r} is not a Property")
            self.ancestor_prop = self.parent_prop

    @staticmethod
    def new(
        value: dict[str, SomeValue] | None,
        type: "TypeInfoBase",
        is_revealed: bool = True,
        parent: ValueParent | None = None,
        parent_property: ValueProperty | None = None,
    ) -> "Object":
        return Object(
            _type=type,
            _value=value,
            _is_revealed=is_revealed,
            parent=parent,
            parent_prop=parent_property,
        )

    def __str__(self) -> str:
        if self._value is None:
            return ""
        set_fields: list[str] = []
        for field in self._type._base_fields:
            field_value = self._value.get(field.storage_key)
            if field_value:
                if type(field_value) is list:
                    set_fields.append(f"{field.py_ident}({len(field_value)})")
                else:
                    set_fields.append(field.py_ident or field.name)
        return ", ".join(set_fields)

    def __repr__(self) -> str:
        if self._type.kind != TypeKind.OBJECT or self._type.base_type is None:
            if self._type._resolved_type is not None:
                type_name = cast(TypeKind, self._type._resolved_type.kind).bench_name
            elif self._type.kind is not None:
                type_name = self._type.kind.bench_name
            else:
                type_name = "?Value"
            return f"<{type_name} ({str(self)})>"
        else:
            type_name = self._type.base_type.absolute_path
            return f"<{type_name} ({str(self)})>"

    def equals_content(self, other: Any) -> bool:
        """Checks if all fields of the two Values are equal (recursively)."""
        if other is None or type(other) is not Object:
            return False
        elif self._value is None:
            return other._value is None
        elif other._value is None:
            return False
        for field in self._type._base_fields:
            if self._value.get(field.storage_key) != other._value.get(field.storage_key):
                return False
        return True

    def __eq__(self, other: Any) -> bool:
        return self.equals_content(other)

    def __getattr__(self, ident: str) -> SomeValue:
        # NOTE: __getattr__ is called only when ident is not in the slots, so this is a value lookup
        field = self._type._get_field(ident)
        if field is None:
            raise AttributeError(f"{self._type!r} has no field with identifier {ident}")
        if self._type.base_field_zone is not None and field.zone != self._type.base_field_zone:
            raise AttributeError(f"{field!r} is not in the same zone as {self._type!r}")
        if self._value is None:
            return field.default
        value = self._value.get(field.storage_key)
        # nocheckin: resolve against graph if value is node reference
        if value is None:
            return field.default
        return value

    def __setattr__(self, ident: str, value: SomeValue) -> None:
        # NOTE: __setattr__ is also called for slots so we have to check and set directly
        if ident in VALUE_SLOTS:
            return object.__setattr__(self, ident, value)
        field: Field | None = self._type._get_field(ident)
        if field is None:
            raise AttributeError(f"{self._type!r} has no field with identifier {ident}")
        if self._type.base_field_zone is not None and field.zone != self._type.base_field_zone:
            raise AttributeError(f"{field!r} is not in the same zone as {self._type!r}")

        # coerce & copy if needed
        value = coerce_value(value, field)
        # if field.kind == TypeKind.OBJECT or field.kind == TypeKind.STRUCT:
        #     if not field.is_list:
        #         value = cast("Object | Struct", value)._lazy_copy_to(self, field)
        #     else:
        #         value = ValueList._lazy_copy_for(cast(list["Object | Struct"], value), self, field)
        if self._value is None:
            self._value = {}
        self._value[field.storage_key] = value

        # notify
        self._updated_self((field,))

    def _lazy_copy_to(self, parent: ValueParent, prop: ValueProperty) -> "Object":
        raise NotImplementedError("nocheckin: _lazy_copy_to")

    def _updated_self(self, properties: tuple[Union["Property", "Field", Any], ...]) -> None:
        if self.parent is not None:
            prop = self.ancestor_prop if self.ancestor_prop is not None else self.parent_prop
            assert type(prop) is Property, f"{prop!r} is not a Property"
            self.parent._updated_self((prop,))


VALUE_SLOTS: set[str] = set(Object.__dataclass_fields__.keys())


def coerce_value(value: Any, typ: "TypeInfoBase") -> SomeValue:
    """
    Coerces the given value to the expected type (recursively). Returns value as is if already of correct type.
    To maintain clarity, we try to coerce as little as possible outside the typical python cases.
    Raises TypeError if not possible.
    NOTE :Performance: we re-create and copy lists during coercion even if the type was already good
    """
    if typ.kind == TypeKind.OBJECT:
        if not typ.is_list:
            return _coerce_object_scalar(cast(dict, value), typ)
        else:
            assert isinstance(value, list), f"{value!r} is not a list (expected {typ!r})"
            return [_coerce_object_scalar(cast(dict, element), typ) for element in value]
    else:
        if not typ.is_list:
            return _coerce_value_scalar(value, typ)
        else:
            assert isinstance(value, list), f"{value!r} is not a list (expected {typ!r})"
            return [_coerce_value_scalar(element, typ) for element in value]


def _coerce_value_scalar(value: ScalarValue, typ: "TypeInfoBase") -> ScalarValue:
    # coerce nodes to node references
    if typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
        if not isinstance(value, Struct):
            raise TypeError(f"{value!r} is not a Struct (expected {typ!r})")
        if is_node_type(value.metatype):
            value = cast("Node", value).to_ref()

    return value


def _coerce_object_scalar(value: dict, typ: "TypeInfoBase") -> Object:
    """Coerces a single object from a dict representation (recursively)."""
    raise NotImplementedError


def check_value(value: Any, typ: "TypeInfoBase") -> None:
    """
    Checks whether the given value has the expected type (recursively).
    Raises TypeError if not.
    """
    raise NotImplementedError


def _pack_value_scalar(value: ScalarValue, typ: "TypeInfoBase") -> JsonValue:
    """
    Packs the given scalar runtime value into a JSON-able representation.
    """
    if typ.kind == TypeKind.PRIMITIVE:
        if typ.primitive_type == PrimitiveType.BYTES:
            return base64.b64encode(cast(bytes, value)).decode()
        elif typ.primitive_type == PrimitiveType.DATETIME:
            return cast(datetime, value).isoformat()
        elif typ.primitive_type == PrimitiveType.INTERVAL:
            return cast(timedelta, value).total_seconds()
        else:
            return cast(JsonValue, value)
    elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
        assert (
            cast("Struct", value).metatype == StructType.NODE_REFERENCE
        ), f"{value!r} is not a NodeReference (expected {typ!r})"
        return cast("NodeReference", value)._to_data().to_robust_dict()
    elif typ.kind == TypeKind.ENUM:
        return cast(int, value)
    elif typ.kind == TypeKind.STRUCT:
        return cast(Struct, value)._to_data().to_robust_dict()
    else:
        raise TypeError(f"cannot pack value of type {typ!r}")


def _unpack_value_scalar(value_packed: JsonValue, typ: "TypeInfoBase") -> ScalarValue:
    """
    Unpacks the given scalar value into its runtime representation.
    """
    if typ.kind == TypeKind.PRIMITIVE:
        if typ.primitive_type == PrimitiveType.BYTES:
            return base64.b64decode(cast(str, value_packed))
        elif typ.primitive_type == PrimitiveType.DATETIME:
            return datetime.fromisoformat(cast(str, value_packed))
        elif typ.primitive_type == PrimitiveType.INTERVAL:
            return timedelta(seconds=cast(int, value_packed))
        else:
            return cast(PrimitiveValue, value_packed)
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        return enum_cls(cast(int, value_packed))
    elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
        from bench.proto import wiring

        assert isinstance(value_packed, dict), f"{value_packed!r} is not a dict (expected {typ!r})"
        data_cls = wiring.PROTO_CLASS_BY_TYPE[StructType.NODE_REFERENCE]
        value_struct = cast(_PatchedMessage, data_cls()).from_robust_dict(value_packed)
        return wiring.unpack_struct(cast(AnyStructData, value_struct))
    elif typ.kind == TypeKind.STRUCT:
        from bench.proto import wiring

        assert typ.bench_type is not None, f"unresolved type {typ!r}"
        assert isinstance(value_packed, dict), f"{value_packed!r} is not a dict (expected {typ!r})"
        data_cls = wiring.PROTO_CLASS_BY_TYPE[cast(StructType, typ.bench_type)]
        value_struct = cast(_PatchedMessage, data_cls()).from_robust_dict(value_packed)
        return wiring.unpack_struct(cast(AnyStructData, value_struct))
    else:
        raise TypeError(f"cannot unpack value of type {typ!r}")


def _pack_object_scalar(value: Object, typ: "TypeInfoBase") -> tuple[JsonValue, JsonValue | None]:
    """
    Packs an object value into a packed value & secret packed value.
    The secret split applies only to nested values within the type, not the type itself.
    """
    value_packed: dict[str, JsonValue] = {}
    for field in typ._base_fields:
        field_value = cast(SomeValue, getattr(value, field.name, None))
        if field_value is None:
            continue
        elif field.kind == TypeKind.OBJECT:
            value_packed[field.storage_key], _ = pack_value(field_value, field)
        elif not field.is_list:
            value_packed[field.storage_key] = _pack_value_scalar(
                cast(ScalarValue, field_value), field
            )
        else:  # scalar list
            assert isinstance(
                field_value, list
            ), f"{field_value!r} is not a list (expected {field!r})"
            value_packed[field.storage_key] = [
                _pack_value_scalar(element, field) for element in field_value
            ]
    return value_packed, None


def _unpack_object_scalar(
    value_packed: dict[str, JsonValue], secret_value_packed: JsonValue | None, typ: "TypeInfoBase"
) -> Object:
    """
    Unpacks an object value from a packed value & secret packed value.
    """
    value: dict[str, SomeValue] = {}
    for field in typ._base_fields:
        field_value_packed = value_packed.get(field.storage_key)
        if field_value_packed is None:
            continue
        elif field.kind == TypeKind.OBJECT:
            field_value = unpack_value(field_value_packed, None, field)
            if field_value is None:
                continue
        elif not field.is_list:
            field_value = _unpack_value_scalar(field_value_packed, field)
        else:  # scalar list
            assert isinstance(
                field_value_packed, list
            ), f"{field_value_packed!r} is not a list (expected {field!r})"
            field_value = [_unpack_value_scalar(element, field) for element in field_value_packed]
        value[field.storage_key] = field_value
    return Object.new(value, typ, is_revealed=secret_value_packed is not None)


def pack_value(
    value: SomeValue | None, typ: "TypeInfoBase", wrap_scalar: bool = True
) -> tuple[JsonValue, JsonValue | None]:
    """
    Packs a value into its constituent JSON-able parts (packed value & secret packed value).
    Only minimal type checks are performed, invalid values will error in various ways.
    TODO :Incomplete: handle :SecretValues
    """
    typ = typ._to_resolved()
    assert typ.kind != TypeKind.ALIAS, f"unresolved type {typ!r}"
    if typ.kind == TypeKind.OBJECT:
        # nested object
        if not typ.is_list:
            assert type(value) is Object, f"{value!r} is not an Object (expected {typ!r})"
            return _pack_object_scalar(value, typ)
        else:
            assert isinstance(value, list), f"{value!r} is not a list (expected {typ!r})"
            value_packed: JsonValue = []
            secret_value_packed: JsonValue = []
            for element in value:
                assert type(element) is Object, f"{element!r} is not an Object (expected {typ!r})"
                inner_value_packed, inner_secret_value_packed = _pack_object_scalar(element, typ)
                value_packed.append(inner_value_packed)
                secret_value_packed.append(inner_secret_value_packed)
            return value_packed, secret_value_packed
    else:
        # wrap scalar
        value_packed: JsonValue
        if value is None:
            value_packed = None  # no value
        elif not typ.is_list:
            value_packed = _pack_value_scalar(cast(ScalarValue, value), typ)
        else:
            assert isinstance(value, list), f"{value!r} is not a list (expected {typ!r})"
            value_packed = [_pack_value_scalar(element, typ) for element in value]
        if wrap_scalar:
            value_packed = {typ.identity_key: value_packed}
        return value_packed, None


def unpack_value(
    value_packed: JsonValue,
    secret_value_packed: JsonValue | None,
    typ: "TypeInfoBase",
    unwrap_scalar: bool = True,
) -> SomeValue | None:
    """
    Unpacks a value from its constituent JSON-able parts (packed value & secret packed value).
    Only minimal type checks are performed, invalid values will error in various ways.
    TODO :Incomplete: handle :SecretValues
    """
    typ = typ._to_resolved()
    assert typ.kind != TypeKind.ALIAS, f"unresolved type {typ!r}"
    if typ.kind == TypeKind.OBJECT:
        # nested object
        if not typ.is_list:
            assert isinstance(
                value_packed, dict
            ), f"{value_packed!r} is not a dict (expected {typ!r})"
            return _unpack_object_scalar(value_packed, secret_value_packed, typ)
        else:
            assert isinstance(
                value_packed, list
            ), f"{value_packed!r} is not a list (expected {typ!r})"
            return [
                _unpack_object_scalar(cast(dict[str, JsonValue], element), None, typ)
                for element in value_packed
            ]
    else:
        # unwrap scalar
        if unwrap_scalar and isinstance(value_packed, dict):
            value_packed = value_packed.get(typ.identity_key)
        if value_packed is None:
            return None
        elif not typ.is_list:
            return _unpack_value_scalar(value_packed, typ)
        else:
            assert isinstance(
                value_packed, list
            ), f"{value_packed!r} is not a list (expected {typ!r})"
            return [_unpack_value_scalar(element, typ) for element in value_packed]


# import later to avoid circular imports
from bench.language.node import Struct, struct_component  # noqa: E402


@struct_component()
class HasValues(Struct):
    pass

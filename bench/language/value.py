import base64
import dataclasses
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Collection, Optional, Union, cast
from uuid import UUID

import structlog

from bench.language.const import (
    EnumType,
    NodeType,
    PrimitiveType,
    PrimitiveValue,
    StructType,
    TypeKind,
)
from bench.language.node import Node, Property, Struct, new_struct_id, struct, struct_component
from bench.language.property import p_internal
from bench.language.setup import ENUM_CLASS_BY_TYPE
from bench.proto.wire import AnyStructData, NodeReferenceData

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Block,
        Branch,
        Environment,
        Field,
        NodeReference,
        Package,
        Trigger,
        User,
    )
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
    parent_key: str | None = None
    order_key: str | None = None

    @staticmethod
    def new(
        value: dict[str, SomeValue] | None, type: "TypeInfoBase", is_revealed: bool = True
    ) -> "Object":
        return Object(_type=type, _value=value, _is_revealed=is_revealed)

    def equals_content(self, other: Any) -> bool:
        """Checks if all fields of the two Values are equal (recursively)."""
        if other is None or type(other) is not Object:
            return False
        for field in self._type._resolved_fields:
            if getattr(self, field.storage_key) != getattr(other, field.storage_key):
                return False
        return True

    def __eq__(self, other: Any) -> bool:
        return self.equals_content(other)

    def __getattr__(self, ident: str) -> SomeValue:
        # NOTE: __getattr__ is called only when ident is not in the slots, so this is a value lookup
        field = self._type._resolve_field(ident)
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
        field: Field | None = self._type._resolve_field(ident)
        if field is None:
            raise AttributeError(f"{self._type!r} has no field with identifier {ident}")
        if self._type.base_field_zone is not None and field.zone != self._type.base_field_zone:
            raise AttributeError(f"{field!r} is not in the same zone as {self._type!r}")
        if self._value is None:
            self._value = {}
        self._value[field.storage_key] = value

    def _updated_self(self, properties: tuple[Union[Property, "Field", Any], ...]) -> None:
        raise NotImplementedError(":Incomplete")


VALUE_SLOTS: set[str] = set(Object.__dataclass_fields__.keys())


@struct_component()
class HasValues(Struct):
    pass


def coerce_value(value: Any, typ: "TypeInfoBase") -> SomeValue:
    """
    Coerces the given value to the expected type (recursively).
    Returns value as is if already correct.
    Raises TypeError if not possible.
    """
    raise NotImplementedError


def _coerce_value_scalar(value: ScalarValue, typ: "TypeInfoBase") -> ScalarValue:
    raise NotImplementedError


def _coerce_object_scalar(value: Object, typ: "TypeInfoBase") -> Object:
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
        return cast("NodeReference", value)._to_data().to_robust_json()
    elif typ.kind == TypeKind.ENUM:
        return cast(int, value)
    elif typ.kind == TypeKind.STRUCT:
        return cast(Struct, value)._to_data().to_robust_json()
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
    elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
        from bench.proto import wiring

        return wiring.unpack_struct(cast(NodeReferenceData, value_packed))
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        return enum_cls(cast(int, value_packed))
    elif typ.kind == TypeKind.STRUCT:
        return Struct._from_data(cast(AnyStructData, value_packed))
    else:
        raise TypeError(f"cannot unpack value of type {typ!r}")


def _pack_object_scalar(value: Object, typ: "TypeInfoBase") -> tuple[JsonValue, JsonValue | None]:
    """
    Packs an object value into a packed value & secret packed value.
    The secret split applies only to nested values within the type, not the type itself.
    """
    value_packed: dict[str, JsonValue] = {}
    for field in typ._resolved_fields:
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
    for field in typ._resolved_fields:
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


@struct(StructType.CONTEXT)
class Context(Struct):
    """A semi-magical value that accumulates context down the graph (starting with system context)."""

    # system
    bench: Optional["Bench"] = p_internal(30, require=False, array=False, references=NodeType.BENCH)
    environment: Optional["Environment"] = p_internal(
        31, require=False, array=False, references=NodeType.ENVIRONMENT
    )
    branch: Optional["Branch"] = p_internal(
        32, require=False, array=False, references=NodeType.BRANCH
    )
    package: Optional["Package"] = p_internal(
        33, require=False, array=False, references=NodeType.PACKAGE
    )
    module: Optional["Block"] = p_internal(
        34, require=False, array=False, references=NodeType.BLOCK
    )
    page: Optional["Block"] = p_internal(35, require=False, array=False, references=NodeType.BLOCK)

    user: Optional["User"] = p_internal(40, require=False, array=False, references=NodeType.USER)
    trigger: Optional["Trigger"] = p_internal(
        41, require=False, array=False, references=NodeType.TRIGGER
    )

    # custom
    # value_packed: Any = p_value_packed(50)
    # secret_value_packed: Any = p_secret_value_packed(51)
    # value: Any = p_value_runtime(50, 51)

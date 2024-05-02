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
        TypeInfo,
        User,
    )

logger = structlog.get_logger(__name__)

ScalarValue = Union["Value", PrimitiveValue, "Struct", "Node"]
SomeValue = Union[ScalarValue, Collection[ScalarValue]]
JsonPrimitive = Union[str, int, float, bool, None]
JsonValue = Union[JsonPrimitive, dict[str, "JsonValue"], list["JsonValue"]]


@dataclass(slots=True)
class Value:
    """
    Any user-defined non-primitive Value (unpacked).
    (e.g., the variable value of a Block, inputs to a Run, an Object instance).
    TODO :Incomplete: handle :SecretValues
    """

    # content
    _type: "TypeInfo"
    _value: dict[str, SomeValue] | None = None
    _is_revealed: bool = False

    # local identity (conforms with Struct protocol)
    id: int = dataclasses.field(default_factory=new_struct_id)
    parent: Union["Value", Struct, Node, None] = None
    parent_id: int | UUID | None = None
    parent_prop: Union[Property, "Field", None] = None
    parent_key: str | None = None
    order_key: str | None = None

    @staticmethod
    def new(value: dict[str, SomeValue] | None, type: "TypeInfo") -> "Value":
        return Value(_type=type, _value=value)

    def equals_content(self, other: Any) -> bool:
        """Checks if all fields of the two Values are equal (recursively)."""
        if other is None or type(other) is not Value:
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


VALUE_SLOTS: set[str] = set(Value.__dataclass_fields__.keys())


@struct_component()
class HasValues(Struct):
    pass


def coerce_value(value: Any, type: "TypeInfo") -> SomeValue:
    """
    Coerces the given value to the expected type (recursively).
    Returns value as is if already correct.
    Raises TypeError if not possible.
    """
    raise NotImplementedError


def check_value(value: Any, type: "TypeInfo") -> None:
    """
    Checks whether the given value has the expected type (recursively).
    Raises TypeError if not.
    """
    raise NotImplementedError


def pack_value_scalar(value: ScalarValue, type: "TypeInfo") -> JsonValue:
    """
    Packs the given scalar value into a JSON-able representation.
    """
    if type.kind == TypeKind.PRIMITIVE:
        if type.primitive_type == PrimitiveType.BYTES:
            return base64.b64encode(cast(bytes, value)).decode()
        elif type.primitive_type == PrimitiveType.DATETIME:
            return cast(datetime, value).isoformat()
        elif type.primitive_type == PrimitiveType.INTERVAL:
            return cast(timedelta, value).total_seconds()
        else:
            return cast(JsonValue, value)
    elif type.kind == TypeKind.NODE or type.kind == TypeKind.BASED_NODE:
        return cast("NodeReference", value)._to_data().to_robust_json()
    elif type.kind == TypeKind.ENUM:
        return cast(int, value)
    elif type.kind == TypeKind.STRUCT:
        return cast(Struct, value)._to_data().to_robust_json()
    else:
        raise TypeError(f"cannot pack value of type {type!r}")


def unpack_value_scalar(value_packed: JsonValue, type: "TypeInfo") -> ScalarValue:
    """
    Unpacks the given value into a Bench Value representation.
    """
    if type.kind == TypeKind.PRIMITIVE:
        if type.primitive_type == PrimitiveType.BYTES:
            return base64.b64decode(cast(str, value_packed))
        elif type.primitive_type == PrimitiveType.DATETIME:
            return datetime.fromisoformat(cast(str, value_packed))
        elif type.primitive_type == PrimitiveType.INTERVAL:
            return timedelta(seconds=cast(int, value_packed))
        else:
            return cast(PrimitiveValue, value_packed)
    elif type.kind == TypeKind.NODE or type.kind == TypeKind.BASED_NODE:
        from bench.proto import wiring

        return wiring.unpack_struct(cast(NodeReferenceData, value_packed))
    elif type.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, type.bench_type)]
        return enum_cls(cast(int, value_packed))
    elif type.kind == TypeKind.STRUCT:
        return Struct._from_data(cast(AnyStructData, value_packed))
    else:
        raise TypeError(f"cannot unpack value of type {type!r}")


def pack_value(value: SomeValue, type: "TypeInfo") -> tuple[JsonValue, JsonValue | None]:
    """
    Packs a value into its constituent JSON-able parts (packed value & secret packed value).
    TODO :Incomplete: handle :SecretValues
    """

    # if type.kind == TypeKind.ALIAS and type.base_type is not None and type.base_type.type in
    raise NotImplementedError("nocheckin: pack_value")


def unpack_value(
    value_packed: JsonValue, secret_value_packed: JsonValue | None, type: "TypeInfo"
) -> SomeValue:
    """
    Unpacks a value from its constituent JSON-able parts.
    TODO :Incomplete: handle :SecretValues
    """

    raise NotImplementedError("nocheckin: unpack_value")


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

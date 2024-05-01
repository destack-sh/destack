import dataclasses
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Collection, Optional, Union
from uuid import UUID

import structlog

from bench.language.const import NodeType, PrimitiveValue, StructType
from bench.language.node import Node, Property, Struct, new_struct_id, struct, struct_component
from bench.language.notice import NoticeHandler
from bench.language.property import p_internal
from bench.language.validation import ValidationHandler

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Block,
        Branch,
        Environment,
        Package,
        Session,
        Trigger,
        User,
    )
    from bench.language.field import Field, TypeInfo

logger = structlog.get_logger(__name__)

OneValue = Union["Value", PrimitiveValue, "Struct"]
ManyValue = Union[Collection["Value"], Collection[PrimitiveValue]]
SomeValue = Union[OneValue, ManyValue]


@dataclass(slots=True)
class Value:
    """
    Any user-defined non-primitive Value (unpacked).
    (e.g., the variable value of a Block, inputs to a Run, an Object instance).
    """

    # content
    _type: "TypeInfo"
    _value: dict[str, SomeValue] | None = None

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
    def _validate_component(
        self, properties: Collection[Property], on_invalid: "ValidationHandler"
    ) -> None:
        pass

    def _interp_component(self, scope: Optional["Node"], on_notice: "NoticeHandler") -> None:
        pass

    def _track_component(self, session: "Session") -> None:
        pass

    def _untrack_component(self) -> None:
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


def pack_value(value: SomeValue, type: "TypeInfo") -> tuple[Any, Any | None]:
    """
    Packs/serializes the given value into a JSON-able representation (and a 'secret' one if required).
    Raises TypeError if there is a type mismatch.
    """
    # if type.kind == TypeKind.ALIAS and type.base_type is not None and type.base_type.type in
    raise NotImplementedError("nocheckin: pack_value")


def unpack_value(value_packed: Any, secret_value_packed: Any | None, type: "TypeInfo") -> SomeValue:
    """
    Unpacks/deserializes the given value into a Bench Value representation.
    Raises TypeError if there is a type mismatch.
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

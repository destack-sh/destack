import abc
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    cast,
    dataclass_transform,
)

import structlog
from opentelemetry import trace

from destack.language.registry import STRUCT_CLASS_BY_TYPE, STRUCT_TYPE_BY_CLASS
from destack.proto import AnyStructProto

from .const import StructType
from .object import BuiltinObjectBase, BuiltinObjectFrozen, BuiltinObjectMutable, object_
from .property import _PROPERTY_SPECIFIERS, property_runtime_

if TYPE_CHECKING:
    from destack.language import Json, StructDefinition

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)
type_ = type


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_struct[ObjectT: BuiltinObjectBase](struct_type: StructType, frozen: bool = False):
    """Register a class as a concrete struct for the given struct type."""

    def decorate(cls: type[ObjectT]) -> type[ObjectT]:
        cls = object_(object_type=struct_type, concrete=True, struct=True, frozen=frozen)(cls)

        # register struct
        if struct_type:
            assert issubclass(cls, StructBase), f"struct class {cls} is not a StructBase"
            cls.metatype = struct_type
            if struct_type in STRUCT_CLASS_BY_TYPE:
                raise ValueError(
                    f"struct class conflict for {struct_type}: {cls}, {STRUCT_CLASS_BY_TYPE[struct_type]}"
                )
            STRUCT_CLASS_BY_TYPE[struct_type] = cls
            STRUCT_TYPE_BY_CLASS[cls] = struct_type

        return cast(type[ObjectT], cls)

    return decorate


class StructBase[StructProtoT: AnyStructProto](BuiltinObjectBase[StructProtoT], abc.ABC):
    """A Struct is an ordered collection of Properties."""

    metatype: ClassVar[StructType]
    __definition__: ClassVar["StructDefinition"]

    __is_struct__: ClassVar[bool] = True

    __slots__ = ()

    # _value?

    def __eq__(self, other: Any):
        """Equals the Struct contents."""
        raise NotImplementedError  # generated


@object_()
class StructMutable[StructProtoT: AnyStructProto](
    StructBase[StructProtoT], BuiltinObjectMutable[StructProtoT]
):
    """A mutable Struct."""

    pass


@object_(frozen=True)
class StructFrozen[StructProtoT: AnyStructProto](
    StructBase[StructProtoT], BuiltinObjectFrozen[StructProtoT]
):
    """A frozen Struct."""

    # cached for frozen Structs
    _hash: "int | None" = property_runtime_()
    _repr: "str | None" = property_runtime_()
    _proto: "StructProtoT | None" = property_runtime_()
    _value: "Json | None" = property_runtime_()

    def _invalidate_frozen_cache(self) -> None:
        # frozen Structs should be immutable, but sometimes we need to break out of that
        object.__setattr__(self, "_hash", None)
        object.__setattr__(self, "_proto", None)
        object.__setattr__(self, "_value", None)
        object.__setattr__(self, "_repr", None)

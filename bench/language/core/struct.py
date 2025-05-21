import abc
from typing import (
    TYPE_CHECKING,
    ClassVar,
    cast,
    dataclass_transform,
    final,
)

import structlog
from opentelemetry import trace

from bench.pb2 import AnyStructData
from bench.utils.env import IS_DEV

from .const import StructType
from .object import BuiltinObject, object_
from .property import _PROPERTY_SPECIFIERS

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def struct_[_ObjectT: BuiltinObject](struct_type: StructType):
    """Register a class as a concrete struct for the given struct type."""

    def decorate(cls: type[_ObjectT]) -> type[_ObjectT]:
        cls = object_(struct_type=struct_type, is_concrete=True, is_struct=True)(cls)
        if IS_DEV and cls.__name__ != "Struct" and cls.__name__ != "Struct":
            if not issubclass(cls, (Struct, Struct)):
                raise ValueError(f"{cls} is not a struct")

        return cast(type[_ObjectT], cls)

    return decorate


@object_()
class Struct[StructDataT: AnyStructData](BuiltinObject[StructDataT], abc.ABC):
    """A Struct is an ordered collection of Properties."""

    metatype: ClassVar[StructType]  # type: ignore

    __is_struct__: ClassVar[bool] = True

    @final
    def __repr__(self):
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str}>"
        else:
            return f"<{self.__class__.__name__}>"

import abc
from typing import (
    TYPE_CHECKING,
    ClassVar,
    Sequence,
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
        cls = object_(struct_type=struct_type, is_final=True, is_struct=True)(cls)
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

    def __content_str__(self) -> str:
        # default __content_str__ for Structs with all set properties
        value_strs = []
        for prop in self.__declared_properties__.values():
            prop_value = getattr(self, prop.name)
            if prop_value is not None and not (isinstance(prop_value, Sequence) and not prop_value):
                if prop.is_enum:
                    if prop.is_list:
                        prop_value_str = "|".join(p.bench_name for p in prop_value)
                    else:
                        prop_value_str = prop_value.bench_name  # type: ignore
                elif prop.reference_struct:
                    if prop.is_list:
                        prop_value_str = f"{prop.reference_struct.bench_name}[{len(prop_value)}]"
                    else:
                        prop_value_str = f"<{prop.reference_struct.bench_name} ...>"
                else:
                    prop_value_str = repr(prop_value)
                value_strs.append(f"{prop.name}={prop_value_str}")
        return ", ".join(value_strs)

    @final
    def __repr__(self):
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str}>"
        else:
            return f"<{self.__class__.__name__}>"

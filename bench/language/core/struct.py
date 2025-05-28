import abc
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Optional,
    Self,
    cast,
    dataclass_transform,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench import pb2
from bench.language.registry import BUILTIN_OBJECT_CLASS_BY_TYPE
from bench.pb2 import AnyStructData, NodeReferenceData, ScopeData

from .const import NodeType, Region, StructType
from .object import BuiltinObject, object_
from .property import _PROPERTY_SPECIFIERS, Property, property_, property_runtime_

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def struct_[_ObjectT: BuiltinObject](struct_type: StructType, is_frozen: bool = False):
    """Register a class as a concrete struct for the given struct type."""

    def decorate(cls: type[_ObjectT]) -> type[_ObjectT]:
        cls = object_(
            struct_type=struct_type, is_concrete=True, is_struct=True, is_frozen=is_frozen
        )(cls)
        return cast(type[_ObjectT], cls)

    return decorate


@object_()
class Struct[StructDataT: AnyStructData](BuiltinObject[StructDataT], abc.ABC):
    """A Struct is an ordered collection of Properties."""

    metatype: ClassVar[StructType]  # type: ignore

    __is_struct__: ClassVar[bool] = True

    _proto: "StructDataT | None" = property_runtime_()  # cached for frozen Structs
    # _value?

    def replace(self, **kwargs: Any) -> Self:
        """Replace the properties of the Struct with the given values."""
        raise NotImplementedError


@struct_(StructType.SCOPE, is_frozen=True)
class Scope(Struct[ScopeData]):
    """The scope in the Bench graph."""

    region: Optional[Region] = property_(31, is_repr=True)
    bench_id: Optional[UUID] = property_(32, is_repr=True)


EMPTY_SCOPE_DATA = pb2.ScopeData(metatype=pb2.StructType.STRUCT_TYPE_SCOPE)


@struct_(StructType.PROPERTY_REFERENCE, is_frozen=True)
class PropertyReference(Struct):
    """
    A reference to a builtin object's Property.
    If type is unset, this refers to a base property in one of the base BuiltinObject types.
    """

    node_type: NodeType | None = property_(31, is_repr=True)
    struct_type: StructType | None = property_(32, is_repr=True)
    id: int = property_(33, is_repr=True)

    @property
    def object_cls(self) -> type[BuiltinObject] | None:
        if self.node_type is not None:
            return BUILTIN_OBJECT_CLASS_BY_TYPE.get(self.node_type)
        elif self.struct_type is not None:
            return BUILTIN_OBJECT_CLASS_BY_TYPE.get(self.struct_type)
        else:
            return None

    def resolve_or_error(self) -> Property:
        """Resolves the property reference to a Property."""
        resolved = self.resolve()
        if resolved is None:
            raise ValueError(f"could not resolve {self!r}")
        return resolved

    def resolve(self) -> Property | None:
        """Resolves the property reference to a Property."""
        from .node import Node

        object_cls = self.object_cls
        prop = (object_cls or Node).__properties_by_id__.get(self.id)
        return prop


@struct_(StructType.NODE_REFERENCE, is_frozen=True)
class NodeReference(Struct[NodeReferenceData]):
    """
    A reference to a Node.
    """

    node_type: NodeType = property_(31, is_repr=True)
    id: UUID = property_(32, is_repr=True)
    ck: Optional[UUID] = property_(33, is_repr=True)
    bench_id: Optional[UUID] = property_(34, is_repr=True)
    table_id: Optional[UUID] = property_(35, is_repr=True)
    # area? external_id?

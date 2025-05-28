import abc
from typing import (
    TYPE_CHECKING,
    ClassVar,
    Optional,
    cast,
    dataclass_transform,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench import pb2
from bench.language.registry import BUILTIN_OBJECT_CLASS_BY_TYPE
from bench.pb2 import AnyStructData, NodeReferenceData, PropertyReferenceData, ScopeData

from .const import NodeType, Region, StructType
from .object import BuiltinObjectBase, BuiltinObjectFrozen, BuiltinObjectMutable, object_
from .property import _PROPERTY_SPECIFIERS, Property, property_, property_runtime_

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def struct_[_ObjectT: BuiltinObjectBase](struct_type: StructType, frozen: bool = False):
    """Register a class as a concrete struct for the given struct type."""

    def decorate(cls: type[_ObjectT]) -> type[_ObjectT]:
        cls = object_(struct_type=struct_type, concrete=True, struct=True, frozen=frozen)(cls)
        return cast(type[_ObjectT], cls)

    return decorate


class StructBase[StructDataT: AnyStructData](BuiltinObjectBase[StructDataT], abc.ABC):
    """A Struct is an ordered collection of Properties."""

    metatype: ClassVar[StructType]  # type: ignore

    __is_struct__: ClassVar[bool] = True

    # _value?


@object_()
class StructMutable[StructDataT: AnyStructData](
    StructBase[StructDataT], BuiltinObjectMutable[StructDataT]
):
    """A mutable Struct."""

    pass


@object_(frozen=True)
class StructFrozen[StructDataT: AnyStructData](
    StructBase[StructDataT], BuiltinObjectFrozen[StructDataT]
):
    """A frozen Struct."""

    _proto: "StructDataT | None" = property_runtime_()  # cached for frozen Structs


@struct_(StructType.SCOPE, frozen=True)
class Scope(StructFrozen[ScopeData]):
    """The scope in the Bench graph."""

    region: Optional[Region] = property_(31, is_repr=True)
    bench_id: Optional[UUID] = property_(32, is_repr=True)


EMPTY_SCOPE_DATA = pb2.ScopeData(metatype=pb2.StructType.STRUCT_TYPE_SCOPE)


@struct_(StructType.PROPERTY_REFERENCE, frozen=True)
class PropertyReference(StructFrozen[PropertyReferenceData]):
    """
    A reference to a builtin object's Property.
    If type is unset, this refers to a base property in one of the base BuiltinObject types.
    """

    node_type: NodeType | None = property_(31, is_repr=True)
    struct_type: StructType | None = property_(32, is_repr=True)
    id: int = property_(33, is_repr=True)

    @property
    def object_cls(self) -> type[BuiltinObjectBase] | None:
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


@struct_(StructType.NODE_REFERENCE, frozen=True)
class NodeReference(StructFrozen[NodeReferenceData]):
    """
    A reference to a Node.
    """

    node_type: NodeType = property_(31, is_repr=True)
    id: UUID = property_(32, is_repr=True)
    ck: Optional[UUID] = property_(33, is_repr=True)
    bench_id: Optional[UUID] = property_(34, is_repr=True)
    table_id: Optional[UUID] = property_(35, is_repr=True)
    # area? external_id?

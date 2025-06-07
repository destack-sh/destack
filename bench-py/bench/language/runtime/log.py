from typing import TYPE_CHECKING

from fastuuid import UUID

from bench.language.core import (
    UNSET,
    EditOperation,
    EditType,
    HasEnvironment,
    IsInPackage,
    IsLog,
    Node,
    NodeReference,
    NodeType,
    Property,
    PropertyReference,
    node_,
    property_,
)

if TYPE_CHECKING:
    from bench.language import Field, Value

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.LOG)
class Log(HasEnvironment, IsLog, IsInPackage, Node):
    """A Log."""

    pass


@node_(NodeType.EDIT_LOG)
class EditLog(HasEnvironment, IsLog, IsInPackage, Node):
    """A Log of an Edit."""

    # key
    type: EditType = property_(30, is_repr=True)
    operation: EditOperation | None = property_(31, is_repr=True)
    node: Node = property_(32, is_repr=True)
    prop: Property | None = property_(33, is_repr=True)
    field: "Field | None" = property_(34, is_repr=True)  # for IsExtensible.value
    key: "Value | None" = property_(35, is_repr=True)  # for map operations
    if TYPE_CHECKING:
        node_id: UUID = UNSET
        node_ptr: NodeReference = UNSET
        node_type: NodeType = UNSET
        field_id: UUID | None = None
        field_ptr: NodeReference | None = None
        property_ptr: PropertyReference | None = None

    # value
    value: "Value | None" = property_(40)


@node_(NodeType.CHANGE_LOG)
class ChangeLog(HasEnvironment, IsLog, IsInPackage, Node):
    """A Log of a Change."""

    pass


@node_(NodeType.QUERY_LOG)
class QueryLog(HasEnvironment, IsLog, IsInPackage, Node):
    """A Log of a Query."""

    pass

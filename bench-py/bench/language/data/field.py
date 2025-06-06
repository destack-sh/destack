from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    BuiltinEnum,
    CascadeAction,
    EdgeType,
    EnumType,
    HasEnvironment,
    HasIcon,
    HasName,
    IntoQuery,
    IsArchivable,
    IsDeletable,
    IsInPackage,
    IsOrdered,
    IsTemplatable,
    Node,
    NodeType,
    TypeBase,
    enum_,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import FieldData

if TYPE_CHECKING:
    from bench.language import Action, Agent, CustomNodeDefinition, IsView, Scene, Schema


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@enum_(EnumType.FIELD_TYPE)
class FieldType(BuiltinEnum):
    MEMBER = 1, "Member", "Member", "fas fa-arrow-down"
    INPUT = 2, "Input", "Input", "fas fa-arrow-down"
    OUTPUT = 3, "Output", "Output", "fas fa-arrow-up"


@node_(NodeType.FIELD)
class Field(
    HasEnvironment,
    HasName,
    HasIcon,
    IsTemplatable,
    IsOrdered,
    IsDeletable,
    IsArchivable,
    IsInPackage,
    IntoQuery,
    TypeBase,
    Node[FieldData],
):
    """
    A Field is a user-defined attribute.
    """

    parent: Union["Agent", "Action", "Schema", "CustomNodeDefinition", "Scene", "IsView", None] = (
        property_parent_()
    )
    type: FieldType = property_(30, default=FieldType.MEMBER)

    # type identity
    # ...TypeBase[40-69]

    # relationship (to TypeBase.table)
    edge_type: Optional[EdgeType] = property_(70)
    cascade: Optional[CascadeAction] = property_(71)

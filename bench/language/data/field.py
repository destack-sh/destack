from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    BuiltinEnum,
    CascadeAction,
    EdgeType,
    EnumType,
    HasIcon,
    HasName,
    IntoQuery,
    IsArchivable,
    IsDeletable,
    IsInPackage,
    IsInstantiable,
    IsModal,
    IsOrdered,
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
    from bench.language import Action, Agent, Flow, IsView, Scene, Schema, Table


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@enum_(EnumType.FIELD_TYPE)
class FieldType(BuiltinEnum):
    """The type of a Field within its Block. Overlaps with ObjectKind."""

    VARIABLE = 1, "Variable", "Variable", "fas fa-arrow-down"
    INPUT = 2, "Input", "Input", "fas fa-arrow-down"
    OUTPUT = 3, "Output", "Output", "fas fa-arrow-up"


@node_(NodeType.FIELD)
class Field(
    IsInstantiable,
    IsModal,
    HasName,
    IsOrdered,
    IsDeletable,
    IsArchivable,
    IsInPackage,
    IntoQuery,
    HasIcon,
    TypeBase,
    Node[FieldData],
):
    """
    A Field is a user-defined attribute.
    """

    parent: Union["Agent", "Action", "Schema", "Flow", "Table", "Scene", "IsView", None] = (
        property_parent_()
    )
    type: FieldType = property_(30)

    # type identity
    # ...TypeBase[40-69]

    # relationship (to TypeBase.table)
    edge_type: Optional[EdgeType] = property_(70)
    cascade: Optional[CascadeAction] = property_(71)

    def __eq__(self, other):  # type: ignore
        return IntoQuery.__eq__(self, other)  # override to avoid recursion

    __hash__ = IsInPackage.__hash__  # type: ignore

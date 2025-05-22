from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    BuiltinEnum,
    Constraint,
    EnumType,
    IntoQuery,
    IsArchivable,
    IsDeletable,
    IsInPackage,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOrdered,
    Node,
    NodeType,
    Property,
    TypeBase,
    TypeIn,
    encode_storage_key,
    enum_,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
    p_runtime,
    to_type_scalar,
)
from bench.pb2 import FieldData

if TYPE_CHECKING:
    from bench.language import Action, Agent, Flow, Icon, IsView, Scene, Schema, Table


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@enum_(EnumType.CASCADE_ACTION)
class CascadeAction(BuiltinEnum):
    CASCADE = 1
    SELF = 2
    NONE = 3


@enum_(EnumType.FIELD_TYPE)
class FieldType(BuiltinEnum):
    """The type of a Field within its Block. Overlaps with ObjectKind."""

    VARIABLE = 20, "Variable", "Variable", "fas fa-arrow-down"
    INPUT = 30, "Input", "Input", "fas fa-arrow-down"
    OUTPUT = 40, "Output", "Output", "fas fa-arrow-up"


@node_(NodeType.FIELD)
class Field(
    IsInstantiable,
    IsModal,
    IsNamed,
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

    parent: Union["Agent", "Action", "Schema", "Flow", "Table", "Scene", "IsView", None] = (
        p_node_parent()
    )
    type: FieldType = p_internal(30)
    icon: Optional["Icon"] = p_regular(35)

    # type identity
    # ...TypeBase[40-69]

    cascade: Optional[CascadeAction] = p_regular(70)

    _introspected_from: Optional[Property] = p_runtime(default=None)  # should match Field.property

    def __content_str__(self) -> str:
        return TypeBase.__content_str__(self)

    def __eq__(self, other):  # type: ignore
        return IntoQuery.__eq__(self, other)  # override to avoid recursion

    __hash__ = IsInPackage.__hash__  # type: ignore

    @property
    def type_info(self) -> TypeBase:
        return self

    @property
    def storage_key(self) -> str:
        return encode_storage_key(self)

    key = storage_key

    @staticmethod
    def new(
        name: str,
        type: FieldType,
        typ: TypeIn,
        constraint: Constraint | None = None,
        **kwargs,
    ) -> "Field":
        typ = to_type_scalar(typ)
        for prop in TypeBase.__declared_properties__.values():
            if prop.name not in kwargs:
                kwargs[prop.name] = getattr(typ, prop.name)
        if constraint is not None:
            kwargs["constraint"] = constraint
        field = Field(name=name, type=type, **kwargs)
        return field

    @staticmethod
    def member(
        name: str,
        typ: TypeIn,
        constraint: Constraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, type=FieldType.VARIABLE, typ=typ, constraint=constraint, **kwargs)

    @staticmethod
    def input(
        name: str,
        typ: TypeIn,
        constraint: Constraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, type=FieldType.INPUT, typ=typ, constraint=constraint, **kwargs)

    @staticmethod
    def output(
        name: str,
        typ: TypeIn,
        constraint: Constraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, type=FieldType.OUTPUT, typ=typ, constraint=constraint, **kwargs)

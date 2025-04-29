from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    FieldType,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsTemplatable,
    IsType,
    NodeMode,
    NodeType,
    PackageNode,
    Property,
    PropertyReference,
    StructType,
    TypeConstraint,
    TypeConstraintIn,
    TypeIn,
    _IntoQuery,
    encode_storage_key,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
    p_runtime,
    to_type_scalar,
)
from bench.pb2 import FieldData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Action, Agent, Class, Database, Flow, Icon, Thread


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

property_ = property


@node_(NodeType.FIELD)
class Field(
    IsInstantiable,
    IsTemplatable,
    IsModal,
    IsNamed,
    IsType,
    PackageNode[FieldData],
    _IntoQuery,
):
    """
    A Field is a user-defined attribute.
    """

    parent: Union["Thread", "Agent", "Action", "Class", "Flow", "Database", None] = p_node_parent(
        4,
        NodeType.THREAD,
        NodeType.AGENT,
        NodeType.ACTION,
        NodeType.CLASS,
        NodeType.FLOW,
        NodeType.DATABASE,
    )
    type: FieldType = p_internal(30)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    icon: Optional["Icon"] = p_regular(35, require=False, array=False, struct=StructType.ICON)
    property: Optional[Property] = p_regular(
        36,
        require=False,
        default=None,
        array=False,
        struct=StructType.PROPERTY_REFERENCE,
        description="The Property the Field refers to (for builtin Fields).",
    )
    if TYPE_CHECKING:
        property_ptr: Optional["PropertyReference"] = None

    # type identity
    # ...TypeInfo[40-69]

    _introspected_from: Optional[Property] = p_runtime(default=None)  # should match Field.property

    def __content_str__(self) -> str:
        return IsType.__content_str__(self)

    def __eq__(self, other):  # type: ignore
        return _IntoQuery.__eq__(self, other)  # override to avoid recursion

    __hash__ = PackageNode.__hash__  # type: ignore

    @property_
    def type_info(self) -> IsType:
        return self

    @property_
    def storage_key(self) -> str:
        return encode_storage_key(self)

    key = storage_key

    @staticmethod
    def new(
        name: str,
        type: FieldType,
        typ: TypeIn | Property,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        if isinstance(typ, Property):
            property = typ
            typ = typ.type_info
            mode = NodeMode.BUILTIN
        else:
            property = None
            mode = None
        typ = to_type_scalar(typ)
        for prop in IsType.__declared_properties__.values():
            if prop.name not in kwargs:
                kwargs[prop.name] = getattr(typ, prop.name)
        if constraint is not None:
            if isinstance(constraint, TypeConstraintIn):
                constraint = constraint.into()
            kwargs["constraint"] = constraint
        field = Field(name=name, type=type, property=property, **kwargs)
        if mode is not None:
            field.mode = mode
        return field

    @staticmethod
    def member(
        name: str,
        typ: TypeIn | Property,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, type=FieldType.MEMBER, typ=typ, constraint=constraint, **kwargs)

    @staticmethod
    def input(
        name: str,
        typ: TypeIn,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, type=FieldType.INPUT, typ=typ, constraint=constraint, **kwargs)

    @staticmethod
    def output(
        name: str,
        typ: TypeIn,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, type=FieldType.OUTPUT, typ=typ, constraint=constraint, **kwargs)

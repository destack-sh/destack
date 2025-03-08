import typing
from typing import Optional, Union

from bench.language.core import (
    FIELD_BASE_NODE_TYPES,
    NAME_CONSTRAINT,
    FieldBaseNode,
    FieldType,
    InlineNode,
    IsInstantiable,
    IsTemplatable,
    IsTraceable,
    NodeType,
    PackageNode,
    Property,
    StructType,
    TypeBase,
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

if typing.TYPE_CHECKING:
    from bench.language import Icon, Text


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@node_(NodeType.FIELD, has_subtypes=True)
class Field(
    IsInstantiable, IsTemplatable, IsTraceable, PackageNode[FieldData], TypeBase, _IntoQuery
):
    """
    A custom attribute of some value, the user-defined counterpart to Properties in BuiltinObjects.
    """

    parent: Union[FieldBaseNode, None] = p_node_parent(4, *FIELD_BASE_NODE_TYPES)
    type: FieldType = p_internal(30)
    name: str | None = p_regular(31, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(32, default=INTEGER_ZERO)
    text: Optional["Text"] = p_regular(
        33, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(34, require=False, array=False, struct=StructType.ICON)

    # type identity
    # ...TypeInfo[40-69]

    _introspected_from: Optional[Property] = p_runtime(default=None)

    def __content_str__(self) -> str:
        return TypeBase.__content_str__(self)

    def __eq__(self, other):  # type: ignore
        return _IntoQuery.__eq__(self, other)  # override to avoid recursion

    __hash__ = InlineNode.__hash__  # type: ignore
    # (not entirely sure why we need to override Field.__hash__ but not for any other node, maybe
    #  one of the base structs takes precende for some reason (but SourceNode is first in MRO...))

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
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        typ = to_type_scalar(typ)
        for prop in TypeBase.__declared_properties__.values():
            if prop.name not in kwargs:
                kwargs[prop.name] = getattr(typ, prop.name)
        if constraint is not None:
            if isinstance(constraint, TypeConstraintIn):
                constraint = constraint.into()
            kwargs["constraint"] = constraint
        field = Field(name=name, type=type, **kwargs)
        return field

    @staticmethod
    def resource(
        name: str,
        typ: TypeIn,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, type=FieldType.RESOURCE, typ=typ, constraint=constraint, **kwargs)

    @staticmethod
    def member(
        name: str,
        typ: TypeIn,
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

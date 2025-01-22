import typing
from typing import Optional, Union, cast
from uuid import UUID

from bench.language.core import (
    NAME_CONSTRAINT,
    FieldType,
    HasNodeBase,
    Node,
    NodeType,
    Property,
    SourceNode,
    StructType,
    TypeBase,
    TypeConstraint,
    TypeConstraintIn,
    TypeIn,
    TypeKind,
    _IntoQuery,
    encode_storage_key,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
    p_runtime,
    subnode_,
    to_type,
    to_type_scalar,
)
from bench.pb2 import AnyNodeData, FieldData, NodeReferenceData
from bench.utils.fractional import INTEGER_ZERO

if typing.TYPE_CHECKING:
    from bench.language import Action, Block, Icon, Text


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@node_(NodeType.FIELD, has_subtypes=True)
class Field(SourceNode[FieldData], HasNodeBase, TypeBase, _IntoQuery):
    """
    A custom attribute of some value, the user-defined counterpart to Properties in builtin objects.
    """

    parent: Union["Block", "Action", None] = p_node_parent(4, NodeType.BLOCK, NodeType.ACTION)
    type: FieldType = p_internal(30, default=FieldType.VARIABLE)
    name: str = p_regular(31, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(32, default=INTEGER_ZERO)
    text: Optional["Text"] = p_regular(
        33, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(34, require=False, array=False, struct=StructType.ICON)

    # type identity
    # ...TypeInfo[40-69]

    _introspected_from: Optional[Property] = p_runtime(default=None)

    def __content_str__(self) -> str:
        if self.type == FieldType.OPTION:
            return ""  # nothing to show
        else:
            return TypeBase.__content_str__(self)

    def __eq__(self, other):  # type: ignore
        return _IntoQuery.__eq__(self, other)  # override to avoid recursion

    __hash__ = SourceNode.__hash__  # type: ignore
    # (not entirely sure why we need to override Field.__hash__ but not for any other node, maybe
    #  one of the base structs takes precende for some reason (but SourceNode is first in MRO...))

    @property
    def base(self) -> Optional[Node]:
        return self.parent

    @property
    def base_ck(self) -> Optional[UUID]:
        return self.base.ck if self.base is not None else None

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return (cast(FieldData, data)).parent_ptr

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
        field = Field(name=name, **kwargs)
        return field

    @staticmethod
    def option(name: str, **kwargs) -> "Field":
        return Field(kind=TypeKind.LITERAL, type=FieldType.OPTION, name=name, **kwargs)

    @staticmethod
    def variable(
        name: str,
        typ: TypeIn,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        typ = to_type(typ, is_required=True, is_list=False)  # Variables are required by default
        return Field.new(name, typ, type=FieldType.VARIABLE, constraint=constraint, **kwargs)

    @staticmethod
    def member(
        name: str,
        typ: TypeIn,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, typ, type=FieldType.MEMBER, constraint=constraint, **kwargs)

    @staticmethod
    def input(
        name: str,
        typ: TypeIn,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, typ, type=FieldType.INPUT, constraint=constraint, **kwargs)

    @staticmethod
    def output(
        name: str,
        typ: TypeIn,
        constraint: TypeConstraintIn | TypeConstraint | None = None,
        **kwargs,
    ) -> "Field":
        return Field.new(name, typ, type=FieldType.OUTPUT, constraint=constraint, **kwargs)


@subnode_(FieldType.INPUT)
class InputField(Field):
    """An input field."""

    # generation_options, ...?
    pass


@subnode_(FieldType.OUTPUT)
class OutputField(Field):
    """An output field."""

    # generation_options, ...?
    pass


@subnode_(FieldType.MEMBER)
class MemberField(Field):
    """A member field."""

    # member-only flags
    # is_stored/is_computed, is_indexed, is_unique, ...
    # resource_options, ...?
    pass


@subnode_(FieldType.VARIABLE)
class VariableField(Field):
    """A variable field."""

    pass


@subnode_(FieldType.OPTION)
class OptionField(Field):
    """An option field."""

    pass

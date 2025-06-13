from typing import TYPE_CHECKING, Optional

from ..builtin import (
    CascadeAction,
    EdgeType,
    Enum,
    EnumType,
    NodeType,
    Property,
    StructBase,
    StructFrozen,
    StructType,
    TraitType,
    builtin_struct,
    property_,
)
from .type import (
    CollectionConstraint,
    DefaultFactory,
    NodeConstraint,
    NumberConstraint,
    PrimitiveType,
    ScalarType,
    StringConstraint,
    TypeCardinality,
)

if TYPE_CHECKING:
    from destack.language import Icon, Node, Type, Value


_type = type


@builtin_struct(StructType.PROPERTY_INFO, frozen=True)
class PropertyInfo(StructFrozen):
    """Information about a Property."""

    id: int = property_(2)
    name: str = property_(31)
    icon: "Icon | None" = property_(34)
    description: str | None = property_(36)

    # scalar
    cardinality: TypeCardinality = property_(40, default=TypeCardinality.SCALAR, is_repr=True)
    scalar_type: ScalarType = property_(41, is_repr=True)
    primitive_type: Optional[PrimitiveType] = property_(42, is_repr=True)
    enum_type: Optional[EnumType] = property_(43, is_repr=True)
    node_type: Optional[NodeType] = property_(44, is_repr=True)
    struct_type: Optional[StructType] = property_(46, is_repr=True)
    key_type: Optional["Type"] = property_(48, is_repr=True)  # for maps

    # meta
    is_required: bool | None = property_(50)
    is_variable: bool | None = property_(51)
    # is_external
    default: Optional["Value"] = property_(55)
    default_factory: Optional[DefaultFactory] = property_(56)

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = property_(60)
    string_constraint: Optional["StringConstraint"] = property_(61)
    number_constraint: Optional["NumberConstraint"] = property_(62)
    node_constraint: Optional["NodeConstraint"] = property_(63)

    node_is_customizable: bool = property_(73)
    edge_type: EdgeType | None = property_(74)
    cascade: CascadeAction | None = property_(75)

    is_wired: bool = property_(80)
    is_stored: bool = property_(81)
    is_repr: bool = property_(82)
    is_hash: bool = property_(83)
    is_eq: bool = property_(84)
    is_managed: bool = property_(85)
    is_computed: bool = property_(86)

    @classmethod
    def from_property(cls, property: Property) -> "PropertyInfo":
        """Create PropertyInfo from a Property."""
        assert property.id is not None, f"{property!r} has no id"
        type = property._to_type()

        return cls(
            id=property.id,
            name=property.name,
            description=property.description,
            # type
            cardinality=type.cardinality,
            scalar_type=type.scalar_type,
            primitive_type=type.primitive_type,
            enum_type=type.enum_type,
            node_type=type.node_type,
            struct_type=type.struct_type,
            key_type=type.key_type,
            is_required=type.is_required,
            default=type.default,
            default_factory=type.default_factory,
            collection_constraint=type.collection_constraint,
            string_constraint=type.string_constraint,
            number_constraint=type.number_constraint,
            node_constraint=type.node_constraint,
            # node
            node_is_customizable=property.node_is_customizable,
            edge_type=property.edge_type,
            cascade=property.cascade,
            # flags
            is_wired=property.is_wired,
            is_stored=property.is_stored,
            is_repr=property.is_repr,
            is_hash=property.is_hash,
            is_eq=property.is_eq,
            is_managed=property.is_managed,
            is_computed=property.is_computed,
        )


@builtin_struct(StructType.TRAIT_INFO, frozen=True)
class TraitInfo(StructFrozen):
    """Information about a Trait."""

    id: int = property_(2)
    type: TraitType = property_(30)
    name: str = property_(31)
    icon: "Icon | None" = property_(34)
    description: str | None = property_(36)
    properties: list["PropertyInfo"] = property_(50)
    nodes: list[NodeType] = property_(51)


@builtin_struct(StructType.NODE_INFO, frozen=True)
class NodeInfo(StructFrozen):
    """Information about a Node."""

    id: int = property_(2)
    type: NodeType = property_(30)
    name: str = property_(31)
    icon: "Icon | None" = property_(34)
    description: str | None = property_(36)
    properties: list["PropertyInfo"] = property_(50)
    traits: list[TraitType] = property_(51)

    @classmethod
    def from_node(cls, node_cls: _type["Node"]) -> "NodeInfo":
        """Create NodeInfo from a Node class."""
        from .icon import to_icon

        return cls(
            id=node_cls.metatype.value,
            type=node_cls.metatype,
            name=node_cls.metatype.camel_name,
            icon=to_icon(node_cls.metatype.icon) if node_cls.metatype.icon else None,
            description=node_cls.__doc__,
            properties=[prop.info for prop in node_cls.__properties__.values() if prop.is_wired],
            traits=list(node_cls.__traits__),
        )


@builtin_struct(StructType.STRUCT_INFO, frozen=True)
class StructInfo(StructFrozen):
    """Information about a Struct."""

    id: int = property_(2)
    type: StructType = property_(30)
    name: str = property_(31)
    icon: "Icon | None" = property_(34)
    description: str | None = property_(36)
    properties: list["PropertyInfo"] = property_(50)

    @classmethod
    def from_struct(cls, struct_cls: _type[StructBase]) -> "StructInfo":
        """Create StructInfo from a Struct class."""
        from .icon import to_icon

        return cls(
            id=struct_cls.metatype.value,
            type=struct_cls.metatype,
            name=struct_cls.metatype.camel_name,
            icon=to_icon(struct_cls.metatype.icon) if struct_cls.metatype.icon else None,
            description=struct_cls.__doc__,
            properties=[prop.info for prop in struct_cls.__properties__.values() if prop.is_wired],
        )


@builtin_struct(StructType.ENUM_INFO, frozen=True)
class EnumInfo(StructFrozen):
    """Information about an Enum."""

    id: int = property_(2)
    type: EnumType = property_(30)
    name: str = property_(31)
    icon: "Icon | None" = property_(34)
    description: str | None = property_(36)
    options: list["EnumOptionInfo"] = property_(50)

    @classmethod
    def from_enum(cls, enum_type: EnumType, enum_cls: _type[Enum]) -> "EnumInfo":
        """Create EnumInfo from an Enum class."""
        from .icon import to_icon

        return cls(
            id=enum_type.value,
            type=enum_type,
            name=enum_type.camel_name,
            icon=to_icon(enum_type.icon) if enum_type.icon else None,
            description=enum_type.__doc__,
            options=[
                EnumOptionInfo.from_enum_option(enum_type, option)
                for option in enum_cls.__members__.values()
            ],
        )


@builtin_struct(StructType.ENUM_OPTION_INFO, frozen=True)
class EnumOptionInfo(StructFrozen):
    """Information about an Enum Option."""

    id: int = property_(2)
    type: EnumType = property_(30)
    name: str = property_(31)
    icon: "Icon | None" = property_(34)
    description: str | None = property_(36)

    @classmethod
    def from_enum_option(cls, enum_type: EnumType, option: Enum) -> "EnumOptionInfo":
        """Create EnumOptionInfo from an Enum option."""
        from .icon import to_icon

        return cls(
            id=option.value,
            type=enum_type,
            name=option.name,
            icon=to_icon(option.icon) if option.icon else None,
            description=option.__doc__,
        )

from typing import TYPE_CHECKING, Any, Optional, assert_never

from destack.language.registry import OBJECT_REF_BY_CLASS

from ..builtin import (
    CascadeAction,
    EdgeType,
    Enum,
    EnumType,
    NodeType,
    PropertyDeclaration,
    StructBase,
    StructFrozen,
    StructType,
    TraitType,
    builtin_struct,
    property_,
)
from .query import Condition, ConditionalType, Sort, SortType
from .relation import ObjectReference, ObjectType, PropertyReference, PropertyReferenceType
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
    from destack.language import Icon, Node, NodeBase, ObjectReference, Type, Value


_type = type


@builtin_struct(StructType.PROPERTY_DEFINITION, frozen=True)
class PropertyDefinition(StructFrozen):
    """Definition of a builtin Property."""

    id: int = property_(2)
    name: str = property_(31)
    icon: "Icon | None" = property_(34)
    description: str | None = property_(36)
    object: "ObjectReference" = property_(
        37, description="The object that this property is defined on."
    )
    original_object: "ObjectReference" = property_(
        38, description="The original object that this property was defined on."
    )

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
    is_unique: bool | None = property_(51)
    default_value: Optional["Value"] = property_(55)
    default_factory: Optional[DefaultFactory] = property_(56)

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = property_(60)
    string_constraint: Optional["StringConstraint"] = property_(61)
    number_constraint: Optional["NumberConstraint"] = property_(62)
    node_constraint: Optional["NodeConstraint"] = property_(63)

    node_is_customizable: bool = property_(73)
    node_has_type: bool = property_(74)
    node_has_space: bool = property_(75)
    node_has_definition: bool = property_(76)
    edge_type: EdgeType | None = property_(77)
    cascade: CascadeAction | None = property_(78)

    is_wired: bool = property_(80)
    is_stored: bool = property_(81)
    is_repr: bool = property_(82)
    is_hash: bool = property_(83)
    is_eq: bool = property_(84)
    is_managed: bool = property_(85)
    is_computed: bool = property_(86)

    @classmethod
    def from_property(cls, prop: PropertyDeclaration) -> "PropertyDefinition":
        """Create PropertyDefinition from a Property."""
        assert prop.id is not None, f"{prop!r} has no id"
        type = prop._to_type()
        object_ref = OBJECT_REF_BY_CLASS[prop.component]
        original_object_ref = OBJECT_REF_BY_CLASS.get(prop.original_component, object_ref)

        return cls(
            id=prop.id,
            name=prop.name,
            description=prop.description,
            object=object_ref,
            original_object=original_object_ref,
            # type
            cardinality=type.cardinality,
            scalar_type=type.scalar_type,
            primitive_type=type.primitive_type,
            enum_type=type.enum_type,
            node_type=type.node_type,
            struct_type=type.struct_type,
            key_type=type.key_type,
            is_required=type.is_required,
            is_unique=prop.is_unique,
            default_value=type.default_value,
            default_factory=type.default_factory,
            collection_constraint=type.collection_constraint,
            string_constraint=type.string_constraint,
            number_constraint=type.number_constraint,
            node_constraint=type.node_constraint,
            # node
            node_is_customizable=prop.node_is_customizable,
            node_has_type=prop.node_has_type,
            node_has_space=prop.node_has_space,
            node_has_definition=prop.node_has_definition,
            edge_type=prop.edge_type,
            cascade=prop.cascade,
            # flags
            is_wired=prop.is_wired,
            is_stored=prop.is_stored,
            is_repr=prop.is_repr,
            is_hash=prop.is_hash,
            is_eq=prop.is_eq,
            is_managed=prop.is_managed,
            is_computed=prop.is_computed,
        )

    def to_ref(self) -> PropertyReference:
        if self.object.type == ObjectType.BUILTIN_NODE:
            return PropertyReference(
                type=PropertyReferenceType.BUILTIN,
                node_type=self.object.node_type,
                id=self.id,
            )
        elif self.object.type == ObjectType.BUILTIN_STRUCT:
            return PropertyReference(
                type=PropertyReferenceType.BUILTIN,
                struct_type=self.object.struct_type,
                id=self.id,
            )
        elif self.object.type == ObjectType.TRAIT:
            return PropertyReference(
                type=PropertyReferenceType.BUILTIN,
                trait_type=self.object.trait_type,
                id=self.id,
            )
        elif self.object.type == ObjectType.CUSTOM_NODE:
            raise RuntimeError(f"{self!r} cannot be associated with a custom node")
        else:
            assert_never(self.object.type)

    def eq(self, value: Any) -> Condition:
        if value is None:
            return self.not_exists()
        return Condition.of(self, ConditionalType.EQUALS, value=value)

    def neq(self, value: Any) -> Condition:
        if value is None:
            return self.exists()
        return Condition.of(self, ConditionalType.NOT_EQUALS, value=value)

    def gt(self, value: Any) -> Condition:
        return Condition.of(self, ConditionalType.GREATER_THAN, value=value)

    def gte(self, value: Any) -> Condition:
        return Condition.of(self, ConditionalType.GREATER_THAN_OR_EQUALS, value=value)

    def lt(self, value: Any) -> Condition:
        return Condition.of(self, ConditionalType.LESS_THAN, value=value)

    def lte(self, value: Any) -> Condition:
        return Condition.of(self, ConditionalType.LESS_THAN_OR_EQUALS, value=value)

    def starts_with(self, value: str) -> Condition:
        return Condition.of(self, ConditionalType.STARTS_WITH, value=value)

    def ends_with(self, value: str) -> Condition:
        return Condition.of(self, ConditionalType.ENDS_WITH, value=value)

    def in_(self, *values: Any) -> Condition:
        return Condition.of(self, ConditionalType.IN, value=values)

    def not_in(self, *values: Any) -> Condition:
        return Condition.of(self, ConditionalType.NOT_IN, value=values)

    def exists(self) -> Condition:
        return Condition.of(self, ConditionalType.EXISTS)

    def is_not_none(self) -> Condition:
        return Condition.of(self, ConditionalType.EXISTS)

    def not_exists(self) -> "Condition":
        return Condition.of(self, ConditionalType.NOT_EXISTS)

    def is_none(self) -> "Condition":
        return Condition.of(self, ConditionalType.NOT_EXISTS)

    def asc(self) -> "Sort":
        return Sort.of(self, SortType.ASCENDING)

    def desc(self) -> "Sort":
        return Sort.of(self, SortType.DESCENDING)


@builtin_struct(StructType.TRAIT_DEFINITION, frozen=True)
class TraitDefinition(StructFrozen):
    """Definition of a builtin Trait."""

    id: int = property_(2)
    type: TraitType = property_(30)
    name: str = property_(31)
    alias: str = property_(32)
    icon: "Icon | None" = property_(34)
    description: str | None = property_(36)
    properties: list["PropertyDefinition"] = property_(50)
    traits: list[TraitType] = property_(51)

    @classmethod
    def from_trait(cls, trait_cls: _type["NodeBase"]) -> "TraitDefinition":
        """Create TraitDefinition from a Trait class."""
        from .icon import to_icon

        trait_type = TraitType(trait_cls.metatype)
        return cls(
            id=trait_cls.metatype.value,
            type=trait_type,
            name=trait_cls.metatype.camel_name,
            alias=trait_cls.__name__,
            icon=to_icon(trait_cls.metatype.icon) if trait_cls.metatype.icon else None,
            description=trait_cls.__doc__,
            properties=[
                prop.definition for prop in trait_cls.__properties__.values() if prop.is_wired
            ],
            traits=list(trait_cls.__traits__),
        )


@builtin_struct(StructType.NODE_DEFINITION, frozen=True)
class NodeDefinition(StructFrozen):
    """Definition of a builtin Node."""

    id: int = property_(2)
    type: NodeType = property_(30)
    name: str = property_(31)
    icon: "Icon | None" = property_(34)
    description: str | None = property_(36)
    properties: list["PropertyDefinition"] = property_(50)
    traits: list[TraitType] = property_(51)
    root_type: NodeType | None = property_(52)
    parent_types: list[NodeType] = property_(53)
    child_types: list[NodeType] = property_(54)
    ancestor_types: list[NodeType] = property_(55)
    descendant_types: list[NodeType] = property_(56)

    @classmethod
    def from_node(cls, node_cls: _type["Node"]) -> "NodeDefinition":
        """Create NodeDefinition from a Node class."""
        from .icon import to_icon

        return cls(
            id=node_cls.metatype.value,
            type=node_cls.metatype,
            name=node_cls.__name__,
            icon=to_icon(node_cls.metatype.icon) if node_cls.metatype.icon else None,
            description=node_cls.__doc__,
            properties=[
                prop.definition for prop in node_cls.__properties__.values() if prop.is_wired
            ],
            traits=list(node_cls.__traits__),
            root_type=node_cls.__root_type__,
            parent_types=list(node_cls.__parent_types__),
            child_types=list(node_cls.__child_types__),
            ancestor_types=list(node_cls.__ancestor_types__),
            descendant_types=list(node_cls.__descendant_types__),
        )


@builtin_struct(StructType.STRUCT_DEFINITION, frozen=True)
class StructDefinition(StructFrozen):
    """Definition of a builtin Struct."""

    id: int = property_(2)
    type: StructType = property_(30)
    name: str = property_(31)
    icon: "Icon | None" = property_(34)
    description: str | None = property_(36)
    properties: list["PropertyDefinition"] = property_(50)
    is_frozen: bool = property_(60)

    @classmethod
    def from_struct(cls, struct_cls: _type[StructBase]) -> "StructDefinition":
        """Create StructDefinition from a Struct class."""
        from .icon import to_icon

        return cls(
            id=struct_cls.metatype.value,
            type=struct_cls.metatype,
            name=struct_cls.__name__,
            icon=to_icon(struct_cls.metatype.icon) if struct_cls.metatype.icon else None,
            description=struct_cls.__doc__,
            properties=[
                prop.definition for prop in struct_cls.__properties__.values() if prop.is_wired
            ],
            is_frozen=struct_cls.__is_frozen__,
        )


@builtin_struct(StructType.ENUM_DEFINITION, frozen=True)
class EnumDefinition(StructFrozen):
    """Definition of a builtin Enum."""

    id: int = property_(2)
    type: EnumType = property_(30)
    name: str = property_(31)
    icon: "Icon | None" = property_(34)
    description: str | None = property_(36)
    options: list["OptionDefinition"] = property_(50)

    @classmethod
    def from_enum(cls, enum_type: EnumType, enum_cls: _type[Enum]) -> "EnumDefinition":
        """Create EnumDefinition from an Enum class."""
        from .icon import to_icon

        return cls(
            id=enum_type.value,
            type=enum_type,
            name=enum_type.camel_name,
            icon=to_icon(enum_type.icon) if enum_type.icon else None,
            description=enum_type.__doc__,
            options=[
                OptionDefinition.from_enum_option(enum_type, option)
                for option in enum_cls.__members__.values()
            ],
        )


@builtin_struct(StructType.OPTION_DEFINITION, frozen=True)
class OptionDefinition(StructFrozen):
    """Definition of a builtin Enum Option."""

    id: int = property_(2)
    type: EnumType = property_(30)
    name: str = property_(31)
    icon: "Icon | None" = property_(34)
    description: str | None = property_(36)

    @classmethod
    def from_enum_option(cls, enum_type: EnumType, option: Enum) -> "OptionDefinition":
        """Create OptionDefinition from an Enum option."""
        from .icon import to_icon

        return cls(
            id=option.value,
            type=enum_type,
            name=option.name,
            icon=to_icon(option.icon) if option.icon else None,
            description=option.__doc__,
        )


@builtin_struct(StructType.PERMISSION_DEFINITION, frozen=True)
class PermissionDefinition(StructFrozen):
    """Definition of a builtin Permission for a builtin Node."""

    id: int = property_(2)
    type: EnumType = property_(30)
    name: str = property_(31)
    node_type: NodeType = property_(32)
    icon: "Icon | None" = property_(34)


@builtin_struct(StructType.CONSTANT_DEFINITION, frozen=True)
class ConstantDefinition(StructFrozen):
    """Definition of a builtin Constant."""

    name: str = property_(31)
    path: str = property_(35)
    value: "Value" = property_(40)

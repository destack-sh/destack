from typing import TYPE_CHECKING, Any, Optional, assert_never

from destack.language.registry import (
    ENUM_DEFINITION_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    OBJECT_DEFINITION_REFERENCE_BY_CLASS,
    STRUCT_DEFINITION_BY_TYPE,
    TRAIT_DEFINITION_BY_TYPE,
)

from ..builtin.common import (
    CascadeAction,
    DefaultFactory,
    EdgeType,
    Enum,
    EnumType,
    NodeType,
    PrimitiveType,
    ScalarType,
    StructType,
    TraitType,
    TypeCardinality,
)
from ..builtin.constant import ConstantDeclaration, register_constant
from ..builtin.property import PropertyDeclaration, builtin_property, builtin_property_runtime
from ..builtin.relation import (
    ObjectDefinitionReference,
    ObjectDefinitionType,
    PropertyReference,
    PropertyReferenceType,
)
from ..builtin.struct import StructBase, StructFrozen, builtin_struct

if TYPE_CHECKING:
    from destack.language import (
        CollectionConstraint,
        Condition,
        ConditionalType,
        Icon,
        Node,
        NodeBase,
        NodeConstraint,
        NumberConstraint,
        ObjectDefinitionReference,
        Sort,
        SortType,
        StringConstraint,
        Type,
        Value,
    )


_type = type


@builtin_struct(StructType.PROPERTY_DEFINITION, frozen=True)
class PropertyDefinition(StructFrozen):
    """Definition of a builtin Property."""

    id: int = builtin_property(2, is_repr=True)
    name: str = builtin_property(31, is_repr=True)
    icon: "Icon | None" = builtin_property(34)
    description: str | None = builtin_property(36, is_repr=True)
    object: "ObjectDefinitionReference" = builtin_property(
        37, description="The object that this property is defined on."
    )
    original_object: "ObjectDefinitionReference" = builtin_property(
        38, description="The original object that this property was defined on."
    )

    # scalar
    cardinality: TypeCardinality = builtin_property(
        40, default=TypeCardinality.SCALAR, is_repr=True
    )
    scalar_type: ScalarType = builtin_property(41, is_repr=True)
    primitive_type: Optional[PrimitiveType] = builtin_property(42, is_repr=True)
    enum_type: Optional[EnumType] = builtin_property(43, is_repr=True)
    node_type: Optional[NodeType] = builtin_property(44, is_repr=True)
    struct_type: Optional[StructType] = builtin_property(46, is_repr=True)
    key_type: Optional["Type"] = builtin_property(48, is_repr=True)  # for maps

    # meta
    is_required: bool | None = builtin_property(50, is_repr=True)
    is_unique: bool | None = builtin_property(51, is_repr=True)
    default_value: Optional["Value"] = builtin_property(55, is_repr=True)
    default_factory: Optional["DefaultFactory"] = builtin_property(56, is_repr=True)

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = builtin_property(60)
    string_constraint: Optional["StringConstraint"] = builtin_property(61)
    number_constraint: Optional["NumberConstraint"] = builtin_property(62)
    node_constraint: Optional["NodeConstraint"] = builtin_property(63)

    # relationship
    node_is_extensible: bool = builtin_property(73)
    node_has_type: bool = builtin_property(74)
    node_has_space: bool = builtin_property(75)
    node_has_definition: bool = builtin_property(76)
    edge_type: EdgeType | None = builtin_property(77)
    cascade: CascadeAction | None = builtin_property(78)

    # flags
    is_wired: bool = builtin_property(80)
    is_stored: bool = builtin_property(81)
    is_repr: bool = builtin_property(82)
    is_hash: bool = builtin_property(83)
    is_eq: bool = builtin_property(84)
    is_managed: bool = builtin_property(85)
    is_computed: bool = builtin_property(86)
    is_readonly: bool = builtin_property(87)
    is_static: bool = builtin_property(88)

    @classmethod
    def from_property(cls, prop: PropertyDeclaration) -> "PropertyDefinition":
        """Create PropertyDefinition from a Property."""
        assert prop.id is not None, f"{prop!r} has no id"
        type = prop._to_type()
        object_ref = OBJECT_DEFINITION_REFERENCE_BY_CLASS[prop.component]
        original_object_ref = OBJECT_DEFINITION_REFERENCE_BY_CLASS.get(
            prop.original_component, object_ref
        )

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
            node_is_extensible=prop.node_is_extensible,
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
            is_readonly=prop.is_readonly,
            is_static=prop.is_static,
        )

    def to_ref(self) -> PropertyReference:
        if self.object.type == ObjectDefinitionType.BUILTIN_NODE:
            return PropertyReference(
                type=PropertyReferenceType.BUILTIN,
                node_type=self.object.node_type,
                id=self.id,
            )
        elif self.object.type == ObjectDefinitionType.BUILTIN_STRUCT:
            return PropertyReference(
                type=PropertyReferenceType.BUILTIN,
                struct_type=self.object.struct_type,
                id=self.id,
            )
        elif self.object.type == ObjectDefinitionType.BUILTIN_TRAIT:
            return PropertyReference(
                type=PropertyReferenceType.BUILTIN,
                trait_type=self.object.trait_type,
                id=self.id,
            )
        elif (
            self.object.type == ObjectDefinitionType.CUSTOM_NODE
            or self.object.type == ObjectDefinitionType.CUSTOM_TRAIT
            or self.object.type == ObjectDefinitionType.CUSTOM_STRUCT
        ):
            raise RuntimeError(f"{self!r} cannot be associated with a custom object")
        else:
            assert_never(self.object.type)

    def eq(self, value: Any) -> "Condition":
        from . import Condition

        if value is None:
            return self.not_exists()
        return Condition.of(self, ConditionalType.EQUALS, value=value)

    def neq(self, value: Any) -> "Condition":
        from . import Condition

        if value is None:
            return self.exists()
        return Condition.of(self, ConditionalType.NOT_EQUALS, value=value)

    def gt(self, value: Any) -> "Condition":
        from . import Condition

        return Condition.of(self, ConditionalType.GREATER_THAN, value=value)

    def gte(self, value: Any) -> "Condition":
        from . import Condition

        return Condition.of(self, ConditionalType.GREATER_THAN_OR_EQUALS, value=value)

    def lt(self, value: Any) -> "Condition":
        from . import Condition

        return Condition.of(self, ConditionalType.LESS_THAN, value=value)

    def lte(self, value: Any) -> "Condition":
        from . import Condition

        return Condition.of(self, ConditionalType.LESS_THAN_OR_EQUALS, value=value)

    def starts_with(self, value: str) -> "Condition":
        from . import Condition

        return Condition.of(self, ConditionalType.STARTS_WITH, value=value)

    def ends_with(self, value: str) -> "Condition":
        from . import Condition

        return Condition.of(self, ConditionalType.ENDS_WITH, value=value)

    def in_(self, *values: Any) -> "Condition":
        from . import Condition

        return Condition.of(self, ConditionalType.IN, value=values)

    def not_in(self, *values: Any) -> "Condition":
        from . import Condition

        return Condition.of(self, ConditionalType.NOT_IN, value=values)

    def exists(self) -> "Condition":
        from . import Condition

        return Condition.of(self, ConditionalType.EXISTS)

    def is_not_none(self) -> "Condition":
        from . import Condition

        return Condition.of(self, ConditionalType.EXISTS)

    def not_exists(self) -> "Condition":
        from . import Condition

        return Condition.of(self, ConditionalType.NOT_EXISTS)

    def is_none(self) -> "Condition":
        from . import Condition

        return Condition.of(self, ConditionalType.NOT_EXISTS)

    def asc(self) -> "Sort":
        from . import Sort

        return Sort.of(self, SortType.ASCENDING)

    def desc(self) -> "Sort":
        from . import Sort

        return Sort.of(self, SortType.DESCENDING)


@builtin_struct(StructType.TRAIT_DEFINITION, frozen=True)
class TraitDefinition(StructFrozen):
    """Definition of a builtin Trait."""

    id: int = builtin_property(2, is_repr=True)
    type: TraitType = builtin_property(30, is_repr=True)
    name: str = builtin_property(31, is_repr=True)
    alias: str = builtin_property(32, is_repr=True)
    icon: "Icon | None" = builtin_property(34)
    description: str | None = builtin_property(36, is_repr=True)
    properties: list["PropertyDefinition"] = builtin_property(40)
    traits: list[TraitType] = builtin_property(
        51, description="Traits directly and indirectly inherited by this trait."
    )
    base_traits: list[TraitType] = builtin_property(
        52, description="Traits directly inherited by this trait."
    )

    @classmethod
    def from_trait(cls, trait_cls: _type["NodeBase"]) -> "TraitDefinition":
        """Create TraitDefinition from a Trait class."""
        from . import to_icon

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
            base_traits=list(trait_cls.__base_traits__),
        )


@builtin_struct(StructType.NODE_DEFINITION, frozen=True)
class NodeDefinition(StructFrozen):
    """Definition of a builtin Node."""

    id: int = builtin_property(2, is_repr=True)
    type: NodeType = builtin_property(30, is_repr=True)
    name: str = builtin_property(31, is_repr=True)
    icon: "Icon | None" = builtin_property(34)
    description: str | None = builtin_property(36, is_repr=True)
    is_abstract: bool = builtin_property(37, is_repr=True)
    is_global: bool = builtin_property(38, is_repr=True)
    is_spatial: bool = builtin_property(39, is_repr=True)

    properties: list["PropertyDefinition"] = builtin_property(40)

    base_type: NodeType | None = builtin_property(
        50, description="The base type this Node extends (directly)."
    )
    extends: list[NodeType] = builtin_property(
        51, description="Nodes that this Node extends (directly and indirectly)."
    )
    extended_by: list[NodeType] = builtin_property(
        52, description="Nodes that extend this Node type (directly)."
    )
    inherited_by: list[NodeType] = builtin_property(
        53, description="Nodes that inherit this Node type (directly and indirectly)."
    )
    base_traits: list[TraitType] = builtin_property(
        55, description="Traits directly inherited by this Node (directly)."
    )
    traits: list[TraitType] = builtin_property(
        56,
        description="Traits directly and indirectly inherited by this Node (directly and indirectly).",
    )

    root_type: NodeType | None = builtin_property(60)
    parent_types: list[NodeType] = builtin_property(61)
    child_types: list[NodeType] = builtin_property(62)
    ancestor_types: list[NodeType] = builtin_property(63)
    descendant_types: list[NodeType] = builtin_property(64)

    @classmethod
    def from_node(cls, node_cls: _type["Node"]) -> "NodeDefinition":
        """Create NodeDefinition from a Node class."""
        from . import to_icon

        return cls(
            id=node_cls.metatype.value,
            type=node_cls.metatype,
            name=node_cls.__name__,
            icon=to_icon(node_cls.metatype.icon) if node_cls.metatype.icon else None,
            description=node_cls.__doc__,
            is_abstract=node_cls.__is_abstract__,
            is_global=TraitType.GLOBAL in node_cls.__traits__,
            is_spatial=TraitType.SPATIAL in node_cls.__traits__,
            properties=[
                prop.definition for prop in node_cls.__properties__.values() if prop.is_wired
            ],
            base_type=node_cls.__base_type__,
            extended_by=list(node_cls.__extended_by__),
            inherited_by=list(node_cls.__inherited_by__),
            traits=list(node_cls.__traits__),
            base_traits=list(node_cls.__base_traits__),
            root_type=node_cls.__root_type__,
            parent_types=list(node_cls.__parent_types__),
            child_types=list(node_cls.__child_types__),
            ancestor_types=list(node_cls.__ancestor_types__),
            descendant_types=list(node_cls.__descendant_types__),
        )


@builtin_struct(StructType.STRUCT_DEFINITION, frozen=True)
class StructDefinition(StructFrozen):
    """Definition of a builtin Struct."""

    id: int = builtin_property(2, is_repr=True)
    type: StructType = builtin_property(30, is_repr=True)
    name: str = builtin_property(31, is_repr=True)
    icon: "Icon | None" = builtin_property(34)
    description: str | None = builtin_property(36, is_repr=True)
    properties: list["PropertyDefinition"] = builtin_property(50)
    is_frozen: bool = builtin_property(60)

    @classmethod
    def from_struct(cls, struct_cls: _type[StructBase]) -> "StructDefinition":
        """Create StructDefinition from a Struct class."""
        from . import to_icon

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

    id: int = builtin_property(2, is_repr=True)
    type: EnumType = builtin_property(30, is_repr=True)
    name: str = builtin_property(31, is_repr=True)
    icon: "Icon | None" = builtin_property(34)
    description: str | None = builtin_property(36, is_repr=True)
    options: list["OptionDefinition"] = builtin_property(50)

    @classmethod
    def from_enum(cls, enum_type: EnumType, enum_cls: _type[Enum]) -> "EnumDefinition":
        """Create EnumDefinition from an Enum class."""
        from . import to_icon

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

    id: int = builtin_property(2, is_repr=True)
    type: EnumType = builtin_property(30, is_repr=True)
    name: str = builtin_property(31, is_repr=True)
    icon: "Icon | None" = builtin_property(34)
    description: str | None = builtin_property(36, is_repr=True)

    @classmethod
    def from_enum_option(cls, enum_type: EnumType, option: Enum) -> "OptionDefinition":
        """Create OptionDefinition from an Enum option."""
        from . import to_icon

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

    id: int = builtin_property(2, is_repr=True)
    type: EnumType = builtin_property(30, is_repr=True)
    name: str = builtin_property(31, is_repr=True)
    node_type: NodeType = builtin_property(32, is_repr=True)
    icon: "Icon | None" = builtin_property(34)


@builtin_struct(StructType.CONSTANT_DEFINITION, frozen=True)
class ConstantDefinition(StructFrozen):
    """Definition of a builtin Constant."""

    name: str = builtin_property(31, is_repr=True)
    description: str | None = builtin_property(36, is_repr=True)
    value: "Value" = builtin_property(40)
    is_deferred: bool = builtin_property(50)

    _declaration: "ConstantDeclaration | None" = builtin_property_runtime()

    @classmethod
    def from_constant(cls, constant_declaration: ConstantDeclaration) -> "ConstantDefinition":
        """Create ConstantDefinition from a ConstantDeclaration."""
        from . import to_value

        if constant_declaration.value is None:
            assert constant_declaration.getter is not None, (
                f"missing getter for {constant_declaration.name}"
            )
            value_raw = constant_declaration.getter()
            is_deferred = True
        else:
            value_raw = constant_declaration.value
            is_deferred = False
        value = to_value(value_raw)

        return cls(
            name=constant_declaration.name,
            description=constant_declaration.description,
            value=value,
            is_deferred=is_deferred,
            _declaration=constant_declaration,
        )


register_constant("NODE_DEFINITIONS", lambda: list(NODE_DEFINITION_BY_TYPE.values()))
register_constant("TRAIT_DEFINITIONS", lambda: list(TRAIT_DEFINITION_BY_TYPE.values()))
register_constant("STRUCT_DEFINITIONS", lambda: list(STRUCT_DEFINITION_BY_TYPE.values()))
register_constant("ENUM_DEFINITIONS", lambda: list(ENUM_DEFINITION_BY_TYPE.values()))

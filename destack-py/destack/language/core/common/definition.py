from typing import TYPE_CHECKING, Any, Optional, assert_never, cast

from destack.language.registry import (
    ENUM_DEFINITION_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    OBJECT_DEFINITION_REFERENCE_BY_CLASS,
    STRUCT_DEFINITION_BY_TYPE,
    TRAIT_DEFINITION_BY_TYPE,
)

from ..builtin.common import (
    CascadeAction,
    EdgeType,
    Enum,
    EnumType,
    NodeType,
    PrimitiveType,
    PropertyType,
    ScalarType,
    StoreDomain,
    StoreKey,
    StructType,
    TraitType,
    TypeCardinality,
    ValueFactory,
)
from ..builtin.constant import ConstantDeclaration, register_constant
from ..builtin.property import PropertyDeclaration, builtin_property, builtin_property_runtime
from ..builtin.relation import (
    ObjectDefinitionReference,
    ObjectDefinitionType,
    PropertyReference,
    PropertyReferenceType,
)
from ..builtin.struct import Struct, StructFrozen, builtin_struct

if TYPE_CHECKING:
    from destack.language import (
        ActionDefinition,
        CollectionConstraint,
        Condition,
        ConditionalType,
        ConstraintDefinition,
        Icon,
        IndexDefinition,
        MethodDefinition,
        Node,
        NodeConstraint,
        NumberConstraint,
        ObjectDefinitionReference,
        PermissionDefinition,
        Sort,
        SortType,
        StringConstraint,
        Trait,
        Type,
        Value,
    )


_type = type


@builtin_struct(StructType.BUILTIN_DEFINITION, frozen=True, is_abstract=True)
class BuiltinDefinition(StructFrozen):
    """Definition of a builtin object."""

    id: int = builtin_property(2, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
    description: str | None = builtin_property(103, is_repr=True)


@builtin_struct(StructType.NODE_DEFINITION, frozen=True)
class NodeDefinition(BuiltinDefinition):
    """Definition of a builtin Node."""

    # meta
    type: NodeType = builtin_property(100, is_repr=True)

    # flags
    is_abstract: bool = builtin_property(
        110,
        is_repr=True,
        description="Whether this Node cannot be instantiated directly.",
    )
    is_extensible: bool = builtin_property(
        111,
        is_repr=True,
        description="Whether this Node can be extended by custom Nodes.",
    )
    is_frozen: bool = builtin_property(
        112,
        is_repr=True,
        description="Whether this Node cannot be modified.",
    )
    # is_singleton? is_static?

    # content
    properties: list["PropertyDefinition"] = builtin_property(
        120,
        description="All properties of this Node.",
    )
    indexes: list["IndexDefinition"] = builtin_property(
        121,
        description="All indexes of this Node.",
    )
    constraints: list["ConstraintDefinition"] = builtin_property(
        122,
        description="All constraints of this Node.",
    )
    permissions: list["PermissionDefinition"] = builtin_property(
        123,
        description="All permissions of this Node.",
    )
    methods: list["MethodDefinition"] = builtin_property(
        125,
        description="All methods of this Node (excluding actions).",
    )
    actions: list["ActionDefinition"] = builtin_property(
        126,
        description="All actions of this Node.",
    )

    # inheritance
    base_type: NodeType | None = builtin_property(
        130,
        is_repr=True,
        description="The base type this Node extends (directly).",
    )
    extended_by: list[NodeType] = builtin_property(
        131,
        description="Nodes that extend this Node type (directly).",
    )
    inherits: list[NodeType] = builtin_property(
        132,
        description="Nodes that this Node inherits (directly and indirectly).",
    )
    inherited_by: list[NodeType] = builtin_property(
        133,
        description="Nodes that inherit this Node type (directly and indirectly).",
    )
    base_traits: list[TraitType] = builtin_property(
        134,
        description="Traits directly inherited by this Node (directly).",
    )
    traits: list[TraitType] = builtin_property(
        135,
        description="Traits directly and indirectly inherited by this Node (directly and indirectly).",
    )

    # event
    event_types: list[NodeType] = builtin_property(
        140,
        description="The event types related to this Node (directly and indirectly).",
    )
    base_event_types: list[NodeType] = builtin_property(
        141,
        description="The base event types related to this Node (directly).",
    )

    # enum
    enum_types: list[EnumType] = builtin_property(
        150,
        description="The enum types related to this Node (directly and indirectly).",
    )
    base_enum_types: list[EnumType] = builtin_property(
        151,
        description="The base enum types related to this Node (directly).",
    )

    # tree
    parent_types: list[NodeType] = builtin_property(
        160,
        is_repr=True,
        description="The parent types of this Node type (directly).",
    )
    child_types: list[NodeType] = builtin_property(
        161,
        is_repr=True,
        description="The child types of this Node type (directly).",
    )
    ancestor_types: list[NodeType] = builtin_property(
        162,
        description="The ancestor types of this Node type (directly and indirectly).",
    )
    descendant_types: list[NodeType] = builtin_property(
        163,
        description="The descendant types of this Node type (directly and indirectly).",
    )

    # expected tree
    expected_parent_types: list[NodeType] = builtin_property(
        170,
        description="The parent types expected for this Node type (any of).",
    )
    expected_child_types: list[NodeType] = builtin_property(
        171,
        description="The child types expected for this Node type (any of).",
    )
    expected_ancestor_types: list[NodeType] = builtin_property(
        172,
        description="The ancestor types expected for this Node type (any of).",
    )
    expected_descendant_types: list[NodeType] = builtin_property(
        173,
        description="The descendant types expected for this Node type (any of).",
    )

    # store
    primary_store_keys: list[StoreKey] = builtin_property(200)
    store_domain: StoreDomain | None = builtin_property(201)

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
            # flags
            is_abstract=node_cls.__is_abstract__,
            is_extensible=node_cls.__is_extensible__,
            is_frozen=node_cls.__is_frozen__,
            # content
            properties=[
                prop.definition for prop in node_cls.__properties__.values() if prop.is_wired
            ],
            indexes=list(node_cls.__indexes__),
            constraints=list(node_cls.__constraints__),
            permissions=list(node_cls.__permissions__),
            methods=list(node_cls.__methods__),
            actions=list(node_cls.__actions__),
            # inheritance
            base_type=node_cls.__base_type__,
            extended_by=list(node_cls.__extended_by__),
            inherits=list(node_cls.__inherits__),
            inherited_by=list(node_cls.__inherited_by__),
            traits=list(node_cls.__traits__),
            base_traits=list(node_cls.__base_traits__),
            # event
            event_types=list(node_cls.__event_types__),
            base_event_types=list(node_cls.__base_event_types__),
            # tree
            parent_types=list(node_cls.__parent_types__),
            child_types=list(node_cls.__child_types__),
            ancestor_types=list(node_cls.__ancestor_types__),
            descendant_types=list(node_cls.__descendant_types__),
            # expected tree
            expected_parent_types=list(node_cls.__expected_parent_types__),
            expected_child_types=list(node_cls.__expected_child_types__),
            expected_ancestor_types=list(node_cls.__expected_ancestor_types__),
            expected_descendant_types=list(node_cls.__expected_descendant_types__),
            # store
            primary_store_keys=list(node_cls.__primary_store_keys__),
            store_domain=node_cls.__store_domain__,
        )


@builtin_struct(StructType.TRAIT_DEFINITION, frozen=True)
class TraitDefinition(BuiltinDefinition):
    """Definition of a builtin Trait."""

    # meta
    type: TraitType = builtin_property(100, is_repr=True)

    # flags
    alias: str = builtin_property(110, is_repr=True)
    is_extensible: bool = builtin_property(
        111,
        is_repr=True,
        description="Whether this Trait can be extended by custom Nodes and custom Traits.",
    )

    # content
    permissions: list["PermissionDefinition"] = builtin_property(
        123,
        description="All permissions of this Trait.",
    )

    # inheritance
    base_traits: list[TraitType] = builtin_property(
        130,
        description="Traits directly inherited by this Trait (directly).",
    )
    traits: list[TraitType] = builtin_property(
        131,
        description="Traits directly and indirectly inherited by this Trait (directly and indirectly).",
    )

    # event
    event_types: list[NodeType] = builtin_property(
        140,
        description="The event types related to this Trait (directly and indirectly).",
    )
    base_event_types: list[NodeType] = builtin_property(
        141,
        description="The base event types related to this Trait (directly).",
    )

    # enum
    enum_types: list[EnumType] = builtin_property(
        150,
        description="The enum types related to this Trait (directly and indirectly).",
    )
    base_enum_types: list[EnumType] = builtin_property(
        151,
        description="The base enum types related to this Trait (directly).",
    )

    @classmethod
    def from_trait(cls, trait_cls: _type["Trait"]) -> "TraitDefinition":
        """Create TraitDefinition from a Trait class."""
        from . import to_icon

        trait_type = TraitType(trait_cls.metatype)
        return cls(
            id=trait_cls.metatype.value,
            # meta
            type=trait_type,
            name=trait_cls.metatype.camel_name,
            icon=to_icon(trait_cls.metatype.icon) if trait_cls.metatype.icon else None,
            description=trait_cls.__doc__,
            # flags
            alias=trait_cls.__name__,
            is_extensible=cast(type["Trait"], trait_cls).__is_extensible__,
            # content
            permissions=list(trait_cls.__permissions__),
            # inheritance
            base_traits=list(trait_cls.__base_traits__),
            traits=list(trait_cls.__traits__),
            # event
            event_types=list(trait_cls.__event_types__),
            base_event_types=list(trait_cls.__base_event_types__),
            # enum
            enum_types=list(trait_cls.__enum_types__),
            base_enum_types=list(trait_cls.__base_enum_types__),
        )


@builtin_struct(StructType.STRUCT_DEFINITION, frozen=True)
class StructDefinition(BuiltinDefinition):
    """Definition of a builtin Struct."""

    # meta
    type: StructType = builtin_property(100, is_repr=True)

    # flags
    is_frozen: bool = builtin_property(
        110,
        description="Whether this Struct is read-only (cannot be modified).",
    )
    is_abstract: bool = builtin_property(
        111,
        description="Whether this Struct is abstract (cannot be instantiated directly).",
    )
    is_extensible: bool = builtin_property(
        112,
        description="Whether this Struct can be extended by custom Structs.",
    )

    # content
    properties: list["PropertyDefinition"] = builtin_property(
        120,
        description="All properties of this Struct.",
    )
    methods: list["MethodDefinition"] = builtin_property(
        125,
        description="All methods of this Struct (excluding actions).",
    )
    actions: list["ActionDefinition"] = builtin_property(
        126,
        description="All actions of this Struct.",
    )

    # inheritance
    base_type: StructType | None = builtin_property(
        130, description="The base type this Struct extends (directly)."
    )
    extended_by: list[StructType] = builtin_property(
        131, description="Structs that extend this Struct type (directly)."
    )
    inherits: list[StructType] = builtin_property(
        132, description="Structs that this Struct inherits (directly and indirectly)."
    )
    inherited_by: list[StructType] = builtin_property(
        133, description="Structs that inherit this Struct type (directly and indirectly)."
    )

    # enum
    enum_types: list[EnumType] = builtin_property(
        150,
        description="The enum types related to this Node (directly and indirectly).",
    )
    base_enum_types: list[EnumType] = builtin_property(
        151,
        description="The base enum types related to this Node (directly).",
    )

    @classmethod
    def from_struct(cls, struct_cls: _type[Struct]) -> "StructDefinition":
        """Create StructDefinition from a Struct class."""
        from . import to_icon

        return cls(
            id=struct_cls.metatype.value,
            type=struct_cls.metatype,
            name=struct_cls.__name__,
            icon=to_icon(struct_cls.metatype.icon) if struct_cls.metatype.icon else None,
            description=struct_cls.__doc__,
            # flags
            is_frozen=struct_cls.__is_frozen__,
            is_abstract=struct_cls.__is_abstract__,
            is_extensible=struct_cls.__is_extensible__,
            # content
            properties=[
                prop.definition for prop in struct_cls.__properties__.values() if prop.is_wired
            ],
            methods=list(struct_cls.__methods__),
            actions=list(struct_cls.__actions__),
            # inheritance
            base_type=struct_cls.__base_type__,
            extended_by=list(struct_cls.__extended_by__),
            inherits=list(struct_cls.__inherits__),
            inherited_by=list(struct_cls.__inherited_by__),
            # enum
            enum_types=list(struct_cls.__enum_types__),
            base_enum_types=list(struct_cls.__base_enum_types__),
        )


@builtin_struct(StructType.ENUM_DEFINITION, frozen=True)
class EnumDefinition(BuiltinDefinition):
    """Definition of a builtin Enum."""

    type: EnumType = builtin_property(100, is_repr=True)
    options: list["OptionDefinition"] = builtin_property(104)

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


@builtin_struct(StructType.PROPERTY_DEFINITION, frozen=True)
class PropertyDefinition(BuiltinDefinition):
    """Definition of a builtin Property."""

    type: PropertyType = builtin_property(100)
    object: "ObjectDefinitionReference" = builtin_property(
        104, description="The object that this property is defined on."
    )
    original_object: "ObjectDefinitionReference" = builtin_property(
        105, description="The original object that this property was defined on."
    )
    group_id: int | None = builtin_property(106)

    # scalar
    cardinality: TypeCardinality = builtin_property(
        110, default=TypeCardinality.SCALAR, is_repr=True
    )
    scalar_type: ScalarType = builtin_property(111, is_repr=True)
    primitive_type: Optional[PrimitiveType] = builtin_property(112, is_repr=True)
    enum_type: Optional[EnumType] = builtin_property(113, is_repr=True)
    node_type: Optional[NodeType] = builtin_property(114, is_repr=True)
    struct_type: Optional[StructType] = builtin_property(115, is_repr=True)
    # definition.. not needed?
    key_type: Optional["Type"] = builtin_property(116, is_repr=True)  # for maps

    # value
    value: Optional["Value"] = builtin_property(120, is_repr=True)
    value_factory: Optional["ValueFactory"] = builtin_property(121, is_repr=True)

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = builtin_property(130)
    string_constraint: Optional["StringConstraint"] = builtin_property(131)
    number_constraint: Optional["NumberConstraint"] = builtin_property(132)
    node_constraint: Optional["NodeConstraint"] = builtin_property(133)

    # relationship
    edge_type: EdgeType | None = builtin_property(140)
    cascade: CascadeAction | None = builtin_property(141)

    # flags
    is_required: bool = builtin_property(
        150,
        is_repr=True,
        description="Whether this property must be set.",
    )
    is_unique: bool = builtin_property(
        151,
        is_repr=True,
        description="Whether this property must have a unique value.",
    )
    is_readonly: bool = builtin_property(
        153,
        is_repr=True,
        description="Whether this property is read-only.",
    )
    is_main: bool = builtin_property(154)

    is_wired: bool = builtin_property(160)
    is_stored: bool = builtin_property(
        161,
        description="Whether this property is stored in the database.",
    )
    is_repr: bool = builtin_property(162)
    is_hash: bool = builtin_property(163)
    is_eq: bool = builtin_property(164)
    is_internal: bool = builtin_property(165)

    _type: "Type | None" = builtin_property_runtime()

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
            type=prop.type,
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
            value=type.value,
            value_factory=type.value_factory,
            collection_constraint=type.collection_constraint,
            string_constraint=type.string_constraint,
            number_constraint=type.number_constraint,
            node_constraint=type.node_constraint,
            # node
            edge_type=prop.edge_type,
            cascade=prop.cascade,
            # flags
            is_required=prop.is_required,
            is_unique=prop.is_unique,
            is_readonly=prop.is_readonly,
            is_main=prop.is_main,
            is_wired=prop.is_wired,
            is_stored=prop.is_stored,
            is_repr=prop.is_repr,
            is_hash=prop.is_hash,
            is_eq=prop.is_eq,
            is_internal=prop.is_internal,
        )

    def to_type(self) -> "Type":
        """Convert to a Type."""
        from .type import Type

        if self._type is None:
            type = Type(
                cardinality=self.cardinality,
                scalar_type=self.scalar_type,
                primitive_type=self.primitive_type,
                enum_type=self.enum_type,
                node_type=self.node_type,
                struct_type=self.struct_type,
                key_type=self.key_type,
                value=self.value,
                value_factory=self.value_factory,
                collection_constraint=self.collection_constraint,
                string_constraint=self.string_constraint,
                number_constraint=self.number_constraint,
                node_constraint=self.node_constraint,
                is_required=self.is_required,
            )
            self._type = type  # type: ignore (frozen)

        return self._type

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


@builtin_struct(StructType.OPTION_DEFINITION, frozen=True)
class OptionDefinition(BuiltinDefinition):
    """Definition of a builtin Enum Option."""

    type: EnumType = builtin_property(100, is_repr=True)

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


@builtin_struct(StructType.CONSTANT_DEFINITION, frozen=True)
class ConstantDefinition(StructFrozen):
    """Definition of a builtin Constant."""

    name: str = builtin_property(101, is_repr=True)
    description: str | None = builtin_property(103, is_repr=True)
    value: "Value" = builtin_property(120)
    is_deferred: bool = builtin_property(130)

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

from typing import TYPE_CHECKING, Any, Self, assert_never, cast

from destack.language.registry import (
    OBJECT_DEFINITION_REFERENCE_BY_CLASS,
)

from .builtin import (
    NodeType,
    StructType,
    TraitType,
)
from .common import (
    CascadeAction,
    EdgeType,
    Enum,
    EnumType,
    GraphDomain,
    UInt8,
    UInt32,
)
from .meta import (
    ConstraintDeclaration,
    IndexDeclaration,
    PermissionDeclaration,
    TagDeclaration,
    builtin_method,
)
from .object import BuiltinObject
from .property import PropertyDeclaration, builtin_property, builtin_property_runtime
from .relation import (
    ObjectDefinitionReference,
    ObjectDefinitionType,
    PropertyReference,
    PropertyReferenceType,
)
from .struct import Struct, StructFrozen, builtin_struct
from .type import CheckedType
from .value import Value

if TYPE_CHECKING:
    from destack.language import (
        ActionDefinition,
        CheckedType,
        Condition,
        ConditionalType,
        ConstantDeclaration,
        ConstraintType,
        Icon,
        IndexType,
        MethodDefinition,
        Node,
        ObjectDefinitionReference,
        Sort,
        SortType,
        Trait,
        Value,
    )


type_ = type


def resolve_tagging(
    object_cls: type_["Node | Struct"], tagging: str
) -> "TagDefinition | TagDeclaration":
    """Resolve a tagging to a definition."""
    from .node import Node
    from .struct import Struct

    for tag in object_cls.__tags__:
        if tag.name == tagging:
            return tag
    for cls in object_cls.__mro__:
        if issubclass(cls, Node):
            for tag in cls.__tags__:
                if tag.name == tagging:
                    return tag
        elif issubclass(cls, Struct):
            for tag in cls.__declared_tags__:
                if tag.name == tagging:
                    return tag

    raise ValueError(f"tagging '{tagging}' not found for {object_cls.__name__}")


@builtin_struct(
    StructType.BUILTIN_OBJECT_DEFINITION,
    frozen=True,
    is_abstract=True,
    tags=(
        TagDeclaration(
            id=100,
            name="meta",
            description="Meta information of a definition.",
        ),
        TagDeclaration(
            id=101,
            name="content",
            description="Content of a definition (properties, methods, etc.).",
        ),
        TagDeclaration(
            id=102,
            name="inheritance",
            description="Inheritance of builtin definitions.",
        ),
        TagDeclaration(
            id=103,
            name="graph",
            description="Graph information of a definition (ancestors, descendants, domain, etc.).",
        ),
        TagDeclaration(
            id=104,
            name="associations",
            description="Associations of a definition (enums, events, etc.).",
        ),
    ),
)
class BuiltinObjectDefinition(StructFrozen):
    """Definition of a builtin Trait, Node or Struct."""

    pass


@builtin_struct(StructType.NODE_DEFINITION, frozen=True)
class NodeDefinition(BuiltinObjectDefinition):
    """Definition of a builtin Node."""

    # meta
    type: NodeType = builtin_property(
        100,
        is_repr=True,
        tags=("meta",),
    )
    id: UInt32 = builtin_property(
        2,
        is_repr=True,
        tags=("meta",),
    )
    name: str = builtin_property(
        101,
        is_repr=True,
        tags=("meta",),
    )
    icon: "Icon | None" = builtin_property(
        102,
        tags=("meta",),
    )
    description: str | None = builtin_property(
        103,
        is_repr=True,
        tags=("meta",),
    )
    is_abstract: bool = builtin_property(
        110,
        is_repr=True,
        description="Whether this Node cannot be instantiated directly.",
        tags=("meta",),
    )
    is_extensible: bool = builtin_property(
        111,
        is_repr=True,
        description="Whether this Node can be extended by custom Nodes.",
        tags=("meta",),
    )
    is_final: bool = builtin_property(
        112,
        is_repr=True,
        description="Whether this Node cannot be extended by custom Nodes.",
        tags=("meta",),
    )
    is_frozen: bool = builtin_property(
        113,
        is_repr=True,
        description="Whether this Node cannot be modified.",
        tags=("meta",),
    )
    # is_singleton? is_static?

    # content
    properties: list["PropertyDefinition"] = builtin_property(
        120,
        description="All properties of this Node (including inherited).",
        tags=("content",),
    )
    indexes: list["IndexDefinition"] = builtin_property(
        121,
        description="All indexes of this Node (including inherited).",
        tags=("content",),
    )
    constraints: list["ConstraintDefinition"] = builtin_property(
        122,
        description="All constraints of this Node (including inherited).",
        tags=("content",),
    )
    permissions: list["PermissionDefinition"] = builtin_property(
        123,
        description="All permissions of this Node (including inherited).",
        tags=("content",),
    )
    methods: list["MethodDefinition"] = builtin_property(
        125,
        description="All methods of this Node (including inherited, excluding actions).",
        tags=("content",),
    )
    actions: list["ActionDefinition"] = builtin_property(
        126,
        description="All actions of this Node (including inherited).",
        tags=("content",),
    )
    constants: list["ConstantDefinition"] = builtin_property(
        128,
        description="All constants of this Node (including inherited).",
        tags=("content",),
    )
    tags: list["TagDefinition"] = builtin_property(
        129,
        description="All tags of this Node.",
        tags=("content",),
    )

    # inheritance
    base_type: NodeType | None = builtin_property(
        130,
        is_repr=True,
        description="The base type this Node extends (directly).",
        tags=("inheritance",),
    )
    extended_by: list[NodeType] = builtin_property(
        131,
        description="Nodes that extend this Node type (directly).",
        tags=("inheritance",),
    )
    inherits: list[NodeType] = builtin_property(
        132,
        description="Nodes that this Node inherits.",
        tags=("inheritance",),
    )
    inherited_by: list[NodeType] = builtin_property(
        133,
        description="Nodes that inherit this Node type.",
        tags=("inheritance",),
    )
    traits: list[TraitType] = builtin_property(
        134,
        description="Traits implemented by this Node.",
        tags=("inheritance",),
    )
    self_traits: list[TraitType] = builtin_property(
        135,
        description="Traits declared by this Node (directly).",
        tags=("inheritance",),
    )

    # graph
    parent_types: list[NodeType] = builtin_property(
        160,
        is_repr=True,
        description="The parent types of this Node type (directly).",
        tags=("graph",),
    )
    child_types: list[NodeType] = builtin_property(
        161,
        is_repr=True,
        description="The child types of this Node type (directly).",
        tags=("graph",),
    )
    ancestor_types: list[NodeType] = builtin_property(
        162,
        description="The ancestor types of this Node type.",
        tags=("graph",),
    )
    descendant_types: list[NodeType] = builtin_property(
        163,
        description="The descendant types of this Node type.",
        tags=("graph",),
    )
    expected_parent_types: list[NodeType] = builtin_property(
        170,
        description="The parent types expected for this Node type (any of).",
        tags=("graph",),
    )
    expected_child_types: list[NodeType] = builtin_property(
        171,
        description="The child types expected for this Node type (any of).",
        tags=("graph",),
    )
    expected_ancestor_types: list[NodeType] = builtin_property(
        172,
        description="The ancestor types expected for this Node type (any of).",
        tags=("graph",),
    )
    expected_descendant_types: list[NodeType] = builtin_property(
        173,
        description="The descendant types expected for this Node type (any of).",
        tags=("graph",),
    )
    domain: GraphDomain | None = builtin_property(
        180,
        tags=("graph",),
    )

    # associations
    event_types: list[NodeType] = builtin_property(
        200,
        description="The event types related to this Node.",
        tags=("associations",),
    )
    self_event_types: list[NodeType] = builtin_property(
        201,
        description="The event types declared by this Node (directly).",
        tags=("associations",),
    )
    enum_types: list[EnumType] = builtin_property(
        210,
        description="The enum types related to this Node.",
        tags=("associations",),
    )
    self_enum_types: list[EnumType] = builtin_property(
        211,
        description="The enum types declared by this Node (directly).",
        tags=("associations",),
    )

    @classmethod
    def from_declaration(cls, node_cls: type_["Node"]) -> "NodeDefinition":
        """Create NodeDefinition from a Node class."""
        from ..common import to_icon

        return cls(
            id=node_cls.metatype.value,
            type=node_cls.metatype,
            name=node_cls.__name__,
            icon=to_icon(node_cls.metatype.icon) if node_cls.metatype.icon else None,
            description=node_cls.__doc__,
            is_abstract=node_cls.__is_abstract__,
            is_extensible=node_cls.__is_extensible__,
            is_final=node_cls.__is_final__,
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
            constants=list(node_cls.__constants__),
            tags=list(node_cls.__tags__),
            # inheritance
            base_type=node_cls.__base_type__,
            extended_by=list(node_cls.__extended_by__),
            inherits=list(node_cls.__inherits__),
            inherited_by=list(node_cls.__inherited_by__),
            traits=list(node_cls.__traits__),
            self_traits=list(node_cls.__self_traits__),
            # graph
            parent_types=list(node_cls.__parent_types__),
            child_types=list(node_cls.__child_types__),
            ancestor_types=list(node_cls.__ancestor_types__),
            descendant_types=list(node_cls.__descendant_types__),
            expected_parent_types=list(node_cls.__expected_parent_types__),
            expected_child_types=list(node_cls.__expected_child_types__),
            expected_ancestor_types=list(node_cls.__expected_ancestor_types__),
            expected_descendant_types=list(node_cls.__expected_descendant_types__),
            domain=node_cls.__domain__,
            # associations
            event_types=list(node_cls.__event_types__),
            self_event_types=list(node_cls.__self_event_types__),
            enum_types=list(node_cls.__enum_types__),
            self_enum_types=list(node_cls.__self_enum_types__),
        )


@builtin_struct(StructType.TRAIT_DEFINITION, frozen=True)
class TraitDefinition(BuiltinObjectDefinition):
    """Definition of a builtin Trait."""

    # meta
    type: TraitType = builtin_property(100, is_repr=True)
    id: UInt32 = builtin_property(2, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
    description: str | None = builtin_property(103, is_repr=True)
    taggings: list[UInt8] = builtin_property(109)
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
    self_traits: list[TraitType] = builtin_property(
        130,
        description="Traits directly inherited by this Trait (directly).",
    )
    traits: list[TraitType] = builtin_property(
        131,
        description="Traits directly and indirectly inherited by this Trait.",
    )

    # associations
    event_types: list[NodeType] = builtin_property(
        200,
        description="The event types related to this Trait.",
    )
    self_event_types: list[NodeType] = builtin_property(
        201,
        description="The base event types related to this Trait (directly).",
    )
    enum_types: list[EnumType] = builtin_property(
        210,
        description="The enum types related to this Trait.",
    )
    self_enum_types: list[EnumType] = builtin_property(
        211,
        description="The base enum types related to this Trait (directly).",
    )

    @classmethod
    def from_declaration(cls, trait_cls: type_["Trait"]) -> "TraitDefinition":
        """Create TraitDefinition from a Trait class."""
        from ..common import to_icon

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
            self_traits=list(trait_cls.__self_traits__),
            traits=list(trait_cls.__traits__),
            # event
            event_types=list(trait_cls.__event_types__),
            self_event_types=list(trait_cls.__self_event_types__),
            # enum
            enum_types=list(trait_cls.__enum_types__),
            self_enum_types=list(trait_cls.__self_enum_types__),
        )


@builtin_struct(StructType.STRUCT_DEFINITION, frozen=True)
class StructDefinition(BuiltinObjectDefinition):
    """Definition of a builtin Struct."""

    # meta
    type: StructType = builtin_property(
        100,
        is_repr=True,
        tags=("meta",),
    )
    id: UInt32 = builtin_property(
        2,
        is_repr=True,
        tags=("meta",),
    )
    name: str = builtin_property(
        101,
        is_repr=True,
        tags=("meta",),
    )
    icon: "Icon | None" = builtin_property(
        102,
        tags=("meta",),
    )
    description: str | None = builtin_property(
        103,
        is_repr=True,
        tags=("meta",),
    )
    taggings: list[UInt8] = builtin_property(
        109,
        tags=("meta",),
    )
    is_frozen: bool = builtin_property(
        110,
        description="Whether this Struct is read-only (cannot be modified).",
        tags=("meta",),
    )
    is_abstract: bool = builtin_property(
        111,
        description="Whether this Struct is abstract (cannot be instantiated directly).",
        tags=("meta",),
    )
    is_stable: bool = builtin_property(
        112,
        description="Whether this Struct is stable (cannot be redefined by the system).",
        tags=("meta",),
    )

    # content
    properties: list["PropertyDefinition"] = builtin_property(
        120,
        description="All properties of this Struct.",
        tags=("content",),
    )
    methods: list["MethodDefinition"] = builtin_property(
        125,
        description="All methods of this Struct (excluding actions).",
        tags=("content",),
    )
    actions: list["ActionDefinition"] = builtin_property(
        126,
        description="All actions of this Struct.",
        tags=("content",),
    )
    constants: list["ConstantDefinition"] = builtin_property(
        128,
        tags=("content",),
    )
    tags: list["TagDefinition"] = builtin_property(
        129,
        tags=("content",),
    )

    # inheritance
    base_type: StructType | None = builtin_property(
        130,
        description="The base type this Struct extends (directly).",
        tags=("inheritance",),
    )
    extended_by: list[StructType] = builtin_property(
        131,
        description="Structs that extend this Struct type (directly).",
        tags=("inheritance",),
    )
    inherits: list[StructType] = builtin_property(
        132,
        description="Structs that this Struct inherits.",
        tags=("inheritance",),
    )
    inherited_by: list[StructType] = builtin_property(
        133,
        description="Structs that inherit this Struct type.",
        tags=("inheritance",),
    )

    # associations
    enum_types: list[EnumType] = builtin_property(
        210,
        description="The enum types related to this Node.",
        tags=("associations",),
    )
    self_enum_types: list[EnumType] = builtin_property(
        211,
        description="The base enum types related to this Node (directly).",
        tags=("associations",),
    )

    @classmethod
    def from_declaration(cls, struct_cls: type_[Struct]) -> "StructDefinition":
        """Create StructDefinition from a Struct class."""
        from ..common import to_icon

        return cls(
            id=struct_cls.metatype.value,
            type=struct_cls.metatype,
            name=struct_cls.__name__,
            icon=to_icon(struct_cls.metatype.icon) if struct_cls.metatype.icon else None,
            description=struct_cls.__doc__,
            is_frozen=struct_cls.__is_frozen__,
            is_abstract=struct_cls.__is_abstract__,
            is_stable=struct_cls.__is_stable__,
            # content
            properties=[
                prop.definition for prop in struct_cls.__properties__.values() if prop.is_wired
            ],
            methods=list(struct_cls.__methods__),
            actions=list(struct_cls.__actions__),
            constants=list(struct_cls.__constants__),
            tags=list(struct_cls.__tags__),
            # inheritance
            base_type=struct_cls.__base_type__,
            extended_by=list(struct_cls.__extended_by__),
            inherits=list(struct_cls.__inherits__),
            inherited_by=list(struct_cls.__inherited_by__),
            # associations
            enum_types=list(struct_cls.__enum_types__),
            self_enum_types=list(struct_cls.__self_enum_types__),
        )


@builtin_struct(StructType.ENUM_DEFINITION, frozen=True)
class EnumDefinition(StructFrozen):
    """Definition of a builtin Enum."""

    type: EnumType = builtin_property(100, is_repr=True)
    id: UInt32 = builtin_property(2, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
    description: str | None = builtin_property(103, is_repr=True)
    taggings: list[UInt8] = builtin_property(109)

    # content
    options: list["OptionDefinition"] = builtin_property(120)

    @classmethod
    def from_declaration(cls, enum_type: EnumType, enum_cls: type_[Enum]) -> "EnumDefinition":
        """Create EnumDefinition from an Enum class."""
        from ..common import to_icon

        return cls(
            id=enum_type.value,
            type=enum_type,
            name=enum_type.camel_name,
            icon=to_icon(enum_type.icon) if enum_type.icon else None,
            description=enum_type.__doc__,
            options=[
                OptionDefinition.from_declaration(enum_type, option)
                for option in enum_cls.__members__.values()
            ],
        )


@builtin_struct(StructType.PROPERTY_DEFINITION, frozen=True)
class PropertyDefinition(CheckedType):
    """Definition of a builtin Property."""

    id: UInt8 = builtin_property(2, is_repr=True)
    name: str | None = builtin_property(
        101, is_repr=True, description="The name of this Type when it was used."
    )
    description: str | None = builtin_property(103, is_repr=True)
    object: "ObjectDefinitionReference" = builtin_property(
        104, description="The object that this property is defined on."
    )
    original_object: "ObjectDefinitionReference" = builtin_property(
        105, description="The original object that this property was defined on."
    )
    taggings: list[UInt8] = builtin_property(109)

    # relationship
    edge_type: EdgeType | None = builtin_property(190)
    cascade: CascadeAction | None = builtin_property(191)

    # property flags
    is_identity: bool = builtin_property(
        200,
        is_repr=True,
        description="""\
Whether this Property is part of the object's identity.
 (And thus is always required, in every instance including partials; only for Nodes.)
""",
    )
    is_unique: bool = builtin_property(
        201,
        is_repr=True,
        description="Whether this Property must have a unique value.",
    )
    is_readonly: bool = builtin_property(
        202,
        is_repr=True,
        description="Whether this Property is read-only.",
    )

    # internal flags
    is_wired: bool = builtin_property(210)
    is_stored: bool = builtin_property(211)
    is_repr: bool = builtin_property(212)
    is_hash: bool = builtin_property(213)
    is_eq: bool = builtin_property(214)
    is_internal: bool = builtin_property(215)

    @classmethod
    def from_declaration(cls, prop: PropertyDeclaration) -> "PropertyDefinition":
        """Create PropertyDefinition from a Property."""
        from .node import Node
        from .struct import Struct

        assert prop.id is not None, f"{prop!r} has no id"
        type = prop.to_type()
        object_ref = OBJECT_DEFINITION_REFERENCE_BY_CLASS[prop.component]
        original_object_ref = OBJECT_DEFINITION_REFERENCE_BY_CLASS.get(
            prop.original_component, object_ref
        )
        object_cls = prop.component if issubclass(prop.component, (Node, Struct)) else Node
        taggings = [
            resolve_tagging(cast(type_["Node"] | type_["Struct"], object_cls), tag).id
            for tag in prop.tags
        ]

        return cls(
            id=prop.id,
            name=prop.name,
            description=prop.description,
            taggings=taggings,
            object=object_ref,
            original_object=original_object_ref,
            # type
            cardinality=type.cardinality,
            scalar_type=type.scalar_type,
            primitive_type=type.primitive_type,
            enum_type=type.enum_type,
            struct_type=type.struct_type,
            key_type=type.key_type,
            default_value=type.default_value,
            default_factory=type.default_factory,
            collection_constraint=type.collection_constraint,
            string_constraint=type.string_constraint,
            number_constraint=type.number_constraint,
            # node
            edge_type=prop.edge_type,
            cascade=prop.cascade,
            # flags
            is_required=prop.is_required,
            is_identity=prop.is_identity,
            is_unique=prop.is_unique,
            is_readonly=prop.is_readonly,
            is_wired=prop.is_wired,
            is_stored=prop.is_stored,
            is_repr=prop.is_repr,
            is_hash=prop.is_hash,
            is_eq=prop.is_eq,
            is_internal=prop.is_internal,
        )

    @builtin_method(101)
    def to_type(self) -> "CheckedType":
        """Convert to a Type (returns self for convenience)."""
        return self

    @builtin_method(102)
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

    @builtin_method(110)
    def eq(self, value: Any) -> "Condition":
        from ..common import Condition

        if value is None:
            return self.not_exists()
        return Condition.of(self, ConditionalType.EQUALS, value=value)

    @builtin_method(111)
    def neq(self, value: Any) -> "Condition":
        from ..common import Condition

        if value is None:
            return self.exists()
        return Condition.of(self, ConditionalType.NOT_EQUALS, value=value)

    @builtin_method(112)
    def gt(self, value: Any) -> "Condition":
        from ..common import Condition

        return Condition.of(self, ConditionalType.GREATER_THAN, value=value)

    @builtin_method(113)
    def gte(self, value: Any) -> "Condition":
        from ..common import Condition

        return Condition.of(self, ConditionalType.GREATER_THAN_OR_EQUALS, value=value)

    @builtin_method(114)
    def lt(self, value: Any) -> "Condition":
        from ..common import Condition

        return Condition.of(self, ConditionalType.LESS_THAN, value=value)

    @builtin_method(115)
    def lte(self, value: Any) -> "Condition":
        from ..common import Condition

        return Condition.of(self, ConditionalType.LESS_THAN_OR_EQUALS, value=value)

    @builtin_method(116)
    def starts_with(self, value: str) -> "Condition":
        from ..common import Condition

        return Condition.of(self, ConditionalType.STARTS_WITH, value=value)

    @builtin_method(117)
    def ends_with(self, value: str) -> "Condition":
        from ..common import Condition

        return Condition.of(self, ConditionalType.ENDS_WITH, value=value)

    @builtin_method(118)
    def in_(self, *values: Any) -> "Condition":
        from ..common import Condition

        return Condition.of(self, ConditionalType.IN, value=values)

    @builtin_method(119)
    def not_in(self, *values: Any) -> "Condition":
        from ..common import Condition

        return Condition.of(self, ConditionalType.NOT_IN, value=values)

    @builtin_method(120)
    def exists(self) -> "Condition":
        from ..common import Condition

        return Condition.of(self, ConditionalType.EXISTS)

    @builtin_method(121)
    def is_not_none(self) -> "Condition":
        from ..common import Condition

        return Condition.of(self, ConditionalType.EXISTS)

    @builtin_method(122)
    def not_exists(self) -> "Condition":
        from ..common import Condition

        return Condition.of(self, ConditionalType.NOT_EXISTS)

    @builtin_method(123)
    def is_none(self) -> "Condition":
        from ..common import Condition

        return Condition.of(self, ConditionalType.NOT_EXISTS)

    @builtin_method(124)
    def asc(self) -> "Sort":
        from ..common import Sort

        return Sort.of(self, SortType.ASCENDING)

    @builtin_method(125)
    def desc(self) -> "Sort":
        from ..common import Sort

        return Sort.of(self, SortType.DESCENDING)


@builtin_struct(StructType.OPTION_DEFINITION, frozen=True)
class OptionDefinition(StructFrozen):
    """Definition of a builtin Enum Option."""

    id: UInt8 = builtin_property(2, is_repr=True)
    type: EnumType = builtin_property(100, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
    description: str | None = builtin_property(103, is_repr=True)
    taggings: list[UInt8] = builtin_property(109)

    @classmethod
    def from_declaration(cls, enum_type: EnumType, option: Enum) -> "OptionDefinition":
        """Create OptionDefinition from an Enum option."""
        from ..common import to_icon

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

    id: UInt8 = builtin_property(2, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    description: str | None = builtin_property(103, is_repr=True)
    taggings: list[UInt8] = builtin_property(109)

    # content
    value: "Value" = builtin_property(120)

    _is_deferred: bool = builtin_property_runtime()

    @classmethod
    def from_declaration(cls, declaration: "ConstantDeclaration") -> "ConstantDefinition":
        """Create ConstantDefinition from a ConstantDeclaration."""
        assert declaration.name is not None, f"{declaration!r} has no name"
        return cls(
            id=declaration.id,
            name=declaration.name,
            description=declaration.description,
            value=Value.wrap(declaration.value),
            _is_deferred=declaration.is_deferred,
        )


@builtin_struct(StructType.TAG_DEFINITION, frozen=True)
class TagDefinition(StructFrozen):
    """Definition of a builtin Tag to associate builtin definitions to."""

    id: UInt8 = builtin_property(2, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    description: str | None = builtin_property(103, is_repr=True)

    @classmethod
    def from_declaration(cls, declaration: TagDeclaration) -> "TagDefinition":
        """Create TagDefinition from a TagDeclaration."""
        return cls(
            id=declaration.id,
            name=declaration.name,
            description=declaration.description,
        )


@builtin_struct(StructType.INDEX_DEFINITION, frozen=True)
class IndexDefinition(StructFrozen):
    """Definition of a builtin Index."""

    id: UInt8 = builtin_property(2, is_repr=True)
    type: "IndexType" = builtin_property(100, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    description: str | None = builtin_property(103, is_repr=True)

    # content
    properties: list["PropertyReference"] = builtin_property(120)
    cover: list["PropertyReference"] = builtin_property(121)

    @classmethod
    def from_declaration(
        cls, object_cls: type_["BuiltinObject"], declaration: "IndexDeclaration"
    ) -> "Self":
        return cls(
            id=declaration.id,
            type=declaration.type,
            name=declaration.name or "Index",
            properties=[object_cls.property(p).to_ref() for p in declaration.properties],
            cover=[object_cls.property(p).to_ref() for p in declaration.cover],
        )


@builtin_struct(StructType.CONSTRAINT_DEFINITION, frozen=True)
class ConstraintDefinition(StructFrozen):
    """Definition of a builtin Constraint."""

    id: UInt8 = builtin_property(2, is_repr=True)
    type: "ConstraintType" = builtin_property(100, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    description: str | None = builtin_property(103, is_repr=True)

    # content
    properties: list["PropertyReference"] = builtin_property(120)

    @classmethod
    def from_declaration(
        cls, object_cls: type_["BuiltinObject"], declaration: "ConstraintDeclaration"
    ) -> "Self":
        return cls(
            id=declaration.id,
            type=declaration.type,
            name=declaration.name or "Constraint",
            properties=[object_cls.property(p).to_ref() for p in declaration.properties],
        )


@builtin_struct(StructType.PERMISSION_DEFINITION, frozen=True)
class PermissionDefinition(StructFrozen):
    """Definition of a builtin Permission for a builtin Node."""

    id: UInt8 = builtin_property(2, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    description: str | None = builtin_property(103, is_repr=True)

    @classmethod
    def from_declaration(cls, declaration: "PermissionDeclaration") -> "Self":
        return cls(
            id=declaration.id,
            name=declaration.name,
            description=declaration.description,
        )

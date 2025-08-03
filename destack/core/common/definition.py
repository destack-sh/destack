import abc
from typing import TYPE_CHECKING, Any, Self, assert_never, cast, final, override

from destack.registry import OBJECT_DEFINITION_REFERENCE_BY_CLASS

from ..builtin import (
    ActionDeclaration,
    ActionType,
    CascadeAction,
    ConstraintDeclaration,
    EdgeType,
    Enum,
    EnumType,
    HandleDeclaration,
    HandleType,
    IndexDeclaration,
    MethodDeclaration,
    MethodType,
    NodeDeclaration,
    NodeType,
    Object,
    ObjectKind,
    ObjectStability,
    PermissionDeclaration,
    PropertyDeclaration,
    RuntimeLanguage,
    RuntimePlatform,
    Struct,
    StructDeclaration,
    StructFrozen,
    StructType,
    TagDeclaration,
    TraitType,
    UInt8,
    UInt16,
    UInt32,
    ValueFactory,
    declare_method,
    declare_property,
    declare_property_runtime,
    declare_struct,
)
from .relation import ObjectDefinitionReference, PropertyReference, PropertyReferenceType
from .type import Type
from .value import Value

if TYPE_CHECKING:
    from destack.core import (
        Condition,
        ConditionalType,
        ConstantDeclaration,
        ConstraintType,
        Handle,
        IndexType,
        Node,
        ObjectDefinitionReference,
        Sort,
        SortType,
        Value,
    )


type_ = type


def resolve_tagging(
    object_cls: type_["Node | Struct"], tagging: str
) -> "TagDefinition | TagDeclaration":
    """Resolve a tagging to a definition."""
    from ..builtin.node import Node
    from ..builtin.struct import Struct

    for tag in object_cls.__declaration__.tags:
        if tag.name == tagging:
            return tag
    for cls in object_cls.__mro__:
        if issubclass(cls, Node) or issubclass(cls, Struct):
            for tag in cls.__declaration__.tags:
                if tag.name == tagging:
                    return tag

    raise ValueError(f"tagging '{tagging}' not found for {object_cls.__name__}")


@declare_struct(
    StructType.OBJECT_DEFINITION,
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
class ObjectDefinition(StructFrozen):
    """Definition of a builtin Trait, Node or Struct."""

    id: UInt32 = declare_property(
        2,
        is_repr=True,
        tags=("meta",),
    )
    name: str = declare_property(
        101,
        is_repr=True,
        tags=("meta",),
    )
    description: str | None = declare_property(
        103,
        is_repr=True,
        tags=("meta",),
    )

    @abc.abstractmethod
    def to_ref(self) -> "ObjectDefinitionReference":
        raise NotImplementedError


@declare_struct(
    StructType.NODE_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class NodeDefinition(ObjectDefinition):
    """Definition of a builtin Node."""

    # meta
    type: NodeType = declare_property(
        100,
        is_repr=True,
        tags=("meta",),
    )
    stability: ObjectStability = declare_property(
        105,
        description="The stability of this Node (how its definition is expected to change).",
        tags=("meta",),
    )
    is_abstract: bool = declare_property(
        110,
        is_repr=True,
        description="Whether this Node cannot be instantiated directly.",
        tags=("meta",),
    )
    is_final: bool = declare_property(
        111,
        is_repr=True,
        description="Whether this Node cannot be extended by custom Nodes.",
        tags=("meta",),
    )
    is_frozen: bool = declare_property(
        112,
        is_repr=True,
        description="Whether this Node cannot be modified.",
        tags=("meta",),
    )
    is_singleton: bool = declare_property(
        113,
        description="Whether this Node is a singleton (only one instance can exist).",
        tags=("meta",),
    )

    # content
    properties: list["PropertyDefinition"] = declare_property(
        120,
        description="All properties of this Node (including inherited).",
        tags=("content",),
    )
    indexes: list["IndexDefinition"] = declare_property(
        121,
        description="All indexes of this Node (including inherited).",
        tags=("content",),
    )
    constraints: list["ConstraintDefinition"] = declare_property(
        122,
        description="All constraints of this Node (including inherited).",
        tags=("content",),
    )
    permissions: list["PermissionDefinition"] = declare_property(
        123,
        description="All permissions of this Node (including inherited).",
        tags=("content",),
    )
    methods: list["MethodDefinition"] = declare_property(
        125,
        description="All methods of this Node (including inherited).",
        tags=("content",),
    )
    actions: list["ActionDefinition"] = declare_property(
        126,
        description="All actions of this Node (including inherited).",
        tags=("content",),
    )
    constants: list["ConstantDefinition"] = declare_property(
        128,
        description="All constants of this Node (including inherited).",
        tags=("content",),
    )
    tags: list["TagDefinition"] = declare_property(
        129,
        description="All tags of this Node.",
        tags=("content",),
    )

    # inheritance
    base_type: NodeType | None = declare_property(
        130,
        is_repr=True,
        description="The base type this Node extends (directly).",
        tags=("inheritance",),
    )
    extended_by: list[NodeType] = declare_property(
        131,
        description="Nodes that extend this Node type (directly).",
        tags=("inheritance",),
    )
    inherits: list[NodeType] = declare_property(
        132,
        description="Nodes that this Node inherits.",
        tags=("inheritance",),
    )
    inherited_by: list[NodeType] = declare_property(
        133,
        description="Nodes that inherit this Node type.",
        tags=("inheritance",),
    )
    traits: list[TraitType] = declare_property(
        134,
        description="Traits implemented by this Node.",
        tags=("inheritance",),
    )
    self_traits: list[TraitType] = declare_property(
        135,
        description="Traits declared by this Node (directly).",
        tags=("inheritance",),
    )

    # graph
    parent_types: list[NodeType] = declare_property(
        160,
        is_repr=True,
        description="The parent types of this Node type (directly).",
        tags=("graph",),
    )
    child_types: list[NodeType] = declare_property(
        161,
        is_repr=True,
        description="The child types of this Node type (directly).",
        tags=("graph",),
    )
    ancestor_types: list[NodeType] = declare_property(
        162,
        description="The ancestor types of this Node type.",
        tags=("graph",),
    )
    descendant_types: list[NodeType] = declare_property(
        163,
        description="The descendant types of this Node type.",
        tags=("graph",),
    )
    expected_parent_types: list[NodeType] = declare_property(
        170,
        description="The parent types expected for this Node type (any of).",
        tags=("graph",),
    )
    expected_child_types: list[NodeType] = declare_property(
        171,
        description="The child types expected for this Node type (any of).",
        tags=("graph",),
    )
    expected_ancestor_types: list[NodeType] = declare_property(
        172,
        description="The ancestor types expected for this Node type (any of).",
        tags=("graph",),
    )
    expected_descendant_types: list[NodeType] = declare_property(
        173,
        description="The descendant types expected for this Node type (any of).",
        tags=("graph",),
    )

    # associations
    event_types: list[NodeType] = declare_property(
        200,
        description="The event types related to this Node.",
        tags=("associations",),
    )
    self_event_types: list[NodeType] = declare_property(
        201,
        description="The event types declared by this Node (directly).",
        tags=("associations",),
    )
    enum_types: list[EnumType] = declare_property(
        210,
        description="The enum types related to this Node.",
        tags=("associations",),
    )
    self_enum_types: list[EnumType] = declare_property(
        211,
        description="The enum types declared by this Node (directly).",
        tags=("associations",),
    )
    message_types: list[StructType] = declare_property(
        212,
        description="The message types related to this Node.",
        tags=("associations",),
    )
    self_message_types: list[StructType] = declare_property(
        213,
        description="The message types declared by this Node (directly).",
        tags=("associations",),
    )

    @override
    def to_ref(self) -> "ObjectDefinitionReference":
        return ObjectDefinitionReference(kind=ObjectKind.NODE, node_type=self.type)

    @classmethod
    def from_declaration(
        cls, node_cls: type_["Node"], declaration: NodeDeclaration
    ) -> "NodeDefinition":
        """Create NodeDefinition from a Node class."""

        return cls(
            id=node_cls.metatype.value,
            type=node_cls.metatype,
            name=node_cls.__name__,
            description=node_cls.__doc__,
            stability=declaration.stability,
            is_abstract=node_cls.__declaration__.is_abstract,
            is_final=node_cls.__declaration__.is_final,
            is_frozen=node_cls.__declaration__.is_frozen,
            is_singleton=node_cls.__declaration__.is_singleton,
            # content
            properties=[
                PropertyDefinition.from_declaration(prop)
                for prop in node_cls.__properties__.values()
            ],
            indexes=[
                IndexDefinition.from_declaration(node_cls, index) for index in declaration.indexes
            ],
            constraints=[
                ConstraintDefinition.from_declaration(node_cls, constraint)
                for constraint in declaration.constraints
            ],
            permissions=[
                PermissionDefinition.from_declaration(permission)
                for permission in declaration.permissions
            ],
            methods=[
                MethodDefinition.from_declaration(node_cls, method)
                for method in declaration.methods
            ],
            actions=[
                ActionDefinition.from_declaration(node_cls, action)
                for action in declaration.actions
            ],
            constants=[
                ConstantDefinition.from_declaration(constant) for constant in declaration.constants
            ],
            tags=[TagDefinition.from_declaration(tag) for tag in declaration.tags],
            # inheritance
            base_type=declaration.base_type,
            extended_by=list(declaration.extended_by),
            inherits=list(declaration.inherits),
            inherited_by=list(declaration.inherited_by),
            traits=list(declaration.traits),
            self_traits=list(declaration.self_traits),
            # graph
            parent_types=list(declaration.parent_types),
            child_types=list(declaration.child_types),
            ancestor_types=list(declaration.ancestor_types),
            descendant_types=list(declaration.descendant_types),
            expected_parent_types=list(declaration.expected_parent_types),
            expected_child_types=list(declaration.expected_child_types),
            expected_ancestor_types=list(declaration.expected_ancestor_types),
            expected_descendant_types=list(declaration.expected_descendant_types),
            # associations
            event_types=list(declaration.event_types),
            self_event_types=list(declaration.self_event_types),
            enum_types=list(declaration.enum_types),
            self_enum_types=list(declaration.self_enum_types),
        )


@declare_struct(
    StructType.STRUCT_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class StructDefinition(ObjectDefinition):
    """Definition of a builtin Struct."""

    # meta
    type: StructType = declare_property(
        100,
        is_repr=True,
        tags=("meta",),
    )
    stability: ObjectStability = declare_property(
        105,
        description="The stability of this Struct (how its definition is expected to change).",
        tags=("meta",),
    )
    taggings: list[UInt8] = declare_property(
        109,
        tags=("meta",),
    )
    is_frozen: bool = declare_property(
        110,
        description="Whether this Struct is read-only (cannot be modified).",
        tags=("meta",),
    )
    is_abstract: bool = declare_property(
        111,
        description="Whether this Struct is abstract (cannot be instantiated directly).",
        tags=("meta",),
    )

    # content
    properties: list["PropertyDefinition"] = declare_property(
        120,
        description="All properties of this Struct.",
        tags=("content",),
    )
    methods: list["MethodDefinition"] = declare_property(
        125,
        description="All methods of this Struct (excluding actions).",
        tags=("content",),
    )
    constants: list["ConstantDefinition"] = declare_property(
        128,
        tags=("content",),
    )
    tags: list["TagDefinition"] = declare_property(
        129,
        tags=("content",),
    )

    # inheritance
    base_type: StructType | None = declare_property(
        130,
        description="The base type this Struct extends (directly).",
        tags=("inheritance",),
    )
    extended_by: list[StructType] = declare_property(
        131,
        description="Structs that extend this Struct type (directly).",
        tags=("inheritance",),
    )
    inherits: list[StructType] = declare_property(
        132,
        description="Structs that this Struct inherits.",
        tags=("inheritance",),
    )
    inherited_by: list[StructType] = declare_property(
        133,
        description="Structs that inherit this Struct type.",
        tags=("inheritance",),
    )

    # associations
    enum_types: list[EnumType] = declare_property(
        210,
        description="The enum types related to this Node.",
        tags=("associations",),
    )
    self_enum_types: list[EnumType] = declare_property(
        211,
        description="The base enum types related to this Node (directly).",
        tags=("associations",),
    )

    @override
    def to_ref(self) -> "ObjectDefinitionReference":
        return ObjectDefinitionReference(kind=ObjectKind.STRUCT, struct_type=self.type)

    @classmethod
    def from_declaration(
        cls, struct_cls: type_[Struct], declaration: StructDeclaration
    ) -> "StructDefinition":
        """Create StructDefinition from a Struct class."""

        return cls(
            id=struct_cls.metatype.value,
            type=struct_cls.metatype,
            name=struct_cls.__name__,
            description=struct_cls.__doc__,
            stability=declaration.stability,
            is_frozen=declaration.is_frozen,
            is_abstract=declaration.is_abstract,
            # content
            properties=[prop.definition for prop in struct_cls.__properties__.values()],
            methods=[
                MethodDefinition.from_declaration(struct_cls, method)
                for method in declaration.methods
            ],
            constants=[
                ConstantDefinition.from_declaration(constant) for constant in declaration.constants
            ],
            tags=[TagDefinition.from_declaration(tag) for tag in declaration.tags],
            # inheritance
            base_type=declaration.base_type,
            extended_by=list(declaration.extended_by),
            inherits=list(declaration.inherits),
            inherited_by=list(declaration.inherited_by),
            # associations
            enum_types=list(declaration.enum_types),
            self_enum_types=list(declaration.self_enum_types),
        )


@declare_struct(
    StructType.HANDLE_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class HandleDefinition(ObjectDefinition):
    """Definition of a builtin Handle."""

    # meta
    type: HandleType = declare_property(
        100,
        is_repr=True,
        tags=("meta",),
    )
    stability: ObjectStability = declare_property(
        105,
        description="The stability of this Handle (how its definition is expected to change).",
        tags=("meta",),
    )
    taggings: list[UInt8] = declare_property(
        109,
        tags=("meta",),
    )
    is_frozen: bool = declare_property(
        110,
        description="Whether this Handle is read-only (cannot be modified).",
        tags=("meta",),
    )
    is_abstract: bool = declare_property(
        111,
        description="Whether this Handle is abstract (cannot be instantiated directly).",
        tags=("meta",),
    )

    # content
    properties: list["PropertyDefinition"] = declare_property(
        120,
        description="All properties of this Handle.",
        tags=("content",),
    )
    methods: list["MethodDefinition"] = declare_property(
        125,
        description="All methods of this Handle (excluding actions).",
        tags=("content",),
    )
    constants: list["ConstantDefinition"] = declare_property(
        128,
        tags=("content",),
    )
    tags: list["TagDefinition"] = declare_property(
        129,
        tags=("content",),
    )

    # inheritance
    base_type: HandleType | None = declare_property(
        130,
        description="The base type this Handle extends (directly).",
        tags=("inheritance",),
    )
    extended_by: list[HandleType] = declare_property(
        131,
        description="Handles that extend this Handle type (directly).",
        tags=("inheritance",),
    )
    inherits: list[HandleType] = declare_property(
        132,
        description="Handles that this Handle inherits.",
        tags=("inheritance",),
    )
    inherited_by: list[HandleType] = declare_property(
        133,
        description="Handles that inherit this Handle type.",
        tags=("inheritance",),
    )

    # associations
    enum_types: list[EnumType] = declare_property(
        210,
        description="The enum types related to this Handle.",
        tags=("associations",),
    )
    self_enum_types: list[EnumType] = declare_property(
        211,
        description="The enum types declared by this Handle (directly).",
        tags=("associations",),
    )
    event_types: list[NodeType] = declare_property(
        212,
        description="The event types related to this Handle.",
        tags=("associations",),
    )
    self_event_types: list[NodeType] = declare_property(
        213,
        description="The event types declared by this Handle (directly).",
        tags=("associations",),
    )

    @override
    def to_ref(self) -> "ObjectDefinitionReference":
        return ObjectDefinitionReference(kind=ObjectKind.HANDLE, handle_type=self.type)

    @classmethod
    def from_declaration(
        cls, handle_cls: type_["Handle"], declaration: HandleDeclaration
    ) -> "HandleDefinition":
        """Create HandleDefinition from a Handle class."""
        return cls(
            id=handle_cls.metatype.value,
            type=handle_cls.metatype,
            name=handle_cls.__name__,
            description=handle_cls.__doc__,
            stability=declaration.stability,
            is_frozen=declaration.is_frozen,
            is_abstract=declaration.is_abstract,
            # content
            properties=[prop.definition for prop in handle_cls.__properties__.values()],
            methods=[
                MethodDefinition.from_declaration(handle_cls, method)
                for method in declaration.methods
            ],
            constants=[
                ConstantDefinition.from_declaration(constant) for constant in declaration.constants
            ],
            tags=[TagDefinition.from_declaration(tag) for tag in declaration.tags],
            # inheritance
            base_type=declaration.base_type,
            extended_by=list(declaration.extended_by),
            inherits=list(declaration.inherits),
            inherited_by=list(declaration.inherited_by),
            # associations
            enum_types=list(declaration.enum_types),
            self_enum_types=list(declaration.self_enum_types),
            event_types=list(declaration.event_types),
            self_event_types=list(declaration.self_event_types),
        )


@declare_struct(
    StructType.ENUM_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class EnumDefinition(StructFrozen):
    """Definition of a builtin Enum."""

    type: EnumType = declare_property(100, is_repr=True)
    id: UInt32 = declare_property(2, is_repr=True)
    name: str = declare_property(101, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)
    taggings: list[UInt8] = declare_property(109)

    # content
    options: list["OptionDefinition"] = declare_property(120)

    @classmethod
    def from_declaration(cls, enum_type: EnumType, enum_cls: type_[Enum]) -> "EnumDefinition":
        """Create EnumDefinition from an Enum class."""

        return cls(
            id=enum_type.value,
            type=enum_type,
            name=enum_type.camel_name,
            description=enum_type.__doc__,
            options=[
                OptionDefinition.from_declaration(enum_type, option)
                for option in enum_cls.__members__.values()
            ],
        )


@declare_struct(
    StructType.PROPERTY_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class PropertyDefinition(StructFrozen):
    """Definition of a builtin Property."""

    id: UInt8 = declare_property(2, is_repr=True)
    type: Type = declare_property(100)
    name: str | None = declare_property(
        101, is_repr=True, description="The name of this Type when it was used."
    )
    description: str | None = declare_property(103, is_repr=True)
    object: "ObjectDefinitionReference" = declare_property(
        104, description="The object that this property is defined on."
    )
    original_object: "ObjectDefinitionReference" = declare_property(
        105, description="The original object that this property was defined on."
    )
    taggings: list[UInt8] = declare_property(109)

    # defaults
    default_value: Value | None = declare_property(120)
    default_factory: ValueFactory | None = declare_property(121)

    # relationships
    edge_type: EdgeType | None = declare_property(130)
    cascade: CascadeAction | None = declare_property(131)

    # property flags
    is_identity: bool = declare_property(
        140,
        is_repr=True,
        description="""\
Whether this Property is part of the object's identity.
 (And thus is always required, in every instance including partials; only for Nodes.)
""",
    )
    is_unique: bool = declare_property(
        141,
        is_repr=True,
        description="Whether this Property must have a unique value.",
    )
    is_readonly: bool = declare_property(
        142,
        is_repr=True,
        description="Whether this Property is read-only.",
    )
    is_repr: bool = declare_property(143)
    is_hash: bool = declare_property(144)
    is_eq: bool = declare_property(145)
    is_internal: bool = declare_property(146)

    @classmethod
    def from_declaration(cls, prop: PropertyDeclaration) -> "PropertyDefinition":
        """Create PropertyDefinition from a Property."""
        from ..builtin.node import Node
        from ..builtin.struct import Struct

        type = prop.type.to_type()
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
            type=type,
            default_value=prop.default_value,
            default_factory=prop.default_factory,
            # node
            edge_type=prop.edge_type,
            cascade=prop.cascade,
            # flags
            is_identity=prop.is_identity,
            is_unique=prop.is_unique,
            is_readonly=prop.is_readonly,
            is_repr=prop.is_repr,
            is_hash=prop.is_hash,
            is_eq=prop.is_eq,
            is_internal=prop.is_internal,
        )

    @declare_method(102)
    def to_ref(self) -> PropertyReference:
        """Convert to a PropertyReference."""
        if self.object.kind == ObjectKind.NODE:
            return PropertyReference(
                type=PropertyReferenceType.BUILTIN,
                node_type=self.object.node_type,
                id=self.id,
            )
        elif self.object.kind == ObjectKind.STRUCT:
            return PropertyReference(
                type=PropertyReferenceType.BUILTIN,
                struct_type=self.object.struct_type,
                id=self.id,
            )
        elif self.object.kind == ObjectKind.HANDLE:
            return PropertyReference(
                type=PropertyReferenceType.BUILTIN, handle_type=self.object.handle_type, id=self.id
            )
        else:
            assert_never(self.object.kind)

    @declare_method(110)
    def eq(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is equal to a value."""
        from . import Condition

        if value is None:
            return self.not_exists()
        return Condition.of(self, ConditionalType.EQUALS, value=value)

    @declare_method(111)
    def neq(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is not equal to a value."""
        from . import Condition

        if value is None:
            return self.exists()
        return Condition.of(self, ConditionalType.NOT_EQUALS, value=value)

    @declare_method(112)
    def gt(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is greater than a value."""
        from . import Condition

        return Condition.of(self, ConditionalType.GREATER_THAN, value=value)

    @declare_method(113)
    def gte(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is greater than or equal to a value."""
        from . import Condition

        return Condition.of(self, ConditionalType.GREATER_THAN_OR_EQUALS, value=value)

    @declare_method(114)
    def lt(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is less than a value."""
        from . import Condition

        return Condition.of(self, ConditionalType.LESS_THAN, value=value)

    @declare_method(115)
    def lte(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is less than or equal to a value."""
        from . import Condition

        return Condition.of(self, ConditionalType.LESS_THAN_OR_EQUALS, value=value)

    @declare_method(116)
    def starts_with(self, value: str) -> "Condition":
        """Create a Condition that checks if this Property starts with a value."""
        from . import Condition

        return Condition.of(self, ConditionalType.STARTS_WITH, value=value)

    @declare_method(117)
    def ends_with(self, value: str) -> "Condition":
        """Create a Condition that checks if this Property ends with a value."""
        from . import Condition

        return Condition.of(self, ConditionalType.ENDS_WITH, value=value)

    @declare_method(118)
    def in_(self, *values: Any) -> "Condition":
        """Create a Condition that checks if this Property is in a list of values."""
        from . import Condition

        return Condition.of(self, ConditionalType.IN, value=values)

    @declare_method(119)
    def not_in(self, *values: Any) -> "Condition":
        """Create a Condition that checks if this Property is not in a list of values."""
        from . import Condition

        return Condition.of(self, ConditionalType.NOT_IN, value=values)

    @declare_method(120)
    def exists(self) -> "Condition":
        """Create a Condition that checks if this Property exists."""
        from . import Condition

        return Condition.of(self, ConditionalType.EXISTS)

    @declare_method(121)
    def is_not_none(self) -> "Condition":
        """Create a Condition that checks if this Property is not None."""
        from . import Condition

        return Condition.of(self, ConditionalType.EXISTS)

    @declare_method(122)
    def not_exists(self) -> "Condition":
        """Create a Condition that checks if this Property does not exist."""
        from . import Condition

        return Condition.of(self, ConditionalType.NOT_EXISTS)

    @declare_method(123)
    def is_none(self) -> "Condition":
        """Create a Condition that checks if this Property is None."""
        from . import Condition

        return Condition.of(self, ConditionalType.NOT_EXISTS)

    @declare_method(124)
    def asc(self) -> "Sort":
        """Create a Sort that sorts this Property in ascending order."""
        from . import Sort

        return Sort.of(self, SortType.ASCENDING)

    @declare_method(125)
    def desc(self) -> "Sort":
        """Create a Sort that sorts this Property in descending order."""
        from . import Sort

        return Sort.of(self, SortType.DESCENDING)


@declare_struct(
    StructType.OPTION_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class OptionDefinition(StructFrozen):
    """Definition of a builtin Enum Option."""

    id: UInt8 = declare_property(2, is_repr=True)
    type: EnumType = declare_property(100, is_repr=True)
    name: str = declare_property(101, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)
    taggings: list[UInt8] = declare_property(109)

    @classmethod
    def from_declaration(cls, enum_type: EnumType, option: Enum) -> "OptionDefinition":
        """Create OptionDefinition from an Enum option."""
        return cls(
            id=option.value,
            type=enum_type,
            name=option.name,
            description=option.__doc__,
        )


@declare_struct(
    StructType.CONSTANT_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class ConstantDefinition(StructFrozen):
    """Definition of a builtin Constant."""

    id: UInt8 = declare_property(2, is_repr=True)
    name: str = declare_property(101, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)
    taggings: list[UInt8] = declare_property(109)

    # content
    value: "Value" = declare_property(120)

    _is_deferred: bool = declare_property_runtime(401)

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


@declare_struct(
    StructType.TAG_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class TagDefinition(StructFrozen):
    """Definition of a builtin Tag to associate builtin definitions to."""

    id: UInt8 = declare_property(2, is_repr=True)
    name: str = declare_property(101, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)

    @classmethod
    def from_declaration(cls, declaration: TagDeclaration) -> "TagDefinition":
        """Create TagDefinition from a TagDeclaration."""
        return cls(
            id=declaration.id,
            name=declaration.name,
            description=declaration.description,
        )


@declare_struct(
    StructType.INDEX_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class IndexDefinition(StructFrozen):
    """Definition of a builtin Index."""

    id: UInt8 = declare_property(2, is_repr=True)
    type: "IndexType" = declare_property(100, is_repr=True)
    name: str = declare_property(101, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)

    # content
    properties: list["PropertyReference"] = declare_property(120)
    cover: list["PropertyReference"] = declare_property(121)

    @classmethod
    def from_declaration(
        cls, object_cls: type_["Object"], declaration: "IndexDeclaration"
    ) -> "Self":
        return cls(
            id=declaration.id,
            type=declaration.type,
            name=declaration.name or "Index",
            properties=[object_cls.property(p).to_ref() for p in declaration.properties],
            cover=[object_cls.property(p).to_ref() for p in declaration.cover],
        )


@declare_struct(
    StructType.CONSTRAINT_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class ConstraintDefinition(StructFrozen):
    """Definition of a builtin Constraint."""

    id: UInt8 = declare_property(2, is_repr=True)
    type: "ConstraintType" = declare_property(100, is_repr=True)
    name: str = declare_property(101, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)

    # content
    properties: list["PropertyReference"] = declare_property(120)

    @classmethod
    def from_declaration(
        cls, object_cls: type_["Object"], declaration: "ConstraintDeclaration"
    ) -> "Self":
        return cls(
            id=declaration.id,
            type=declaration.type,
            name=declaration.name or "Constraint",
            properties=[object_cls.property(p).to_ref() for p in declaration.properties],
        )


@declare_struct(
    StructType.PERMISSION_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class PermissionDefinition(StructFrozen):
    """Definition of a builtin Permission for a builtin Node."""

    id: UInt8 = declare_property(2, is_repr=True)
    name: str = declare_property(101, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)

    @classmethod
    def from_declaration(cls, declaration: "PermissionDeclaration") -> "Self":
        return cls(
            id=declaration.id,
            name=declaration.name,
            description=declaration.description,
        )


@declare_struct(
    StructType.FUNCTION_DEFINITION,
    frozen=True,
    is_abstract=True,
)
class FunctionDefinition(StructFrozen):
    """Definition of a builtin Function."""

    # meta
    id: UInt16 = declare_property(2, is_repr=True)
    name: str = declare_property(101)
    description: str | None = declare_property(103, is_repr=True)
    is_async: bool = declare_property(110)
    is_abstract: bool = declare_property(111)

    # availability
    platforms: list[RuntimePlatform] | None = declare_property(
        130,
        description="The platforms this Method is available on (all if empty).",
    )
    languages: list[RuntimeLanguage] | None = declare_property(
        131,
        description="The languages this Method is available in (all if empty).",
    )


@declare_struct(
    StructType.METHOD_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class MethodDefinition(FunctionDefinition):
    """Definition of a builtin Method."""

    type: MethodType = declare_property(100)

    # content
    input_properties: list["PropertyDefinition"] = declare_property(121)
    output_properties: list["PropertyDefinition"] | None = declare_property(122)
    output_is_scalar: bool = declare_property(
        123, description="Whether the output is just the first output property."
    )

    @classmethod
    def from_declaration(
        cls, object_cls: type_["Object"], declaration: "MethodDeclaration"
    ) -> "Self":
        return cls(
            id=declaration.id,
            type=declaration.type,
            name=declaration.name,
            description=declaration.description,
        )


@declare_struct(
    StructType.ACTION_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class ActionDefinition(FunctionDefinition):
    """Definition of a builtin Action."""

    type: ActionType = declare_property(100)
    request_message: "StructDefinition" = declare_property(120)
    response_message: "StructDefinition" = declare_property(121)

    @classmethod
    def from_declaration(
        cls, object_cls: type_["Object"], declaration: "ActionDeclaration"
    ) -> "Self":
        return cls(
            id=declaration.id,
            type=declaration.type,
            name=declaration.name,
            description=declaration.description,
        )

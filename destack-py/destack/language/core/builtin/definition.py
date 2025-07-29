from typing import TYPE_CHECKING, Any, Self, assert_never, cast

from destack.language.registry import OBJECT_DEFINITION_REFERENCE_BY_CLASS

from .builtin import NodeType, ObjectStability, StructType, TraitType
from .common import CascadeAction, EdgeType, Enum, EnumType, UInt8, UInt32
from .declaration import (
    ConstraintDeclaration,
    IndexDeclaration,
    NodeDeclaration,
    PermissionDeclaration,
    StructDeclaration,
    TagDeclaration,
    builtin_method,
)
from .object import Object
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
        Value,
    )


type_ = type


def resolve_tagging(
    object_cls: type_["Node | Struct"], tagging: str
) -> "TagDefinition | TagDeclaration":
    """Resolve a tagging to a definition."""
    from .node import Node
    from .struct import Struct

    for tag in object_cls.__declaration__.tags:
        if tag.name == tagging:
            return tag
    for cls in object_cls.__mro__:
        if issubclass(cls, Node) or issubclass(cls, Struct):
            for tag in cls.__declaration__.tags:
                if tag.name == tagging:
                    return tag

    raise ValueError(f"tagging '{tagging}' not found for {object_cls.__name__}")


@builtin_struct(
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


@builtin_struct(StructType.NODE_DEFINITION, frozen=True)
class NodeDefinition(ObjectDefinition):
    """Definition of a builtin Node."""

    # meta
    type: NodeType = builtin_property(
        100,
        is_repr=True,
        tags=("meta",),
    )
    stability: ObjectStability = builtin_property(
        105,
        description="The stability of this Node (how its definition is expected to change).",
        tags=("meta",),
    )
    is_abstract: bool = builtin_property(
        110,
        is_repr=True,
        description="Whether this Node cannot be instantiated directly.",
        tags=("meta",),
    )
    is_final: bool = builtin_property(
        111,
        is_repr=True,
        description="Whether this Node cannot be extended by custom Nodes.",
        tags=("meta",),
    )
    is_frozen: bool = builtin_property(
        112,
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
    def from_declaration(
        cls, node_cls: type_["Node"], declaration: NodeDeclaration
    ) -> "NodeDefinition":
        """Create NodeDefinition from a Node class."""
        from ..common import to_icon

        return cls(
            id=node_cls.metatype.value,
            type=node_cls.metatype,
            name=node_cls.__name__,
            icon=to_icon(node_cls.metatype.icon) if node_cls.metatype.icon else None,
            description=node_cls.__doc__,
            stability=node_cls.__stability__,
            is_abstract=node_cls.__declaration__.is_abstract,
            is_final=node_cls.__declaration__.is_final,
            is_frozen=node_cls.__declaration__.is_frozen,
            # content
            properties=[
                PropertyDefinition.from_declaration(prop)
                for prop in node_cls.__properties__.values()
                if not prop.is_runtime_only
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
            methods=[],
            actions=[],
            constants=[],
            tags=[],
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


@builtin_struct(StructType.STRUCT_DEFINITION, frozen=True)
class StructDefinition(ObjectDefinition):
    """Definition of a builtin Struct."""

    # meta
    type: StructType = builtin_property(
        100,
        is_repr=True,
        tags=("meta",),
    )
    stability: ObjectStability = builtin_property(
        105,
        description="The stability of this Struct (how its definition is expected to change).",
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
    def from_declaration(
        cls, struct_cls: type_[Struct], declaration: StructDeclaration
    ) -> "StructDefinition":
        """Create StructDefinition from a Struct class."""
        from ..common import to_icon

        return cls(
            id=struct_cls.metatype.value,
            type=struct_cls.metatype,
            name=struct_cls.__name__,
            icon=to_icon(struct_cls.metatype.icon) if struct_cls.metatype.icon else None,
            description=struct_cls.__doc__,
            stability=struct_cls.__stability__,
            is_frozen=declaration.is_frozen,
            is_abstract=declaration.is_abstract,
            # content
            properties=[
                prop.definition
                for prop in struct_cls.__properties__.values()
                if not prop.is_runtime_only
            ],
            methods=[],
            actions=[],
            constants=[],
            tags=[],
            # inheritance
            base_type=declaration.base_type,
            extended_by=list(declaration.extended_by),
            inherits=list(declaration.inherits),
            inherited_by=list(declaration.inherited_by),
            # associations
            enum_types=list(declaration.enum_types),
            self_enum_types=list(declaration.self_enum_types),
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
        elif (
            self.object.type == ObjectDefinitionType.CUSTOM_NODE
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
        cls, object_cls: type_["Object"], declaration: "IndexDeclaration"
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
        cls, object_cls: type_["Object"], declaration: "ConstraintDeclaration"
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

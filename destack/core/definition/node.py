from typing import TYPE_CHECKING, final, override

from ..builtin import (
    Node,
    NodeDeclaration,
    NodeType,
    ObjectKind,
    ObjectStability,
    StructType,
    TraitType,
    declare_property,
    declare_struct,
)
from .object import ObjectDefinition

if TYPE_CHECKING:
    from destack import ObjectDefinitionReference

    from .action import ActionDefinition
    from .constant import ConstantDefinition
    from .constraint import ConstraintDefinition
    from .index import IndexDefinition
    from .method import MethodDefinition
    from .permission import PermissionDefinition
    from .property import PropertyDefinition
    from .tag import TagDefinition


type_ = type


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
    is_singleton: bool = declare_property(
        112,
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
    base_struct_type: StructType | None = declare_property(
        220,
        description="The Struct type this Node implements (if any).",
        tags=("inheritance",),
    )

    @override
    def to_ref(self) -> "ObjectDefinitionReference":
        from ..common import ObjectDefinitionReference

        return ObjectDefinitionReference(kind=ObjectKind.NODE, node_type=self.type)

    @classmethod
    def from_declaration(
        cls, node_cls: type_["Node"], declaration: NodeDeclaration
    ) -> "NodeDefinition":
        """Create NodeDefinition from a Node class."""
        from .action import ActionDefinition
        from .constant import ConstantDefinition
        from .constraint import ConstraintDefinition
        from .index import IndexDefinition
        from .method import MethodDefinition
        from .permission import PermissionDefinition
        from .property import PropertyDefinition
        from .tag import TagDefinition

        return cls(
            id=node_cls.metatype.value,
            type=node_cls.metatype,
            name=node_cls.__name__,
            description=node_cls.__doc__,
            stability=declaration.stability,
            is_abstract=node_cls.__declaration__.is_abstract,
            is_final=node_cls.__declaration__.is_final,
            is_singleton=node_cls.__declaration__.is_singleton,
            # content
            properties=[
                PropertyDefinition.from_declaration(prop)
                for prop in node_cls.__properties__.values()
            ],
            indexes=[IndexDefinition.from_declaration(index) for index in declaration.indexes],
            constraints=[
                ConstraintDefinition.from_declaration(constraint)
                for constraint in declaration.constraints
            ],
            permissions=[
                PermissionDefinition.from_declaration(permission)
                for permission in declaration.permissions
            ],
            methods=[MethodDefinition.from_declaration(method) for method in declaration.methods],
            actions=[ActionDefinition.from_declaration(action) for action in declaration.actions],
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
            base_struct_type=declaration.base_struct_type,
        )

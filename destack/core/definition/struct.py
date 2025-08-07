from typing import TYPE_CHECKING, final, override

from ..builtin import (
    NodeType,
    ObjectKind,
    ObjectStability,
    Struct,
    StructDeclaration,
    StructType,
    UInt8,
    declare_property,
    declare_struct,
)
from .object import ObjectDefinition

if TYPE_CHECKING:
    from destack import (
        ConstantDefinition,
        MethodDefinition,
        ObjectDefinitionReference,
        PropertyDefinition,
        TagDefinition,
    )


type_ = type


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
    is_immutable: bool = declare_property(
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
    into_node_types: list[NodeType] = declare_property(
        220,
        description="The node types that this Struct can be turned into.",
        tags=("associations",),
    )

    @override
    def to_ref(self) -> "ObjectDefinitionReference":
        from ..common import ObjectDefinitionReference

        return ObjectDefinitionReference(kind=ObjectKind.STRUCT, struct_type=self.type)

    @classmethod
    def from_declaration(
        cls, struct_cls: type_[Struct], declaration: StructDeclaration
    ) -> "StructDefinition":
        """Create StructDefinition from a Struct class."""
        from .constant import ConstantDefinition
        from .method import MethodDefinition
        from .property import PropertyDefinition
        from .tag import TagDefinition

        properties = sorted(
            [
                PropertyDefinition.from_declaration(prop)
                for prop in struct_cls.__properties__.values()
            ],
            key=lambda p: p.id,
        )
        methods = sorted(
            [MethodDefinition.from_declaration(method) for method in declaration.methods],
            key=lambda m: m.id,
        )
        constants = sorted(
            [ConstantDefinition.from_declaration(constant) for constant in declaration.constants],
            key=lambda c: c.id,
        )
        tags = sorted(
            [TagDefinition.from_declaration(tag) for tag in declaration.tags],
            key=lambda t: t.id,
        )

        return cls(
            # meta
            id=struct_cls.metatype.value,
            type=struct_cls.metatype,
            name=struct_cls.__name__,
            description=struct_cls.__doc__,
            stability=declaration.stability,
            is_immutable=declaration.is_immutable,
            is_abstract=declaration.is_abstract,
            # content
            properties=properties,
            methods=methods,
            constants=constants,
            tags=tags,
            # inheritance
            base_type=declaration.base_type,
            extended_by=list(declaration.extended_by),
            inherits=list(declaration.inherits),
            inherited_by=list(declaration.inherited_by),
            # associations
            into_node_types=list(declaration.into_node_types),
        )

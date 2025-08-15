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
    is_final=True,
)
@final
class StructDefinition(ObjectDefinition):
    """Definition of a builtin Struct."""

    # meta
    type: StructType = declare_property(
        100,
        is_repr=True,
        tag="meta",
    )
    stability: ObjectStability = declare_property(
        105,
        description="The stability of this Struct (how its definition is expected to change).",
        tag="meta",
    )
    taggings: list[UInt8] = declare_property(
        109,
        tag="meta",
    )
    is_immutable: bool = declare_property(
        110,
        description="Whether this Struct is read-only (cannot be modified).",
        tag="meta",
    )
    is_abstract: bool = declare_property(
        111,
        description="Whether this Struct is abstract (cannot be instantiated directly).",
        tag="meta",
    )
    is_interned: bool = declare_property(
        112,
        description="Whether this Struct is interned (managed as a constant pool in core).",
        tag="meta",
    )

    # content
    properties: list["PropertyDefinition"] = declare_property(
        120,
        description="All properties of this Struct.",
        tag="content",
    )
    methods: list["MethodDefinition"] = declare_property(
        125,
        description="All methods of this Struct (excluding actions).",
        tag="content",
    )
    constants: list["ConstantDefinition"] = declare_property(
        128,
        tag="content",
    )
    tags: list["TagDefinition"] = declare_property(
        129,
        tag="content",
    )

    # inheritance
    base_type: StructType | None = declare_property(
        130,
        description="The base type this Struct extends (directly).",
        tag="inheritance",
    )
    extended_by: list[StructType] = declare_property(
        131,
        description="Structs that extend this Struct type (directly).",
        tag="inheritance",
    )
    inherits: list[StructType] = declare_property(
        132,
        description="Structs that this Struct inherits.",
        tag="inheritance",
    )
    inherited_by: list[StructType] = declare_property(
        133,
        description="Structs that inherit this Struct type.",
        tag="inheritance",
    )

    # associations
    into_node_types: list[NodeType] = declare_property(
        220,
        description="The node types that this Struct can be turned into.",
        tag="associations",
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
            description=struct_cls.__doc__ or "",
            stability=declaration.stability,
            is_abstract=declaration.is_abstract,
            is_immutable=declaration.is_immutable,
            is_interned=declaration.is_interned,
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

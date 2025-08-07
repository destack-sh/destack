from typing import TYPE_CHECKING, final, override

from ..builtin import (
    HandleDeclaration,
    HandleType,
    ObjectKind,
    ObjectStability,
    StructType,
    UInt8,
    declare_property,
    declare_struct,
)
from .object import ObjectDefinition

if TYPE_CHECKING:
    from destack import ObjectDefinitionReference

    from .constant import ConstantDefinition
    from .method import MethodDefinition
    from .property import PropertyDefinition
    from .tag import TagDefinition


type_ = type


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
    is_immutable: bool = declare_property(
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

    @override
    def to_ref(self) -> "ObjectDefinitionReference":
        from ..common import ObjectDefinitionReference

        return ObjectDefinitionReference(kind=ObjectKind.HANDLE, handle_type=self.type)

    @classmethod
    def from_declaration(cls, declaration: HandleDeclaration) -> "HandleDefinition":
        """Create HandleDefinition from a Handle class."""
        from .constant import ConstantDefinition
        from .method import MethodDefinition
        from .property import PropertyDefinition
        from .tag import TagDefinition

        return cls(
            # meta
            id=declaration.id,
            type=declaration.type,
            name=declaration.name,
            description=declaration.description,
            stability=declaration.stability,
            is_immutable=declaration.is_immutable,
            is_abstract=declaration.is_abstract,
            # content
            properties=[
                PropertyDefinition.from_declaration(prop) for prop in declaration.properties
            ],
            methods=[MethodDefinition.from_declaration(method) for method in declaration.methods],
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
        )

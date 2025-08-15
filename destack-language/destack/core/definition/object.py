import abc
from typing import TYPE_CHECKING

from ..builtin import (
    Node,
    Struct,
    StructType,
    TagDeclaration,
    UInt32,
    UniverseCategory,
    UniverseDomain,
    declare_property,
    declare_struct,
)
from .definition import Definition

if TYPE_CHECKING:
    from destack import ObjectDefinitionReference

    from .tag import TagDefinition


type_ = type


def resolve_tagging(
    object_cls: type_["Node | Struct"], tagging: str
) -> "TagDefinition | TagDeclaration":
    """Resolve a tagging to a definition."""

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
class ObjectDefinition(Definition):
    """Definition of a builtin Trait, Node or Struct."""

    id: UInt32 = declare_property(
        2,
        is_repr=True,
        tag="meta",
    )
    domain: UniverseDomain = declare_property(103, tag="meta")
    category: UniverseCategory = declare_property(104, tag="meta")

    @abc.abstractmethod
    def to_ref(self) -> "ObjectDefinitionReference":
        """Get a reference to this ObjectDefinition."""
        raise NotImplementedError

from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import assert_never

from destack.language import BuiltinObjectBase, Enum, EnumType, NodeType, StructType, TraitType
from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    TRAIT_CLASS_BY_TYPE,
)

from .const import Definition, Kind


@dataclass(slots=True)
class TypescriptFile:
    name: str
    module: str
    path: Path
    definitions: Mapping[str, "TypescriptDefinition"]
    dependencies: Mapping[str, "TypescriptDefinition"]
    existing_str: str | None = None
    new_str: str | None = None

    def get_definition(self, kind: Kind, id: int) -> "TypescriptDefinition | None":
        """Resolve a definition by kind and id."""
        if kind == "ENUM":
            cls = ENUM_CLASS_BY_TYPE[EnumType(id)]
        elif kind == "STRUCT":
            cls = STRUCT_CLASS_BY_TYPE[StructType(id)]
        elif kind == "TRAIT":
            cls = TRAIT_CLASS_BY_TYPE[TraitType(id)]
        elif kind == "NODE":
            cls = NODE_CLASS_BY_TYPE[NodeType(id)]
        else:
            assert_never(kind)
        definition = self.definitions.get(cls.__name__)
        return definition


@dataclass(slots=True)
class TypescriptDefinition:
    name: str
    cls: type[BuiltinObjectBase] | type[Enum]
    module: str
    kind: Kind
    id: int
    definition: Definition
    definition_str: str
    dependencies: Mapping[str, Definition]


@dataclass(slots=True)
class TypescriptDefinitionBlock:
    """A generated definition block with optional custom content."""

    kind: Kind
    id: int
    custom_content: str = ""


@dataclass(slots=True)
class TypescriptCodeBlock:
    """A block of non-generated code."""

    content: str

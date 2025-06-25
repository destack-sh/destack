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
            metatype = EnumType(id)
            cls = ENUM_CLASS_BY_TYPE[metatype]
        elif kind == "STRUCT":
            metatype = StructType(id)
            cls = STRUCT_CLASS_BY_TYPE[StructType(id)]
        elif kind == "TRAIT":
            metatype = TraitType(id)
            cls = TRAIT_CLASS_BY_TYPE[metatype]
        elif kind == "NODE":
            metatype = NodeType(id)
            cls = NODE_CLASS_BY_TYPE[metatype]
        else:
            assert_never(kind)
        definition = self.definitions.get(cls.__name__)
        if definition is None and kind == "TRAIT":
            definition = self.definitions.get(metatype.camel_name)
        return definition


@dataclass(slots=True)
class TypescriptDefinition:
    name: str
    cls: type[BuiltinObjectBase] | type[Enum]
    module: str  # destack.language.core.common.icon
    submodule: str  # core.builtin or space
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


@dataclass(slots=True)
class TypescriptImportBlock:
    """A block of import statements."""

    content: str
    imports: list["TypescriptImport"]


@dataclass(slots=True)
class TypescriptImport:
    """An import statement."""

    content: str
    path: str
    is_type: bool
    names: list[str]

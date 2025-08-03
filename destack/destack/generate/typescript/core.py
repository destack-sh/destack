from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import assert_never

from destack.language import EnumType, NodeType, StructType
from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
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

    def get_definition(self, kind: Kind, id: int | str) -> "TypescriptDefinition | None":
        """Resolve a definition by kind and id."""
        if kind == "ENUM":
            if id not in EnumType:
                return None
            enum_cls = ENUM_CLASS_BY_TYPE[EnumType(id)]
            return self.definitions.get(enum_cls.__name__)
        elif kind == "STRUCT":
            if id not in StructType:
                return None
            struct_cls = STRUCT_CLASS_BY_TYPE[StructType(id)]
            return self.definitions.get(struct_cls.__name__)
        elif kind == "NODE":
            if id not in NodeType:
                return None
            node_cls = NODE_CLASS_BY_TYPE[NodeType(id)]
            return self.definitions.get(node_cls.__name__)
        elif kind == "CONSTANT":
            assert isinstance(id, str), f"invalid constant id: {id}"
            return self.definitions.get(id)
        else:
            assert_never(kind)


@dataclass(slots=True)
class TypescriptDefinition:
    name: str
    alias: str
    module: str  # destack.language.core.common.icon
    submodule: str  # core.builtin or space
    kind: Kind
    id: int | str
    definition: Definition
    definition_str: str
    dependencies: Mapping[str, Definition]
    value_dependencies: set[str]


@dataclass(slots=True)
class TypescriptDefinitionBlock:
    """A generated definition block with optional custom content."""

    kind: Kind
    id: int | str
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

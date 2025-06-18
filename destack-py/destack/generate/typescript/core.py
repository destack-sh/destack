from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

from destack.language import BuiltinObjectBase, Enum

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

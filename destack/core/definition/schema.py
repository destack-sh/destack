from typing import final

from ..builtin import StructType, declare_property, declare_struct
from .definition import Definition
from .enum import EnumDefinition
from .handle import HandleDefinition
from .module import ModuleDefinition
from .node import NodeDefinition
from .struct import StructDefinition


@declare_struct(
    StructType.SCHEMA_DEFINITION,
    is_final=True,
)
@final
class SchemaDefinition(Definition):
    """Definition of the entire Destack Schema ("language definition")."""

    # meta
    version: str = declare_property(108)

    # content
    modules: list[ModuleDefinition] = declare_property(120)
    nodes: list[NodeDefinition] = declare_property(121)
    structs: list[StructDefinition] = declare_property(122)
    handles: list[HandleDefinition] = declare_property(123)
    enums: list[EnumDefinition] = declare_property(124)

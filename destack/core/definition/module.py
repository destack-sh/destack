from typing import TYPE_CHECKING, final

from ..builtin import (
    ModuleDeclaration,
    ModuleType,
    StructType,
    UInt32,
    declare_property,
    declare_struct,
)
from .definition import Definition

if TYPE_CHECKING:
    from destack import (
        ConstantDefinition,
        EnumType,
        HandleType,
        MethodDefinition,
        NodeType,
        StructType,
    )


@declare_struct(
    StructType.MODULE_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class ModuleDefinition(Definition):
    """Definition of a builtin Module."""

    id: UInt32 = declare_property(2, is_repr=True)
    type: ModuleType = declare_property(100, is_repr=True)

    # content
    methods: list["MethodDefinition"] = declare_property(120)
    constants: list["ConstantDefinition"] = declare_property(121)
    node_types: list["NodeType"] = declare_property(130)
    struct_types: list["StructType"] = declare_property(131)
    handle_types: list["HandleType"] = declare_property(132)
    enum_types: list["EnumType"] = declare_property(133)

    @classmethod
    def from_declaration(
        cls, module_type: ModuleType, declaration: ModuleDeclaration
    ) -> "ModuleDefinition":
        """Create ModuleDefinition from a ModuleDeclaration."""
        return cls(
            # meta
            id=declaration.id,
            type=module_type,
            name=declaration.name,
            description=declaration.description,
        )

from typing import TYPE_CHECKING, final

from ..builtin import ModuleDeclaration, StructType, declare_property, declare_struct
from .definition import Definition

if TYPE_CHECKING:
    from destack import (
        ConstantDefinition,
        EnumType,
        HandleType,
        MethodDefinition,
        NodeType,
        StructType,
        UniverseCategory,
        UniverseDomain,
    )


@declare_struct(
    StructType.MODULE_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class ModuleDefinition(Definition):
    """Definition of a builtin Module."""

    # meta
    is_global: bool = declare_property(105)
    domain: "UniverseDomain" = declare_property(106)
    category: "UniverseCategory" = declare_property(107)

    # content
    methods: list["MethodDefinition"] = declare_property(120)
    constants: list["ConstantDefinition"] = declare_property(121)
    node_types: list["NodeType"] = declare_property(130)
    struct_types: list["StructType"] = declare_property(131)
    handle_types: list["HandleType"] = declare_property(132)
    enum_types: list["EnumType"] = declare_property(133)

    # graph
    children: list["ModuleDefinition"] = declare_property(140)

    @classmethod
    def from_declaration(cls, declaration: ModuleDeclaration) -> "ModuleDefinition":
        """Create ModuleDefinition from a ModuleDeclaration."""
        from .constant import ConstantDefinition
        from .method import MethodDefinition

        return cls(
            # meta
            name=declaration.name,
            description=declaration.description,
            is_global=declaration.is_global,
            domain=declaration.domain,
            category=declaration.category,
            # content
            methods=[MethodDefinition.from_declaration(method) for method in declaration.methods],
            constants=[
                ConstantDefinition.from_declaration(constant) for constant in declaration.constants
            ],
            node_types=list(declaration.node_types),
            struct_types=list(declaration.struct_types),
            handle_types=list(declaration.handle_types),
            enum_types=list(declaration.enum_types),
            # graph
            children=[ModuleDefinition.from_declaration(child) for child in declaration.children],
        )

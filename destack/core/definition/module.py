from typing import TYPE_CHECKING, Optional, final

from ..builtin import (
    EnumType,
    HandleType,
    NodeType,
    OptionEnum,
    StructType,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)
from .definition import Definition

if TYPE_CHECKING:
    from destack import (
        ConstantDefinition,
        MethodDefinition,
        UniverseCategory,
        UniverseDomain,
    )


@declare_enum(EnumType.MODULE_TYPE)
class ModuleType(OptionEnum):
    """Built-in module types."""

    ROOT = declare_option(
        1,
        "Root",
        description="Root module for the entire Universe",
    )
    DOMAIN = declare_option(
        2,
        "Domain",
        description="Module for an entire UniverseDomain",
    )
    CATEGORY = declare_option(
        3,
        "Category",
        description="Module for an entire UniverseCategory",
    )
    OBJECT = declare_option(
        4,
        "Object",
        description="Module for one or more Objects",
    )


@declare_struct(
    StructType.MODULE_DEFINITION,
    is_final=True,
)
@final
class ModuleDefinition(Definition):
    """Definition of a builtin Module."""

    # meta
    type: ModuleType = declare_property(100, is_repr=True, tag=None)
    path: str = declare_property(104, is_repr=True, tag=None)
    domain: Optional["UniverseDomain"] = declare_property(106, tag=None)
    category: Optional["UniverseCategory"] = declare_property(107, tag=None)

    # content
    methods: list["MethodDefinition"] = declare_property(120, tag=None)
    constants: list["ConstantDefinition"] = declare_property(121, tag=None)
    node_types: list["NodeType"] = declare_property(130, tag=None)
    struct_types: list["StructType"] = declare_property(131, tag=None)
    handle_types: list["HandleType"] = declare_property(132, tag=None)
    enum_types: list["EnumType"] = declare_property(133, tag=None)

    # graph
    parent_path: str | None = declare_property(140, tag=None)
    children_paths: list[str] = declare_property(141, tag=None)

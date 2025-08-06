from typing import TYPE_CHECKING, final

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
    frozen=True,
    is_final=True,
)
@final
class ModuleDefinition(Definition):
    """Definition of a builtin Module."""

    # meta
    type: ModuleType = declare_property(100, is_repr=True)
    path: str = declare_property(104, is_repr=True)
    domain: "UniverseDomain | None" = declare_property(106)
    category: "UniverseCategory | None" = declare_property(107)

    # content
    methods: list["MethodDefinition"] = declare_property(120)
    node_types: list["NodeType"] = declare_property(130)
    struct_types: list["StructType"] = declare_property(131)
    handle_types: list["HandleType"] = declare_property(132)
    enum_types: list["EnumType"] = declare_property(133)

    # graph
    parent_path: str | None = declare_property(140)
    children_paths: list[str] = declare_property(141)

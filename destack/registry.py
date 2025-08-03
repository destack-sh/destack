from typing import TYPE_CHECKING, assert_never

from .core.builtin.enum import _ENUM_CLASS_BY_TYPE, _ENUM_TYPE_BY_CLASS

if TYPE_CHECKING:
    from destack.core import (
        EnumDeclaration,
        EnumDefinition,
        EnumType,
        Handle,
        HandleDefinition,
        HandleType,
        Node,
        NodeDefinition,
        NodeType,
        Object,
        ObjectDefinitionReference,
        Struct,
        StructDefinition,
        StructType,
        TraitType,
        Type,
    )

ENUM_CLASS_BY_TYPE = _ENUM_CLASS_BY_TYPE  # re-exported to avoid circular imports
ENUM_TYPE_BY_CLASS = _ENUM_TYPE_BY_CLASS  # re-exported to avoid circular imports

NODE_CLASS_BY_TYPE: dict["NodeType", type["Node"]] = {}
NODE_TYPE_BY_CLASS: dict[type["Node"], "NodeType"] = {}

STRUCT_CLASS_BY_TYPE: dict["StructType", type["Struct"]] = {}
STRUCT_TYPE_BY_CLASS: dict[type["Struct"], StructType] = {}

HANDLE_CLASS_BY_TYPE: dict["HandleType", type["Handle"]] = {}
HANDLE_TYPE_BY_CLASS: dict[type["Handle"], "HandleType"] = {}

OBJECT_DEFINITION_REFERENCE_BY_CLASS: dict[type["Object"], "ObjectDefinitionReference"] = {}
ENUM_DEFINITION_BY_TYPE: dict["EnumType", "EnumDefinition"] = {}
STRUCT_DEFINITION_BY_TYPE: dict["StructType", "StructDefinition"] = {}
HANDLE_DEFINITION_BY_TYPE: dict["HandleType", "HandleDefinition"] = {}
NODE_DEFINITION_BY_TYPE: dict["NodeType", "NodeDefinition"] = {}

NODE_TYPE_SCALAR_BY_TYPE: dict["NodeType", "Type"] = {}

BUILTIN_CLASS_BY_NAME: dict[
    str, type["Node"] | type["Struct"] | type["EnumDeclaration"] | type["Handle"]
] = {}


def get_object_cls(
    object_type: "NodeType | StructType",
) -> type["Object"]:
    if isinstance(object_type, NodeType):
        return NODE_CLASS_BY_TYPE[object_type]
    elif isinstance(object_type, StructType):
        return STRUCT_CLASS_BY_TYPE[object_type]
    else:
        assert_never(object_type)


def get_builtin_class(
    destack_tgype: "NodeType | StructType | EnumType",
) -> type["Object"] | type["EnumDeclaration"]:
    if isinstance(destack_tgype, NodeType):
        return NODE_CLASS_BY_TYPE[destack_tgype]
    elif isinstance(destack_tgype, StructType):
        return STRUCT_CLASS_BY_TYPE[destack_tgype]
    elif isinstance(destack_tgype, EnumType):
        return ENUM_CLASS_BY_TYPE[destack_tgype]
    else:
        assert_never(destack_tgype)


def get_builtin_type(
    cls: type["Object"] | type["EnumDeclaration"],
) -> "NodeType | StructType | TraitType | EnumType":
    from .core import EnumDeclaration, Node, Struct

    if issubclass(cls, (Node, Struct)):
        return cls.metatype
    elif issubclass(cls, EnumDeclaration):
        return ENUM_TYPE_BY_CLASS[cls]
    else:
        raise ValueError(f"invalid destack type: {cls!r}")

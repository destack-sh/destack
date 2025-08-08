from typing import TYPE_CHECKING, Any, assert_never

if TYPE_CHECKING:
    from destack import (
        Enum,
        EnumDefinition,
        EnumType,
        Handle,
        HandleDefinition,
        HandleType,
        ModuleDefinition,
        Node,
        NodeDefinition,
        NodeType,
        Object,
        ObjectDefinitionReference,
        Struct,
        StructDefinition,
        StructType,
        TraitType,
        UniverseCategory,
        UniverseDomain,
    )

ENUM_CLASS_BY_TYPE: dict["EnumType", type["Enum"]] = {}
ENUM_TYPE_BY_CLASS: dict[type["Enum"], "EnumType"] = {}

NODE_CLASS_BY_TYPE: dict["NodeType", type["Node"]] = {}
NODE_TYPE_BY_CLASS: dict[type["Node"], "NodeType"] = {}

STRUCT_CLASS_BY_TYPE: dict["StructType", type["Struct"]] = {}
STRUCT_TYPE_BY_CLASS: dict[type["Struct"], "StructType"] = {}

HANDLE_CLASS_BY_TYPE: dict["HandleType", type["Handle"]] = {}
HANDLE_TYPE_BY_CLASS: dict[type["Handle"], "HandleType"] = {}

MODULE_BY_PATH: dict[str, Any] = {}

OBJECT_DEFINITION_REFERENCE_BY_CLASS: dict[type["Object"], "ObjectDefinitionReference"] = {}
ENUM_DEFINITION_BY_TYPE: dict["EnumType", "EnumDefinition"] = {}
NODE_DEFINITION_BY_TYPE: dict["NodeType", "NodeDefinition"] = {}
STRUCT_DEFINITION_BY_TYPE: dict["StructType", "StructDefinition"] = {}
HANDLE_DEFINITION_BY_TYPE: dict["HandleType", "HandleDefinition"] = {}
MODULE_DEFINITION_BY_PATH: dict[str, "ModuleDefinition"] = {}
MODULE_DEFINITION_BY_DOMAIN: dict["UniverseDomain", "ModuleDefinition"] = {}
MODULE_DEFINITION_BY_CATEGORY: dict["UniverseCategory", "ModuleDefinition"] = {}

BUILTIN_CLASS_BY_NAME: dict[str, type["Node"] | type["Struct"] | type["Enum"] | type["Handle"]] = {}
BUILTIN_DEFINITION_BY_NAME: dict[
    str, "NodeDefinition | StructDefinition | EnumDefinition | HandleDefinition | ModuleDefinition"
] = {}


def get_object_cls(
    object_type: "NodeType | StructType | HandleType",
) -> type["Object"]:
    if isinstance(object_type, NodeType):
        return NODE_CLASS_BY_TYPE[object_type]
    elif isinstance(object_type, StructType):
        return STRUCT_CLASS_BY_TYPE[object_type]
    elif isinstance(object_type, HandleType):
        return HANDLE_CLASS_BY_TYPE[object_type]
    else:
        assert_never(object_type)


def get_builtin_class(
    destack_tgype: "NodeType | StructType | HandleType | EnumType",
) -> type["Object"] | type["Enum"]:
    if isinstance(destack_tgype, NodeType):
        return NODE_CLASS_BY_TYPE[destack_tgype]
    elif isinstance(destack_tgype, StructType):
        return STRUCT_CLASS_BY_TYPE[destack_tgype]
    elif isinstance(destack_tgype, EnumType):
        return ENUM_CLASS_BY_TYPE[destack_tgype]
    elif isinstance(destack_tgype, HandleType):
        return HANDLE_CLASS_BY_TYPE[destack_tgype]
    else:
        assert_never(destack_tgype)


def get_builtin_type(
    cls: type["Object"] | type["Enum"],
) -> "NodeType | StructType | HandleType | TraitType | EnumType":
    from .core import Enum, Node, Struct

    if issubclass(cls, (Node, Struct)):
        return cls.metatype
    elif issubclass(cls, Enum):
        return ENUM_TYPE_BY_CLASS[cls]
    elif issubclass(cls, Handle):
        return HANDLE_TYPE_BY_CLASS[cls]
    else:
        raise ValueError(f"invalid destack type: {cls!r}")


def get_builtin_definition(
    name: str,
) -> "NodeDefinition | StructDefinition | EnumDefinition | HandleDefinition | ModuleDefinition":
    definition = BUILTIN_DEFINITION_BY_NAME.get(name)
    if definition is None:
        raise ValueError(f"builtin definition not found: {name!r}")
    return definition

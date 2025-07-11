from typing import TYPE_CHECKING, assert_never

from .core.builtin.common import (
    EnumType,
    NodeType,
    StoreKey,
    StructType,
    TraitType,
)
from .core.builtin.enum import _ENUM_CLASS_BY_TYPE, _ENUM_TYPE_BY_CLASS

if TYPE_CHECKING:
    from destack.language import (
        BuiltinObject,
        ConstantDefinition,
        Enum,
        EnumDefinition,
        Node,
        NodeDefinition,
        NodeDefinitionReference,
        ObjectDefinitionReference,
        Struct,
        StructDefinition,
        Trait,
        TraitDefinition,
    )

ENUM_CLASS_BY_TYPE = _ENUM_CLASS_BY_TYPE  # re-exported to avoid circular imports
ENUM_TYPE_BY_CLASS = _ENUM_TYPE_BY_CLASS  # re-exported to avoid circular imports

NODE_CLASS_BY_TYPE: dict[NodeType, type["Node"]] = {}
NODE_TYPE_BY_CLASS: dict[type["Node"], NodeType] = {}
NODE_TYPES_BY_PRIMARY_STORE_KEY: dict[StoreKey, tuple[NodeType, ...]] = {}
NODE_TYPES_BY_TRAIT_TYPE: dict[TraitType, tuple[NodeType, ...]] = {}

TRAIT_CLASS_BY_TYPE: dict[TraitType, type["Trait"]] = {}
TRAIT_TYPE_BY_CLASS: dict[type["Trait"], TraitType] = {}

STRUCT_CLASS_BY_TYPE: dict[StructType, type["Struct"]] = {}
STRUCT_TYPE_BY_CLASS: dict[type["Struct"], StructType] = {}

NODE_DEFINITION_REFERENCE_BY_CLASS: dict[type["Node"], "NodeDefinitionReference"] = {}
OBJECT_DEFINITION_REFERENCE_BY_CLASS: dict[type["BuiltinObject"], "ObjectDefinitionReference"] = {}
ENUM_DEFINITION_BY_TYPE: dict[EnumType, "EnumDefinition"] = {}
STRUCT_DEFINITION_BY_TYPE: dict[StructType, "StructDefinition"] = {}
TRAIT_DEFINITION_BY_TYPE: dict[TraitType, "TraitDefinition"] = {}
NODE_DEFINITION_BY_TYPE: dict[NodeType, "NodeDefinition"] = {}
CONSTANT_DEFINITIONS: dict[str, "ConstantDefinition"] = {}

DESCENDANT_NODE_TYPES_BY_TYPE: dict[NodeType, tuple[NodeType, ...]] = {}
ANCESTOR_NODE_TYPES_BY_TYPE: dict[NodeType, tuple[NodeType, ...]] = {}

SUBDEFINITIONS_BY_NODE_TYPE: dict[NodeType, tuple["NodeDefinitionReference", ...]] = {}


def get_builtin_object_cls(
    object_type: NodeType | StructType | TraitType,
) -> type["BuiltinObject"]:
    if isinstance(object_type, NodeType):
        return NODE_CLASS_BY_TYPE[object_type]
    elif isinstance(object_type, StructType):
        return STRUCT_CLASS_BY_TYPE[object_type]
    elif isinstance(object_type, TraitType):
        return TRAIT_CLASS_BY_TYPE[object_type]
    else:
        assert_never(object_type)


def get_builtin_class(
    destack_tgype: NodeType | StructType | EnumType,
) -> type["BuiltinObject"] | type["Enum"]:
    if isinstance(destack_tgype, NodeType):
        return NODE_CLASS_BY_TYPE[destack_tgype]
    elif isinstance(destack_tgype, StructType):
        return STRUCT_CLASS_BY_TYPE[destack_tgype]
    elif isinstance(destack_tgype, EnumType):
        return ENUM_CLASS_BY_TYPE[destack_tgype]
    else:
        assert_never(destack_tgype)


def get_builtin_type(
    cls: type["BuiltinObject"] | type["Enum"],
) -> NodeType | StructType | TraitType | EnumType:
    from .core import Enum, Node, Struct, Trait

    if issubclass(cls, (Node, Struct, Trait)):
        return cls.metatype
    elif issubclass(cls, Enum):
        return ENUM_TYPE_BY_CLASS[cls]
    else:
        raise ValueError(f"invalid destack type: {cls!r}")


def get_node_or_trait_cls(
    node_type: NodeType | TraitType,
) -> type["Node"] | type["Trait"]:
    if isinstance(node_type, NodeType):
        return NODE_CLASS_BY_TYPE[node_type]
    elif isinstance(node_type, TraitType):
        return TRAIT_CLASS_BY_TYPE[node_type]
    else:
        assert_never(node_type)


def get_node_types_for_stores(store_keys: tuple[StoreKey, ...]) -> tuple[NodeType, ...]:
    return tuple(
        {
            node_type
            for store_key in store_keys
            for node_type in NODE_TYPES_BY_PRIMARY_STORE_KEY[store_key]
        }
    )


def get_subdefinitions_for_node_type(node_type: NodeType) -> tuple["NodeDefinitionReference", ...]:
    node_cls = NODE_CLASS_BY_TYPE[node_type]
    if not node_cls.__inherited_by__:
        return (node_cls.__definition_reference__,)
    subdefinitions: list[NodeDefinitionReference] = []
    if not node_cls.__is_abstract__:
        subdefinitions.append(node_cls.__definition_reference__)
    for subnode_type in node_cls.__inherited_by__:
        subnode_cls = NODE_CLASS_BY_TYPE[subnode_type]
        if not subnode_cls.__is_abstract__:
            subdefinitions.append(subnode_cls.__definition_reference__)
    return tuple(subdefinitions)

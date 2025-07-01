from typing import TYPE_CHECKING, assert_never

from .core.builtin.common import (
    EnumType,
    NodeType,
    StoreType,
    StructType,
    TraitType,
)
from .core.builtin.enum import _ENUM_CLASS_BY_TYPE, _ENUM_TYPE_BY_CLASS

if TYPE_CHECKING:
    from destack.language import (
        BuiltinObjectBase,
        ConstantDefinition,
        Enum,
        EnumDefinition,
        Node,
        NodeBase,
        NodeDefinition,
        NodeDefinitionReference,
        ObjectDefinitionReference,
        StructBase,
        StructDefinition,
        Trait,
        TraitDefinition,
    )

ENUM_CLASS_BY_TYPE = _ENUM_CLASS_BY_TYPE  # re-exported to avoid circular imports
ENUM_TYPE_BY_CLASS = _ENUM_TYPE_BY_CLASS  # re-exported to avoid circular imports

NODE_CLASS_BY_TYPE: dict[NodeType, type["Node"]] = {}
NODE_TYPE_BY_CLASS: dict[type["Node"], NodeType] = {}
NODE_TYPES_BY_MAIN_STORE_TYPE: dict[StoreType, tuple[NodeType, ...]] = {}

TRAIT_CLASS_BY_TYPE: dict[TraitType, type["NodeBase"]] = {}
TRAIT_TYPE_BY_CLASS: dict[type["Trait"], TraitType] = {}
NODE_TYPES_BY_TRAIT_TYPE: dict[TraitType, tuple[NodeType, ...]] = {}

STRUCT_CLASS_BY_TYPE: dict[StructType, type["StructBase"]] = {}
STRUCT_TYPE_BY_CLASS: dict[type["StructBase"], StructType] = {}

NODE_DEFINITION_REFERENCE_BY_CLASS: dict[type["NodeBase"], "NodeDefinitionReference"] = {}
OBJECT_DEFINITION_REFERENCE_BY_CLASS: dict[
    type["BuiltinObjectBase"], "ObjectDefinitionReference"
] = {}
ENUM_DEFINITION_BY_TYPE: dict[EnumType, "EnumDefinition"] = {}
STRUCT_DEFINITION_BY_TYPE: dict[StructType, "StructDefinition"] = {}
TRAIT_DEFINITION_BY_TYPE: dict[TraitType, "TraitDefinition"] = {}
NODE_DEFINITION_BY_TYPE: dict[NodeType, "NodeDefinition"] = {}
CONSTANT_DEFINITIONS: dict[str, "ConstantDefinition"] = {}

DESCENDANT_NODE_TYPES_BY_TYPE: dict[NodeType, tuple[NodeType, ...]] = {}
ANCESTOR_NODE_TYPES_BY_TYPE: dict[NodeType, tuple[NodeType, ...]] = {}


def get_builtin_object_cls(
    object_type: NodeType | StructType | TraitType,
) -> type["BuiltinObjectBase"]:
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
) -> type["BuiltinObjectBase"] | type["Enum"]:
    if isinstance(destack_tgype, NodeType):
        return NODE_CLASS_BY_TYPE[destack_tgype]
    elif isinstance(destack_tgype, StructType):
        return STRUCT_CLASS_BY_TYPE[destack_tgype]
    elif isinstance(destack_tgype, EnumType):
        return ENUM_CLASS_BY_TYPE[destack_tgype]
    else:
        assert_never(destack_tgype)


def get_builtin_type(
    cls: type["BuiltinObjectBase"] | type["Enum"],
) -> NodeType | StructType | TraitType | EnumType:
    from .core import Enum, Node, StructBase, Trait

    if issubclass(cls, (Node, StructBase, Trait)):
        return cls.metatype
    elif issubclass(cls, Enum):
        return ENUM_TYPE_BY_CLASS[cls]
    else:
        raise ValueError(f"invalid destack type: {cls!r}")


def get_node_or_trait_cls(
    node_type: NodeType | TraitType,
) -> type["NodeBase"] | type["Trait"]:
    if isinstance(node_type, NodeType):
        return NODE_CLASS_BY_TYPE[node_type]
    elif isinstance(node_type, TraitType):
        return TRAIT_CLASS_BY_TYPE[node_type]
    else:
        assert_never(node_type)

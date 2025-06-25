from collections import defaultdict
from itertools import chain
from typing import TYPE_CHECKING, Any, assert_never

from destack import proto
from destack.utils.code import exec_
from destack.utils.env import IS_DEV, IS_TEST
from destack.utils.uuid import UUID

from .core.builtin.common import (
    ENUM_TYPES,
    EnumType,
    NodeType,
    StoreType,
    StructType,
    TraitType,
)
from .core.builtin.const import UNSET
from .core.builtin.enum import _ENUM_CLASS_BY_TYPE, _ENUM_TYPE_BY_CLASS

if TYPE_CHECKING:
    from destack.language import (
        BuiltinObjectBase,
        Enum,
        EnumDefinition,
        Node,
        NodeBase,
        NodeDefinition,
        ObjectReference,
        RelationReference,
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

ORDER_GROUP_BY_NODE_TYPE: dict[NodeType, TraitType | NodeType] = {}

TRAIT_CLASS_BY_TYPE: dict[TraitType, type["NodeBase"]] = {}
TRAIT_TYPE_BY_CLASS: dict[type["Trait"], TraitType] = {}
NODE_TYPES_BY_TRAIT_TYPE: dict[TraitType, tuple[NodeType, ...]] = {}

STRUCT_CLASS_BY_TYPE: dict[StructType, type["StructBase"]] = {}
STRUCT_TYPE_BY_CLASS: dict[type["StructBase"], StructType] = {}

RELATION_REF_BY_CLASS: dict[type["NodeBase"], "RelationReference"] = {}
OBJECT_REF_BY_CLASS: dict[type["BuiltinObjectBase"], "ObjectReference"] = {}
ENUM_DEFINITION_BY_TYPE: dict[EnumType, "EnumDefinition"] = {}
STRUCT_DEFINITION_BY_TYPE: dict[StructType, "StructDefinition"] = {}
TRAIT_DEFINITION_BY_TYPE: dict[TraitType, "TraitDefinition"] = {}
NODE_DEFINITION_BY_TYPE: dict[NodeType, "NodeDefinition"] = {}

DESCENDANT_NODE_TYPES_BY_TYPE: dict[NodeType, tuple[NodeType, ...]] = {}
ANCESTOR_NODE_TYPES_BY_TYPE: dict[NodeType, tuple[NodeType, ...]] = {}


def get_builtin_object_cls(object_type: NodeType | StructType) -> type["BuiltinObjectBase"]:
    if isinstance(object_type, NodeType):
        return NODE_CLASS_BY_TYPE[object_type]
    elif isinstance(object_type, StructType):
        return STRUCT_CLASS_BY_TYPE[object_type]
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
) -> NodeType | StructType | EnumType:
    from .core import Enum, Node, StructBase

    if issubclass(cls, (Node, StructBase)):
        return cls.metatype
    elif issubclass(cls, Enum):
        return ENUM_TYPE_BY_CLASS[cls]
    else:
        raise ValueError(f"invalid destack type: {cls!r}")


def _complete_setup():
    """Finalize setup of all language constructs after everything is imported."""
    from destack.language.core.builtin.object import (
        _is_setup_complete,
        _set_setup_complete,
    )
    from destack.language.core.builtin.trait import expand_node_types

    if _is_setup_complete():
        return

    # index node types by store type
    node_types_by_store_type: dict[StoreType, list[NodeType]] = defaultdict(list)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        if TraitType.ENTITY in node_cls.__traits__:
            if TraitType.GLOBAL in node_cls.__traits__:
                node_types_by_store_type[StoreType.GLOBAL_ENTITY].append(node_cls.metatype)
            elif TraitType.SPATIAL in node_cls.__traits__:
                node_types_by_store_type[StoreType.SPATIAL_ENTITY].append(node_cls.metatype)
            else:
                raise ValueError(f"unexpected entity node type: {node_cls!r}")
    for store_type in StoreType:
        NODE_TYPES_BY_MAIN_STORE_TYPE[store_type] = tuple(
            node_types_by_store_type.get(store_type, ())
        )

    # index node types by trait
    node_types_by_trait: dict[TraitType, list[NodeType]] = defaultdict(list)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        for trait in node_cls.__traits__:
            node_types_by_trait[trait].append(node_cls.metatype)
    for trait, node_types in node_types_by_trait.items():
        NODE_TYPES_BY_TRAIT_TYPE[trait] = tuple(node_types)
    for trait_type in TraitType:
        assert trait_type in TRAIT_CLASS_BY_TYPE, f"missing trait type: {trait_type!r}"
        if trait_type not in NODE_TYPES_BY_TRAIT_TYPE:
            NODE_TYPES_BY_TRAIT_TYPE[trait_type] = ()

    # index order groups
    from destack.language.core.builtin.trait import INTER_ORDER_TRAITS

    for trait_type in INTER_ORDER_TRAITS:
        order_group = NODE_TYPES_BY_TRAIT_TYPE[trait_type]
        for node_type in order_group:
            ORDER_GROUP_BY_NODE_TYPE[node_type] = trait_type
    for node_type in NODE_TYPES_BY_TRAIT_TYPE[TraitType.ORDERED]:
        if node_type not in ORDER_GROUP_BY_NODE_TYPE:
            ORDER_GROUP_BY_NODE_TYPE[node_type] = node_type

    # index parent types
    for node_cls in NODE_CLASS_BY_TYPE.values():
        assert node_cls.__parent_property__ is not UNSET
        if node_cls.__root_type__ is None:
            node_cls.__parent_types__ = ()
            if node_cls.__parent_property__ is not None:
                node_cls.__parent_property__.node_types = ()
        else:
            parent_types = expand_node_types(node_cls.__parent_property__.node_types or ())
            assert len(parent_types) < len(NodeType), f"generic parent for '{node_cls.__name__}'"
            node_cls.__parent_types__ = parent_types

    # index child types
    child_types_by_parent: dict[NodeType, list[NodeType]] = defaultdict(list)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        for parent_type in node_cls.__parent_types__:
            child_types_by_parent[parent_type].append(node_cls.metatype)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        node_cls.__child_types__ = tuple(child_types_by_parent[node_cls.metatype])

    # index ancestor/descendant types (recursive)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        # collect all ancestor types recursively
        ancestors = set()
        to_visit = list(node_cls.__parent_types__)
        while to_visit:
            parent_type = to_visit.pop()
            if parent_type not in ancestors:
                ancestors.add(parent_type)
                parent_cls = NODE_CLASS_BY_TYPE[parent_type]
                to_visit.extend(parent_cls.__parent_types__)

        # collect all descendant types recursively
        descendants = set()
        to_visit = list(node_cls.__child_types__)
        while to_visit:
            child_type = to_visit.pop()
            if child_type not in descendants:
                descendants.add(child_type)
                child_cls = NODE_CLASS_BY_TYPE[child_type]
                to_visit.extend(child_cls.__child_types__)

        node_cls.__ancestor_types__ = tuple(ancestors)
        node_cls.__descendant_types__ = tuple(descendants)
        DESCENDANT_NODE_TYPES_BY_TYPE[node_cls.metatype] = node_cls.__descendant_types__
        ANCESTOR_NODE_TYPES_BY_TYPE[node_cls.metatype] = node_cls.__ancestor_types__

    # finalize properties
    for metatype, object_cls in chain(NODE_CLASS_BY_TYPE.items(), STRUCT_CLASS_BY_TYPE.items()):
        for prop in object_cls.__properties__.values():
            prop.finalize(metatype)

    # generate pack/unpack methods
    from destack.grpc.wiring import generate_pack_proto_impl
    from destack.language.core.common.value import generate_pack_value_impl

    builtin_class_by_name: dict[str, Any] = {**proto.__dict__, "UUID": UUID}
    builtin_class_by_name.update(
        {
            cls.__name__: cls
            for cls in chain(
                NODE_CLASS_BY_TYPE.values(),
                STRUCT_CLASS_BY_TYPE.values(),
                ENUM_CLASS_BY_TYPE.values(),
            )
        }
    )
    for cls in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values()):
        cls_dict_copy = cls.__dict__.copy()
        # __pack_proto__/__unpack_proto__/_to_proto
        proto_impl, proto_glbls = generate_pack_proto_impl(cls)
        exec_(
            proto_impl,
            {**builtin_class_by_name, **proto_glbls},
            cls_dict_copy,
            f"{cls.__name__}:proto",
        )
        setattr(cls, "__pack_proto__", cls_dict_copy["__pack_proto__"])
        setattr(cls, "__unpack_proto__", cls_dict_copy["__unpack_proto__"])
        setattr(cls, "to_proto", cls_dict_copy["to_proto"])
        setattr(cls, "from_proto", cls_dict_copy["from_proto"])
        # __pack_value__/__unpack_value__/_to_value
        value_impl, value_glbls = generate_pack_value_impl(cls)
        exec_(
            value_impl,
            {**builtin_class_by_name, **value_glbls},
            cls_dict_copy,
            f"{cls.__name__}:value",
        )
        setattr(cls, "__pack_value__", cls_dict_copy["__pack_value__"])
        setattr(cls, "__unpack_value__", cls_dict_copy["__unpack_value__"])
        setattr(cls, "to_value", cls_dict_copy["to_value"])
        setattr(cls, "from_value", cls_dict_copy["from_value"])

    # generate relation refs
    from destack.language.core import ObjectReference, RelationReference

    for cls in NODE_CLASS_BY_TYPE.values():
        RELATION_REF_BY_CLASS[cls] = RelationReference.of(cls)
        OBJECT_REF_BY_CLASS[cls] = ObjectReference.of(cls)
    for cls in TRAIT_TYPE_BY_CLASS:
        RELATION_REF_BY_CLASS[cls] = RelationReference.of(cls)
        OBJECT_REF_BY_CLASS[cls] = ObjectReference.of(cls)
    for cls in STRUCT_CLASS_BY_TYPE.values():
        OBJECT_REF_BY_CLASS[cls] = ObjectReference.of(cls)

    # generate meta info
    from destack.language.core import (
        EnumDefinition,
        NodeDefinition,
        StructDefinition,
        TraitDefinition,
    )

    for trait_type, trait_cls in TRAIT_CLASS_BY_TYPE.items():
        trait_definition = TraitDefinition.from_trait(trait_cls)
        TRAIT_DEFINITION_BY_TYPE[trait_type] = trait_definition
    for node_cls in NODE_CLASS_BY_TYPE.values():
        node_definition = NodeDefinition.from_node(node_cls)
        NODE_DEFINITION_BY_TYPE[node_cls.metatype] = node_definition
        node_cls.__definition__ = node_definition
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        struct_definition = StructDefinition.from_struct(struct_cls)
        STRUCT_DEFINITION_BY_TYPE[struct_cls.metatype] = struct_definition
        struct_cls.__definition__ = struct_definition
    for enum_type in ENUM_TYPES:
        enum_definition = EnumDefinition.from_enum(enum_type, ENUM_CLASS_BY_TYPE[enum_type])
        ENUM_DEFINITION_BY_TYPE[enum_type] = enum_definition

    # sanity check stuff
    if IS_DEV or IS_TEST:
        from destack.language.core.builtin.trait import (
            AT_LEAST_ONE_TRAITS,
            AT_MOST_ONE_TRAITS,
            INFECTIOUS_TRAITS,
        )

        # check traits
        for cls in NODE_CLASS_BY_TYPE.values():
            for traits in AT_LEAST_ONE_TRAITS:
                if not any(trait in cls.__traits__ for trait in traits):
                    raise AssertionError(
                        f"{cls.__name__} must have at least one of {[t.name for t in traits]} traits (has {[t.name for t in cls.__traits__]})"
                    )
            for traits in AT_MOST_ONE_TRAITS:
                matching_traits = [trait for trait in traits if trait in cls.__traits__]
                if len(matching_traits) > 1:
                    raise AssertionError(
                        f"{cls.__name__} must have at most one of {[t.name for t in traits]} traits (has {[t.name for t in matching_traits]})"
                    )
        for trait_type in INFECTIOUS_TRAITS:
            for node_type in NODE_TYPES_BY_TRAIT_TYPE[trait_type]:
                descendant_types = DESCENDANT_NODE_TYPES_BY_TYPE[node_type]
                for descendant_type in descendant_types:
                    descendant_cls = NODE_CLASS_BY_TYPE[descendant_type]
                    if trait_type not in descendant_cls.__traits__:
                        raise AssertionError(
                            f"{descendant_type.name} must inherit {trait_type.name} trait from {node_type.name} (has {[t.name for t in descendant_cls.__traits__]}, parents: {[t.name for t in ANCESTOR_NODE_TYPES_BY_TYPE[descendant_type]]})"
                        )

    _set_setup_complete()

from collections import defaultdict
from itertools import chain
from typing import TYPE_CHECKING, Any, assert_never

from fastuuid import UUID

from destack import pb2
from destack.utils.code import exec_
from destack.utils.env import IS_DEV, IS_TEST

from .core.builtin.const import (
    _ENUM_CLASS_BY_TYPE,
    _ENUM_TYPE_BY_CLASS,
    ENUM_TYPES,
    UNSET,
    EnumType,
    NodeType,
    StoreType,
    StructType,
    TraitType,
)

if TYPE_CHECKING:
    from destack.language import (
        BuiltinEnum,
        BuiltinObjectBase,
        EnumInfo,
        Node,
        NodeBase,
        NodeInfo,
        RelationReference,
        StructBase,
        StructInfo,
        Trait,
    )

ENUM_CLASS_BY_TYPE = _ENUM_CLASS_BY_TYPE  # re-exported to avoid circular imports
ENUM_TYPE_BY_CLASS = _ENUM_TYPE_BY_CLASS  # re-exported to avoid circular imports

NODE_CLASS_BY_TYPE: dict[NodeType, type["Node"]] = {}
NODE_TYPE_BY_CLASS: dict[type["Node"], NodeType] = {}
NODE_TYPES_BY_MAIN_STORE_TYPE: dict[StoreType, tuple[NodeType, ...]] = {}

TRAIT_CLASS_BY_TRAIT: dict[TraitType, type["BuiltinObjectBase"]] = {}
TRAIT_TYPE_BY_CLASS: dict[type["Trait"], TraitType] = {}
NODE_TYPES_BY_TRAIT_TYPE: dict[TraitType, tuple[NodeType, ...]] = {}

STRUCT_CLASS_BY_TYPE: dict[StructType, type["StructBase"]] = {}
STRUCT_TYPE_BY_CLASS: dict[type["StructBase"], StructType] = {}

RELATION_REF_BY_CLASS: dict[type["NodeBase"], "RelationReference"] = {}
STRUCT_INFO_BY_TYPE: dict[StructType, "StructInfo"] = {}
ENUM_INFO_BY_TYPE: dict[EnumType, "EnumInfo"] = {}
NODE_INFO_BY_TYPE: dict[NodeType, "NodeInfo"] = {}

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
) -> type["BuiltinObjectBase"] | type["BuiltinEnum"]:
    if isinstance(destack_tgype, NodeType):
        return NODE_CLASS_BY_TYPE[destack_tgype]
    elif isinstance(destack_tgype, StructType):
        return STRUCT_CLASS_BY_TYPE[destack_tgype]
    elif isinstance(destack_tgype, EnumType):
        return ENUM_CLASS_BY_TYPE[destack_tgype]
    else:
        assert_never(destack_tgype)


def get_builtin_type(
    cls: type["BuiltinObjectBase"] | type["BuiltinEnum"],
) -> NodeType | StructType | EnumType:
    from .core import BuiltinEnum, Node, StructBase

    if issubclass(cls, (Node, StructBase)):
        return cls.metatype
    elif issubclass(cls, BuiltinEnum):
        return ENUM_TYPE_BY_CLASS[cls]
    else:
        raise ValueError(f"invalid destack type: {cls!r}")


def _complete_destack_setup():
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
    for trait_type in TRAIT_TYPE_BY_CLASS.values():
        if trait_type not in NODE_TYPES_BY_TRAIT_TYPE:
            NODE_TYPES_BY_TRAIT_TYPE[trait_type] = ()

    # index parent types
    for node_cls in NODE_CLASS_BY_TYPE.values():
        assert node_cls.__parent_property__ is not UNSET
        if node_cls.__root_type__ is None:
            node_cls.__parent_types__ = ()
            if node_cls.__parent_property__.ptr_prop is not None:
                node_cls.__parent_property__.ptr_prop.node_types = ()
        else:
            node_cls.__parent_types__ = expand_node_types(
                node_cls.__parent_property__.node_types or ()
            )

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
    from destack.language.core.common.value import generate_pack_value_impl
    from destack.proto.wiring import generate_pack_proto_impl

    builtin_class_by_name: dict[str, Any] = {**pb2.__dict__, "UUID": UUID}
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

    # hook IntoQuery methods into Property
    from destack.language.core import IntoQuery, Property

    for name, attr in IntoQuery.__dict__.items():
        if name not in Property.__dict__ and name not in ("__annotations__", "__dict__"):
            setattr(Property, name, attr)

    # generate relation refs
    from destack.language.core import relation_ref

    for cls in NODE_CLASS_BY_TYPE.values():
        RELATION_REF_BY_CLASS[cls] = relation_ref(cls)
    for cls in TRAIT_TYPE_BY_CLASS:
        RELATION_REF_BY_CLASS[cls] = relation_ref(cls)

    # generate meta info
    from destack.language.core import EnumInfo, NodeInfo, StructInfo

    for node_cls in NODE_CLASS_BY_TYPE.values():
        node_info = NodeInfo.from_node(node_cls)
        NODE_INFO_BY_TYPE[node_cls.metatype] = node_info
        node_cls.__info__ = node_info
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        struct_info = StructInfo.from_struct(struct_cls)
        STRUCT_INFO_BY_TYPE[struct_cls.metatype] = struct_info
        struct_cls.__info__ = struct_info
    for enum_type in ENUM_TYPES:
        enum_info = EnumInfo.from_enum(enum_type, ENUM_CLASS_BY_TYPE[enum_type])
        ENUM_INFO_BY_TYPE[enum_type] = enum_info

    # sanity check stuff
    if IS_DEV or IS_TEST:
        from destack.language.core.builtin.trait import (
            AT_LEAST_ONE_TRAITS,
            EXACT_ONE_TRAITS,
            INFECTIOUS_TRAITS,
        )

        # check traits
        for cls in NODE_CLASS_BY_TYPE.values():
            for traits in AT_LEAST_ONE_TRAITS:
                if not any(trait in cls.__traits__ for trait in traits):
                    raise AssertionError(
                        f"{cls.__name__} must have at least one of {[t.name for t in traits]} traits (has {[t.name for t in cls.__traits__]})"
                    )
            for traits in EXACT_ONE_TRAITS:
                matching_traits = set(traits) & set(cls.__traits__)
                if len(matching_traits) != 1:
                    raise AssertionError(
                        f"{cls.__name__} must have exactly one of {[t.name for t in traits]} traits (has {[t.name for t in cls.__traits__]})"
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

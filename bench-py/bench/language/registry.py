from collections import defaultdict
from typing import TYPE_CHECKING, Any, Union

from fastuuid import UUID

from bench import pb2
from bench.utils.code import exec_

from .core.const import (
    _ENUM_CLASS_BY_TYPE,
    ENUM_TYPES,
    NODE_TYPES,
    STRUCT_TYPES,
    UNSET,
    BuiltinEnum,
    EnumType,
    NodeType,
    StructType,
    TraitType,
)

if TYPE_CHECKING:
    from bench.language import BuiltinObjectBase, EnumInfo, Node, NodeInfo, StructBase, StructInfo

ENUM_CLASS_BY_TYPE = _ENUM_CLASS_BY_TYPE  # re-exported to avoid circular imports
ENUM_TYPE_BY_CLASS: dict[type, EnumType] = {}
NODE_CLASS_BY_TYPE: dict[NodeType, type["Node"]] = {}
NODE_CLASS_BY_TRAIT: dict[TraitType, type["BuiltinObjectBase"]] = {}
NODE_TYPES_BY_TRAIT: dict[TraitType, tuple[NodeType, ...]] = {}
STRUCT_CLASS_BY_TYPE: dict[StructType, type["StructBase"]] = {}
BUILTIN_OBJECT_CLASS_BY_TYPE: dict[NodeType | StructType, type["BuiltinObjectBase"]] = {}
BUILTIN_OBJECT_TYPE_BY_CLASS: dict[type["BuiltinObjectBase"], NodeType | StructType] = {}

BENCH_CLASS_BY_TYPE: dict[
    EnumType | NodeType | StructType, type["StructBase"] | type["Node"] | type[BuiltinEnum]
] = {}
BENCH_TYPE_BY_CLASS: dict[
    type[Union["BuiltinObjectBase", BuiltinEnum]], EnumType | NodeType | StructType
] = {}

STRUCT_INFO_BY_TYPE: dict[StructType, "StructInfo"] = {}
ENUM_INFO_BY_TYPE: dict[EnumType, "EnumInfo"] = {}
NODE_INFO_BY_TYPE: dict[NodeType, "NodeInfo"] = {}


def _complete_bench_setup():
    """Finalize setup of all language constructs after everything is imported."""
    from bench.language.core.object import (
        _is_setup_complete,
        _set_setup_complete,
    )
    from bench.language.core.trait import expand_node_types

    if _is_setup_complete():
        return

    # populate known types
    for node_t in NODE_TYPES:
        node_cls = NODE_CLASS_BY_TYPE[node_t]
        BUILTIN_OBJECT_CLASS_BY_TYPE[node_t] = node_cls
        BUILTIN_OBJECT_TYPE_BY_CLASS[node_cls] = node_t
        BENCH_CLASS_BY_TYPE[node_t] = node_cls
        BENCH_TYPE_BY_CLASS[node_cls] = node_t
    for struct_t in STRUCT_TYPES:
        BUILTIN_OBJECT_CLASS_BY_TYPE[struct_t] = STRUCT_CLASS_BY_TYPE[struct_t]
        BUILTIN_OBJECT_TYPE_BY_CLASS[STRUCT_CLASS_BY_TYPE[struct_t]] = struct_t
        BENCH_CLASS_BY_TYPE[struct_t] = STRUCT_CLASS_BY_TYPE[struct_t]
        BENCH_TYPE_BY_CLASS[STRUCT_CLASS_BY_TYPE[struct_t]] = struct_t
    for enum_type in ENUM_TYPES:
        BENCH_CLASS_BY_TYPE[enum_type] = ENUM_CLASS_BY_TYPE[enum_type]
        BENCH_TYPE_BY_CLASS[ENUM_CLASS_BY_TYPE[enum_type]] = enum_type
        ENUM_TYPE_BY_CLASS[ENUM_CLASS_BY_TYPE[enum_type]] = enum_type

    # index node types by trait
    node_types_by_trait: dict[TraitType, list[NodeType]] = defaultdict(list)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        for trait in node_cls.__traits__:
            node_types_by_trait[trait].append(node_cls.metatype)
    for trait, node_types in node_types_by_trait.items():
        NODE_TYPES_BY_TRAIT[trait] = tuple(node_types)

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

    # generate pack/unpack methods
    from bench.language.core.value import generate_pack_value_impl
    from bench.proto.wiring import generate_pack_proto_impl

    builtin_class_by_name: dict[str, Any] = {**pb2.__dict__, "UUID": UUID}
    builtin_class_by_name.update({cls.__name__: cls for cls in BENCH_CLASS_BY_TYPE.values()})
    for cls in BUILTIN_OBJECT_CLASS_BY_TYPE.values():
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
    from bench.language.core import IntoQuery, Property

    for name, attr in IntoQuery.__dict__.items():
        if name not in Property.__dict__ and name not in ("__annotations__", "__dict__"):
            setattr(Property, name, attr)

    # generate info
    from bench.language.core import EnumInfo, NodeInfo, StructInfo

    for node_cls in NODE_CLASS_BY_TYPE.values():
        node_info = NodeInfo.from_node(node_cls)
        NODE_INFO_BY_TYPE[node_cls.metatype] = node_info
        node_cls.info = node_info
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        struct_info = StructInfo.from_struct(struct_cls)
        STRUCT_INFO_BY_TYPE[struct_cls.metatype] = struct_info
        struct_cls.info = struct_info
    for enum_type in ENUM_TYPES:
        enum_info = EnumInfo.from_enum(enum_type, ENUM_CLASS_BY_TYPE[enum_type])
        ENUM_INFO_BY_TYPE[enum_type] = enum_info

    _set_setup_complete()

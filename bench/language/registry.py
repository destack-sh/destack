import functools
from collections import defaultdict
from typing import TYPE_CHECKING, Callable, Union

from .core.const import (
    _ENUM_CLASS_BY_TYPE,
    ENUM_TYPES,
    NODE_TYPES,
    STRUCT_TYPES,
    BuiltinEnum,
    EnumType,
    NodeTrait,
    NodeType,
    StructType,
)

if TYPE_CHECKING:
    from bench.language import BuiltinObject, Node, Struct

ENUM_CLASS_BY_TYPE = _ENUM_CLASS_BY_TYPE  # re-exported to avoid circular imports
ENUM_TYPE_BY_CLASS: dict[type, EnumType] = {}
NODE_CLASS_BY_TYPE: dict[NodeType, type["Node"]] = {}
NODE_CLASS_BY_TRAIT: dict[NodeTrait, type["BuiltinObject"]] = {}
STRUCT_CLASS_BY_TYPE: dict[StructType, type["Struct"]] = {}

BUILTIN_OBJECT_CLASS_BY_TYPE: dict[NodeType | StructType, type["BuiltinObject"]] = {}
BUILTIN_OBJECT_TYPE_BY_CLASS: dict[type["BuiltinObject"], NodeType | StructType] = {}

BENCH_CLASS_BY_TYPE: dict[
    EnumType | NodeType | StructType, type["Struct"] | type["Node"] | type[BuiltinEnum]
] = {}
BENCH_TYPE_BY_CLASS: dict[
    type[Union["BuiltinObject", BuiltinEnum]], EnumType | NodeType | StructType
] = {}


_setup_hooks: list[Callable] = []


def _on_completing_setup(func: Callable | None = None):
    """Register a finalization function."""
    if func is None:
        return functools.partial(_on_completing_setup)
    _setup_hooks.append(func)
    return func


def _complete_bench_setup():
    """Finalize setup of all language constructs after everything is imported."""
    from bench.language.core.object import _is_setup_complete, _set_setup_complete

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

    # index child types
    child_types_by_parent: dict[NodeType, list[NodeType]] = defaultdict(list)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        for parent_type in node_cls.__parent_types__:
            child_types_by_parent[parent_type].append(node_cls.metatype)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        node_cls.__child_types__ = tuple(child_types_by_parent[node_cls.metatype])

    _set_setup_complete()

    # run completion hooks
    for hook in _setup_hooks:
        hook()

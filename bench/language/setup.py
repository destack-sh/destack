import functools
from collections import defaultdict
from itertools import chain
from typing import TYPE_CHECKING, Callable, Union, cast

from bench.language.const import (
    _ENUM_CLASS_BY_TYPE,
    IN_BENCH_NODE_TYPES,
    IN_PACKAGE_NODE_TYPES,
    LOCAL_NODE_TYPES,
    NODE_TYPES,
    STRUCT_TYPES,
    BenchType,
    EnumType,
    NodeType,
    ObjectType,
    StructType,
)
from bench.utils.env import IS_DEV
from bench.utils.func import IdEnum, assert_collections_equal, bittuple, get_subclasses

if TYPE_CHECKING:
    from bench.language import BuiltinObject, Node, Struct

# some global indexes for language types/classes
# NOTE :Cleanup: organize global type/class indexes better
ENUM_CLASS_BY_TYPE = _ENUM_CLASS_BY_TYPE  # re-exported to avoid circular imports
ENUM_TYPE_BY_CLASS: dict[type, EnumType] = {}
NODE_CLASS_BY_TYPE: dict[NodeType, type["Node"]] = {}
NODE_COMPONENT_CLASS_BY_NAME: dict[str, type["Node"]] = {}
STRUCT_CLASS_BY_TYPE: dict[StructType, type["Struct"]] = {}
OBJECT_CLASS_BY_TYPE: dict[ObjectType, type["BuiltinObject"]] = {}
OBJECT_TYPE_BY_CLASS: dict[type["BuiltinObject"], ObjectType] = {}
BENCH_CLASS_BY_TYPE: dict[BenchType, type["Struct"] | type["Node"] | type[IdEnum]] = {}
FINAL_BENCH_CLASSES_BY_NAME: dict[str, type[Union["BuiltinObject", IdEnum]]] = {}
FINAL_BENCH_CLASSES: list[type[Union["BuiltinObject", IdEnum]]] = []
BENCH_CLASS_BY_NAME: dict[str, type[Union["BuiltinObject", IdEnum]]] = {}
BENCH_CLASSES: list[type[Union["BuiltinObject", IdEnum]]] = []
NODE_CLASSES: list[type["Node"]] = []
STRUCT_CLASSES: list[type["Struct"]] = []

# direct parent/child
PARENT_NODE_TYPES: dict[NodeType, bittuple[NodeType]] = {}
CHILD_NODE_TYPES: dict[NodeType, bittuple[NodeType]] = {}
HAS_CHILD_NODE_TYPES: set[NodeType] = set()
# transient parent/child
ANCESTOR_NODE_TYPES: dict[NodeType, bittuple[NodeType]] = {}
DESCENDANT_NODE_TYPES: dict[NodeType, bittuple[NodeType]] = {}
DESCENDANT_NODE_TYPES_IN_STORE: dict[NodeType, bittuple[NodeType]] = {}

_COMPLETED_SETUP = False


def _is_setup_complete() -> bool:
    return _COMPLETED_SETUP


_setup_hooks: list[Callable] = []


def _on_completing_setup(func: Callable | None = None):
    """Register a finalization function."""
    if func is None:
        return functools.partial(_on_completing_setup)
    _setup_hooks.append(func)
    return func


def _complete_bench_setup():
    """Finalize setup of all language constructs after everything is imported."""
    from bench.language import BuiltinObject, Node, Struct, ValueObject, const
    from bench.language.node import HasNodeBase

    global _COMPLETED_SETUP
    if _COMPLETED_SETUP:
        return

    # populate known types (all nodes/classes + enums in the files they're defined in)
    for bench_t in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values()):
        FINAL_BENCH_CLASSES_BY_NAME[bench_t.__name__] = bench_t
        FINAL_BENCH_CLASSES.append(bench_t)
    for bench_t in const.__dict__.values():
        if isinstance(bench_t, type) and issubclass(bench_t, IdEnum):
            FINAL_BENCH_CLASSES_BY_NAME[bench_t.__name__] = bench_t
            FINAL_BENCH_CLASSES.append(bench_t)
    BENCH_CLASSES.extend(chain(get_subclasses(BuiltinObject), (ValueObject,)))
    for cls in BENCH_CLASSES:
        BENCH_CLASS_BY_NAME[cls.__name__] = cls
    for node_t in NODE_TYPES:
        OBJECT_CLASS_BY_TYPE[node_t] = NODE_CLASS_BY_TYPE[node_t]
        OBJECT_TYPE_BY_CLASS[NODE_CLASS_BY_TYPE[node_t]] = node_t
        BENCH_CLASS_BY_TYPE[node_t] = NODE_CLASS_BY_TYPE[node_t]
        NODE_CLASSES.append(NODE_CLASS_BY_TYPE[node_t])
    for struct_t in STRUCT_TYPES:
        OBJECT_CLASS_BY_TYPE[struct_t] = STRUCT_CLASS_BY_TYPE[struct_t]
        OBJECT_TYPE_BY_CLASS[STRUCT_CLASS_BY_TYPE[struct_t]] = struct_t
        BENCH_CLASS_BY_TYPE[struct_t] = STRUCT_CLASS_BY_TYPE[struct_t]
        STRUCT_CLASSES.append(STRUCT_CLASS_BY_TYPE[struct_t])

    # check that we have all the enums & add them
    missing_enums = set(EnumType) - set(ENUM_CLASS_BY_TYPE)
    if missing_enums:
        raise ValueError(f"missing enums: {missing_enums}")
    for enum_type, enum_cls in ENUM_CLASS_BY_TYPE.items():
        BENCH_CLASS_BY_NAME[enum_cls.__name__] = enum_cls
        BENCH_CLASS_BY_TYPE[enum_type] = enum_cls
        BENCH_CLASSES.append(enum_cls)
        FINAL_BENCH_CLASSES_BY_NAME[enum_cls.__name__] = enum_cls
        FINAL_BENCH_CLASSES.append(enum_cls)
        ENUM_TYPE_BY_CLASS[enum_cls] = enum_type

    # finalize classes
    for cls in get_subclasses(BuiltinObject):
        # misc finalization on properties
        for name, prop in cls.__properties__.items():
            # finalize type info
            prop._finalize_type()

            # set introspectable properties as <cls>.<property>
            if prop.is_introspectable:
                setattr(cls, name, prop)

            if IS_DEV:
                # check deferred/encrypted properties
                if prop.is_deferred and not prop.is_stored:
                    raise ValueError(f"{prop!r} cannot be deferred and not stored on {cls!r}")

                # check py_type matches struct type as defined
                if (
                    isinstance(prop.py_type_raw, type)
                    and issubclass(prop.py_type_raw, Struct)
                    and not issubclass(prop.py_type_raw, Node)
                ):
                    if not prop.reference_struct:
                        raise ValueError(
                            f"cannot store {prop!r} as {prop.py_type_raw!r} (missing struct_type)"
                        )
                    if STRUCT_CLASS_BY_TYPE[prop.reference_struct] is not prop.py_type_raw:
                        raise ValueError(f"{prop!r} {prop.reference_struct} != {prop.py_type_raw}")

    # determine node ancestry relationships (parent/child)
    parent_types: dict[NodeType, set[NodeType]] = {nt: set() for nt in NODE_TYPES}
    child_types: dict[NodeType, set[NodeType]] = {nt: set() for nt in NODE_TYPES}
    for node_cls in NODE_CLASS_BY_TYPE.values():
        for parent_type in node_cls.__parent_property__.reference_nodes or ():
            parent_types[node_cls.metatype].add(parent_type)
            child_types[parent_type].add(node_cls.metatype)
    # ancestor/descendant: extend parent/child transitively
    ancestor_types: dict[NodeType, set[NodeType]] = defaultdict(set)
    descendant_types: dict[NodeType, set[NodeType]] = defaultdict(set)
    for node_type in NODE_TYPES:
        new_parents = list(parent_types[node_type])
        while new_parents:
            new_parent = new_parents.pop()
            ancestor_types[node_type].add(new_parent)
            ancestor_types[node_type] |= ancestor_types[new_parent]
            new_parents.extend(parent_types[new_parent] - ancestor_types[node_type])
        new_children = list(child_types[node_type])
        while new_children:
            new_child = new_children.pop()
            descendant_types[node_type].add(new_child)
            descendant_types[node_type] |= descendant_types[new_child]
            new_children.extend(child_types[new_child] - descendant_types[node_type])
    global ANCESTOR_NODE_TYPES, DESCENDANT_NODE_TYPES, PARENT_NODE_TYPES, CHILD_NODE_TYPES
    global DESCENDANT_NODE_TYPES_IN_STORE, HAS_CHILD_NODE_TYPES
    for node_type in NODE_TYPES:
        is_local = node_type in LOCAL_NODE_TYPES
        ANCESTOR_NODE_TYPES[node_type] = bittuple(*ancestor_types[node_type], enum_cls=NodeType)
        DESCENDANT_NODE_TYPES[node_type] = bittuple(*descendant_types[node_type], enum_cls=NodeType)
        DESCENDANT_NODE_TYPES_IN_STORE[node_type] = bittuple(
            *(t for t in descendant_types[node_type] if (t in LOCAL_NODE_TYPES) == (is_local)),
            enum_cls=NodeType,
        )
        PARENT_NODE_TYPES[node_type] = bittuple(*parent_types[node_type], enum_cls=NodeType)
        CHILD_NODE_TYPES[node_type] = bittuple(*child_types[node_type], enum_cls=NodeType)
        if child_types[node_type]:
            HAS_CHILD_NODE_TYPES.add(node_type)

    _COMPLETED_SETUP = True

    # run completion hooks
    for hook in _setup_hooks:
        hook()

    # set tables (depends on completion hooks)
    from bench.sql.engine import TABLE_BY_NODE_TYPE

    for node_cls in NODE_CLASS_BY_TYPE.values():
        if node_cls.__is_stored__ and not node_cls.__is_stored_custom__:
            # NOTE: table usually should exist, but maybe we're just creating the node type
            node_cls.__table__ = TABLE_BY_NODE_TYPE.get(node_cls.metatype)

    if IS_DEV:
        # check that is_in_package/is_in_bench was declared correctly
        #  (need to set that in @node upfront because traversing parents like here can only happen in finalization)
        for node_cls in NODE_CLASS_BY_TYPE.values():
            in_bench = (
                node_cls.metatype == NodeType.BENCH
                or node_cls.metatype in DESCENDANT_NODE_TYPES[NodeType.BENCH]
            )
            in_package = (
                node_cls.metatype == NodeType.PACKAGE
                or node_cls.metatype in DESCENDANT_NODE_TYPES[NodeType.PACKAGE]
            )
            if in_bench != node_cls.__is_in_bench__ or in_package != node_cls.__is_in_package__:
                raise ValueError(
                    f"{node_cls!r} parent types are inconsistent: root={node_cls.__roots__} implies in_bench={in_bench} and in_package={in_package}, but configured in_bench={node_cls.__is_in_bench__} and in_package={node_cls.__is_in_package__}"
                )
        assert_collections_equal(
            IN_BENCH_NODE_TYPES.tuple,
            [t.metatype for t in NODE_CLASS_BY_TYPE.values() if t.__is_in_bench__],
        )
        assert_collections_equal(
            IN_PACKAGE_NODE_TYPES.tuple,
            [t.metatype for t in NODE_CLASS_BY_TYPE.values() if t.__is_in_package__],
        )

        # everything in local node types must be declared local
        for node_cls in NODE_CLASS_BY_TYPE.values():
            should_be_local = node_cls.metatype in LOCAL_NODE_TYPES
            is_local = node_cls.__is_local__
            assert should_be_local == is_local, f"{node_cls!r} is inconsistent"

        # check that BASED_NODE_TYPES is consistent with HasBase
        base_node_types = [
            cast(Node, n).metatype for n in get_subclasses(HasNodeBase) if hasattr(n, "metatype")
        ]
        assert_collections_equal(base_node_types, const.BASED_NODE_TYPES.tuple)

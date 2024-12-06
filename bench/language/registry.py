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
    from bench.language import BuiltinObject, Node, NodeSubtypeStub, Struct

# some global indexes for language types/classes
# NOTE :Cleanup: organize global type/class indexes better
ENUM_CLASS_BY_TYPE = _ENUM_CLASS_BY_TYPE  # re-exported to avoid circular imports
ENUM_TYPE_BY_CLASS: dict[type, EnumType] = {}
NODE_CLASS_BY_TYPE: dict[NodeType, type["Node"]] = {}
NODE_CLASS_BY_NAME: dict[str, type["Node"]] = {}
NODE_CLASS_STUBS_BY_TYPE: dict[NodeType, dict[int, "NodeSubtypeStub"]] = {}
NODE_CLASS_STUBS_BY_NAME: dict[str, "NodeSubtypeStub"] = {}
STRUCT_CLASS_BY_TYPE: dict[StructType, type["Struct"]] = {}
BUILTIN_OBJECT_CLASS_BY_TYPE: dict[ObjectType, type["BuiltinObject"]] = {}
BUILTIN_OBJECT_TYPE_BY_CLASS: dict[type["BuiltinObject"], ObjectType] = {}
BENCH_CLASS_BY_TYPE: dict[BenchType, type["Struct"] | type["Node"] | type[IdEnum]] = {}
BENCH_TYPE_BY_CLASS: dict[type[Union["BuiltinObject", IdEnum]], BenchType] = {}
FINAL_BENCH_CLASSES_BY_NAME: dict[str, type[Union["BuiltinObject", IdEnum]]] = {}
FINAL_BENCH_CLASSES: list[type[Union["BuiltinObject", IdEnum]]] = []
BENCH_CLASS_BY_NAME: dict[str, type[Union["BuiltinObject", IdEnum]]] = {}
BENCH_CLASSES: list[type[Union["BuiltinObject", IdEnum]]] = []
NODE_CLASSES: list[type["Node"]] = []
SUBNODE_CLASSES: list[type["Node"]] = []
STRUCT_CLASSES: list[type["Struct"]] = []

# direct parent/child
PARENT_NODE_TYPES: dict[NodeType, bittuple[NodeType]] = {}
CHILD_NODE_TYPES: dict[NodeType, bittuple[NodeType]] = {}
HAS_CHILD_NODE_TYPES: set[NodeType] = set()
# transient parent/child
ANCESTOR_NODE_TYPES: dict[NodeType, bittuple[NodeType]] = {}
DESCENDANT_NODE_TYPES: dict[NodeType, bittuple[NodeType]] = {}
DESCENDANT_NODE_TYPES_IN_STORE: dict[NodeType, bittuple[NodeType]] = {}


_setup_hooks: list[Callable] = []


def _on_completing_setup(func: Callable | None = None):
    """Register a finalization function."""
    if func is None:
        return functools.partial(_on_completing_setup)
    _setup_hooks.append(func)
    return func


def _complete_bench_setup():
    """Finalize setup of all language constructs after everything is imported."""
    from bench.language import BuiltinObject, CustomObject, Node, Struct, const
    from bench.language.node import (
        HasNodeBase,
        _is_setup_complete,
        _set_setup_complete,
        _set_setup_finalizing,
    )

    if _is_setup_complete():
        return

    #
    # Index
    #

    # populate known types (all nodes/classes + enums in the files they're defined in)
    for bench_t in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values()):
        FINAL_BENCH_CLASSES_BY_NAME[bench_t.__name__] = bench_t
        FINAL_BENCH_CLASSES.append(bench_t)
    for bench_t in const.__dict__.values():
        if isinstance(bench_t, type) and issubclass(bench_t, IdEnum):
            FINAL_BENCH_CLASSES_BY_NAME[bench_t.__name__] = bench_t
            FINAL_BENCH_CLASSES.append(bench_t)
    BENCH_CLASSES.extend(chain(get_subclasses(BuiltinObject), (CustomObject,)))
    for cls in BENCH_CLASSES:
        BENCH_CLASS_BY_NAME[cls.__name__] = cls
    for node_t in NODE_TYPES:
        node_cls = NODE_CLASS_BY_TYPE[node_t]
        BUILTIN_OBJECT_CLASS_BY_TYPE[node_t] = node_cls
        BUILTIN_OBJECT_TYPE_BY_CLASS[node_cls] = node_t
        BENCH_CLASS_BY_TYPE[node_t] = node_cls
        BENCH_TYPE_BY_CLASS[node_cls] = node_t
        NODE_CLASSES.append(node_cls)
        for subnode_cls in node_cls.__subclass_by_subtype__.values():
            SUBNODE_CLASSES.append(subnode_cls)
    for struct_t in STRUCT_TYPES:
        BUILTIN_OBJECT_CLASS_BY_TYPE[struct_t] = STRUCT_CLASS_BY_TYPE[struct_t]
        BUILTIN_OBJECT_TYPE_BY_CLASS[STRUCT_CLASS_BY_TYPE[struct_t]] = struct_t
        BENCH_CLASS_BY_TYPE[struct_t] = STRUCT_CLASS_BY_TYPE[struct_t]
        BENCH_TYPE_BY_CLASS[STRUCT_CLASS_BY_TYPE[struct_t]] = struct_t
        STRUCT_CLASSES.append(STRUCT_CLASS_BY_TYPE[struct_t])

    # check that we have all the enums & add them
    missing_enums = set(EnumType) - set(ENUM_CLASS_BY_TYPE)
    if missing_enums:
        raise ValueError(f"missing enums: {missing_enums}")
    for enum_type, enum_cls in ENUM_CLASS_BY_TYPE.items():
        BENCH_CLASS_BY_NAME[enum_cls.__name__] = enum_cls
        BENCH_CLASS_BY_TYPE[enum_type] = enum_cls
        BENCH_TYPE_BY_CLASS[enum_cls] = enum_type
        BENCH_CLASSES.append(enum_cls)
        FINAL_BENCH_CLASSES_BY_NAME[enum_cls.__name__] = enum_cls
        FINAL_BENCH_CLASSES.append(enum_cls)
        ENUM_TYPE_BY_CLASS[enum_cls] = enum_type

    # determine node ancestry relationships (parent/child)
    parent_types: dict[NodeType, set[NodeType]] = {nt: set() for nt in NODE_TYPES}
    child_types: dict[NodeType, set[NodeType]] = {nt: set() for nt in NODE_TYPES}
    for node_cls in NODE_CLASS_BY_TYPE.values():
        assert node_cls.__parent_property__.reference_nodes != "any"
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
    global \
        ANCESTOR_NODE_TYPES, \
        DESCENDANT_NODE_TYPES, \
        PARENT_NODE_TYPES, \
        CHILD_NODE_TYPES, \
        DESCENDANT_NODE_TYPES_IN_STORE, \
        HAS_CHILD_NODE_TYPES
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

    #
    # Finalize
    #

    _set_setup_finalizing()

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

    # add all subtype stubs
    from bench.language.node import NodeSubtypeStub

    for node_type in NODE_TYPES:
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        if node_cls.__has_subtypes__:
            subtype_prop = node_cls.__subtype_base_property__
            assert subtype_prop is not None and subtype_prop.enum_type is not None
            enum_cls = ENUM_CLASS_BY_TYPE[subtype_prop.enum_type]
            NODE_CLASS_STUBS_BY_TYPE[node_type] = {}
            for subtype in enum_cls:
                stub = NodeSubtypeStub(node_cls, node_type, subtype)
                NODE_CLASS_STUBS_BY_TYPE[node_type][subtype] = stub
                NODE_CLASS_STUBS_BY_NAME[stub._name] = stub

    #
    # Complete
    #

    _set_setup_complete()

    # run completion hooks
    for hook in _setup_hooks:
        hook()

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

import enum
import functools

from collections import defaultdict
from itertools import chain
from typing import TYPE_CHECKING, Union, Callable

from bench.language.const import (
    NodeType,
    StructType,
    BenchType,
    NODE_TYPES,
    STRUCT_TYPES,
    IN_BENCH_NODE_TYPES,
    IN_PACKAGE_NODE_TYPES,
)
from bench.utils.func import bytetuple, IdEnum, get_subclasses, check_collections_equal
from bench.utils.utils import frozendict

if TYPE_CHECKING:
    from bench.language import Node, Struct

# some global indexes for language types/classes
NODE_CLASS_BY_TYPE: dict[NodeType, type["Node"]] = {}
NODE_COMPONENT_CLASS_BY_NAME: dict[str, type["Node"]] = {}
STRUCT_CLASS_BY_TYPE: dict[StructType, type["Struct"]] = {}
BENCH_CLASS_BY_TYPE: dict[BenchType, type["Node"] | type["Struct"]] = {}
FINAL_BENCH_CLASSES_BY_NAME: dict[str, type[Union["Node", "Struct", IdEnum]]] = {}
FINAL_BENCH_CLASSES: frozenset[type[Union["Node", "Struct", IdEnum]]] = frozenset()
BENCH_CLASSES_BY_NAME: dict[str, type[Union["Node", "Struct", IdEnum]]] = {}
BENCH_CLASSES: frozenset[type[Union["Node", "Struct", IdEnum]]] = frozenset()
NODE_CLASSES: frozenset[type["Node"]] = frozenset()
STRUCT_CLASSES: frozenset[type["Struct"]] = frozenset()

# direct parent/child
PARENT_NODE_TYPES: dict[NodeType, bytetuple[NodeType]] = {}
CHILD_NODE_TYPES: dict[NodeType, bytetuple[NodeType]] = {}
HAS_CHILD_NODE_TYPES: set[NodeType] = set()
FERTILE_CHILD_NODE_TYPES: dict[NodeType, bytetuple[NodeType]] = {}
# transient parent/child
ANCESTOR_NODE_TYPES: dict[NodeType, bytetuple[NodeType]] = {}
DESCENDANT_NODE_TYPES: dict[NodeType, bytetuple[NodeType]] = {}

_COMPLETED_SETUP = False


def _is_setup_complete() -> bool:
    return _COMPLETED_SETUP


_setup_hooks: list[Callable] = []


def _on_completing_setup(func: Callable = None):
    """Decorator to register finalization functions."""
    if func is None:
        return functools.partial(_on_completing_setup)
    _setup_hooks.append(func)
    return func


def _complete_bench_setup():
    """Finalize setup of all language constructs after everything is imported."""
    global FINAL_BENCH_CLASSES, BENCH_CLASSES, NODE_CLASSES, STRUCT_CLASSES
    from bench.language import const
    from bench.language.node import Node, Struct

    # populate known types
    for bench_t in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values()):
        FINAL_BENCH_CLASSES_BY_NAME[bench_t.__name__] = bench_t
    for bench_t in const.__dict__.values():
        if isinstance(bench_t, type) and issubclass(bench_t, IdEnum):
            FINAL_BENCH_CLASSES_BY_NAME[bench_t.__name__] = bench_t
    FINAL_BENCH_CLASSES = frozenset(FINAL_BENCH_CLASSES_BY_NAME.values())
    BENCH_CLASSES = frozenset(chain(FINAL_BENCH_CLASSES_BY_NAME.values(), get_subclasses(Struct)))
    for cls in BENCH_CLASSES:
        BENCH_CLASSES_BY_NAME[cls.__name__] = cls
    for node_t in NODE_TYPES:
        BENCH_CLASS_BY_TYPE[node_t] = NODE_CLASS_BY_TYPE[node_t]
    for struct_t in STRUCT_TYPES:
        BENCH_CLASS_BY_TYPE[struct_t] = STRUCT_CLASS_BY_TYPE[struct_t]
    NODE_CLASSES = frozenset(NODE_CLASS_BY_TYPE.values())
    STRUCT_CLASSES = frozenset(STRUCT_CLASS_BY_TYPE.values())

    # finalize classes
    for cls in chain(get_subclasses(Node), get_subclasses(Struct)):
        # misc finalization on properties
        for name, prop in cls.__properties__.items():
            if prop.reference_wired_ptr or prop.reference_stored_ptrs:
                # Properties with reference ptrs (like Node.parent -> parent_ptr/parent_id)
                #  aren't stored directly, we just use is_wired/is_stored to indicate what
                #  the contributed properties should do. Now that they're all contributed,
                #  we can set them to False, so they don't get indexed.
                prop.is_wired = False
                prop.is_stored = False

            # finalize type info
            prop._finalize()

            # set properties (that exist at runtime) on class
            if prop.is_runtime and not prop.is_runtime_only and not prop.is_computed:
                setattr(cls, name, prop)

            # ensure the introspected property type works (and cache it)
            if prop.is_introspectable:
                prop._as_type  # noqa

            # check deferred/encrypted properties
            if prop.is_deferred and not prop.is_stored:
                raise ValueError(f"{prop!r} cannot be deferred and not stored on {cls!r}")

            # check py_type matches struct type as defined
            if (
                isinstance(prop.py_type_raw, type)
                and issubclass(prop.py_type_raw, Struct)
                and not issubclass(prop.py_type_raw, Node)
            ):
                if not prop.struct_type:
                    raise ValueError(
                        f"cannot store {prop!r} as {prop.py_type_raw!r} (missing struct_type)"
                    )
                if STRUCT_CLASS_BY_TYPE[prop.struct_type] is not prop.py_type_raw:
                    raise ValueError(f"{prop!r} {prop.struct_type} != {prop.py_type_raw}")

        cls.__stored_properties__ = frozendict(
            {p.name: p for p in cls.__properties__.values() if p.is_stored is True}
        )
        cls.__wired_properties__ = frozendict(
            {p.name: p for p in cls.__properties__.values() if p.is_wired is True}
        )
        cls.__runtime_properties__ = frozendict(
            {p.name: p for p in cls.__properties__.values() if p.is_runtime is True}
        )

    # determine node ancestry relationships (parent/child)
    parent_types: dict[NodeType, set[NodeType]] = {nt: set() for nt in NODE_TYPES}
    child_types: dict[NodeType, set[NodeType]] = {nt: set() for nt in NODE_TYPES}
    for node_cls in NODE_CLASS_BY_TYPE.values():
        for parent_type in node_cls.__parent_property__.reference_types:
            parent_types[node_cls.metatype].add(parent_type)
            child_types[parent_type].add(node_cls.metatype)
    # fertile child types: child types that can have children
    fertile_child_types: dict[NodeType, set[NodeType]] = defaultdict(set)
    for node_type, child_type in child_types.items():
        for parent_type in child_type:
            fertile_child_types[parent_type].add(node_type)
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
    global HAS_CHILD_NODE_TYPES, FERTILE_CHILD_NODE_TYPES
    for node_type in NODE_TYPES:
        ANCESTOR_NODE_TYPES[node_type] = bytetuple(ancestor_types[node_type], enum_cls=NodeType)
        DESCENDANT_NODE_TYPES[node_type] = bytetuple(descendant_types[node_type], enum_cls=NodeType)
        PARENT_NODE_TYPES[node_type] = bytetuple(parent_types[node_type], enum_cls=NodeType)
        CHILD_NODE_TYPES[node_type] = bytetuple(child_types[node_type], enum_cls=NodeType)
        if child_types[node_type]:
            HAS_CHILD_NODE_TYPES.add(node_type)
        FERTILE_CHILD_NODE_TYPES[node_type] = bytetuple(
            fertile_child_types[node_type], enum_cls=NodeType
        )

    # check that is_in_package/is_in_bench was declared correctly
    #  (need to set that in @node upfront because traversing parents can only happen in finalization)
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
    check_collections_equal(
        IN_BENCH_NODE_TYPES, [t.metatype for t in NODE_CLASS_BY_TYPE.values() if t.__is_in_bench__]
    )
    check_collections_equal(
        IN_PACKAGE_NODE_TYPES,
        [t.metatype for t in NODE_CLASS_BY_TYPE.values() if t.__is_in_package__],
    )

    # check that all enum types are valid proto-able enums
    for struct_t in chain(STRUCT_CLASS_BY_TYPE.values(), NODE_CLASS_BY_TYPE.values()):
        for prop in struct_t.__properties__.values():
            if prop.is_enum and not issubclass(prop.py_type_stripped, (IdEnum, enum.IntFlag)):
                raise ValueError(f"{prop!r} is not a valid proto enum")

    global _COMPLETED_SETUP
    _COMPLETED_SETUP = True

    # run completion hooks
    for hook in _setup_hooks:
        hook()

    # set tables (depends on completion hooks)
    from bench.sql.engine import TABLE_BY_NODE_TYPE

    for node_cls in NODE_CLASS_BY_TYPE.values():
        if node_cls.__is_stored__ and not node_cls.__is_stored_custom__:
            node_cls.__table__ = TABLE_BY_NODE_TYPE.get(node_cls.metatype)
        else:
            node_cls.__table__ = None

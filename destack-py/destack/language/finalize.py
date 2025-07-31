import time
from collections import defaultdict
from itertools import chain
from typing import TYPE_CHECKING, Callable

from destack.utils.env import IS_DEV, IS_TEST
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

from .core.builtin.builtin import (
    ENUM_TYPES,
    EnumType,
    NodeType,
    StructType,
)
from .registry import (
    BUILTIN_CLASS_BY_NAME,
    ENUM_CLASS_BY_TYPE,
    ENUM_DEFINITION_BY_TYPE,
    HANDLE_CLASS_BY_TYPE,
    HANDLE_DEFINITION_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    NODE_TYPE_SCALAR_BY_TYPE,
    OBJECT_DEFINITION_REFERENCE_BY_CLASS,
    STRUCT_CLASS_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
)

if TYPE_CHECKING:
    pass

logger = get_logger(__name__)
tracer = get_tracer(__name__)

_finalize_start: float | None = None


def finalize():
    """Finalize the Destack language SDK."""
    global _finalize_start

    from destack.language.core.builtin.object import _is_finalized, _set_finalized

    if _is_finalized():
        return

    _finalize_start = time.time()

    BUILTIN_CLASS_BY_NAME.update(
        {
            cls.__name__: cls
            for cls in chain(
                NODE_CLASS_BY_TYPE.values(),
                STRUCT_CLASS_BY_TYPE.values(),
                ENUM_CLASS_BY_TYPE.values(),
            )
        }
    )

    # index Node extended_by (direct) / inherited_by (direct and indirect)
    node_type_extended_by: dict[NodeType, list[NodeType]] = defaultdict(list)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        if node_cls.__declaration__.base_type is not None:
            node_type_extended_by[node_cls.__declaration__.base_type].append(node_cls.metatype)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        node_cls.__declaration__.extended_by = list(node_type_extended_by[node_cls.metatype])

    # index Node inherited_by (recursive)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        # collect all types that inherit from this node recursively
        inherited_by_nodes: list[NodeType] = []
        to_visit = list(reversed(node_cls.__declaration__.extended_by))
        while to_visit:
            inheriting_type = to_visit.pop()
            if inheriting_type not in inherited_by_nodes:
                inherited_by_nodes.append(inheriting_type)
                inheriting_cls = NODE_CLASS_BY_TYPE[inheriting_type]
                to_visit.extend(reversed(inheriting_cls.__declaration__.extended_by))
        node_cls.__declaration__.inherited_by = list(inherited_by_nodes)

    # index Struct extended_by (direct) / inherited_by (direct and indirect)
    struct_type_extended_by: dict[StructType, list[StructType]] = defaultdict(list)
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        if struct_cls.__declaration__.base_type is not None:
            struct_type_extended_by[struct_cls.__declaration__.base_type].append(
                struct_cls.metatype
            )
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        struct_cls.__declaration__.extended_by = list(struct_type_extended_by[struct_cls.metatype])

    # index Struct inherited_by (recursive)
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        # collect all types that inherit from this struct recursively
        inherited_by_structs: list[StructType] = []
        to_visit = list(reversed(struct_cls.__declaration__.extended_by))
        while to_visit:
            inheriting_type = to_visit.pop()
            if inheriting_type not in inherited_by_structs:
                inherited_by_structs.append(inheriting_type)
                inheriting_cls = STRUCT_CLASS_BY_TYPE[inheriting_type]
                to_visit.extend(reversed(inheriting_cls.__declaration__.extended_by))
        struct_cls.__declaration__.inherited_by = list(inherited_by_structs)

    from destack.language.core import Node

    # index Node parent types
    for node_cls in NODE_CLASS_BY_TYPE.values():
        if node_cls.__declaration__.parent_property is None:
            continue
        elif node_cls.metatype == NodeType.SPACE:
            node_cls.__declaration__.parent_types = []
            node_cls.__declaration__.parent_property.type.node_types = ()
        else:
            parent_types = node_cls.__declaration__.parent_property.type.node_types or ()
            assert len(parent_types) < len(NodeType), f"generic parent for '{node_cls.__name__}'"
            node_cls.__declaration__.parent_types = list(parent_types)

    # index child types
    child_types_by_parent: dict[NodeType, list[NodeType]] = defaultdict(list)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        for parent_type in node_cls.__declaration__.parent_types:
            child_types_by_parent[parent_type].append(node_cls.metatype)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        node_cls.__declaration__.child_types = list(child_types_by_parent[node_cls.metatype])

    # index ancestor/descendant types (recursive)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        # collect all ancestor types recursively
        ancestors: list[NodeType] = []
        to_visit = list(node_cls.__declaration__.parent_types)
        while to_visit:
            parent_type = to_visit.pop()
            if parent_type not in ancestors:
                ancestors.append(parent_type)
                parent_cls = NODE_CLASS_BY_TYPE[parent_type]
                to_visit.extend(parent_cls.__declaration__.parent_types)

        # collect all descendant types recursively
        descendants: list[NodeType] = []
        to_visit = list(node_cls.__declaration__.child_types)
        while to_visit:
            child_type = to_visit.pop()
            if child_type not in descendants:
                descendants.append(child_type)
                child_cls = NODE_CLASS_BY_TYPE[child_type]
                to_visit.extend(child_cls.__declaration__.child_types)

        node_cls.__declaration__.ancestor_types = list(ancestors)
        node_cls.__declaration__.descendant_types = list(descendants)

    # index Node event types
    for node_cls in NODE_CLASS_BY_TYPE.values():
        all_event_types: set[NodeType] = set()
        for base in node_cls.__bases__:
            if issubclass(base, Node) and base.__declaration__.self_event_types:
                all_event_types.update(base.__declaration__.self_event_types)
        node_cls.__declaration__.event_types = list(all_event_types)

    # index Node enum types
    for node_cls in NODE_CLASS_BY_TYPE.values():
        all_enum_types: set[EnumType] = set()
        for base in node_cls.__bases__:
            if issubclass(base, Node) and base.__declaration__.self_enum_types:
                all_enum_types.update(base.__declaration__.self_enum_types)
        node_cls.__declaration__.enum_types = list(all_enum_types)

    # finalize encoders
    from destack.encoder import JsoncEncoder, JsonEncoder, KompaktEncoder
    from destack.language.core import ENCODERS, Encoding

    ENCODERS[Encoding.JSON] = JsonEncoder.generate()
    ENCODERS[Encoding.JSONC] = JsoncEncoder.generate()
    ENCODERS[Encoding.KOMPAKT] = KompaktEncoder.generate()
    assert len(ENCODERS) == len(Encoding), f"missing {len(Encoding) - len(ENCODERS)} encoders"

    # generate definition refs
    from destack.language.core import ObjectDefinitionReference

    for node_cls in NODE_CLASS_BY_TYPE.values():
        OBJECT_DEFINITION_REFERENCE_BY_CLASS[node_cls] = ObjectDefinitionReference.of(node_cls)
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        OBJECT_DEFINITION_REFERENCE_BY_CLASS[struct_cls] = ObjectDefinitionReference.of(struct_cls)
    for handle_cls in HANDLE_CLASS_BY_TYPE.values():
        OBJECT_DEFINITION_REFERENCE_BY_CLASS[handle_cls] = ObjectDefinitionReference.of(handle_cls)

    # generate meta info
    from destack.language.core import (
        EnumDefinition,
        HandleDefinition,
        NodeDefinition,
        ScalarType,
        StructDefinition,
        Type,
        TypeCardinality,
    )

    for node_cls in NODE_CLASS_BY_TYPE.values():
        node_definition = NodeDefinition.from_declaration(node_cls, node_cls.__declaration__)
        NODE_DEFINITION_BY_TYPE[node_cls.metatype] = node_definition
        node_cls.__definition__ = node_definition
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        struct_definition = StructDefinition.from_declaration(
            struct_cls, struct_cls.__declaration__
        )
        STRUCT_DEFINITION_BY_TYPE[struct_cls.metatype] = struct_definition
        struct_cls.__definition__ = struct_definition
    for handle_cls in HANDLE_CLASS_BY_TYPE.values():
        handle_definition = HandleDefinition.from_declaration(
            handle_cls, handle_cls.__declaration__
        )
        HANDLE_DEFINITION_BY_TYPE[handle_cls.metatype] = handle_definition
        handle_cls.__definition__ = handle_definition
    for enum_type in ENUM_TYPES:
        enum_definition = EnumDefinition.from_declaration(enum_type, ENUM_CLASS_BY_TYPE[enum_type])
        ENUM_DEFINITION_BY_TYPE[enum_type] = enum_definition

    # index node scalar types
    for node_type in NodeType:
        scalar_type = Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE_VALUE,
            node_types=[node_type],
        )
        NODE_TYPE_SCALAR_BY_TYPE[node_type] = scalar_type

    # finalize constants
    from destack.language.core import ConstantDeclaration, ConstantDefinition

    for object_cls in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values()):
        constants: list[ConstantDefinition] = []
        for name, attribute in object_cls.__dict__.items():
            if isinstance(attribute, ConstantDeclaration):
                if isinstance(attribute.value, Callable):
                    attribute.value = attribute.value()
                constant = ConstantDefinition.from_declaration(attribute)
                constants.append(constant)
                # replace constant with value
                setattr(object_cls, name, attribute.value)
        object_cls.__definition__.constants = list(constants)  # type: ignore (frozen)

    # sanity check stuff
    if IS_DEV or IS_TEST:
        from destack.language.core import Event, PropertyDeclaration

        # check we have all the declared builtin objects
        if len(StructType) != len(STRUCT_CLASS_BY_TYPE):
            missing_struct_types = set(StructType) - set(STRUCT_CLASS_BY_TYPE.keys())
            raise ValueError(
                f"missing {len(missing_struct_types)} Structs: {list(missing_struct_types)}"
            )
        if len(NodeType) != len(NODE_CLASS_BY_TYPE):
            missing_node_types = set(NodeType) - set(NODE_CLASS_BY_TYPE.keys())
            raise ValueError(f"missing {len(missing_node_types)} Nodes: {list(missing_node_types)}")
        if len(EnumType) != len(ENUM_CLASS_BY_TYPE):
            missing_enum_types = set(EnumType) - set(ENUM_CLASS_BY_TYPE.keys())
            raise ValueError(f"missing {len(missing_enum_types)} Enums: {list(missing_enum_types)}")

        # check event types
        for node_cls in NODE_CLASS_BY_TYPE.values():
            if issubclass(node_cls, Event):
                assert not node_cls.__declaration__.event_types, (
                    f"{node_cls.__name__} is an Event but has event types: {node_cls.__declaration__.event_types}"
                )

        # check parent types
        for node_cls in NODE_CLASS_BY_TYPE.values():
            if node_cls.__declaration__.parent_property is None:
                continue
            # check if parent is compatible with bases
            parent_node_types = node_cls.__declaration__.parent_property.type.node_types or ()
            for base_cls in node_cls.__bases__:
                if isinstance(
                    base_parent_property := getattr(base_cls, "__parent_property__", None),
                    PropertyDeclaration,
                ):
                    base_parent_node_types = base_parent_property.type.node_types or ()
                    if (
                        NodeType.NODE in base_parent_node_types
                        or NodeType.ENTITY in base_parent_node_types
                    ):
                        continue  # covers everything
                    missing_base_node_types: list[NodeType] = []
                    for parent_node_type in parent_node_types:
                        if not any(
                            issubclass(
                                NODE_CLASS_BY_TYPE[parent_node_type],
                                NODE_CLASS_BY_TYPE[base_parent_node_type],
                            )
                            for base_parent_node_type in base_parent_node_types
                        ):
                            missing_base_node_types.append(parent_node_type)
                    if missing_base_node_types:
                        raise ValueError(
                            f"{node_cls.__name__}.parent is not compatible with {base_cls.__name__}.parent (missing {[t.name for t in missing_base_node_types]})"
                        )

    _set_finalized()

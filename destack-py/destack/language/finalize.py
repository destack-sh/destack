import time
from collections import defaultdict
from itertools import chain
from typing import TYPE_CHECKING, Callable

from destack.utils.env import IS_DEV, IS_TEST

from .core.builtin.builtin import (
    ENUM_TYPES,
    EnumType,
    NodeType,
    StructType,
    TraitType,
)
from .registry import (
    ANCESTOR_NODE_TYPES_BY_TYPE,
    DESCENDANT_NODE_TYPES_BY_TYPE,
    ENUM_CLASS_BY_TYPE,
    ENUM_DEFINITION_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    NODE_DEFINITION_REFERENCE_BY_CLASS,
    NODE_TYPE_SCALAR_BY_NODE_TYPE,
    NODE_TYPES_BY_TRAIT_TYPE,
    OBJECT_DEFINITION_REFERENCE_BY_CLASS,
    STRUCT_CLASS_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
    SUBDEFINITIONS_BY_NODE_TYPE,
    TRAIT_CLASS_BY_TYPE,
    TRAIT_DEFINITION_BY_TYPE,
    get_node_or_trait_cls,
    get_subdefinitions_for_node_type,
)

if TYPE_CHECKING:
    pass


def finalize():
    """Finalize the Destack language SDK."""

    from destack.language.core.builtin.object import _is_finalized, _set_finalized

    if _is_finalized():
        return

    start = time.time()

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

    # index Node extended_by (direct) / inherited_by (direct and indirect)
    node_type_extended_by: dict[NodeType, list[NodeType]] = defaultdict(list)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        if node_cls.__base_type__ is not None:
            node_type_extended_by[node_cls.__base_type__].append(node_cls.metatype)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        node_cls.__extended_by__ = tuple(node_type_extended_by[node_cls.metatype])

    # index Node inherited_by (recursive)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        # collect all types that inherit from this node recursively
        inherited_by_nodes: list[NodeType] = []
        to_visit = list(reversed(node_cls.__extended_by__))
        while to_visit:
            inheriting_type = to_visit.pop()
            if inheriting_type not in inherited_by_nodes:
                inherited_by_nodes.append(inheriting_type)
                inheriting_cls = NODE_CLASS_BY_TYPE[inheriting_type]
                to_visit.extend(reversed(inheriting_cls.__extended_by__))
        node_cls.__inherited_by__ = tuple(inherited_by_nodes)

    # index Struct extended_by (direct) / inherited_by (direct and indirect)
    struct_type_extended_by: dict[StructType, list[StructType]] = defaultdict(list)
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        if struct_cls.__base_type__ is not None:
            struct_type_extended_by[struct_cls.__base_type__].append(struct_cls.metatype)
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        struct_cls.__extended_by__ = tuple(struct_type_extended_by[struct_cls.metatype])

    # index Struct inherited_by (recursive)
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        # collect all types that inherit from this struct recursively
        inherited_by_structs: list[StructType] = []
        to_visit = list(reversed(struct_cls.__extended_by__))
        while to_visit:
            inheriting_type = to_visit.pop()
            if inheriting_type not in inherited_by_structs:
                inherited_by_structs.append(inheriting_type)
                inheriting_cls = STRUCT_CLASS_BY_TYPE[inheriting_type]
                to_visit.extend(reversed(inheriting_cls.__extended_by__))
        struct_cls.__inherited_by__ = tuple(inherited_by_structs)

    from destack.language.core import Node

    # index Node parent types
    for node_cls in NODE_CLASS_BY_TYPE.values():
        if node_cls.__parent_property__ is None:
            continue
        elif node_cls.metatype == NodeType.SPACE:
            node_cls.__parent_types__ = ()
            node_cls.__parent_property__.node_types = ()
        else:
            parent_types = node_cls.__parent_property__.node_types or ()
            assert len(parent_types) < len(NodeType), f"generic parent for '{node_cls.__name__}'"
            node_cls.__parent_types__ = tuple(parent_types)
        parent_classes = tuple(
            NODE_CLASS_BY_TYPE[parent_type] for parent_type in node_cls.__parent_types__
        )
        node_cls.__parent_classes__ = parent_classes

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
        ancestors: list[NodeType] = []
        to_visit = list(node_cls.__parent_types__)
        while to_visit:
            parent_type = to_visit.pop()
            if parent_type not in ancestors:
                ancestors.append(parent_type)
                parent_cls = NODE_CLASS_BY_TYPE[parent_type]
                to_visit.extend(parent_cls.__parent_types__)

        # collect all descendant types recursively
        descendants: list[NodeType] = []
        to_visit = list(node_cls.__child_types__)
        while to_visit:
            child_type = to_visit.pop()
            if child_type not in descendants:
                descendants.append(child_type)
                child_cls = NODE_CLASS_BY_TYPE[child_type]
                to_visit.extend(child_cls.__child_types__)

        node_cls.__ancestor_types__ = tuple(ancestors)
        node_cls.__descendant_types__ = tuple(descendants)
        DESCENDANT_NODE_TYPES_BY_TYPE[node_cls.metatype] = node_cls.__descendant_types__
        ANCESTOR_NODE_TYPES_BY_TYPE[node_cls.metatype] = node_cls.__ancestor_types__

    # index Node event types
    for node_cls in chain(NODE_CLASS_BY_TYPE.values(), TRAIT_CLASS_BY_TYPE.values()):
        all_event_types: set[NodeType] = set()
        for base in node_cls.__bases__:
            if issubclass(base, Node) and base.__self_event_types__:
                all_event_types.update(base.__self_event_types__)
        node_cls.__event_types__ = tuple(all_event_types)

    # index Node enum types
    for node_cls in NODE_CLASS_BY_TYPE.values():
        all_enum_types: set[EnumType] = set()
        for base in node_cls.__bases__:
            if issubclass(base, Node) and base.__self_enum_types__:
                all_enum_types.update(base.__self_enum_types__)
        node_cls.__enum_types__ = tuple(all_enum_types)

    # finalize encoders
    from destack.encoder import CsonEncoder, JsonEncoder, KompaktEncoder
    from destack.language.core import ENCODERS, Encoding

    ENCODERS[Encoding.JSON] = JsonEncoder()
    ENCODERS[Encoding.CSON] = CsonEncoder()
    ENCODERS[Encoding.KOMPAKT] = KompaktEncoder()
    assert len(ENCODERS) == len(Encoding), f"missing {len(Encoding) - len(ENCODERS)} encoders"

    # generate definition refs
    from destack.language.core import NodeDefinitionReference, ObjectDefinitionReference

    for node_cls in NODE_CLASS_BY_TYPE.values():
        NODE_DEFINITION_REFERENCE_BY_CLASS[node_cls] = NodeDefinitionReference.of(node_cls)
        OBJECT_DEFINITION_REFERENCE_BY_CLASS[node_cls] = ObjectDefinitionReference.of(node_cls)
    for node_cls in TRAIT_CLASS_BY_TYPE.values():
        OBJECT_DEFINITION_REFERENCE_BY_CLASS[node_cls] = ObjectDefinitionReference.of(node_cls)
    for node_cls in STRUCT_CLASS_BY_TYPE.values():
        OBJECT_DEFINITION_REFERENCE_BY_CLASS[node_cls] = ObjectDefinitionReference.of(node_cls)

    # generate meta info
    from destack.language.core import (
        EnumDefinition,
        NodeDefinition,
        ScalarType,
        StructDefinition,
        TraitDefinition,
        Type,
        TypeCardinality,
    )

    for trait_type, trait_cls in TRAIT_CLASS_BY_TYPE.items():
        trait_definition = TraitDefinition.from_declaration(trait_cls)
        TRAIT_DEFINITION_BY_TYPE[trait_type] = trait_definition
    for node_cls in NODE_CLASS_BY_TYPE.values():
        node_definition = NodeDefinition.from_declaration(node_cls)
        NODE_DEFINITION_BY_TYPE[node_cls.metatype] = node_definition
        node_cls.__definition__ = node_definition
        node_cls.__definition_reference__ = NODE_DEFINITION_REFERENCE_BY_CLASS[node_cls]
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        struct_definition = StructDefinition.from_declaration(struct_cls)
        STRUCT_DEFINITION_BY_TYPE[struct_cls.metatype] = struct_definition
        struct_cls.__definition__ = struct_definition
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
        NODE_TYPE_SCALAR_BY_NODE_TYPE[node_type] = scalar_type

    # index subdefinitions
    for node_type in NodeType:
        SUBDEFINITIONS_BY_NODE_TYPE[node_type] = get_subdefinitions_for_node_type(node_type)

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
        object_cls.__constants__ = tuple(constants)

    # sanity check stuff
    if IS_DEV or IS_TEST:
        from destack.language.core import Event, PropertyDeclaration

        # check we have all the declared builtin objects
        if len(StructType) != len(STRUCT_CLASS_BY_TYPE):
            missing_struct_types = set(StructType) - set(STRUCT_CLASS_BY_TYPE.keys())
            raise ValueError(
                f"missing {len(missing_struct_types)} Structs: {list(missing_struct_types)}"
            )
        if len(TraitType) != len(TRAIT_CLASS_BY_TYPE):
            missing_trait_types = set(TraitType) - set(TRAIT_CLASS_BY_TYPE.keys())
            raise ValueError(
                f"missing {len(missing_trait_types)} Traits: {list(missing_trait_types)}"
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
                assert not node_cls.__event_types__, (
                    f"{node_cls.__name__} is an Event but has event types: {node_cls.__event_types__}"
                )

        # check parent types
        for node_cls in NODE_CLASS_BY_TYPE.values():
            if node_cls.__parent_property__ is None:
                continue
            # check if parent is compatible with bases
            parent_node_types = node_cls.__parent_property__.node_types or ()
            for base_cls in node_cls.__bases__:
                if isinstance(
                    base_parent_property := getattr(base_cls, "__parent_property__", None),
                    PropertyDeclaration,
                ):
                    base_parent_node_types = base_parent_property.node_types or ()
                    if (
                        NodeType.NODE in base_parent_node_types
                        or NodeType.ENTITY in base_parent_node_types
                    ):
                        continue  # covers everything
                    missing_base_node_types: list[NodeType | TraitType] = []
                    for parent_node_type in parent_node_types:
                        if not any(
                            issubclass(
                                get_node_or_trait_cls(parent_node_type),
                                get_node_or_trait_cls(base_parent_node_type),
                            )
                            for base_parent_node_type in base_parent_node_types
                        ):
                            missing_base_node_types.append(parent_node_type)
                    if missing_base_node_types:
                        raise ValueError(
                            f"{node_cls.__name__}.parent is not compatible with {base_cls.__name__}.parent (missing {[t.name for t in missing_base_node_types]})"
                        )

    _set_finalized()
    print(f"finalize took {time.time() - start:.3f}s")

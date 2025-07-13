from collections import defaultdict
from itertools import chain
from typing import TYPE_CHECKING, Any

from destack import proto
from destack.utils.code import exec_
from destack.utils.env import IS_DEV, IS_TEST
from destack.utils.uuid import UUID

from .core.builtin.common import (
    ENUM_TYPES,
    EnumType,
    NodeType,
    StoreKey,
    StructType,
    TraitType,
)
from .core.builtin.const import UNSET
from .registry import (
    ANCESTOR_NODE_TYPES_BY_TYPE,
    CONSTANT_DEFINITIONS,
    DESCENDANT_NODE_TYPES_BY_TYPE,
    ENUM_CLASS_BY_TYPE,
    ENUM_DEFINITION_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    NODE_DEFINITION_REFERENCE_BY_CLASS,
    NODE_TYPE_SCALAR_BY_NODE_TYPE,
    NODE_TYPES_BY_PRIMARY_STORE_KEY,
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

    # index node types by store key
    node_types_by_store_key: dict[StoreKey, list[NodeType]] = defaultdict(list)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        if node_cls.__is_abstract__:
            continue
        primary_store_keys: list[StoreKey] = []
        if NodeType.ENTITY in node_cls.__inherits__:
            primary_store_keys.append(StoreKey.ENTITY_PRIMARY)
        elif NodeType.EVENT in node_cls.__inherits__:
            primary_store_keys.append(StoreKey.EVENT_PRIMARY)
        else:
            raise ValueError(f"unexpected node type: {node_cls}")
        node_cls.__primary_store_keys__ = tuple(primary_store_keys)
        for store_key in primary_store_keys:
            node_types_by_store_key[store_key].append(node_cls.metatype)
    for store_key in StoreKey:
        NODE_TYPES_BY_PRIMARY_STORE_KEY[store_key] = tuple(
            node_types_by_store_key.get(store_key, ())
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

    from destack.language.core import Node, expand_node_traits

    # index parent types
    for node_cls in NODE_CLASS_BY_TYPE.values():
        assert node_cls.__parent_property__ is not UNSET
        if node_cls.__root_type__ is None and not node_cls.__is_abstract__:
            node_cls.__parent_types__ = ()
            if node_cls.__parent_property__ is not None:
                node_cls.__parent_property__.node_types = ()
        else:
            parent_types = expand_node_traits(node_cls.__parent_property__.node_types or ())
            assert len(parent_types) < len(NodeType), f"generic parent for '{node_cls.__name__}'"
            node_cls.__parent_types__ = parent_types
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

    # index event types
    for node_cls in chain(NODE_CLASS_BY_TYPE.values(), TRAIT_CLASS_BY_TYPE.values()):
        all_event_types: set[NodeType] = set()
        for base in node_cls.__bases__:
            if issubclass(base, Node) and base.__base_event_types__:
                for event_type in base.__base_event_types__:
                    all_event_types.add(event_type)
                    event_cls = NODE_CLASS_BY_TYPE[event_type]
                    all_event_types.update(event_cls.__inherited_by__)
        node_cls.__event_types__ = tuple(all_event_types)

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
    for node_cls in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values()):
        cls_dict_copy = node_cls.__dict__.copy()
        # __pack_proto__/__unpack_proto__/_to_proto
        proto_impl, proto_glbls = generate_pack_proto_impl(node_cls)
        exec_(
            proto_impl,
            {**builtin_class_by_name, **proto_glbls},
            cls_dict_copy,
            f"{node_cls.__name__}:proto",
        )
        setattr(node_cls, "__pack_proto__", cls_dict_copy["__pack_proto__"])
        setattr(node_cls, "__unpack_proto__", cls_dict_copy["__unpack_proto__"])
        setattr(node_cls, "to_proto", cls_dict_copy["to_proto"])
        setattr(node_cls, "from_proto", cls_dict_copy["from_proto"])
        # __pack_value__/__unpack_value__/_to_value
        value_impl, value_glbls = generate_pack_value_impl(node_cls)
        exec_(
            value_impl,
            {**builtin_class_by_name, **value_glbls},
            cls_dict_copy,
            f"{node_cls.__name__}:value",
        )
        setattr(node_cls, "__pack_value__", cls_dict_copy["__pack_value__"])
        setattr(node_cls, "__unpack_value__", cls_dict_copy["__unpack_value__"])
        setattr(node_cls, "to_value", cls_dict_copy["to_value"])
        setattr(node_cls, "from_value", cls_dict_copy["from_value"])

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
        CONSTANT_DECLARATIONS,
        ConstantDefinition,
        EnumDefinition,
        NodeDefinition,
        ScalarType,
        StructDefinition,
        TraitDefinition,
        Type,
        TypeCardinality,
    )

    for trait_type, trait_cls in TRAIT_CLASS_BY_TYPE.items():
        trait_definition = TraitDefinition.from_trait(trait_cls)
        TRAIT_DEFINITION_BY_TYPE[trait_type] = trait_definition
    for node_cls in NODE_CLASS_BY_TYPE.values():
        node_definition = NodeDefinition.from_node(node_cls)
        NODE_DEFINITION_BY_TYPE[node_cls.metatype] = node_definition
        node_cls.__definition__ = node_definition
        node_cls.__definition_reference__ = NODE_DEFINITION_REFERENCE_BY_CLASS[node_cls]
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        struct_definition = StructDefinition.from_struct(struct_cls)
        STRUCT_DEFINITION_BY_TYPE[struct_cls.metatype] = struct_definition
        struct_cls.__definition__ = struct_definition
    for enum_type in ENUM_TYPES:
        enum_definition = EnumDefinition.from_enum(enum_type, ENUM_CLASS_BY_TYPE[enum_type])
        ENUM_DEFINITION_BY_TYPE[enum_type] = enum_definition
    for constant_declaration in CONSTANT_DECLARATIONS.values():
        constant_definition = ConstantDefinition.from_constant(constant_declaration)
        CONSTANT_DEFINITIONS[constant_declaration.name] = constant_definition

    # index node scalar types
    for node_type in NodeType:
        scalar_type = Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE_VALUE,
            node_type=node_type,
        )
        NODE_TYPE_SCALAR_BY_NODE_TYPE[node_type] = scalar_type

    # index subdefinitions
    for node_type in NodeType:
        SUBDEFINITIONS_BY_NODE_TYPE[node_type] = get_subdefinitions_for_node_type(node_type)

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

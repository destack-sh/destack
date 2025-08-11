from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    cast,
    dataclass_transform,
)

from destack.registry import NODE_CLASS_BY_TYPE, NODE_TYPE_BY_CLASS, STRUCT_CLASS_BY_TYPE

from ._const import UNSET
from ._hoisted import ReferenceType
from .declaration import NodeDeclaration, TagDeclaration, declare_method
from .object import Object, ValueFactory, _process_object_cls
from .property import _PROPERTY_SPECIFIERS, declare_property, declare_property_runtime
from .universe import (
    EnumType,
    NodeType,
    ObjectKind,
    ObjectStability,
    StructType,
    TraitType,
)
from .uuid import UUID

if TYPE_CHECKING:
    from destack import (
        Branch,
        ConstraintDeclaration,
        IndexDeclaration,
        Node,
        NodeDefinition,
        NodeSpatialReference,
        PermissionDeclaration,
        Snapshot,
        Space,
    )


type_ = type


def _process_node_cls(
    *,
    # meta
    cls: type,
    node_type: NodeType,
    is_abstract: bool,
    is_final: bool,
    is_singleton: bool,
    is_immutable: bool,
    # inheritance
    traits: tuple[TraitType, ...],
    # content
    indexes: tuple["IndexDeclaration", ...],
    constraints: tuple["ConstraintDeclaration", ...],
    permissions: tuple["PermissionDeclaration", ...],
    tags: tuple["TagDeclaration", ...],
    # tree
    expected_parent_types: tuple[NodeType, ...],
    expected_child_types: tuple[NodeType, ...],
    expected_ancestor_types: tuple[NodeType, ...],
    expected_descendant_types: tuple[NodeType, ...],
    # associations
    message_types: tuple[StructType, ...],
    event_types: tuple[NodeType, ...],
    enum_types: tuple[EnumType, ...],
    base_struct_type: StructType | None,
) -> type["Node"]:
    assert cls.__name__ == "Node" or issubclass(cls, Node), f"{cls.__name__} is not a Node"

    # inheritance
    inherits: list[NodeType] = []
    all_traits: list[TraitType] = list(traits)
    all_event_types: list[NodeType] = []
    if cls.__name__ != "Node":
        for base in cls.__mro__:
            if issubclass(base, Node):
                if base.metatype not in inherits:
                    inherits.append(base.metatype)
                for trait in base.__declaration__.traits:
                    if trait not in all_traits:
                        all_traits.append(trait)
                for event_type in base.__declaration__.event_types:
                    if event_type not in all_event_types:
                        all_event_types.append(event_type)
    if NodeType.EVENT in inherits:
        is_immutable = True  # Events are always frozen

    # declaration
    declaration = NodeDeclaration(
        # meta
        cls=cls,
        type=node_type,
        id=node_type.value,
        name=cls.__name__,
        description=cls.__doc__ or "",
        kind=ObjectKind.NODE,
        stability=ObjectStability.DYNAMIC,
        is_abstract=is_abstract,
        is_immutable=is_immutable,
        is_final=is_final,
        is_singleton=is_singleton,
        # inherits
        base_type=inherits[0] if inherits else None,
        inherits=list(reversed(inherits)),
        inherited_by=[],
        extended_by=[],
        traits=list(reversed(all_traits)),
        self_traits=list(all_traits),
        base_struct_type=base_struct_type,
        # content
        properties=[],  # set in _process_object_cls
        methods=[],  # set in finalize
        actions=[],  # set in finalize
        constants=[],  # set in finalize
        indexes=list(indexes),
        constraints=list(constraints),
        permissions=list(permissions),
        tags=list(tags),
        # graph
        parent_property=None,
        parent_types=[],
        child_types=[],
        ancestor_types=[],
        descendant_types=[],
        expected_parent_types=list(expected_parent_types),
        expected_child_types=list(expected_child_types),
        expected_ancestor_types=list(expected_ancestor_types),
        expected_descendant_types=list(expected_descendant_types),
        # associations
        event_types=list(reversed(all_event_types)),
        self_event_types=list(event_types),
    )

    # process class
    cls, _ = _process_object_cls(cast(type["Node"], cls), declaration)
    cls.metatype = node_type

    # register
    NODE_CLASS_BY_TYPE[node_type] = cls
    NODE_TYPE_BY_CLASS[cls] = node_type
    NODE_CLASS_BY_TYPE[node_type] = cls

    # validate
    # check for too many nullable properties (for :Encoding)
    nullable_properties = [p for p in cls.__properties__.values() if not p.type.is_required]
    assert len(nullable_properties) < 64, f"Node {cls.__name__} has too many nullable properties"
    # check for too many properties (for :Encoding)
    wired_properties = [p for p in cls.__properties__.values() if not p.is_runtime_only]
    assert len(wired_properties) < 128, f"Node {cls.__name__} has too many wired properties"
    # non-abstract nodes must have properties
    if not is_abstract and not any(
        not prop.is_runtime_only for prop in cls.__declaration__.properties
    ):
        raise ValueError(f"{cls.__name__} is not abstract but has no properties")
    # abstract nodes cannot extend non-abstract nodes
    if is_abstract and cls.__bases__ and not cls.__bases__[0].__is_abstract__:
        raise ValueError(
            f"{cls.__name__} is abstract but extends non-abstract {cls.__bases__[0].__name__}"
        )
    # cannot be both abstract and final
    if is_abstract and is_final:
        raise ValueError(f"{cls.__name__} cannot be both abstract and final")
    # final objects cannot be extended
    if any(
        hasattr(base, "__declaration__") and base.__declaration__.is_final for base in cls.__bases__
    ):
        bad_base = next(
            base
            for base in cls.__bases__
            if hasattr(base, "__declaration__") and base.__declaration__.is_final
        )
        raise ValueError(f"{cls.__name__} extends final {bad_base.__name__}")
    # struct type must be fully matched
    if base_struct_type is not None:
        struct_cls = STRUCT_CLASS_BY_TYPE[base_struct_type]
        for struct_prop in struct_cls.__declaration__.properties:
            if struct_prop.is_managed:
                continue
            node_prop = cls.__properties_by_alias__.get(struct_prop.name)
            if node_prop is None:
                if struct_prop.name == "template":
                    # nocheckin: proper mechanism for "struct with partial overrides to node/struct"
                    #  (like Styles or TransitionTemplate or any template really..
                    #   .. similarity to Offset2/Inset2/... with base and overrides?
                    #   .. similarity to Entity partials?
                    #    .. just a generic ScalarType.PARTIAL? (or PARTIAL_STRUCT/PARTIAL_NODE?)
                    #   .. also similarity to Context overrides in Entity.context_values?
                    #   .. also similarity to mut/non mut Structs?
                    #   .. also related to (frozen-in-time) Structs & Nodes as values?)
                    #   .. also related to partial Node Values for animation tracks?
                    #   .. if this were a separate Struct we could do a custom Encoder
                    #       instead of stuffing it into materialization logic?)
                    continue
                raise ValueError(f"'{cls.__name__}' has no property {struct_prop!r}")
            if node_prop.type != struct_prop.type:
                diff = node_prop.type.diff(struct_prop.type)
                raise ValueError(
                    f"'{cls.__name__}' property {node_prop!r} has type {node_prop.type!r} but {struct_prop!r} has type {struct_prop.type}\nDifferences: {diff}"
                )

    return cast(type["Node"], cls)


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def _declare_node(
    # meta
    node_type: NodeType,
    *,
    frozen: bool = False,
    is_abstract: bool = False,
    is_final: bool = False,
    is_singleton: bool = False,
    # inheritance
    traits: tuple[TraitType, ...] = (),
    base_struct_type: StructType | None = None,
    # content
    indexes: tuple["IndexDeclaration", ...] = (),
    constraints: tuple["ConstraintDeclaration", ...] = (),
    permissions: tuple["PermissionDeclaration", ...] = (),
    tags: tuple["TagDeclaration", ...] = (),
    # tree
    expected_parent_types: tuple[NodeType, ...] = (),
    expected_child_types: tuple[NodeType, ...] = (),
    expected_ancestor_types: tuple[NodeType, ...] = (),
    expected_descendant_types: tuple[NodeType, ...] = (),
    # associations
    event_types: tuple[NodeType, ...] = (),
    enum_types: tuple[EnumType, ...] = (),
    message_types: tuple[StructType, ...] = (),
):
    """Register a class as a concrete node for the given node type."""

    def decorate(cls: type) -> type:
        cls = _process_node_cls(
            # meta
            cls=cls,
            node_type=node_type,
            is_abstract=is_abstract,
            is_final=is_final,
            is_singleton=is_singleton,
            is_immutable=frozen,
            # inheritance
            traits=traits,
            base_struct_type=base_struct_type,
            # content
            indexes=indexes,
            constraints=constraints,
            permissions=permissions,
            tags=tags,
            # tree
            expected_parent_types=expected_parent_types,
            expected_child_types=expected_child_types,
            expected_ancestor_types=expected_ancestor_types,
            expected_descendant_types=expected_descendant_types,
            # associations
            event_types=event_types,
            enum_types=enum_types,
            message_types=message_types,
        )
        return cls

    return decorate


@_declare_node(
    node_type=NodeType.NODE,
    is_abstract=True,
    tags=(
        TagDeclaration(id=1, name="identity", description="Node identity"),
        TagDeclaration(id=2, name="tracking", description="Node tracking"),
    ),
)
class Node(Object):
    """
    A Node with some Properties and a persistent identity (its id).

    Nodes belong to a Space and are thus identifiable by their (space_id, id) tuple.
    """

    """The kind of Object this is (static)."""
    metakind: ClassVar[ObjectKind] = ObjectKind.NODE
    """The type of Node this is (static)."""
    metatype: ClassVar[NodeType] = UNSET
    """The declaration of this Node (static)."""
    __declaration__: ClassVar["NodeDeclaration"]
    """The definition of this Node (static)."""
    __definition__: ClassVar["NodeDefinition"]

    # 1-20: node identity
    id: UUID = declare_property(
        2,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        is_interned=True,
        description="The universally unique identifier of this Node.",
        tags=("identity",),
    )
    space: "Space" = declare_property(
        3,
        is_managed=True,
        is_eq=False,
        is_hash=False,
        is_readonly=True,
        reference_type=ReferenceType.RAW,
        default_factory=ValueFactory.SPACE,
        description="The Space this Node is in.",
        tags=("identity",),
    )
    branch: "Branch" = declare_property(
        4,
        is_readonly=True,
        is_managed=True,
        is_eq=False,
        is_hash=False,
        reference_type=ReferenceType.RAW,
        default_factory=ValueFactory.BRANCH,
        description="The Branch this Node is part of.",
        tags=("identity",),
    )
    snapshot: "Snapshot" = declare_property(
        5,
        is_readonly=True,
        is_managed=True,
        is_eq=False,
        is_hash=False,
        reference_type=ReferenceType.RAW,
        default_factory=ValueFactory.SNAPSHOT,
        description="The Snapshot this Node is part of.",
        tags=("identity",),
    )

    # 100+ for general properties
    # ...

    """Whether this Node is new."""
    _is_new: bool = declare_property_runtime(402, default=False)

    def __eq__(self, other: Any):
        """Equals the Node's identity."""
        return type(self) is type(other) and (self.id == other.id)

    def __hash__(self):
        """Hash the Node's identity."""
        return self.id.int

    @property
    @declare_method(1)
    def path(self) -> str:
        """The human readable path of this Node."""
        raise NotImplementedError

    @declare_method(2)
    def to_ref(self) -> "NodeSpatialReference":
        """Gets a reference to this Node."""
        raise NotImplementedError

from collections.abc import Collection, Sequence
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Optional,
    Self,
    cast,
    dataclass_transform,
)

from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_REFERENCE_BY_CLASS,
    NODE_TYPE_BY_CLASS,
)
from destack.utils.uuid import UUID

from .builtin import EnumType, NodeType, ObjectKind, ObjectStability, TraitType
from .const import UNSET
from .declaration import NodeDeclaration, TagDeclaration, builtin_method
from .object import Object, ValueFactory, _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    builtin_property,
    builtin_property_runtime,
)

if TYPE_CHECKING:
    from destack.language import (
        Condition,
        ConstraintDeclaration,
        ExpressionIn,
        IndexDeclaration,
        JoinIn,
        Node,
        NodeDefinition,
        NodeDefinitionReference,
        NodeReference,
        PermissionDeclaration,
        Query,
        Session,
        Sort,
        Space,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_node(
    # meta
    node_type: NodeType,
    *,
    frozen: bool = False,
    is_abstract: bool = False,
    is_final: bool = False,
    is_singleton: bool = False,
    # inheritance
    traits: tuple[TraitType, ...] = (),
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
):
    """Register a class as a concrete node for the given node type."""

    def decorate(cls: type) -> type:
        nonlocal frozen
        assert cls.__name__ == "Node" or issubclass(cls, Node), f"{cls.__name__} is not a Node"

        # bases
        inherits: list[NodeType] = []
        all_traits: list[TraitType] = list(traits)
        all_enum_types: list[EnumType] = []
        all_event_types: list[NodeType] = []
        if cls.__name__ != "Node":
            for base in cls.__mro__:
                if issubclass(base, Node):
                    if base.metatype not in inherits:
                        inherits.append(base.metatype)
                    for trait in base.__declaration__.traits:
                        if trait not in all_traits:
                            all_traits.append(trait)
                    for enum_type in base.__declaration__.enum_types:
                        if enum_type not in all_enum_types:
                            all_enum_types.append(enum_type)
                    for event_type in base.__declaration__.event_types:
                        if event_type not in all_event_types:
                            all_event_types.append(event_type)

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
            hasattr(base, "__declaration__") and base.__declaration__.is_final
            for base in cls.__bases__
        ):
            bad_base = next(
                base
                for base in cls.__bases__
                if hasattr(base, "__declaration__") and base.__declaration__.is_final
            )
            raise ValueError(f"{cls.__name__} extends final {bad_base.__name__}")
        if NodeType.EVENT in inherits:
            frozen = True  # Events are always frozen

        # declaration
        declaration = NodeDeclaration(
            # meta
            cls=cls,
            type=node_type,
            id=node_type.value,
            kind=ObjectKind.NODE,
            stability=ObjectStability.DYNAMIC,
            is_abstract=is_abstract,
            is_frozen=frozen,
            is_final=is_final,
            is_singleton=is_singleton,
            # inherits
            base_type=inherits[0] if inherits else None,
            inherits=list(reversed(inherits)),
            inherited_by=[],
            extended_by=[],
            traits=list(reversed(all_traits)),
            self_traits=list(all_traits),
            # content
            properties=[],
            methods=[],
            actions=[],
            constants=[],
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
            enum_types=list(reversed(all_enum_types)),
            self_enum_types=list(enum_types),
        )

        # process class
        cls, _ = _process_object_cls(cast(type["Node"], cls), declaration)
        cls.metatype = node_type

        # register
        NODE_CLASS_BY_TYPE[node_type] = cls
        NODE_TYPE_BY_CLASS[cls] = node_type
        NODE_CLASS_BY_TYPE[node_type] = cls

        return cls

    return decorate


@builtin_node(
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
    Nodes always belong to a Space and are thus identifiable by their (space_id, id) tuple.
    """

    # meta
    metatype: ClassVar[NodeType]
    __declaration__: ClassVar["NodeDeclaration"]
    __definition__: ClassVar["NodeDefinition"]
    __definition_reference__: ClassVar["NodeDefinitionReference"]

    # 1-20: node identity
    # Node.metatype: 1
    id: UUID = builtin_property(
        2,
        is_internal=True,
        is_eq=False,
        is_readonly=True,
        is_identity=True,
        description="The universally unique identifier of this Node.",
        tags=("identity",),
    )
    space: "Space" = builtin_property(
        5,
        is_internal=True,
        is_readonly=True,
        is_identity=True,
        default_factory=ValueFactory.SPACE,
        description="The Space this Node is in.",
        tags=("identity",),
    )
    if TYPE_CHECKING:
        space_ptr: NodeReference = UNSET

    # 100+ for general properties
    # ...

    """The Session this Node is in."""
    _session: "Session" = builtin_property_runtime()
    """The cached reference to this Node instance."""
    _ref: "Optional[NodeReference]" = builtin_property_runtime(default=None)
    """Whether this Node is new."""
    _is_new: bool = builtin_property_runtime(default=False)

    def __eq__(self, other: Any):
        """Equals the Node's identity."""
        return type(self) is type(other) and (self.id == other.id)

    def __hash__(self):
        """Hash the Node's identity."""
        return self.id.int

    @property
    @builtin_method(1)
    def path(self) -> str:
        """The human readable path of this Node."""
        raise NotImplementedError  # generated

    def __to_ref__(self) -> "NodeReference":
        """Gets a reference to this Node."""
        raise NotImplementedError  # generated

    @builtin_method(2)
    def to_ref(self) -> "NodeReference":
        """Gets a reference to this Node."""
        if self._ref is None:
            self._ref = self.__to_ref__()
        return self._ref

    @classmethod
    @builtin_method(60)
    def get(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        **subqueries: "Query",
    ) -> "Query[Self]":  # type: ignore
        """Make a get Query for this Node."""
        from ..common.query import Join, Query, QueryType, to_subqueries

        query = Query(
            type=QueryType.NODE,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            subqueries=to_subqueries(subqueries),
        )
        return query  # type: ignore

    @classmethod
    @builtin_method(61)
    def search(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        having: Optional["Condition"] = None,
        sort: Optional[list["Sort"]] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
        **subqueries: "Query",
    ) -> "Query[Self]":  # type: ignore
        """Make a search Query for this Node."""
        from ..common.query import Expression, Join, Query, QueryType, to_subqueries

        query = Query(
            type=QueryType.NODE if not group_by else QueryType.GROUPED_NODE,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            sort=sort or [],
            limit=limit,
            offset=offset,
            subqueries=to_subqueries(subqueries),
        )
        return query  # type: ignore

    @classmethod
    @builtin_method(62)
    def exists(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
    ) -> "Query[Self]":  # type: ignore
        """Make a count Query for this Node."""
        from ..common.query import (
            Aggregation,
            AggregationType,
            Join,
            Query,
            QueryType,
        )

        query = Query(
            type=QueryType.SCALAR,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            aggregation=Aggregation(type=AggregationType.EXISTS),
        )
        return query  # type: ignore

    @classmethod
    @builtin_method(63)
    def count(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        sort: Optional[list["Sort"]] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        having: Optional["Condition"] = None,
    ) -> "Query[Self]":  # type: ignore
        """Make a min Query for this Node."""
        from ..common.query import Aggregation, AggregationType, Expression, Join, Query, QueryType

        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.COUNT),
            sort=sort or [],
        )
        return query  # type: ignore

    @classmethod
    @builtin_method(64)
    def min(
        cls: type["Self"],
        expression: "ExpressionIn",
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        where: Optional["Condition"] = None,
        having: Optional["Condition"] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        sort: Optional[list["Sort"]] = None,
    ) -> "Query[Self]":  # type: ignore
        from ..common.query import Aggregation, AggregationType, Expression, Join, Query, QueryType

        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.MIN, expression=Expression.of(expression)),
            sort=sort or [],
        )
        return query  # type: ignore

    @classmethod
    @builtin_method(65)
    def max(
        cls: type["Self"],
        expression: "ExpressionIn",
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        where: Optional["Condition"] = None,
        having: Optional["Condition"] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        sort: Optional[list["Sort"]] = None,
    ) -> "Query[Self]":  # type: ignore
        """Make an average Query for this Node."""
        from ..common.query import Aggregation, AggregationType, Expression, Join, Query, QueryType

        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.MAX, expression=Expression.of(expression)),
            sort=sort or [],
        )
        return query  # type: ignore

    @classmethod
    @builtin_method(66)
    def sum(
        cls: type["Self"],
        expression: "ExpressionIn",
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        where: Optional["Condition"] = None,
        having: Optional["Condition"] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        sort: Optional[list["Sort"]] = None,
    ) -> "Query[Self]":  # type: ignore
        """Make an average Query for this Node."""
        from ..common.query import Aggregation, AggregationType, Expression, Join, Query, QueryType

        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.SUM, expression=Expression.of(expression)),
            sort=sort or [],
        )
        return query  # type: ignore


def expand_node_inheritance(types: Collection[NodeType]) -> Sequence[NodeType]:
    """
    Expand a collection of NodeTypes into a flat collection of NodeTypes.
    """
    node_types: set[NodeType] = set()
    for typ in types:
        node_cls = NODE_CLASS_BY_TYPE[typ]
        node_types.update(node_cls.__definition__.inherited_by)
        if not node_cls.__definition__.is_abstract:
            node_types.add(typ)
    return tuple(node_types)


def expand_node_types(
    node_type: "NodeType | Collection[NodeType] | type[Node] | None",
    expand_inheritance: bool = True,
) -> Sequence["NodeType"]:
    """Resolve the NodeTypes for a NodeType, TraitType, or Node class."""
    if node_type is None:
        return ()
    node_types: Sequence[NodeType] = []
    if isinstance(node_type, type):
        node_types.append(node_type.metatype)
    elif isinstance(node_type, Collection):
        node_types.extend(node_type)
    else:
        node_types.append(node_type)
    if expand_inheritance:
        node_types = expand_node_inheritance(node_types)
    return node_types

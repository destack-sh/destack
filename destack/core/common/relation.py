from typing import (
    TYPE_CHECKING,
    Optional,
    Union,
    assert_never,
    final,
)

from destack.registry import HANDLE_CLASS_BY_TYPE, NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE

from ..builtin import (
    UUID,
    HandleType,
    NodeType,
    ObjectKind,
    PropertyDeclaration,
    Struct,
    StructType,
    UInt8,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    from destack import (
        CustomPropertyDefinition,
        Entity,
        Handle,
        Node,
        PropertyDefinition,
        Type,
    )


type_ = type


@declare_struct(StructType.OBJECT_DEFINITION_REFERENCE, is_final=True)
@final
class ObjectDefinitionReference(Struct):
    """Reference to an object "type" (builtin, custom or trait)."""

    kind: ObjectKind = declare_property(101, is_repr=True)
    node_type: Optional[NodeType] = declare_property(102, is_repr=True)
    struct_type: Optional[StructType] = declare_property(103, is_repr=True)
    handle_type: Optional[HandleType] = declare_property(104, is_repr=True)
    definition: Optional["Entity"] = declare_property(106, is_repr=True)

    @classmethod
    def of(
        cls,
        definition: Union[
            "NodeType",
            "type[Node]",
            "type[Struct]",
            "type[Handle]",
        ],
    ) -> "ObjectDefinitionReference":
        from destack import Handle, Node

        if isinstance(definition, NodeType):
            return ObjectDefinitionReference(kind=ObjectKind.NODE, node_type=definition)
        elif isinstance(definition, StructType):
            return ObjectDefinitionReference(kind=ObjectKind.STRUCT, struct_type=definition)
        elif isinstance(definition, type):
            if issubclass(definition, Node):
                return ObjectDefinitionReference(
                    kind=ObjectKind.NODE, node_type=definition.metatype
                )
            elif issubclass(definition, Struct):
                return ObjectDefinitionReference(
                    kind=ObjectKind.STRUCT, struct_type=definition.metatype
                )
            elif issubclass(definition, Handle):
                return ObjectDefinitionReference(
                    kind=ObjectKind.HANDLE, handle_type=definition.metatype
                )
            else:
                assert_never(definition)
        else:
            assert_never(definition)


@declare_struct(
    StructType.PROPERTY_REFERENCE,
    is_final=True,
)
@final
class PropertyReference(Struct):
    """
    A reference to a builtin object's Property.
    """

    node_type: NodeType | None = declare_property(101, is_repr=True)
    struct_type: StructType | None = declare_property(103, is_repr=True)
    handle_type: HandleType | None = declare_property(104, is_repr=True)
    id: Optional[UInt8] = declare_property(
        105,
        is_repr=True,
        description="id of the builtin Property",
    )
    custom_property: Optional["CustomPropertyDefinition"] = declare_property(
        106,
        is_repr=True,
        description="custom Property of a custom Node or Struct",
    )

    def to_type(self) -> "Type":
        """Convert to a Type."""
        prop = self.resolve()
        return prop.type

    def to_ref(self) -> "PropertyReference":
        """Get this PropertyReference (for convenience)."""
        return self

    def resolve_maybe(self) -> "PropertyDefinition | CustomPropertyDefinition | None":
        """Resolves the property reference to a Property."""
        if (node_type := self.node_type) is not None:
            object_cls = NODE_CLASS_BY_TYPE[node_type]
        elif (struct_type := self.struct_type) is not None:
            object_cls = STRUCT_CLASS_BY_TYPE[struct_type]
        elif (handle_type := self.handle_type) is not None:
            object_cls = HANDLE_CLASS_BY_TYPE[handle_type]
        else:
            object_cls = Node
        assert self.id is not None, f"no id for {self!r}"
        prop = object_cls.__properties_by_id__.get(self.id)
        return prop.definition if prop is not None else None

    def resolve(self) -> "PropertyDefinition | CustomPropertyDefinition":
        """Resolves the property reference to a Property."""
        resolved = self.resolve_maybe()
        if resolved is None:
            raise ValueError(f"could not resolve {self!r}")
        return resolved

    @staticmethod
    def of(
        base: "PropertyDeclaration | PropertyDefinition | CustomPropertyDefinition",
    ) -> "PropertyReference":
        """Create a PropertyReference from a PropertyDeclaration, PropertyDefinition or CustomPropertyDefinition."""
        raise NotImplementedError


@declare_struct(StructType.NODE_REFERENCE, is_final=True)
@final
class NodeReference(Struct):
    """
    A reference to a Node in spacetime.
    """

    # identity
    type: NodeType = declare_property(
        100,
        is_repr=True,
        description="The type of the Node.",
    )
    id: UUID = declare_property(
        101,
        is_repr=True,
        description="The unique id of the Node.",
    )
    space_id: UUID = declare_property(
        102,
        is_repr=True,
        description="The id of the Space the Node belonged to.",
    )
    branch_id: UUID = declare_property(
        103,
        is_repr=True,
        description="The id of the Branch the Node belonged to (when it was referenced).",
    )
    snapshot_id: UUID = declare_property(
        104,
        is_repr=True,
        description="The id of the Snapshot the Node belonged to (when it was referenced).",
    )
    # epoch? (but then we would have to re-create NodeReferences every time the Node is updated)

    def to_ref(self) -> "NodeReference":
        """Get this NodeReference (for convenience)."""
        return self

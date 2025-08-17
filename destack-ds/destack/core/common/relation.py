from typing import (
    TYPE_CHECKING,
    Optional,
    final,
)

from destack.registry import NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE

from ..builtin import (
    UUID,
    EncoderStability,
    NodeType,
    PropertyDeclaration,
    ReferenceType,
    Struct,
    StructType,
    UInt8,
    UInt64,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    from destack import (
        CustomPropertyDefinition,
        Node,
        PropertyDefinition,
        Type,
    )


type_ = type


@declare_struct(
    StructType.PROPERTY_REFERENCE,
    is_final=True,
)
@final
class PropertyReference(Struct):
    """
    A reference to a builtin object's Property.
    """

    node_type: NodeType | None = declare_property(101, is_repr=True, tag=None)
    struct_type: StructType | None = declare_property(103, is_repr=True, tag=None)
    id: Optional[UInt8] = declare_property(
        105,
        is_repr=True,
        description="id of the builtin Property",
        tag=None,
    )
    custom_property: Optional["CustomPropertyDefinition"] = declare_property(
        106,
        is_repr=True,
        reference_type=ReferenceType.SPATIAL,
        description="custom Property of a custom Node or Struct",
        tag=None,
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


@declare_struct(
    StructType.NODE_IDENTITY_REFERENCE,
    is_immutable=True,
    is_interned=True,
    is_final=True,
    stability=EncoderStability.STATIC,
)
@final
class NodeIdentityReference(Struct):
    """
    A reference to a Node in an unknown space.
    """

    type: NodeType = declare_property(100, is_repr=True, tag=None)
    id: UUID = declare_property(101, is_repr=True, tag=None)


@declare_struct(
    StructType.NODE_SPATIAL_REFERENCE,
    is_immutable=True,
    is_interned=True,
    is_final=True,
    stability=EncoderStability.STATIC,
)
@final
class NodeSpatialReference(Struct):
    """
    A reference to a Node in space.
    """

    # identity
    type: NodeType = declare_property(
        100,
        is_repr=True,
        description="The type of the Node.",
        tag=None,
    )
    id: UUID = declare_property(
        101,
        is_repr=True,
        description="The unique id of the Node.",
        tag=None,
    )
    space_id: UUID = declare_property(
        102,
        is_repr=True,
        description="The id of the Space the Node belonged to.",
        tag=None,
    )


@declare_struct(
    StructType.NODE_TEMPORAL_REFERENCE,
    is_immutable=True,
    is_interned=True,
    is_final=True,
    stability=EncoderStability.STATIC,
)
@final
class NodeTemporalReference(Struct):
    """
    A reference to a Node in spacetime.
    """

    # identity
    type: NodeType = declare_property(
        100,
        is_repr=True,
        description="The type of the Node.",
        tag=None,
    )
    id: UUID = declare_property(
        101,
        is_repr=True,
        description="The unique id of the Node.",
        tag=None,
    )
    space_id: UUID = declare_property(
        102,
        is_repr=True,
        description="The id of the Space the Node belonged to.",
        tag=None,
    )
    branch_id: UUID = declare_property(
        103,
        is_repr=True,
        description="The id of the Branch the Node belonged to (when it was referenced).",
        tag=None,
    )
    snapshot_id: UUID = declare_property(
        104,
        is_repr=True,
        description="The id of the Snapshot the Node belonged to (when it was referenced).",
        tag=None,
    )
    epoch: UInt64 = declare_property(
        105,
        is_repr=True,
        description="The logical time the Node belonged to (when it was referenced).",
        tag=None,
    )

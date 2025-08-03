from typing import (
    TYPE_CHECKING,
    Optional,
    Union,
    assert_never,
    cast,
    final,
)

from destack.registry import HANDLE_CLASS_BY_TYPE, NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE

from ..builtin import (
    Enum,
    EnumType,
    HandleType,
    NodeType,
    Object,
    ObjectKind,
    PropertyDeclaration,
    Struct,
    StructFrozen,
    StructType,
    UInt8,
    builtin_enum,
    builtin_property,
    builtin_struct,
)
from ..utils.uuid import UUID

if TYPE_CHECKING:
    from destack import (
        CustomEventDefinition,
        CustomPropertyDefinition,
        CustomStructDefinition,
        Entity,
        Handle,
        Node,
        PropertyDefinition,
        Type,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type


@builtin_struct(StructType.OBJECT_DEFINITION_REFERENCE, frozen=True, is_final=True)
@final
class ObjectDefinitionReference(StructFrozen):
    """Reference to an object "type" (builtin, custom or trait)."""

    kind: ObjectKind = builtin_property(101, is_repr=True)
    node_type: Optional[NodeType] = builtin_property(102, is_repr=True)
    struct_type: Optional[StructType] = builtin_property(103, is_repr=True)
    handle_type: Optional[HandleType] = builtin_property(104, is_repr=True)
    definition: "Entity | None" = builtin_property(106, is_repr=True)
    if TYPE_CHECKING:
        definition_ptr: Optional["NodeReference"] = None

    @property
    def object_cls(self) -> type_[Object] | None:
        if self.kind == ObjectKind.NODE:
            assert self.node_type is not None, f"no node_type for {self!r}"
            return NODE_CLASS_BY_TYPE.get(self.node_type)
        elif self.kind == ObjectKind.STRUCT:
            assert self.struct_type is not None, f"no struct_type for {self!r}"
            return STRUCT_CLASS_BY_TYPE.get(self.struct_type)
        elif self.kind == ObjectKind.HANDLE:
            assert self.handle_type is not None, f"no handle_type for {self!r}"
            return HANDLE_CLASS_BY_TYPE.get(self.handle_type)
        else:
            assert_never(self.kind)

    def resolve_property_maybe(self, name: str | int) -> "PropertyDeclaration | None":
        """Resolve a Property in this definition."""
        object_cls = self.object_cls
        if object_cls is None:
            raise ValueError(f"could not resolve {self!r}")
        if isinstance(name, str):
            return object_cls.__properties_by_alias__.get(name)
        elif isinstance(name, int):
            return object_cls.__properties_by_id__.get(name)
        else:
            assert_never(name)

    def resolve_property(self, name: str) -> "PropertyDeclaration":
        """Resolve a Property in this definition (error if not found)."""
        resolved = self.resolve_property_maybe(name)
        if resolved is None:
            raise LookupError(f"could not find property {name!r} in {self!r}")
        return resolved

    @classmethod
    def of(
        cls,
        definition: Union[
            "NodeType",
            "type[Node]",
            "type[Struct]",
            "type[Handle]",
            "CustomEventDefinition",
            "CustomStructDefinition",
        ],
    ) -> "ObjectDefinitionReference":
        from destack import CustomEventDefinition, CustomStructDefinition, Handle, Node

        if isinstance(definition, NodeType):
            return ObjectDefinitionReference(kind=ObjectKind.NODE, node_type=definition)
        elif isinstance(definition, StructType):
            return ObjectDefinitionReference(kind=ObjectKind.STRUCT, struct_type=definition)
        elif isinstance(definition, type):
            if issubclass(definition, Node):
                return ObjectDefinitionReference(
                    kind=ObjectKind.NODE, node_type=cast(NodeType, definition.metatype)
                )
            elif issubclass(definition, Struct):
                return ObjectDefinitionReference(
                    kind=ObjectKind.STRUCT, struct_type=definition.metatype
                )
            elif issubclass(definition, Handle):
                return ObjectDefinitionReference(
                    kind=ObjectKind.HANDLE, handle_type=cast(HandleType, definition.metatype)
                )
            else:
                assert_never(definition)
        elif isinstance(definition, (CustomEventDefinition, CustomStructDefinition)):
            raise NotImplementedError(f"unexpected object definition reference: {definition!r}")
        else:
            assert_never(definition)


@builtin_enum(EnumType.PROPERTY_REFERENCE_TYPE)
class PropertyReferenceType(Enum):
    """The type of a property reference."""

    BUILTIN = 1
    CUSTOM = 2


@builtin_struct(
    StructType.PROPERTY_REFERENCE,
    frozen=True,
    is_final=True,
)
@final
class PropertyReference(StructFrozen):
    """
    A reference to a builtin object's Property.
    """

    type: PropertyReferenceType = builtin_property(100, is_repr=True)
    node_type: NodeType | None = builtin_property(101, is_repr=True)
    struct_type: StructType | None = builtin_property(103, is_repr=True)
    handle_type: HandleType | None = builtin_property(104, is_repr=True)
    id: UInt8 | None = builtin_property(
        105,
        is_repr=True,
        description="id of the builtin Property",
    )
    custom_property: "CustomPropertyDefinition | None" = builtin_property(
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
        if self.type == PropertyReferenceType.BUILTIN:
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
        elif self.type == PropertyReferenceType.CUSTOM:
            prop = self.custom_property
            return prop
        else:
            return None

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
        from destack import CustomPropertyDefinition

        if isinstance(base, PropertyDeclaration):
            return base.to_ref()
        elif isinstance(base, PropertyDefinition):
            raise ValueError(f"cannot convert {base!r} to a PropertyReference")
        elif isinstance(base, CustomPropertyDefinition):
            assert base.parent_ptr is not None, f"no parent for {base!r}"
            return PropertyReference(
                type=PropertyReferenceType.CUSTOM,
                node_type=base.parent_ptr.type,
                custom_property=base,
            )
        else:
            assert_never(base)


@builtin_struct(StructType.NODE_REFERENCE, frozen=True, is_final=True)
@final
class NodeReference(StructFrozen):
    """
    A reference to a Node in spacetime.
    """

    # identity
    type: NodeType = builtin_property(
        100,
        is_repr=True,
        description="The type of the Node.",
    )
    id: UUID = builtin_property(
        101,
        is_repr=True,
        description="The unique id of the Node.",
    )
    space_id: UUID = builtin_property(
        102,
        is_repr=True,
        description="The id of the Space the Node belonged to.",
    )
    definition_id: Optional[UUID] = builtin_property(
        103,
        is_repr=True,
        description="The id of the Node definition.",
    )
    branch_id: UUID = builtin_property(
        104,
        is_repr=True,
        description="The id of the Branch the Node belonged to (when it was referenced).",
    )
    snapshot_id: UUID = builtin_property(
        105,
        is_repr=True,
        description="The id of the Snapshot the Node belonged to (when it was referenced).",
    )
    # epoch? (but then we would have to re-create NodeReferences every time the Node is updated)

    def to_ref(self) -> "NodeReference":
        """Get this NodeReference (for convenience)."""
        return self

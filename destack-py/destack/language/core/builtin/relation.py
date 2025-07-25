from typing import (
    TYPE_CHECKING,
    Optional,
    Union,
    assert_never,
    cast,
)

from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    TRAIT_CLASS_BY_TYPE,
)
from destack.utils.uuid import UUID

from .builtin import (
    EnumType,
    NodeType,
    StructType,
    TraitType,
)
from .common import GraphKey, UInt8
from .enum import Enum, builtin_enum
from .object import BuiltinObject
from .property import PropertyDeclaration, builtin_property
from .struct import Struct, StructFrozen, builtin_struct
from .trait import Trait

if TYPE_CHECKING:
    from destack.language import (
        CustomEvent,
        CustomProperty,
        CustomStruct,
        Entity,
        Node,
        PropertyDefinition,
        Type,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type


@builtin_enum(EnumType.NODE_DEFINITION_TYPE)
class NodeDefinitionType(Enum):
    BUILTIN = 1
    CUSTOM = 2


@builtin_struct(StructType.NODE_DEFINITION_REFERENCE, frozen=True)
class NodeDefinitionReference(StructFrozen):
    """Reference to a Node definition."""

    type: NodeDefinitionType = builtin_property(100, is_repr=True)
    node_type: NodeType = builtin_property(101, is_repr=True)
    definition: Optional["Entity"] = builtin_property(105, is_repr=True)
    if TYPE_CHECKING:
        definition_ptr: Optional["NodeReference"] = None

    @property
    def is_multi(self) -> bool:
        """Whether this definition references multiple Node definitions"""
        if self.type == NodeDefinitionType.BUILTIN:
            assert self.node_type is not None, f"no node_type for {self!r}"
            node_cls = NODE_CLASS_BY_TYPE[self.node_type]
            return bool(node_cls.__inherited_by__)
        elif self.type == NodeDefinitionType.CUSTOM:
            raise NotImplementedError(f"unexpected node definition reference: {self!r}")
        else:
            assert_never(self.type)

    @property
    def object_cls(self) -> type_[BuiltinObject] | None:
        return NODE_CLASS_BY_TYPE.get(self.node_type)

    def resolve_property_maybe(self, key: str | int) -> "PropertyDeclaration | None":
        """Resolve a Property in this definition."""
        object_cls = self.object_cls
        if object_cls is None:
            raise ValueError(f"could not resolve {self!r}")
        if isinstance(key, str):
            return object_cls.__properties_by_alias__.get(key)
        elif isinstance(key, int):
            return object_cls.__properties_by_id__.get(key)
        else:
            assert_never(key)

    def resolve_property(self, name: str) -> "PropertyDeclaration":
        """Resolve a Property in this definition (error if not found)."""
        resolved = self.resolve_property_maybe(name)
        if resolved is None:
            raise LookupError(f"could not find property {name!r} in {self!r}")
        return resolved

    @classmethod
    def of(cls, base: "NodeType | type[Node] | NodeReference") -> "NodeDefinitionReference":
        if isinstance(base, NodeType):
            return NodeDefinitionReference(type=NodeDefinitionType.BUILTIN, node_type=base)
        elif isinstance(base, type):
            return NodeDefinitionReference(type=NodeDefinitionType.BUILTIN, node_type=base.metatype)
        elif isinstance(base, NodeReference):
            if base.definition_id is not None:
                node_cls = NODE_CLASS_BY_TYPE[base.type]
                if NodeType.ENTITY in node_cls.__inherits__:
                    definition_node_type = base.type  # same as instance
                elif NodeType.EVENT in node_cls.__inherits__:
                    definition_node_type = NodeType.CUSTOM_EVENT
                else:
                    raise ValueError(f"unexpected node reference: {base!r}")
                definition_ptr = NodeReference(
                    type=definition_node_type, id=base.definition_id, space_id=base.space_id
                )
                return NodeDefinitionReference(
                    type=NodeDefinitionType.CUSTOM,
                    node_type=base.type,
                    definition_ptr=definition_ptr,
                )
            else:
                return NodeDefinitionReference(type=NodeDefinitionType.BUILTIN, node_type=base.type)
        else:
            assert_never(base)


@builtin_enum(EnumType.OBJECT_DEFINITION_TYPE)
class ObjectDefinitionType(Enum):
    BUILTIN_NODE = 1
    CUSTOM_NODE = 2
    BUILTIN_TRAIT = 3
    CUSTOM_TRAIT = 4
    BUILTIN_STRUCT = 5
    CUSTOM_STRUCT = 6
    # BUILTIN_ENUM, CUSTOM_ENUM?


@builtin_struct(StructType.OBJECT_DEFINITION_REFERENCE, frozen=True)
class ObjectDefinitionReference(StructFrozen):
    """Reference to an object "type" (builtin, custom or trait)."""

    type: ObjectDefinitionType = builtin_property(100, is_repr=True)
    node_type: Optional[NodeType] = builtin_property(101, is_repr=True)
    trait_type: Optional[TraitType] = builtin_property(102, is_repr=True)
    struct_type: Optional[StructType] = builtin_property(103, is_repr=True)
    custom_definition: Optional["Entity"] = builtin_property(105, is_repr=True)
    if TYPE_CHECKING:
        custom_definition_ptr: Optional["NodeReference"] = None

    def to_ref(self) -> "ObjectDefinitionReference":
        """Get this ObjectDefinitionReference (for convenience)."""
        return self

    @property
    def object_cls(self) -> type_[BuiltinObject] | None:
        if self.type == ObjectDefinitionType.BUILTIN_NODE:
            assert self.node_type is not None, f"no node_type for {self!r}"
            return NODE_CLASS_BY_TYPE.get(self.node_type)
        elif self.type == ObjectDefinitionType.CUSTOM_NODE:
            raise NotImplementedError(f"unexpected object definition reference: {self!r}")
        elif self.type == ObjectDefinitionType.BUILTIN_TRAIT:
            assert self.trait_type is not None, f"no trait_type for {self!r}"
            return TRAIT_CLASS_BY_TYPE.get(self.trait_type)
        elif self.type == ObjectDefinitionType.BUILTIN_STRUCT:
            assert self.struct_type is not None, f"no struct_type for {self!r}"
            return STRUCT_CLASS_BY_TYPE.get(self.struct_type)
        elif (
            self.type == ObjectDefinitionType.CUSTOM_STRUCT
            or self.type == ObjectDefinitionType.CUSTOM_TRAIT
        ):
            raise NotImplementedError(f"unexpected object definition reference: {self!r}")
        else:
            assert_never(self.type)

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
        base: Union[
            "NodeType",
            "type[Node]",
            "type[Trait]",
            "type[Struct]",
            "CustomEvent",
            "CustomStruct",
        ],
    ) -> "ObjectDefinitionReference":
        from ..common.struct import CustomStruct
        from .event import CustomEvent
        from .node import Node

        if isinstance(base, NodeType):
            return ObjectDefinitionReference(type=ObjectDefinitionType.BUILTIN_NODE, node_type=base)
        elif isinstance(base, StructType):
            return ObjectDefinitionReference(
                type=ObjectDefinitionType.BUILTIN_STRUCT, struct_type=base
            )
        elif isinstance(base, type):
            if issubclass(base, Node):
                return ObjectDefinitionReference(
                    type=ObjectDefinitionType.BUILTIN_NODE,
                    node_type=cast(NodeType, base.metatype),
                )
            elif issubclass(base, Trait):
                return ObjectDefinitionReference(
                    type=ObjectDefinitionType.BUILTIN_TRAIT,
                    trait_type=cast(TraitType, base.metatype),
                )
            elif issubclass(base, Struct):
                return ObjectDefinitionReference(
                    type=ObjectDefinitionType.BUILTIN_STRUCT,
                    struct_type=base.metatype,
                )
            else:
                raise ValueError(f"invalid object reference type: {base!r}")
        elif isinstance(base, (CustomEvent, CustomStruct)):
            raise NotImplementedError(f"unexpected object definition reference: {base!r}")
        else:
            assert_never(base)


@builtin_enum(EnumType.STRUCT_DEFINITION_TYPE)
class StructDefinitionType(Enum):
    BUILTIN_STRUCT = 1
    CUSTOM_STRUCT = 2
    BUILTIN_ENUM = 3
    CUSTOM_ENUM = 4


@builtin_struct(StructType.STRUCT_DEFINITION_REFERENCE, frozen=True)
class StructDefinitionReference(StructFrozen):
    """Reference to a Struct definition (builtin, custom or by trait)."""

    type: StructDefinitionType = builtin_property(100, is_repr=True)
    struct_type: Optional[StructType] = builtin_property(101, is_repr=True)
    definition: "CustomStruct" = builtin_property(105, is_repr=True)
    if TYPE_CHECKING:
        definition_ptr: Optional["NodeReference"] = None

    def to_ref(self) -> "StructDefinitionReference":
        """Get this StructDefinitionReference (for convenience)."""
        return self


@builtin_enum(EnumType.PROPERTY_REFERENCE_TYPE)
class PropertyReferenceType(Enum):
    """The type of a property reference."""

    BUILTIN = 1
    CUSTOM = 2


@builtin_struct(StructType.PROPERTY_REFERENCE, frozen=True)
class PropertyReference(StructFrozen):
    """
    A reference to a builtin object's Property.
    """

    type: PropertyReferenceType = builtin_property(100, is_repr=True)
    node_type: NodeType | None = builtin_property(101, is_repr=True)
    trait_type: TraitType | None = builtin_property(102, is_repr=True)
    struct_type: StructType | None = builtin_property(103, is_repr=True)
    id: UInt8 | None = builtin_property(
        105,
        is_repr=True,
        description="id of the builtin Property",
    )
    custom_property: "CustomProperty | None" = builtin_property(
        106,
        is_repr=True,
        description="custom Property of a custom Node or Struct",
    )

    def to_type(self) -> "Type":
        """Convert to a Type."""
        prop = self.resolve()
        return prop.to_type()

    def to_ref(self) -> "PropertyReference":
        """Get this PropertyReference (for convenience)."""
        return self

    def resolve_maybe(self) -> "PropertyDefinition | CustomProperty | None":
        """Resolves the property reference to a Property."""
        if self.type == PropertyReferenceType.BUILTIN:
            if (node_type := self.node_type) is not None:
                object_cls = NODE_CLASS_BY_TYPE[node_type]
            elif (struct_type := self.struct_type) is not None:
                object_cls = STRUCT_CLASS_BY_TYPE[struct_type]
            elif (trait_type := self.trait_type) is not None:
                object_cls = TRAIT_CLASS_BY_TYPE[trait_type]
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

    def resolve(self) -> "PropertyDefinition | CustomProperty":
        """Resolves the property reference to a Property."""
        resolved = self.resolve_maybe()
        if resolved is None:
            raise ValueError(f"could not resolve {self!r}")
        return resolved

    @staticmethod
    def of(
        base: "PropertyDeclaration | PropertyDefinition | CustomProperty",
    ) -> "PropertyReference":
        from destack.language.core import CustomProperty

        if isinstance(base, PropertyDeclaration):
            return base.to_ref()
        elif isinstance(base, PropertyDefinition):
            raise ValueError(f"cannot convert {base!r} to a PropertyReference")
        elif isinstance(base, CustomProperty):
            assert base.parent_ptr is not None, f"no parent for {base!r}"
            return PropertyReference(
                type=PropertyReferenceType.CUSTOM,
                node_type=base.parent_ptr.type,
                custom_property=base,
            )
        else:
            assert_never(base)


@builtin_struct(StructType.NODE_REFERENCE, frozen=True)
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
    store_key: Optional[GraphKey] = builtin_property(
        110,
        is_repr=True,
        description="The type of the Store the Node came from.",
    )

    def to_ref(self) -> "NodeReference":
        """Get this NodeReference (for convenience)."""
        return self

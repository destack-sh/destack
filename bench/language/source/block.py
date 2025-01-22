from functools import cached_property
from typing import (
    TYPE_CHECKING,
    Any,
    Literal,
    Optional,
    Union,
    assert_never,
    cast,
)

from bench.language.core import (
    NAME_CONSTRAINT,
    BenchError,
    BlockType,
    FieldType,
    LocalNodeList,
    NodeSubtypeStub,
    NodeType,
    RemoteNodeList,
    RunType,
    SourceNode,
    StructType,
    TypeBase,
    TypeKind,
    node_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
    subnode_,
)
from bench.pb2 import BlockData
from bench.pb2.lang_pb2 import RecordData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Field,
        Icon,
        Package,
        Pipe,
        Record,
        RunOptions,
        Text,
        Type,
        View,
    )

# pyright: reportIncompatibleVariableOverride=false

_type = type


@node_(NodeType.BLOCK, passthrough_get=("fields",), has_subtypes=True)
class Block(SourceNode[BlockData]):
    """A building block with logic, types, UI, state, auth, AI, ..."""

    parent: Union["Block", "Package", None] = p_node_parent(4, NodeType.BLOCK, NodeType.PACKAGE)

    # content
    type: BlockType = p_system(30, description="The type of block. Cannot be changed.")
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    icon: Optional["Icon"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.ICON
    )
    text: Optional["Text"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.TEXT
    )

    variables_packed: Any = p_value_packed(40)
    variables: Any = p_value_runtime(
        packed=40, type=FieldType.VARIABLE, typ=lambda self: cast("Block", self).variable_type
    )
    # policies, roles, identity, ...?

    # flags
    # is_builtin, is_owned, is_test? (for testing)

    blocks: LocalNodeList["Block"] = p_node_children(NodeType.BLOCK)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    actions: LocalNodeList["Action"] = p_node_children(NodeType.ACTION)
    pipes: LocalNodeList["Pipe"] = p_node_children(NodeType.PIPE)
    views: LocalNodeList["View"] = p_node_children(NodeType.VIEW)
    records: RemoteNodeList["Record", RecordData] = p_node_children(
        NodeType.RECORD, list=RemoteNodeList
    )

    def __content_str__(self):
        return ""

    @property
    def is_page(self) -> bool:
        return self.type == BlockType.PAGE

    @property
    def is_type(self) -> bool:
        return self.type.is_type

    @property
    def is_runnable(self) -> bool:
        return self.type.is_runnable

    @property
    def run_type(self) -> "RunType | None":
        if self.type == BlockType.FLOW:
            return RunType.FLOW
        else:
            return None

    @property
    def has_function_fields(self) -> bool:
        return any(f.type == FieldType.INPUT or f.type == FieldType.OUTPUT for f in self.fields)

    def __call__(self, *args, **kwargs) -> Any:
        if self.type.is_type:
            typ = self.to_type_maybe()
            if typ is None:
                raise ValueError(f"{self!r} does not have an implicit type")
            return typ(*args, **kwargs)
        else:
            raise BenchError(f"{self!r} is not callable")

    def to_type_maybe(
        self,
        *,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "TypeBase | None":
        """Get a type represented by this Block (if any)"""
        from bench.language.core import Type

        if of == "instance":
            if self.type == BlockType.CHOICE:
                instance_type = NodeType.FIELD
            elif self.type == BlockType.MESSAGE:
                instance_type = NodeType.MESSAGE
            elif self.type == BlockType.DATABASE:
                instance_type = NodeType.RECORD
            elif self.type.is_runnable:
                instance_type = NodeType.RUN
            else:
                return None  # no 'instance' type
            return Type(kind=TypeKind.BASED_NODE, base_type=self, bench_type=instance_type)
        elif of == "value":
            field_types = field_types or [FieldType.MEMBER]
            return Type(
                kind=TypeKind.CUSTOM_OBJECT,
                base_type=self,
                base_field_types=field_types,
                property_field_types=field_types,
            )
        else:
            assert_never(of)

    def to_type(
        self,
        *,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "TypeBase":
        typ = self.to_type_maybe(of=of, field_types=field_types)
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @cached_property  # :CachedTypeInfo
    def variable_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.VARIABLE])

    @cached_property  # :CachedTypeInfo
    def input_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.INPUT])

    @cached_property  # :CachedTypeInfo
    def output_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.OUTPUT])

    @staticmethod
    def new[BlockT: "Block" = "Block"](
        typ: BlockType | _type[BlockT] | NodeSubtypeStub[BlockT], name: str, **kwargs
    ) -> "BlockT":
        if isinstance(typ, type):
            typ = Block.__subtype_by_subclass__[typ]  # type: ignore
        elif isinstance(typ, NodeSubtypeStub):
            typ = cast(BlockType, typ._node_subtype)
        return Block(type=typ, name=name, **kwargs)  # type: ignore


@subnode_(BlockType.VARIABLE, passthrough_get=("value",), passthrough_set=("value",))
class VariableBlock(Block):
    value_type: Optional["Type"] = p_regular(100, default=None, struct=StructType.TYPE)
    value_packed: Any = p_value_packed(101)
    value: Any = p_value_runtime(
        101, type=FieldType.MEMBER, typ=lambda self: cast("VariableBlock", self).value_type
    )


@subnode_(BlockType.TEXT)
class TextBlock(Block):
    pass


@subnode_(BlockType.FLOW)
class FlowBlock(Block):
    run_options: Optional["RunOptions"] = p_regular(
        100, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )


@subnode_(BlockType.DATABASE)
class DatabaseBlock(Block):
    pass

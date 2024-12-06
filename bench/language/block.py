from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    Literal,
    Optional,
    Type,
    Union,
    assert_never,
    cast,
)

from bench.language.action import ActionBase
from bench.language.const import (
    BenchError,
    BlockType,
    FieldType,
    NodeType,
    ObjectKind,
    RunType,
    StructType,
    TypeKind,
)
from bench.language.field import TypeBase
from bench.language.list import LocalNodeList, RemoteNodeList
from bench.language.node import NodeSubtypeStub, SourceNode, local_node_, node_subtype_
from bench.language.property import (
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.query import Query
from bench.language.validation import NAME_CONSTRAINT, ValidationHandler, constraint
from bench.proto.wire import BlockData
from bench.proto.wire.lang_pb2 import RecordData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import (
        Badge,
        Field,
        Icon,
        NodeReference,
        Package,
        Pipe,
        Policy,
        Property,
        Record,
        RunOptions,
        Step,
        Text,
        Trigger,
        TypeInfo,
        View,
    )

# pyright: reportIncompatibleVariableOverride=false

# NOTE :UX: auto-generate node names in code just like in the UI (if unset -> block7, etc.)
#  (Maybe postpone name validation if detached so we can leave it unset?,
#   auto-naming currently only works in NodeList where we know the siblings).
#  see :AutoNaming


@local_node_(NodeType.BLOCK, passthrough_get=("fields",))
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
        packed=40, kind=ObjectKind.VARIABLE, typ=lambda self: cast("Block", self).variable_type
    )

    run_options: Optional["RunOptions"] = p_regular(
        41, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )
    policies: list["Policy"] = p_regular(42, require=False, array=True, struct=StructType.POLICY)
    delegated_policies: list["Policy"] = p_regular(
        43, require=False, array=True, struct=StructType.POLICY
    )
    roles: list["Block"] = p_regular(
        44,
        require=False,
        array=True,
        references=NodeType.BLOCK,
        constraint=constraint(node_subtypes=[BlockType.ROLE]),
    )
    identity: Optional["Block"] = p_regular(
        45,
        require=False,
        references=NodeType.BLOCK,
        constraint=constraint(node_subtypes=[BlockType.IDENTITY]),
    )
    if TYPE_CHECKING:
        roles_ptr: tuple["NodeReference", ...] = ()
        identity_ptr: Optional["NodeReference"] = None

    # flags
    # is_builtin, is_owned, is_test? (for testing)

    blocks: LocalNodeList["Block"] = p_node_children(NodeType.BLOCK)
    badges: LocalNodeList["Badge"] = p_node_children(NodeType.BADGE)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    steps: LocalNodeList["Step"] = p_node_children(NodeType.STEP)
    pipes: LocalNodeList["Pipe"] = p_node_children(NodeType.PIPE)
    views: LocalNodeList["View"] = p_node_children(NodeType.VIEW)
    triggers: LocalNodeList["Trigger"] = p_node_children(NodeType.TRIGGER)
    records: RemoteNodeList["Record", RecordData] = p_node_children(
        NodeType.RECORD, list=RemoteNodeList
    )

    def _validate_component(
        self, properties: Collection["Property"], invalid: "ValidationHandler"
    ) -> None:
        pass

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
        if self.type == BlockType.ACTION:
            return RunType.ACTION
        elif self.type == BlockType.FLOW:
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

    # @cachetools.cached({})  # :CachedTypeInfo
    def to_type_maybe(
        self, *, of: Literal["instance", "value"] = "instance", field_type: FieldType | None = None
    ) -> "TypeBase | None":
        """Get a type represented by this Block (if any)"""
        from bench.language.field import TypeInfo

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
            return TypeInfo(kind=TypeKind.BASED_NODE, base_type=self, bench_type=instance_type)
        elif of == "value":
            return TypeInfo(
                kind=TypeKind.CUSTOM_OBJECT,
                base_type=self,
                base_field_type=field_type or FieldType.MEMBER,
            )
        else:
            assert_never(of)

    def to_type(
        self, *, of: Literal["instance", "value"] = "instance", field_type: FieldType | None = None
    ) -> "TypeBase":
        typ = self.to_type_maybe(of=of, field_type=field_type)
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @property
    def variable_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_type=FieldType.VARIABLE)

    @property
    def input_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_type=FieldType.INPUT)

    @property
    def output_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_type=FieldType.OUTPUT)

    @staticmethod
    def new[BlockT: "Block" = "Block"](
        typ: BlockType | Type[BlockT] | NodeSubtypeStub[BlockT], name: str, **kwargs
    ) -> "BlockT":
        if isinstance(typ, type):
            typ = Block.__subtype_by_subclass__[typ]  # type: ignore
        elif isinstance(typ, NodeSubtypeStub):
            typ = cast(BlockType, typ._node_subtype)
        return Block(type=typ, name=name, **kwargs)  # type: ignore


@node_subtype_(BlockType.VALUE, passthrough_get=("value",), passthrough_set=("value",))
class ValueBlock(Block):
    value_type: Optional["TypeInfo"] = p_regular(100, default=None, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(101)
    value: Any = p_value_runtime(
        101, kind=ObjectKind.MEMBER, typ=lambda self: cast("ValueBlock", self).value_type
    )


@node_subtype_(BlockType.ACTION)
class ActionBlock(Block, ActionBase):
    # ...ActionBase[100-129]
    pass


@node_subtype_(BlockType.TEXT)
class TextBlock(Block):
    pass


@node_subtype_(BlockType.FLOW)
class FlowBlock(Block):
    pass


@node_subtype_(BlockType.DATABASE)
class DatabaseBlock(Block):
    query: Optional["Query"] = p_regular(100, require=False, array=False, references=NodeType.QUERY)


@node_subtype_(BlockType.QUERY)
class QueryBlock(Block):
    query: Optional["Query"] = p_regular(100, require=False, array=False, references=NodeType.QUERY)

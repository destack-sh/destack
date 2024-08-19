from typing import TYPE_CHECKING, Any, Collection, Optional, Union, cast, final

from bench.language.const import (
    BenchError,
    BlockType,
    FieldZone,
    NodeType,
    StructType,
    TypeKind,
)
from bench.language.field import TypeInfoBase
from bench.language.graph import NodeList
from bench.language.node import SourceNode, local_node_
from bench.language.property import (
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.validation import NAME_CONSTRAINT, ValidationHandler, constraint
from bench.proto.wire import BlockData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import (
        Badge,
        Code,
        Field,
        Icon,
        NodeReference,
        Package,
        Policy,
        Property,
        RunKind,
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

# NOTE :Architecture: blocks will probably eventually also need type-specific info :NodeInheritance


@local_node_(NodeType.BLOCK, passthrough=("value", "fields"))
class Block(SourceNode[BlockData]):
    """A building block with logic, types, UI, state, auth, AI, ..."""

    parent: Union["Block", "Package", None] = p_node_parent(4, NodeType.BLOCK, NodeType.PACKAGE)

    # content
    type: BlockType = p_internal(30)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    bases: list["Block"] = p_regular(35, require=False, array=True, references=NodeType.BLOCK)
    text: Optional["Text"] = p_regular(
        36, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(
        37, default=None, require=False, array=False, struct=StructType.ICON
    )
    value_type: Optional["TypeInfo"] = p_regular(39, default=None, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(40)
    value: Any = p_value_runtime(40, typ=lambda self: cast("Block", self).value_type)
    code: Optional["Code"] = p_regular(
        42, default=None, require=False, array=False, struct=StructType.CODE
    )
    run_options: Optional["RunOptions"] = p_regular(
        43, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )
    roles: list["Block"] = p_regular(
        46,
        require=False,
        array=True,
        references=NodeType.BLOCK,
        constraint=constraint(block_type=BlockType.ROLE),
    )
    identity: Optional["Block"] = p_regular(
        47,
        require=False,
        references=NodeType.BLOCK,
        constraint=constraint(block_type=BlockType.IDENTITY),
    )
    policies: list["Policy"] = p_regular(48, require=False, array=True, struct=StructType.POLICY)
    delegated_policies: list["Policy"] = p_regular(
        49, require=False, array=True, struct=StructType.POLICY
    )
    if TYPE_CHECKING:
        roles_ptr: tuple["NodeReference", ...] = ()
        identity_ptr: Optional["NodeReference"] = None

    # flags
    is_builtin: bool = p_system(
        60, default=False, description="Whether this is an intrinsic provided by the system."
    )
    is_owned: bool = p_system(
        61, default=False, description="Whether this is owned by its creator."
    )
    is_paused: bool = p_regular(
        64, default=False, description="Whether to pause any runtime activity within this block."
    )
    # is_test? (for testing)

    blocks: NodeList["Block"] = p_node_children(NodeType.BLOCK)
    badges: NodeList["Badge"] = p_node_children(NodeType.BADGE)
    fields: NodeList["Field"] = p_node_children(NodeType.FIELD)
    steps: NodeList["Step"] = p_node_children(NodeType.STEP)
    triggers: NodeList["Trigger"] = p_node_children(NodeType.TRIGGER)
    views: NodeList["View"] = p_node_children(NodeType.VIEW)

    def _validate_component(
        self, properties: Collection["Property"], invalid: "ValidationHandler"
    ) -> None:
        pass

    def __content_str__(self):
        return ""  # implemented by dynamic components

    @final
    def __repr__(self):  # type: ignore we want to override the default repr
        return f"<{self.type.bench_name}Block {self}>"

    @property
    def is_page(self) -> bool:
        return self.type.is_page

    @property
    def is_type(self) -> bool:
        return self.type.is_type

    @property
    def is_runnable(self) -> bool:
        return self.type.is_runnable

    @property
    def run_kind(self) -> "RunKind | None":
        from bench.language.run import RunKind

        if self.type == BlockType.CODE:
            return RunKind.CODE
        elif self.type == BlockType.TEXT:
            return RunKind.TEXT
        elif self.type == BlockType.FLOW:
            return RunKind.FLOW
        elif self.type.is_runnable:
            raise RuntimeError(f"unexpected type {self!r}")
        else:
            return None

    @property
    def has_function_fields(self) -> bool:
        return any(f.zone == FieldZone.INPUT or f.zone == FieldZone.OUTPUT for f in self.fields)

    def __call__(self, *args, **kwargs) -> Any:
        if self.type.is_runnable:
            assert not (args and kwargs), f"{self!r} does not accept both args and kwargs"
            return self.active_session.runtime.run(self, inputs=args or kwargs)
        elif self.type in (BlockType.CLASS, BlockType.CHOICE, BlockType.SIGNAL):
            typ = self.to_type()
            if typ is None:
                raise ValueError(f"{self!r} does not have an implicit type")
            return typ(*args, **kwargs)
        else:
            raise BenchError(f"{self!r} is not callable")

    def to_type(self, *, as_object: bool = False, zone: FieldZone | None = None) -> "TypeInfoBase":
        """Get a type represented by this Block (if any)"""
        from bench.language.field import TypeInfo

        if self.type == BlockType.CLASS:
            # NOTE: we turn Class Blocks into Alias Types here for correctness, but that means
            #  we have to resolve them again (unnecessarily) before instantiating.
            typ = TypeInfo(kind=TypeKind.ALIAS, base_type=self)
        elif self.type == BlockType.CHOICE:
            typ = TypeInfo(
                kind=TypeKind.BASED_NODE,
                base_type=self,
                bench_type=NodeType.FIELD,
                base_field_zone=FieldZone.OPTION,
            )
        elif self.type == BlockType.SIGNAL:
            if not as_object:
                typ = TypeInfo(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.SIGNAL)
            else:
                typ = TypeInfo(
                    kind=TypeKind.OBJECT, base_type=self, base_field_zone=zone or FieldZone.MEMBER
                )
        elif self.type == BlockType.VALUE:
            assert self.value_type is not None, f"{self!r} has no builtin base"
            return self.value_type._to_resolved()
        elif self.type == BlockType.DATABASE:
            if not as_object:
                typ = TypeInfo(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RECORD)
            else:
                typ = TypeInfo(
                    kind=TypeKind.OBJECT, base_type=self, base_field_zone=zone or FieldZone.MEMBER
                )
        elif self.type.is_runnable:
            if not as_object:
                typ = TypeInfo(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RUN)
            else:
                typ = TypeInfo(kind=TypeKind.OBJECT, base_type=self, base_field_zone=zone)
        else:
            raise ValueError(f"{self!r} has no type")
        typ._resolve_type()  # pre-resolve
        return typ

    @property
    def input_type(self) -> "TypeInfoBase":
        return self.to_type(as_object=True, zone=FieldZone.INPUT)

    @property
    def output_type(self) -> "TypeInfoBase":
        return self.to_type(as_object=True, zone=FieldZone.OUTPUT)

    @staticmethod
    def new(typ: BlockType, name: str, **kwargs) -> "Block":
        return Block(type=typ, name=name, **kwargs)

    @staticmethod
    def new_code(name: str, code: "str | Code", **kwargs) -> "Block":
        if isinstance(code, str):
            from bench.language.code import Code

            code = Code.from_string(code)
        return Block.new(BlockType.CODE, name, code=code, **kwargs)

    @staticmethod
    def new_text(name: str, text: "str | Text", **kwargs) -> "Block":
        if isinstance(text, str):
            from bench.language.text import Text

            text = Text.plain(text)
        return Block.new(BlockType.TEXT, name, text=text, **kwargs)

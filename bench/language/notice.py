import functools
from typing import TYPE_CHECKING, Any, Callable, Collection, Optional, TypedDict, Union
from uuid import UUID

from bench.language.const import BenchError, EnumType, NodeType, NoticeKind, StructType, enum_
from bench.language.node import LINK_TARGET_NODE_TYPES, Node, Property, node
from bench.language.property import p_node_parent, p_regular
from bench.language.text import Text
from bench.proto.wire import NoticeData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Field, Path, Step, Struct, View

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.NOTICE_TYPE)
class NoticeType(IdEnum):
    """Built-in notice types."""

    # errors
    MISSING_REFERENCE = 1
    CIRCULAR_BASE = 2
    MISMATCHED_BASE = 3

    # warnings
    AMBIGUOUS_NAME = 100

    # information
    ...

    # hints
    ...

    @property
    def kind(self) -> NoticeKind:
        if self.id < 100:
            return NoticeKind.ERROR
        elif self.id < 200:
            return NoticeKind.WARNING
        elif self.id < 300:
            return NoticeKind.INFO
        else:
            return NoticeKind.HINT


NOTICE_TYPES = tuple(NoticeType)


class NoticeError(BenchError, ValueError):
    def __init__(self, notice: "Notice", cause: Exception | None = None):
        super().__init__(repr(notice))
        self.notice = notice
        self.cause = cause


NoticeParent = Union["Block", "Field", "Step", "View"]
NOTICE_PARENT_TYPES: tuple[NodeType, ...] = (
    NodeType.BLOCK,
    NodeType.FIELD,
    NodeType.STEP,
    NodeType.VIEW,
)


@node(NodeType.NOTICE)
class Notice(Node[NoticeData]):
    """
    An informational or diagnostic note about something in the Bench source.
    Notices are generally 'sticky' until their underlying cause is resolved.
    """

    parent: NoticeParent = p_node_parent(4, *NOTICE_PARENT_TYPES)
    kind: NoticeKind = p_regular(30, default=None)
    type: NoticeType = p_regular(31)
    # -> builtin_type / custom_type / ... 'type' as union
    origin: Optional["Node"] = p_regular(33, require=False, references=LINK_TARGET_NODE_TYPES)
    path: Optional["Path"] = p_regular(34, require=False, array=False, struct=StructType.PATH)
    properties: Optional[list[Property]] = p_regular(
        35, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )

    # content
    title: Optional[str] = p_regular(40, require=False, default=None)
    text: Optional["Text"] = p_regular(41, require=False, default=None, struct=StructType.TEXT)
    # value_packed, value: ... # custom value

    def __content_str__(self):
        return f"{self.kind.bench_name}: {self.type} {self.text}"

    @property
    def subject_id(self) -> UUID | None:
        return self.parent.id if self.parent is not None else None


class NoticeOptions(TypedDict, total=False):
    origin: Optional["Node"]  # if distinct form subject/parent
    title: Optional[str]
    text: Optional[str | Text]
    path: Optional["Path"]
    properties: Optional[Collection[Property] | Collection[Any]]


NoticeHandler = Callable[["Struct", NoticeType, Optional[NoticeOptions]], None]


def on_warning_raise(
    subject: "Struct",
    type: "NoticeType",
    options: Optional[NoticeOptions] = None,
    min_level: NoticeKind = NoticeKind.WARNING,
):
    if type.kind < min_level:
        return
    options = options or {}
    raise NotImplementedError(":Incomplete Notices")


on_error_raise = functools.partial(on_warning_raise, min_level=NoticeKind.ERROR)


def on_notice_ignore(*args, **kwargs):
    pass

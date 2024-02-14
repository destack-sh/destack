import functools

from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.const import BenchError, NodeType, NoticeKind, StructType
from bench.language.expression import FieldPath
from bench.language.node import Node, node, Property
from bench.language.property import p_parent, p_regular
from bench.language.validation import enum_validator
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Package


class NoticeType(IdEnum):
    """Built-in notice types."""

    # errors
    MISSING_REFERENCE = 1
    CIRCULAR_BASE = 2
    MISMATCHED_BASE = 3

    # warnings
    AMBIGUOUS_NAME = 100

    # information

    # hints
    BAD_NAME = 300

    @property
    def kind(self) -> NoticeKind:
        if self.id < 100:
            return NoticeKind.ERROR
        elif self.id < 200:
            return NoticeKind.WARNING
        elif self.id < 300:
            return NoticeKind.INFORMATION
        else:
            return NoticeKind.HINT


NOTICE_TYPES = tuple(NoticeType)


class NoticeError(BenchError, ValueError):
    def __init__(self, notice: "Notice", cause: Exception | None = None):
        super().__init__(repr(notice))
        self.notice = notice
        self.cause = cause


@node(NodeType.NOTICE)
class Notice(Node):
    parent: Union["Block", "Package"] = p_parent(4, NodeType.BLOCK, NodeType.PACKAGE)
    kind: NoticeKind = p_regular(30, default=None, validate=enum_validator(NoticeKind))
    type: NoticeType = p_regular(31, validate=enum_validator(NoticeType))
    # -> builtin_type / custom_type / ... 'type' as union
    message: str = p_regular(33)
    path: Optional[FieldPath] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.FIELD_PATH
    )
    properties: Optional[list[Property]] = p_regular(
        35, default=None, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    # value_packed, value: ... # custom value

    def _init_inner(self):
        self.kind = self.type.kind

    def __content_str__(self):
        return f"{self.kind.bench_name}: {self.type} {self.message}"

    @property
    def subject_id(self) -> UUID | None:
        return self.parent.id if self.parent is not None else None


class NoticeHandler:
    def __call__(
        self,
        subject: Node,
        type: NoticeType,
        message: Optional[str] = None,
        path: Optional[FieldPath] = None,
        properties: Optional[list[Property] | tuple[Property, ...]] = None,
    ):
        pass


def on_warning_raise(
    subject: "Node",
    type: "NoticeType",
    message: Optional[str] = None,
    path: Optional["FieldPath"] = None,
    properties: list["Property"] | None = None,
    min_level: NoticeKind = NoticeKind.WARNING,
):
    if type.kind >= min_level:
        notice = Notice(
            parent=subject,
            type=type,
            kind=type.kind,
            message=message,
            path=path,
            properties=properties,
        )
        raise NoticeError(notice)


on_error_raise = functools.partial(on_warning_raise, min_level=NoticeKind.ERROR)

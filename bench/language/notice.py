from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.node import Package
from bench.language.const import NoticeKind, NodeType, StructType, BenchError
from bench.language.expression import FieldPath, PropertyReference
from bench.language.node import Node, node, node_parent, struct_property
from bench.language.validation import enum_validator
from bench.proto.core import ProtoStrEnum
from bench.utils.casing import Casing, to_casing

if TYPE_CHECKING:
    from bench.language.block import Block


class NoticeType(ProtoStrEnum):
    """Built-in notice types."""

    # errors
    MISSING_REFERENCE = "MISSING_REFERENCE", 1
    CIRCULAR_BASE = "CIRCULAR_BASE", 2
    MISMATCHED_BASE = "MISMATCHED_BASE", 3

    # warnings
    AMBIGUOUS_DEFINITION = "AMBIGUOUS_DEFINITION", 100

    # information

    # hints
    BAD_NAME = "BAD_NAME", 300

    @property
    def bench_name(self):
        return to_casing(self.name, Casing.CAMEL)

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
    parent: Union["Block", "Package"] = node_parent(4, NodeType.BLOCK, NodeType.PACKAGE)
    kind: NoticeKind = struct_property(30, default=None, validate=enum_validator(NoticeKind))
    type: NoticeType = struct_property(31, validate=enum_validator(NoticeType))
    # -> builtin_type / custom_type / ... 'type' as union
    message: str = struct_property(33)
    path: Optional[FieldPath] = struct_property(
        34, default=None, require=False, array=False, struct=StructType.FIELD_PATH
    )
    properties: Optional[list[PropertyReference]] = struct_property(
        35, default=None, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )

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
        subject: "Node",
        type: NoticeType,
        message: Optional[str] = None,
        path: Optional[FieldPath] = None,
        properties: Optional[list[PropertyReference]] = None,
    ):
        pass

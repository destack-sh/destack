from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Collection, Optional, Union
from uuid import UUID

import structlog

from bench.language.const import NodeType, PrimitiveValue, StructType
from bench.language.graph import NodeGraph
from bench.language.node import InterpStatus, Node, Property, Struct, struct, struct_component
from bench.language.notice import NoticeHandler
from bench.language.property import p_internal
from bench.language.validation import ValidationHandler

if TYPE_CHECKING:
    from bench.language import Bench, Block, Branch, Environment, NodeVisitor, Package, Session
    from bench.language.field import Field, FieldKind, TypeInfo

logger = structlog.get_logger(__name__)

OneValue = Union["Value", PrimitiveValue]
ManyValue = Union[Collection["Value"], Collection[PrimitiveValue]]
SomeValue = Union[OneValue, ManyValue]


@dataclass(slots=True)
class Value:
    """
    Any user-defined non-primitive Value
    (e.g., the variable value of a Block, inputs to a Run, an Object instance).
    """

    # local identity (matches Struct)
    id: int
    parent: Union["Value", Struct, Node, None]
    parent_id: int | UUID | None
    parent_prop: Union[Property, "Field", None]
    parent_key: str | None
    order_key: str | None

    # content
    _type: "TypeInfo"
    _base: Optional["Block"]
    _base_field_kind: Optional["FieldKind"]
    _value: dict[str, SomeValue] | None

    # use
    ...  # getattr/setattr

    @property
    def _status(self) -> InterpStatus | None:
        if self.parent is None:
            return None
        else:
            return self.parent._status

    def _updated_self(self, properties: tuple[Union[Property, "Field", Any], ...]):
        raise NotImplementedError(":Incomplete")


@struct_component()
class HasValues(Struct):
    def _validate_inner(
        self, properties: Collection[Property], on_invalid: "ValidationHandler"
    ) -> None:
        pass

    def _visit_inner(self, visitor: "NodeVisitor") -> None:
        pass

    def _clear_inner(self, scope: Optional["Node"] = None):
        pass

    def _interp_inner(self, scope: Optional["Node"], on_notice: "NoticeHandler"):
        pass

    def _track_inner(self, session: "Session") -> None:
        pass

    def _untrack_inner(self) -> None:
        pass


def coerce_value(value: Any, type: "TypeInfo") -> SomeValue:
    """
    Coerces the given value to the expected type (recursively).
    Returns value as is if already correct.
    Raises TypeError if not possible.
    """
    raise NotImplementedError


def type_value(value: Any, type: "TypeInfo") -> None:
    """
    Checks whether the given value has the expected type (recursively).
    Raises TypeError if not.
    """
    raise NotImplementedError


def unpack_value(
    value_packed: Any,
    secret_value_packed: Any | None,
    type: "TypeInfo",
    graph: NodeGraph,
    session: Optional["Session"] = None,
) -> SomeValue:
    """Unpacks/deserializes the given value into a Bench Value representation."""
    raise NotImplementedError


def pack_value(value: SomeValue, type: "TypeInfo", graph: NodeGraph) -> tuple[Any, Any | None]:
    """Packs/serializes the given value into a JSON-able representation."""
    raise NotImplementedError


@struct(StructType.CONTEXT)
class Context(Struct):
    """A semi-magical value that accumulates context down the graph (starting with system context)."""

    # system
    bench: Optional["Bench"] = p_internal(30, require=False, array=False, references=NodeType.BENCH)
    environment: Optional["Environment"] = p_internal(
        31, require=False, array=False, references=NodeType.ENVIRONMENT
    )
    branch: Optional["Branch"] = p_internal(
        32, require=False, array=False, references=NodeType.BRANCH
    )
    package: Optional["Package"] = p_internal(
        33, require=False, array=False, references=NodeType.PACKAGE
    )
    module: Optional["Block"] = p_internal(
        34, require=False, array=False, references=NodeType.BLOCK
    )
    page: Optional["Block"] = p_internal(35, require=False, array=False, references=NodeType.BLOCK)
    # user/trigger/...?

    # custom
    # value_packed: Any = p_value_packed(40)
    # secret_value_packed: Any = p_secret_value_packed(41)
    # value: Any = p_value_runtime(40, 41)

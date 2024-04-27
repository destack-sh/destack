from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Callable, Collection, Iterable, Optional, Union

import structlog

from bench.language.const import NodeType, StructType
from bench.language.node import InterpStatus, Node, Property, Struct, struct, struct_component
from bench.language.notice import NoticeHandler
from bench.language.property import p_internal
from bench.language.validation import ValidationHandler, on_invalid_raise

if TYPE_CHECKING:
    from bench.language import Bench, Block, Branch, Environment, NodeVisitor, Package, Session
    from bench.language.field import Field, TypeInfo

logger = structlog.get_logger(__name__)


# TODO :Broken: implement Value


@dataclass(slots=True)
class Value:
    """
    Any value with fields and an identity that's not a Node (and not inlined).

    """

    # local identity (matches Struct)
    id: int
    parent: Union["Value", Struct, Node, None]
    parent_id: int | None
    parent_key: str | None
    order_key: str | None

    # content
    _type: "TypeInfo"
    _key: Union[Property, "Field"] | None
    _value: dict[str, Any]

    # use
    ...  # getattr/setattr

    @property
    def _status(self) -> InterpStatus | None:
        if self.parent is None:
            return None
        else:
            return self.parent._status

    def _updated_self(self, properties: tuple[Union[Property, "Field"], ...]):
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

    def _interp_inner(self, scope: "Node", on_notice: "NoticeHandler"):
        pass

    def _track_inner(self, session: "Session") -> None:
        pass

    def _untrack_inner(self) -> None:
        pass


def _map_v_noop(value: Any, *args, **kwargs):
    return value


def _map_k_noop(field: "Field"):
    return field.py_ident, field.py_ident


def map_value(
    value: Any,
    type: "TypeInfo",
) -> Any | None:
    """Walks the value and reassembles with new keys and values."""

    raise NotImplementedError


def walk_value(
    value: Any,
    type: "TypeInfo",
) -> Iterable[Any]:
    """Yields all flat values in the instantiated value recursively."""
    raise NotImplementedError


def check_type(
    value: Any,
    type: "TypeInfo",
) -> None:
    """
    Checks whether the given value has the expected type (recursively).
    Raises TypeError if not.
    """
    raise NotImplementedError


def unpack_value(
    value: Any,
    type: "TypeInfo",
    scope: Node,
    session: Optional["Session"] = None,
):
    """Unpacks/deserializes the given value into a Python/Bench representation."""
    raise NotImplementedError


def pack_value(
    value: Any,
    type: "TypeInfo",
):
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
    # log: ...

    # custom
    # value_packed: Any = p_value_packed(40)
    # secret_value_packed: Any = p_secret_value_packed(41)
    # value: Any = p_value_runtime(40, 41)

import typing
from copy import deepcopy
from typing import Optional, Union

from bench.language.builtin import symbolx_lib
from bench.language.const import NodeType, StatementType
from bench.language.node import (
    Node,
    NodeList,
    NRel,
    node,
    node_children,
    node_component,
    node_parent,
    struct_internal,
    struct_property,
)
from bench.language.value import HasValue
from bench.sql.core import ColumnType
from bench.utils.func import dict_minus

if typing.TYPE_CHECKING:
    from bench.language import Field, File, HasFields, Statement


@node(NodeType.TAGGING)
class Tagging(HasValue, Node):
    """An association between a tag and a statement (with optional value)."""

    parent: Union["File", "Statement", "Field"] | None = node_parent(
        4, NodeType.FILE, NodeType.STATEMENT, NodeType.FIELD
    )
    value_packed: typing.Any | None = struct_property(
        31, default=None, copy=deepcopy, column_type=ColumnType.JSON
    )
    reference: Optional["Statement"] = struct_internal(
        32, require=False, array=False, references=NodeType.STATEMENT
    )

    @staticmethod
    def new(
        reference: Union["Statement", "Tagging", str],
        *args,
        for_parent: Union["File", "Statement", "Field"] = None,
        **kwargs,
    ) -> "Tagging":
        from bench.language import Statement

        if isinstance(reference, Tagging):
            reference = reference.reference
        elif isinstance(reference, Statement):
            if reference.type != StatementType.TAG:
                raise TypeError(f"cannot use {reference!r} as a tag")
        elif isinstance(reference, str):
            module = (for_parent.module if for_parent else None) or symbolx_lib
            resolved = symbolx_lib.lookup(".builtins." + reference) or module.lookup(reference)
            if resolved is None:
                raise ValueError(f"cannot find tag {reference!r}")
            reference = resolved
        else:
            raise TypeError(f"cannot use {reference!r} as a tag")

        return Tagging(reference=reference, *args, **kwargs)

    @staticmethod
    def to_python(
        node: "Tagging", props: dict, for_parent: Union["File", "Statement", "Field"] = None
    ) -> tuple[str, dict, dict]:
        assert node.reference is not None, f"missing reference for {node!r}"
        if isinstance(node.reference, Node) and node.reference._type == NodeType.STATEMENT:
            reference = node.reference.name
        else:
            reference = node.reference
        init_args = {"reference": reference}
        return "Tagging.new", init_args, dict_minus(props, "reference")

    @property
    def _type(self) -> "HasFields":
        return None

    def __content_str__(self):
        return f"#{self.reference}"


@node_component
class HasTags(Node):
    tags: NodeList["Tagging"] = node_children(NodeType.TAGGING, NRel.KEYED)

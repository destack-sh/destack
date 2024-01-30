import typing
from copy import deepcopy
from typing import Optional, Union

from bench.language.const import BlockType, NodeType
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
from bench.sql.core import PrimitiveType
from bench.utils.func import dict_minus

if typing.TYPE_CHECKING:
    from bench.language import Block, Field, HasFields, symbolx_lib


@node(NodeType.TAG)
class Tag(HasValue, Node):
    """An association between a tag and a node (with optional value)."""

    parent: Union["Block", "Field"] | None = node_parent(4, NodeType.BLOCK, NodeType.FIELD)
    value_packed: typing.Any | None = struct_property(
        31, default=None, copy=deepcopy, primitive_type=PrimitiveType.JSON
    )
    reference: Optional["Block"] = struct_internal(
        32, require=False, array=False, references=NodeType.BLOCK
    )

    @staticmethod
    def new(
        reference: Union["Block", "Tag", str],
        *args,
        for_parent: Union["Block", "Field"] = None,
        **kwargs,
    ) -> "Tag":
        from bench.language import Block

        if isinstance(reference, Tag):
            reference = reference.reference
        elif isinstance(reference, Block):
            if reference.type != BlockType.TAG:
                raise TypeError(f"cannot use {reference!r} as a tag")
        elif isinstance(reference, str):
            package = (for_parent.package if for_parent else None) or symbolx_lib
            resolved = symbolx_lib.lookup(".builtins." + reference) or package.lookup(reference)
            if resolved is None:
                raise ValueError(f"cannot find tag {reference!r}")
            reference = resolved
        else:
            raise TypeError(f"cannot use {reference!r} as a tag")

        return Tag(reference=reference, *args, **kwargs)

    @staticmethod
    def to_python(
        node, props: dict, for_parent: Union["Block", "Field"] = None
    ) -> tuple[str, dict, dict]:
        assert node.reference is not None, f"missing reference for {node!r}"
        if isinstance(node.reference, Node) and node.reference._type == NodeType.BLOCK:
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
    tags: NodeList["Tag"] = node_children(NodeType.TAG, NRel.KEYED)

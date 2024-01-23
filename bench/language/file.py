from typing import TYPE_CHECKING, Optional, Union

from bench.language.const import NodeType, StructType
from bench.language.node import (
    Module,
    NodeList,
    NRel,
    ScopeNode,
    node,
    node_children,
    node_parent,
    struct_internal,
    struct_property,
)
from bench.language.tagging import HasTags
from bench.language.validation import validate_is_str, validate_name
from bench.utils.casing import IdentifierType

if TYPE_CHECKING:
    from bench.language import Policy, Statement


@node(NodeType.FILE, identifier=IdentifierType.FILE)
class File(ScopeNode, HasTags):
    """
    Files are how a Bench organizes statements. Files can also be folders to other files.
    """

    parent: Union["File", Module] = node_parent(4, NodeType.FILE, NodeType.MODULE)
    policies: Optional[list["Policy"]] = struct_internal(
        20, default_factory=list, struct_t=StructType.POLICY
    )
    name: Optional[str] = struct_property(30, validate=validate_name)
    order_key: Optional[str] = struct_internal(31, default=None)
    text: str | None = struct_property(32, default=None, validate=validate_is_str)
    statements: NodeList["Statement"] = node_children(NodeType.STATEMENT, NRel.ORDERED | NRel.NAMED)

    @staticmethod
    def new(name: str = None, *args, for_parent: Union["File", Module] = None, **kwargs) -> "File":
        return File(name=name, *args, **kwargs)

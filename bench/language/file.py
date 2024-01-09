from typing import TYPE_CHECKING, Optional, Union

from bench.language.const import NodeType, StructType
from bench.language.node import (
    Module,
    NodeList,
    NRel,
    ScopeNode,
    _Passthrough,
    node,
    node_children,
    node_parent,
    struct_internal,
    struct_property,
)
from bench.language.tagging import HasTags
from bench.language.validation import validate_name
from bench.utils.utils import IdentifierType, to_pyidentifier

if TYPE_CHECKING:
    from bench.language import Policy, Statement


@node(NodeType.FILE, passthrough=(("statements", _Passthrough.Full),))
class File(ScopeNode, HasTags):
    """
    Files are how a Bench organizes statements. Files can also be folders to other files.
    """

    parent: Union["File", Module] = node_parent(4, NodeType.FILE, NodeType.MODULE)
    policies: Optional[list["Policy"]] = struct_internal(
        20, default_factory=list, struct_t=StructType.POLICY
    )
    name: Optional[str] = struct_property(30, validate=validate_name)
    order_key: Optional[str] = struct_internal(31, default=None, unique=True)

    children: NodeList[Union["File", "Statement"]] = node_children(
        NodeType.STATEMENT, NRel.Flat | NRel.Ordered | NRel.Named | NRel.Scoped
    )
    statements: NodeList["Statement"] = node_children(
        NodeType.STATEMENT, NRel.Flat | NRel.Ordered | NRel.Named
    )

    @staticmethod
    def new(name: str = None, *args, for_parent: Union["File", Module] = None, **kwargs) -> "File":
        return File(name=name, *args, **kwargs)

    def __str__(self):
        return f"{self.path} '{self.name}' ({len(self.statements)} statements)"

    def __repr__(self):
        return f"<File {str(self)}>"

    @property
    def path(self) -> str:
        if isinstance(self.parent, Module):
            return f"{self.parent.name}.{self.py_ident}"
        elif isinstance(self.parent, File):
            return f"{self.parent.path}.{self.py_ident}"
        else:
            return f"<detached>.{self.py_ident}"

    @property
    def py_ident(self) -> Optional[str]:
        if self.name is None:
            return None
        else:
            return to_pyidentifier(self.name, IdentifierType.PATH)

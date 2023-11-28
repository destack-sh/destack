from typing import TYPE_CHECKING, Optional, Union

from bench.language.const import NodeType
from bench.language.module import (
    Module,
    NodeList,
    NRel,
    ScopeNode,
    _Passthrough,
    bproperty,
    nchildren,
    node,
    nparent,
)
from bench.language.tagging import HasTags
from bench.language.validation import validate_name
from bench.utils.utils import IdentifierType, to_pyidentifier

if TYPE_CHECKING:
    from bench.language.statement import Statement


@node(node_type=NodeType.FILE, passthrough=(("statements", _Passthrough.Full),))
class File(ScopeNode, HasTags):
    parent: Union["File", Module] = nparent(NodeType.FILE, NodeType.MODULE)
    name: str | None = bproperty(validate=validate_name)

    children: NodeList[Union["File", "Statement"]] = nchildren(
        NodeType.STATEMENT, NRel.Flat | NRel.Ordered | NRel.Named | NRel.Scoped
    )
    statements: NodeList["Statement"] = nchildren(
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

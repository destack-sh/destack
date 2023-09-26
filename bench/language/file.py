from typing import TYPE_CHECKING, Optional, Union

from bench.language.const import MNT
from bench.language.module import (
    Module,
    NodeList,
    NRel,
    ScopedNode,
    nchildren,
    node,
    nparent,
    nproperty,
)
from bench.utils.fractional import generate_n_keys_between
from bench.utils.utils import IdentifierType, to_pyidentifier

if TYPE_CHECKING:
    from bench.language.statement import Statement


@node(mnt=MNT.File)
class File(ScopedNode):
    parent: Union["File", Module] = nparent(MNT.File, MNT.Module)
    name: str = nproperty()

    children: NodeList[Union["File", "Statement"]] = nchildren(
        MNT.File, NRel.Flat | NRel.Ordered | NRel.Named
    )
    statements: NodeList["Statement"] = nchildren(
        MNT.Statement, NRel.Flat | NRel.Ordered | NRel.Named
    )

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
            return self.py_ident  # detached file

    @property
    def py_ident(self) -> Optional[str]:
        if self.name is None:
            return None
        else:
            return to_pyidentifier(self.name, IdentifierType.PATH)

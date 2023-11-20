import typing
from typing import Optional

from bench.language.const import MNT, ViewLayout
from bench.language.expression import Conditional, Sort
from bench.language.module import ScopeNode, bproperty, node, nparent
from bench.language.validation import enum_validator

if typing.TYPE_CHECKING:
    from bench.language import File, Statement


@node(mnt=MNT.VIEW)
class View(ScopeNode):
    parent: typing.Union["Statement", "File"] = nparent(MNT.STATEMENT, MNT.FILE)
    name: str | None = bproperty(default=None)
    layout: ViewLayout = bproperty(default=ViewLayout.TABLE, validate=enum_validator(ViewLayout))
    query: Optional[Conditional] = bproperty(default=None)
    sort: Optional[list[Sort]] = bproperty(default=None)

    def __str__(self):
        return f"{self.parent.path}:{self.name} ({self.layout})"

    def __repr__(self):
        return f"<DatabaseView {self}>"

    @property
    def path(self) -> str:
        return f"{self.parent.path}.{self.name}"

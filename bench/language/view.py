import typing
from typing import Optional

from bench.language.const import MNT, ViewLayout
from bench.language.expression import Query, Sort
from bench.language.module import ScopeNode, node, nparent, nproperty
from bench.language.validation import enum_validator

if typing.TYPE_CHECKING:
    from bench.language import File, Statement


@node(mnt=MNT.VIEW)
class View(ScopeNode):
    parent: typing.Union["Statement", "File"] = nparent(MNT.STATEMENT, MNT.FILE)
    name: str | None = nproperty(default=None)
    layout: ViewLayout = nproperty(default=ViewLayout.TABLE, validate=enum_validator(ViewLayout))
    query: Optional[Query] = nproperty(default=None)
    sort: Optional[list[Sort]] = nproperty(default=None)

    def __str__(self):
        return f"{self.parent.path}:{self.name} ({self.layout})"

    def __repr__(self):
        return f"<DatabaseView {self}>"

    @property
    def path(self) -> str:
        return f"{self.parent.path}.{self.name}"

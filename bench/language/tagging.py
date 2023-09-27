import typing
from typing import Union

from bench.language.const import MNT
from bench.language.module import (
    ModuleNode,
    NodeList,
    NRel,
    nchildren,
    node,
    node_component,
    nparent,
    nproperty,
)
from bench.language.reference import HasReference
from bench.language.value import HasValue

if typing.TYPE_CHECKING:
    from bench.language import Field, File, HasFields, Statement


@node(mnt=MNT.Tagging)
class Tagging(HasValue, HasReference, ModuleNode):
    """An association between a tag and a statement (with optional value)."""

    parent: Union["File", "Statement", "Field"] | None = nparent(MNT.File, MNT.Statement, MNT.Field)
    key: str = nproperty()

    @property
    def _type_of_value(self) -> "HasFields":
        from bench.language.libs import symbolx_lib

        return symbolx_lib.resolve(".reflect.TaggingMetadata")

    def __str__(self):
        parent_str = self.parent.path if self.parent is not None else "<detached>"
        if isinstance(self.reference, ModuleNode):
            return f"{parent_str}#{self.reference.path}"
        else:
            return f"{parent_str}#{self.key}"

    def __repr__(self):
        return f"<Tagging {self}>"


@node_component
class HasTags(ModuleNode):
    tags: NodeList["Tagging"] = nchildren(MNT.Tagging, NRel.Keyed)

    @staticmethod
    def _to_tag_key(key: Union[str, "Statement", Tagging]) -> str:
        if hasattr(key, "key"):
            key = key.key
        return key

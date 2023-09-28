import typing
from typing import Union

from bench.language.builtin import symbolx_lib
from bench.language.const import MNT, StatementType
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
            key = reference.key
        elif isinstance(reference, Statement):
            if reference.type != StatementType.TAG:
                raise TypeError(f"cannot use {reference!r} as a tag")
            key = reference.key
        elif isinstance(reference, str):
            module = (for_parent.module if for_parent is not None else None) or symbolx_lib
            resolved = symbolx_lib.lookup(".builtins." + reference) or module.lookup(reference)
            if resolved is None:
                raise ValueError(f"cannot find tag {reference!r}")
            reference = resolved
            key = reference.key
        else:
            raise TypeError(f"cannot use {reference!r} as a tag")

        return Tagging(reference=reference, key=key, *args, **kwargs)

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

import typing
from typing import Union

from bench.language.builtin import symbolx_lib
from bench.language.const import MNT, StatementType
from bench.language.module import (
    Node,
    NodeList,
    NRel,
    nchildren,
    ninternal,
    node,
    node_component,
    nparent,
)
from bench.language.reference import HasReference
from bench.language.value import HasValue
from bench.utils.func import dict_minus

if typing.TYPE_CHECKING:
    from bench.language import Field, File, HasFields, Statement


@node(mnt=MNT.TAGGING)
class Tagging(HasValue, HasReference, Node):
    """An association between a tag and a statement (with optional value)."""

    parent: Union["File", "Statement", "Field"] | None = nparent(MNT.FILE, MNT.STATEMENT, MNT.FIELD)
    key: str = ninternal()

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
            module = (for_parent.module if for_parent else None) or symbolx_lib
            resolved = symbolx_lib.lookup(".builtins." + reference) or module.lookup(reference)
            if resolved is None:
                raise ValueError(f"cannot find tag {reference!r}")
            reference = resolved
            key = reference.key
        else:
            raise TypeError(f"cannot use {reference!r} as a tag")

        return Tagging(reference=reference, key=key, *args, **kwargs)

    @staticmethod
    def to_python(
        node: "Tagging", props: dict, for_parent: Union["File", "Statement", "Field"] = None
    ) -> tuple[str, dict, dict]:
        assert node.reference is not None, f"missing reference for {node!r}"
        if isinstance(node.reference, Node) and node.reference.mnt == MNT.STATEMENT:
            reference = node.reference.name
        else:
            reference = node.reference
        init_args = {"reference": reference}
        return "Tagging.new", init_args, dict_minus(props, "reference")

    @property
    def _type_of_value(self) -> "HasFields":
        return symbolx_lib.resolve(".reflect.TaggingMetadata")

    def __str__(self):
        parent_str = self.parent.path if self.parent is not None else "<detached>"
        if isinstance(self.reference, Node):
            return f"{parent_str}#{self.reference.path}"
        else:
            return f"{parent_str}#{self.key}"

    def __repr__(self):
        return f"<Tagging {self}>"


@node_component
class HasTags(Node):
    tags: NodeList["Tagging"] = nchildren(MNT.TAGGING, NRel.Keyed)

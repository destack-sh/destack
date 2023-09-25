import typing
from typing import Union
from uuid import UUID

from bench.language.const import MNT, StatementReference
from bench.language.module import (
    ModuleNode,
    NodeVisitor,
    ScopedNode,
    node,
    node_component,
    nparent,
    nproperty,
    nchildren,
    NRel,
)
from bench.language.value import HasValue

if typing.TYPE_CHECKING:
    from bench.language import Field, File, HasFields, Statement


@node(mnt=MNT.Tagging)
class Tagging(HasValue, ModuleNode):
    """An association between a tag and a statement (with optional value)."""

    parent: Union["File", "Statement", "Field"] | None = nparent(MNT.File, MNT.Statement, MNT.Field)
    reference: Union["Statement", StatementReference] = nproperty()
    key: str = nproperty()

    @property
    def _type_of_value(self) -> "HasFields":
        from bench.language.libs import symbolx_lib

        return symbolx_lib.lookup_or_error(".reflect.TaggingMetadata")

    def __str__(self):
        if isinstance(self.reference, ModuleNode):
            return f"{self.reference.path}"
        else:
            return self.key

    def __repr__(self):
        return f"<Tagging {self}>"

    def _interp_inner(self, scope: "ScopedNode") -> None:
        if self.reference_ck is not None:
            self.reference = scope._root_scope._nodes_by_ck.get(self.reference_ck)

    def _visit_inner(self, visitor: NodeVisitor) -> None:
        if isinstance(self.reference, ModuleNode):
            visitor.visit_reference(self.reference)

    @property
    def reference_ck(self) -> typing.Optional[UUID]:
        if isinstance(self.reference, ModuleNode):
            return self.reference.ck
        else:
            return self.reference


@node_component
class HasTags(ModuleNode):
    tags: list[Tagging] = nchildren(MNT.Tagging, NRel.INLINE)

    @staticmethod
    def _to_tag_key(key: Union[str, "Statement", Tagging]) -> str:
        if hasattr(key, "key"):
            key = key.key
        return key

    def has_tag(self, key: Union[str, "Statement", Tagging]) -> bool:
        """Returns whether this statement has the given tag."""
        key = self._to_tag_key(key)
        return any(tagging.key == key for tagging in self.tags)

    def get_tag(self, key: Union[str, "Statement", Tagging]) -> Tagging:
        """Returns the tag with the given key."""
        key = self._to_tag_key(key)
        for tagging in self.tags:
            if tagging.key == key:
                return tagging
        raise KeyError(key)

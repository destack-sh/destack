import typing
from dataclasses import field
from typing import Union
from uuid import UUID

from bench.language.const import MNT, StatementReference
from bench.language.module import ModuleNode, NodeVisitor, Scope, node, node_component
from bench.language.value import HasValue
from bench.utils.utils import required_field

if typing.TYPE_CHECKING:
    from bench.language import Field, File, HasFields, Statement


@node(mnt=MNT.Tagging, tracked=[])
class Tagging(HasValue, ModuleNode):
    """An association between a tag and a statement (with optional value)."""

    parent: Union["File", "Statement", "Field"] | None = None
    reference: Union["Statement", StatementReference] = required_field()
    key: str = required_field()

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

    def _visit(self, visitor: NodeVisitor) -> None:
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
    tags: list[Tagging] = field(default_factory=list)

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

    def _clear(self) -> None:
        pass

    def _index(self) -> None:
        pass

    def _interp(self, scope: Scope) -> None:
        for tagging in self.tags:
            if tagging.reference_ck is None:
                continue
            tagging.reference = scope._root_scope._nodes_by_id.get(tagging.reference_ck)
            if tagging.reference is None:
                # is that an error? not sure
                continue

    def _visit(self, visitor: NodeVisitor) -> None:
        for tagging in self.tags:
            visitor.visit_child(tagging)

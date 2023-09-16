import itertools
import typing
from dataclasses import field
from uuid import UUID

from bench.language import Scope
from bench.language.const import MNT, StatementReference, TypeTag
from bench.language.module import ModuleNode, ModuleVisitor, node
from bench.language.text import HasText
from bench.utils.utils import required_field

TAG_KEY_LENGTH = 8


@node(mnt=MNT.Tagging, tracked=[])
class Tagging(ModuleNode):
    """An association between a tag and a statement (with optional metadata)."""

    reference: typing.Union["Tag", StatementReference] = required_field()
    key: str = required_field()
    parent: ModuleNode | None = None
    metadata: dict[str, typing.Any] | None = None

    def __str__(self):
        if isinstance(self.reference, Tag):
            return f"{self.reference.path}"
        else:
            return self.key

    def __repr__(self):
        return f"<Tagging {self}>"

    def _visit(self, visitor: ModuleVisitor) -> None:
        pass

    @property
    def reference_ck(self) -> typing.Optional[UUID]:
        if isinstance(self.reference, Tag):
            return self.reference.ck
        else:
            return self.reference


@node
class HasTags(ModuleNode):
    tags: list[Tagging] = field(default_factory=list)

    @staticmethod
    def _to_tag_key(key: typing.Union[str, "Tag", Tagging]) -> str:
        if isinstance(key, Tagging):
            key = key.key
        elif isinstance(key, Tag):
            key = key.key
        return key

    def set_tag(self, key: typing.Union[str, "Tag", Tagging], value: str = None) -> None:
        """Tags this statement with the given key and value."""
        raise NotImplementedError

    def clear_tag(self, key: typing.Union[str, "Tag", Tagging]) -> None:
        """Clears the tag with the given key."""
        raise NotImplementedError

    def has_tag(self, key: typing.Union[str, "Tag", Tagging]) -> bool:
        """Returns whether this statement has the given tag."""
        key = self._to_tag_key(key)
        return any(tagging.key == key for tagging in self.tags)

    def get_tag(self, key: typing.Union[str, "Tag", Tagging]) -> Tagging:
        """Returns the tag with the given key."""
        key = self._to_tag_key(key)
        for tagging in self.tags:
            if tagging.key == key:
                return tagging
        raise KeyError(key)

    def _clear(self) -> None:
        for tagging in self.tags:
            tagging._tag = None

    def _interp(self, scope: Scope) -> None:
        for tagging in self.tags:
            if tagging.reference_ck is None:
                continue
            tagging.reference = scope._root_scope._nodes_by_id.get(tagging.reference_ck)
            if tagging.reference is None:
                # is that an error? not sure
                continue

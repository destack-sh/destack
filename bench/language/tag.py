import random
import string
import typing
from dataclasses import field
from uuid import UUID

from bench.language.const import TypeTag
from bench.language.core import (
    HasCrud,
    HasSession,
    ModuleNode,
    Scope,
    Statement,
    StatementBase,
    StatementReference,
    StatementType,
    node,
)
from bench.utils.utils import required_field

TAG_KEY_LENGTH = 8


def new_tag_key(seed: str = None) -> str:
    """Gets a random alphabetic key as a persistent key for a type node."""
    # (upper and lower case letters only)
    # :TagKeys
    if seed is not None:
        random.seed(seed)
    return "".join(random.choices(string.ascii_letters, k=TAG_KEY_LENGTH))


@node(tracked=[])
class Tagging(HasCrud, HasSession, ModuleNode):
    """An association between a tag and a statement (with optional metadata)."""

    reference: typing.Union["Tag", StatementReference] = required_field()
    key: str = required_field()
    parent: Statement | None = None
    metadata: dict[str, typing.Any] | None = None

    def __str__(self):
        if isinstance(self.reference, Tag):
            return f"{self.reference.path}"
        else:
            return self.key

    def __repr__(self):
        return f"<Tagging {self}>"

    @property
    def reference_id(self) -> typing.Optional[UUID]:
        if isinstance(self.reference, Tag):
            return self.reference.id
        else:
            return self.reference


@node
class HasTags(StatementBase):
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

    def get_children_by_tag(self, key: typing.Union[str, "Tag", Tagging]) -> list["Statement"]:
        """Returns all resolved children with the given tag."""
        key = self._to_tag_key(key)
        return [c for c in self.resolved_children if isinstance(c, HasTags) and c.has_tag(key)]

    def _clear(self) -> None:
        for tagging in self.tags:
            tagging._tag = None

    def _interp(self, scope: Scope) -> None:
        for tagging in self.tags:
            if tagging.reference_id is None:
                continue
            tagging.reference = scope._root_scope._statements_by_id.get(tagging.reference_id)
            if tagging.reference is None:
                # is that an error? not sure
                continue


# avoid circular import because Tag is HasType but Type is HasTags
from bench.language.type import HasType  # noqa


@node(tracked=["name"])
class Tag(HasType, HasTags, Statement):
    """A tag statement."""

    name: str = None
    description: str = None
    key: str = field(default_factory=new_tag_key)
    type: StatementType = StatementType.TAG
    tag: TypeTag = TypeTag.STRUCT

    def _clear(self):
        Statement._clear(self)
        HasType._clear(self)
        HasTags._clear(self)

    def _interp(self, scope: Scope) -> None:
        Statement._interp(self, scope)
        HasType._interp(self, scope)
        HasTags._interp(self, scope)

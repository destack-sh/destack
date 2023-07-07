import random
import string
import typing
from dataclasses import field

from bench.bench.const import TypeTag
from bench.bench.core import (
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
class Tagging(ModuleNode):
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


@node
class HasTags(StatementBase):
    tags: list[Tagging] = field(default_factory=list)

    def set_tag(self, key: typing.Union[str, "Tag", Tagging], value: str = None) -> None:
        """Tags this statement with the given key and value."""
        raise NotImplementedError

    def clear_tag(self, key: typing.Union[str, "Tag", Tagging]) -> None:
        """Clears the tag with the given key."""
        raise NotImplementedError

    def clear_tags(self) -> None:
        """Clears all tags."""
        raise NotImplementedError

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
from bench.bench.type import HasType  # noqa


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

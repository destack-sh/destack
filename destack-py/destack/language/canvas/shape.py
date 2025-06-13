from destack.language.core import Trait, TraitType, builtin_trait


@builtin_trait(TraitType.SHAPE)
class IsShape(Trait):
    """A Node that is a Shape."""

    pass

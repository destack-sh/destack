from destack.language.core import Trait, TraitType, trait_


@trait_(TraitType.SHAPE)
class IsShape(Trait):
    """A Node that is a Shape."""

    pass

from destack.core import Enum, EnumType, declare_enum


@declare_enum(EnumType.EASING)
class Easing(Enum):
    """Built-in easing types."""

    LINEAR = 1
    EASE_IN_QUAD = 10
    EASE_OUT_QUAD = 11
    EASE_IN_OUT_QUAD = 12
    EASE_IN_CUBIC = 20
    EASE_OUT_CUBIC = 21
    EASE_IN_OUT_CUBIC = 22
    EASE_IN_QUART = 30
    EASE_OUT_QUART = 31
    EASE_IN_OUT_QUART = 32
    EASE_IN_QUINT = 40
    EASE_OUT_QUINT = 41
    EASE_IN_OUT_QUINT = 42
    EASE_IN_SINE = 50
    EASE_OUT_SINE = 51
    EASE_IN_OUT_SINE = 52
    EASE_IN_EXPO = 60
    EASE_OUT_EXPO = 61
    EASE_IN_OUT_EXPO = 62
    EASE_PEN = 70

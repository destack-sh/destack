from destack.core import EnumType, OptionEnum, declare_enum, declare_option


@declare_enum(EnumType.EASING)
class Easing(OptionEnum):
    """Built-in easing types."""

    LINEAR = declare_option(1)
    EASE_IN_QUAD = declare_option(10)
    EASE_OUT_QUAD = declare_option(11)
    EASE_IN_OUT_QUAD = declare_option(12)
    EASE_IN_CUBIC = declare_option(20)
    EASE_OUT_CUBIC = declare_option(21)
    EASE_IN_OUT_CUBIC = declare_option(22)
    EASE_IN_QUART = declare_option(30)
    EASE_OUT_QUART = declare_option(31)
    EASE_IN_OUT_QUART = declare_option(32)
    EASE_IN_QUINT = declare_option(40)
    EASE_OUT_QUINT = declare_option(41)
    EASE_IN_OUT_QUINT = declare_option(42)
    EASE_IN_SINE = declare_option(50)
    EASE_OUT_SINE = declare_option(51)
    EASE_IN_OUT_SINE = declare_option(52)
    EASE_IN_EXPO = declare_option(60)
    EASE_OUT_EXPO = declare_option(61)
    EASE_IN_OUT_EXPO = declare_option(62)
    EASE_PEN = declare_option(70)

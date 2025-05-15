from typing import Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    EnumType,
    NodeType,
    Struct,
    StructType,
    enum_,
    node_,
    node_component_,
    p_regular,
    struct_,
)
from bench.pb2 import EffectStyleData

from .style import StyleBase


@enum_(EnumType.EFFECT_TYPE)
class EffectType(BuiltinEnum):
    """Built-in effect types."""

    STYLE = 2
    APPEAR = 10
    ENTER = 11
    EXIT = 12
    HOVER = 13
    TRANSFORM = 14


@node_component_()
class EffectBase(BuiltinObject):
    """A base class for effects."""

    type: EffectType = p_regular(30, default=EffectType.APPEAR)
    opacity: Optional[float] = p_regular(40, default=None)
    offset_x: Optional[int] = p_regular(41, default=None)
    offset_y: Optional[int] = p_regular(42, default=None)
    scale: Optional[float] = p_regular(43, default=None)
    rotate: Optional[float] = p_regular(44, default=None)
    rotate_x: Optional[float] = p_regular(45, default=None)
    rotate_y: Optional[float] = p_regular(46, default=None)
    rotate_z: Optional[float] = p_regular(47, default=None)
    skew_x: Optional[float] = p_regular(48, default=None)
    skew_y: Optional[float] = p_regular(49, default=None)
    perspective: Optional[float] = p_regular(50, default=None)
    delay: Optional[float] = p_regular(51, default=None)
    duration: Optional[float] = p_regular(52, default=None)
    threshold: Optional[float] = p_regular(53, default=None)
    trigger: Optional[str] = p_regular(54, default=None)


@struct_(StructType.EFFECT)
class Effect(EffectBase, Struct):
    """An effect value."""

    pass


@node_(NodeType.EFFECT_STYLE)
class EffectStyle(EffectBase, StyleBase[EffectStyleData]):
    """An effect style."""

    pass

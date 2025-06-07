from typing import TYPE_CHECKING, Union

from bench.language.core import (
    HasEnvironment,
    HasName,
    IsInPackage,
    IsTemplatable,
    IsTracked,
    TraitType,
    property_parent_,
    trait_,
)

if TYPE_CHECKING:
    from bench.language import IsView, Scene

# pyright: reportIncompatibleVariableOverride=false


@trait_(TraitType.STYLE)
class IsStyle(
    IsTemplatable,
    HasEnvironment,
    HasName,
    IsInPackage,
    IsTracked,
):
    """A Style is a graphical interface."""

    parent: Union["Scene", "IsView", None] = property_parent_(node_is_customizable=True)

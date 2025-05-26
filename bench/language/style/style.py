from typing import TYPE_CHECKING, Union

from bench.language.core import (
    HasName,
    IsBlockable,
    IsInPackage,
    IsModal,
    IsTemplatable,
    TraitType,
    property_parent_,
    trait_,
)

if TYPE_CHECKING:
    from bench.language import IsView, Page, Space

# pyright: reportIncompatibleVariableOverride=false


@trait_(TraitType.STYLE)
class IsStyle(
    IsTemplatable,
    IsModal,
    HasName,
    IsBlockable,
    IsInPackage,
):
    """A Style is a graphical interface."""

    parent: Union["Space", "IsView", "Page", None] = property_parent_()

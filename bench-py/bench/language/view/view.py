from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    HasEnvironment,
    HasName,
    IsBlockable,
    IsInPackage,
    IsScriptable,
    IsTemplatable,
    TraitType,
    property_,
    property_parent_,
    trait_,
)

if TYPE_CHECKING:
    from bench.language import Dimension, Page, Position, Space

# pyright: reportIncompatibleVariableOverride=false


@trait_(TraitType.VIEW)
class IsView(
    HasEnvironment,
    HasName,
    IsScriptable,
    IsTemplatable,
    IsBlockable,
    IsInPackage,
):
    """A View is a graphical interface."""

    parent: Union["Space", "IsView", "Page", None] = property_parent_()
    # variant_of, ...

    # sizing
    position: Optional["Position"] = property_(40)
    width: Optional["Dimension"] = property_(41)
    height: Optional["Dimension"] = property_(42)
    min_width: Optional["Dimension"] = property_(43)
    min_height: Optional["Dimension"] = property_(44)
    max_width: Optional["Dimension"] = property_(45)
    max_height: Optional["Dimension"] = property_(46)

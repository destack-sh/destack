from typing import TYPE_CHECKING, Union

from bench.language.core import (
    IsArchivable,
    IsDeletable,
    IsInPackage,
    IsModal,
    IsTemplatable,
    Node,
    NodeType,
    node_,
    p_node_parent,
    property_,
)
from bench.pb2 import DependencyData

if TYPE_CHECKING:
    from bench.language import Bench, Package

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.DEPENDENCY)
class Dependency(
    IsTemplatable,
    IsModal,
    IsInPackage,
    IsDeletable,
    IsArchivable,
    Node[DependencyData],
):
    """A Dependency on another Package or Bench."""

    # NOTE :Incomplete: Dependency doesn't actually do anything yet
    #  (we just hardcode a dependency on the builtin Bench package)

    parent: Union["Package", None] = p_node_parent()

    dependency: Union["Package", "Bench"] = property_(
        40,
        description="The Package or Bench this Dependency depends on (if Bench it's all Packages).",
    )

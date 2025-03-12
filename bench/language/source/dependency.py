from typing import TYPE_CHECKING, Union

from bench.language.core import (
    IsTemplatable,
    IsTraceable,
    NodeType,
    PackageNode,
    node_,
    p_node_parent,
    p_regular,
)
from bench.pb2 import DependencyData

if TYPE_CHECKING:
    from bench.language import Bench, Package

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.DEPENDENCY)
class Dependency(IsTemplatable, IsTraceable, PackageNode[DependencyData]):
    """A Dependency on another Package or Bench."""

    # NOTE :Incomplete: Dependency doesn't actually do anything yet
    #  (we just hardcode a dependency on the builtin Bench package)

    parent: Union["Package", None] = p_node_parent(4, NodeType.PACKAGE)

    dependency: Union["Package", "Bench"] = p_regular(
        40,
        require=True,
        array=False,
        references=(NodeType.PACKAGE, NodeType.BENCH),
        description="The Package or Bench this Dependency depends on (if Bench it's all Packages).",
    )

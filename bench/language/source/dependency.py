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
    """
    A dependency on another Bench (pointing to a specific Package).
    If scopes are given, only those blocks are included.
    NOTE :Incomplete: Dependency doesn't work yet :Dependencies
    """

    parent: Union["Package", None] = p_node_parent(4, NodeType.PACKAGE)

    depends_on_bench: "Bench" = p_regular(40, require=True, array=False, references=NodeType.BENCH)
    depends_on_packages: list["Package"] = p_regular(
        41, require=True, array=True, references=NodeType.PACKAGE
    )

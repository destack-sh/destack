from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    IsBlockable,
    IsInstantiable,
    IsModal,
    IsNamed,
    Node,
    NodeType,
    node_,
    p_regular,
)
from bench.pb2 import ApplicationData

if TYPE_CHECKING:
    from bench.language import Scene

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.APPLICATION)
class Application(IsInstantiable, IsNamed, IsModal, IsBlockable, Node[ApplicationData]):
    """An Application is an interactive set of Scenes for some purpose."""

    main_scene: Optional["Scene"] = p_regular(
        50,
        description="The main scene of the Application.",
        node_bench_from="self",
    )
    error_scene: Optional["Scene"] = p_regular(
        53,
        description="The error scene of the Application.",
        node_bench_from="self",
    )

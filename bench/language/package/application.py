from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    IsInstantiable,
    IsModal,
    IsNamed,
    NodeType,
    PageNode,
    node_,
    p_regular,
)
from bench.pb2 import ApplicationData

if TYPE_CHECKING:
    from bench.language import Scene

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.APPLICATION)
class Application(IsInstantiable, IsNamed, IsModal, PageNode[ApplicationData]):
    """An Application is an interactive set of Scenes for some purpose."""

    main_scene: Optional["Scene"] = p_regular(
        50,
        description="The main scene of the Application.",
    )
    error_scene: Optional["Scene"] = p_regular(
        53,
        description="The error scene of the Application.",
    )

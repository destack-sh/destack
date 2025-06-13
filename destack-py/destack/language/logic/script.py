from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    HasName,
    IsActionable,
    IsDeletable,
    IsExtensible,
    IsOrdered,
    IsRunnable,
    IsScriptable,
    IsTemplatable,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_,
    property_parent_,
)
from destack.pb2 import ScriptData

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SCRIPT)
class Script(
    Spatial,
    Entity,
    HasName,
    IsOrdered,
    IsActionable,
    IsDeletable,
    IsRunnable,
    IsTemplatable,
    IsExtensible,
    Node[ScriptData],
):
    """A Script."""

    parent: Union["Folder", IsScriptable, "Script", None] = property_parent_(
        node_is_customizable=True
    )
    # type, language, code, ...

    code: str | None = property_(100)

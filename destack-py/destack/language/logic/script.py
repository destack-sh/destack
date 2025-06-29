from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    HasName,
    IsDeletable,
    IsExtensible,
    IsOrdered,
    IsRunnable,
    IsScriptable,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SCRIPT)
class Script(
    Spatial,
    Entity,
    HasName,
    IsOrdered,
    IsDeletable,
    IsRunnable,
    IsExtensible,
    Node,
):
    """A Script."""

    parent: Union["Folder", IsScriptable, "Script", None] = property_parent_(
        node_is_customizable=True
    )
    # type, language, code, ...

    code: str | None = property_(100)

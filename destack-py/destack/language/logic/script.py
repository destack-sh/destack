from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    HasName,
    IsCustomizable,
    IsDeletable,
    IsOrdered,
    IsRunnable,
    IsScriptable,
    IsSpatial,
    NodeType,
    builtin_node,
    property_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SCRIPT)
class Script(
    IsSpatial,
    HasName,
    IsOrdered,
    IsDeletable,
    IsRunnable,
    IsCustomizable,
    Entity,
):
    """A Script."""

    parent: Union["Folder", IsScriptable, "Script", None] = property_parent_(
        node_is_customizable=True
    )
    # type, language, code, ...

    code: str | None = property_(100)

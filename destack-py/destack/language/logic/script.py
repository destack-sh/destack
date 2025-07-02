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
    builtin_property,
    builtin_property_parent,
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

    parent: Union["Folder", IsScriptable, "Script", None] = builtin_property_parent(
        node_is_extensible=True
    )
    # type, language, code, ...

    code: str | None = builtin_property(100)

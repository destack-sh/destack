from typing import Union

from destack.language.core import (
    HasName,
    IsArchivable,
    IsDeletable,
    IsEntity,
    IsRunnable,
    IsScriptable,
    Node,
    NodeType,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import ScriptData

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCRIPT)
class Script(
    HasName,
    IsDeletable,
    IsArchivable,
    IsRunnable,
    IsEntity,
    Node[ScriptData],
):
    """A Script."""

    parent: Union[IsScriptable, "Script", None] = property_parent_(node_is_customizable=True)
    # type, language, code, ...

    code: str | None = property_(100)

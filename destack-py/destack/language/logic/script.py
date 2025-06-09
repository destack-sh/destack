from destack.language.core import (
    HasName,
    IsArchivable,
    IsDeletable,
    IsScriptable,
    IsTracked,
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
    IsTracked,
    Node[ScriptData],
):
    """A Script."""

    parent: IsScriptable | None = property_parent_(node_is_customizable=True)
    # type, language, code, ...

    code: str | None = property_(100)

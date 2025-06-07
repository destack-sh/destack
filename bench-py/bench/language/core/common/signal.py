from bench.language.core import (
    HasName,
    HasSlug,
    IsInPackage,
    IsSourceable,
    Node,
    NodeType,
    node_,
    property_,
)

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SIGNAL_DEFINITION)
class SignalDefinition(
    HasName,
    HasSlug,
    IsSourceable,
    IsInPackage,
    Node,
):
    """A SignalDefinition is a definition of a Signal."""

    pass


@node_(NodeType.SIGNAL_INSTANCE)
class SignalInstance(IsInPackage, Node):
    definition: SignalDefinition = property_(40)

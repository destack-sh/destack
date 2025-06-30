from destack.language.core import NodeType, builtin_node

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.LABEL_VIEW)
class LabelView(ContainerView):
    """A label container View for form-like input views."""

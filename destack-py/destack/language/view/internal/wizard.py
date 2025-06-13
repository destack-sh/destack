from destack.language.core import Node, NodeType, builtin_node
from destack.pb2 import WizardViewData

from .internal import InternalView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.WIZARD_VIEW)
class WizardView(
    InternalView,
    Node[WizardViewData],
):
    pass

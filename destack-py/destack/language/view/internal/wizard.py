from destack.language.core import Node, NodeType, node_
from destack.pb2 import WizardViewData

from .internal import InternalView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.WIZARD_VIEW)
class WizardView(
    InternalView,
    Node[WizardViewData],
):
    pass

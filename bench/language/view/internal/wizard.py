from bench.language.core import Node, NodeType, node_
from bench.pb2 import WizardViewData

from .internal import IsInternalView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.WIZARD_VIEW)
class WizardView(IsInternalView, Node[WizardViewData]):
    pass

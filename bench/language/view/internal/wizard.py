from bench.language.core import NodeType, node_
from bench.pb2 import WizardViewData

from .internal import InternalViewBase

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.WIZARD_VIEW)
class WizardView(InternalViewBase[WizardViewData]):
    pass

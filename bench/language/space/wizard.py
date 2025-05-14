from bench.language.core import NodeType, node_
from bench.language.view import ViewBase
from bench.pb2 import WizardViewData


@node_(NodeType.WIZARD_VIEW)
class WizardView(ViewBase[WizardViewData]):
    pass

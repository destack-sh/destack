from typing import TYPE_CHECKING

from destack.core import Event, NodeType, ReferenceType, declare_event, declare_property

if TYPE_CHECKING:
    from destack import CustomEventDefinition


@declare_event(NodeType.CUSTOM_EVENT, is_abstract=True)
class CustomEvent(Event):
    """
    A CustomEvent is an instance of a CustomEventDefinition.
    """

    definition: "CustomEventDefinition" = declare_property(
        10,
        is_managed=True,
        is_readonly=True,
        reference_type=ReferenceType.SPATIAL,
        description="The CustomEvent this Signal is an instance of.",
        tag=None,
    )

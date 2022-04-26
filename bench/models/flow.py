from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class FlowManager(models.Manager):
    pass


class Flow(UUIDModel):
    """
    A directed acyclic graph of Functions represented as Nodes connected by Edges.
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    # TODO @Feature: version Flow

    objects = FlowManager()


class FlowNode(UUIDModel):
    """
    A node representing an atomic unit in a Flow (i.e. an execution unit).
    """

    flow = models.ForeignKey(Flow, on_delete=models.CASCADE, related_name="nodes")
    function = models.ForeignKey("FunctionVersion", on_delete=models.RESTRICT)
    # TODO @Cleanup: define input_nodes on FlowNode as well as on FlowNodeEdge
    # input_nodes = models.ManyToManyField(
    #     "FlowNode",
    #     through="FlowNodeEdge",
    #     symmetrical=False,
    #     related_name="output_nodes",
    # )


class FlowEdge(UUIDModel):
    """
    An edge connecting two Nodes.
    """

    input_node = models.ForeignKey(
        FlowNode, on_delete=models.CASCADE, related_name="output_nodes"
    )
    output_node = models.ForeignKey(
        FlowNode, on_delete=models.CASCADE, related_name="input_nodes"
    )

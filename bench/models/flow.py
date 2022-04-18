from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class Flow(UUIDModel):
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH)

    input_nodes = models.ManyToManyField("FlowNode")
    output_nodes = models.ManyToManyField("FlowNode")


class FlowNode(UUIDModel):
    flow = models.ForeignKey(Flow, on_delete=models.CASCADE)
    function = models.ForeignKey("Function", on_delete=models.RESTRICT)
    input_nodes = models.ManyToManyField("FlowNode", through="FlowNodeEdge")
    output_nodes = models.ManyToManyField("FlowNode", through="FlowNodeEdge")


class FlowNodeEdge(UUIDModel):
    input_node = models.ForeignKey(FlowNode, on_delete=models.CASCADE)
    output_node = models.ForeignKey(FlowNode, on_delete=models.CASCADE)

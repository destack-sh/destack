from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class FlowManager(models.Manager):
    pass


class Flow(UUIDModel):
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)

    # nodes = models.ManyToManyField("FlowNode")

    objects = FlowManager()


class FlowNode(UUIDModel):
    flow = models.ForeignKey(Flow, on_delete=models.CASCADE)
    function = models.ForeignKey("Function", on_delete=models.RESTRICT)
    # TODO @Cleanup: define input_nodes on FlowNode as well as on FlowNodeEdge
    # input_nodes = models.ManyToManyField(
    #     "FlowNode",
    #     through="FlowNodeEdge",
    #     symmetrical=False,
    #     related_name="output_nodes",
    # )


class FlowNodeEdge(UUIDModel):
    input_node = models.ForeignKey(
        FlowNode, on_delete=models.CASCADE, related_name="output_nodes"
    )
    output_node = models.ForeignKey(
        FlowNode, on_delete=models.CASCADE, related_name="input_nodes"
    )

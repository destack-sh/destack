from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class FlowManager(models.Manager):
    pass


class Flow(UUIDModel):
    """
    A directed acyclic graph of Functions represented as Nodes connected by Edges.
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(
        max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True
    )
    created_at = models.DateTimeField(auto_now_add=True)

    objects = FlowManager()


# TODO @Feature: version Flows better
#  Storing a complete copy of the entire Flow graph for every version
#  seems both cumbersome and inefficient. Maybe only store changed nodes with pointers?
class FlowVersion(UUIDModel):
    """
    A flow version is a specific (generally) immutable snapshot of a flow.
    """

    flow = models.ForeignKey(Flow, on_delete=models.CASCADE, related_name="versions")
    version = models.CharField(max_length=256)
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True, blank=True)
    description = models.CharField(
        max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True
    )
    created_at = models.DateTimeField(auto_now_add=True)


class FlowNode(UUIDModel):
    """
    A node represents a curried variant of a registered function with given arguments,
    including any required configured "init-time" artifacts like datasets and models.

    A function may be managed by or dependent on an external Controller, which provides
    additional configuration arguments and/or augments the function config & execution.
    """

    flow = models.ForeignKey(
        FlowVersion, on_delete=models.CASCADE, related_name="nodes"
    )
    created_at = models.DateTimeField(auto_now_add=True)

    function_id = models.CharField(max_length=256)
    config_arguments = models.JSONField()
    connected_artifacts = models.ManyToManyField("Artifact", through="FlowArtifactEdge")
    depends_on_nodes = models.ManyToManyField(
        "FlowNode",
        through="FlowNodeEdge",
        through_fields=("dependent", "dependency"),  # this node is the dependent
        related_name="dependent_nodes",
        symmetrical=False,
    )
    controller = models.ForeignKey(
        "Controller", on_delete=models.RESTRICT, blank=True, null=True
    )


class FlowNodeEdge(UUIDModel):
    """
    An edge connecting two Flow Nodes in some way.
    """

    class ConnectionType(models.TextChoices):
        Argument = "argument"
        Input = "input"
        # "Output" is unnecessary because flow node connections are asymmetric.

    connection_type = models.CharField(max_length=32, choices=ConnectionType.choices)
    connection_name = models.CharField(max_length=64, null=True, blank=True)
    dependency = models.ForeignKey(FlowNode, on_delete=models.CASCADE, related_name="+")
    dependent = models.ForeignKey(FlowNode, on_delete=models.CASCADE, related_name="+")


class FlowArtifactEdge(UUIDModel):
    """
    An edge connecting a flow node to an artifact in some way.
    """

    class ConnectionType(models.TextChoices):
        Argument = "argument"
        Input = "input"
        Output = "output"

    connection_type = models.CharField(max_length=32, choices=ConnectionType.choices)
    connection_name = models.CharField(max_length=64, null=True, blank=True)
    dependent_node = models.ForeignKey(FlowNode, on_delete=models.CASCADE)
    artifact = models.ForeignKey("Artifact", on_delete=models.RESTRICT)

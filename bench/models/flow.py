from __future__ import annotations

from django.db import models, transaction

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel
from bench.models.versioning import VersionedBlob, VersionedCommit, VersionedRepository


class FlowManager(models.Manager):
    def create_flow_version_by_name(self, name: str) -> FlowVersion:
        """Creates dataset version and corresponding dataset if it doesn't exist"""
        with transaction.atomic():
            flow, _ = Flow.objects.get_or_create(name=name)
            flow_version = FlowVersion.objects.create(flow=flow)
        return flow_version


class Flow(UUIDModel, VersionedRepository):
    """
    A directed acyclic graph of Functions represented as Nodes connected by Edges.

    Flows are versioned. All versions are available in 'versions'.
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)

    objects = FlowManager()

    class Meta:
        indexes = [
            models.Index(name="bench_flow_name_idx", fields=["name"]),
        ]
        constraints = [models.UniqueConstraint(name="bench_flow_name_ak", fields=["name"])]


# TODO @Performance: version Flows on node-level
#  Storing a complete copy of the entire Flow graph for every version
#  seems both cumbersome and inefficient. But node-level versioning for graphs is
#  quite complex and the complexity does not seem worthwhile at this time.
class FlowVersion(UUIDModel, VersionedCommit):
    """
    A flow version is a specific (generally) immutable specification of a flow.
    """

    flow = models.ForeignKey(Flow, on_delete=models.CASCADE, related_name="versions")
    parents = models.ManyToManyField("FlowVersion", symmetrical=False)


class FlowNode(UUIDModel, VersionedBlob):
    """
    A node represents a curried variant of a registered function with given arguments,
    including any required configured "init-time" artifacts like datasets and models.
    """

    flow = models.ForeignKey(FlowVersion, on_delete=models.CASCADE, related_name="nodes")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    committed = models.BooleanField(default=True)

    function_id = models.CharField(max_length=256)
    config_arguments = models.JSONField()
    connected_artifacts = models.ManyToManyField("ArtifactVersion", through="FlowArtifactEdge")
    depends_on_nodes = models.ManyToManyField(
        "FlowNode",
        through="FlowNodeEdge",
        through_fields=("dependent", "dependency"),  # this node is the dependent
        related_name="dependent_nodes",
        symmetrical=False,
    )
    controller = models.ForeignKey("Controller", on_delete=models.RESTRICT, blank=True, null=True)

    @property
    def is_committed(self) -> bool:
        return self.committed


class FlowNodeEdge(UUIDModel):
    """
    An edge connecting two Flow Nodes in some way.
    """

    class ConnectionType(models.TextChoices):
        Argument = "argument"
        Input = "input"
        # "Output" is unnecessary because flow node connections are asymmetric.

    connection_type = models.CharField(max_length=32, choices=ConnectionType.choices)
    connection_name = models.CharField(max_length=64)
    dependent = models.ForeignKey(FlowNode, on_delete=models.CASCADE, related_name="+")
    dependency = models.ForeignKey(FlowNode, on_delete=models.CASCADE, related_name="+")


class FlowArtifactEdge(UUIDModel):
    """
    An edge connecting a flow node to an artifact in some way.
    """

    class ConnectionType(models.TextChoices):
        Argument = "argument"
        Input = "input"
        Output = "output"

    connection_type = models.CharField(max_length=32, choices=ConnectionType.choices)
    connection_name = models.CharField(max_length=64)
    dependent = models.ForeignKey(FlowNode, on_delete=models.CASCADE)
    dependency = models.ForeignKey("ArtifactVersion", on_delete=models.RESTRICT)

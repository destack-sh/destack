from __future__ import annotations

import copy
import secrets

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


def _generate_flow_version(nbytes: int = 4) -> str:
    return secrets.token_hex(nbytes)


# TODO @Performance: version Flows on node-level
#  Storing a complete copy of the entire Flow graph for every version
#  seems both cumbersome and inefficient. But node-level versioning for graphs is
#  quite complex and the complexity does not seem worthwhile at this time.
class FlowVersion(UUIDModel, VersionedCommit):
    """
    A flow version is a specific (generally) immutable specification of a flow.
    """

    version = models.CharField(max_length=256, default=_generate_flow_version)
    flow = models.ForeignKey(Flow, on_delete=models.CASCADE, related_name="versions")
    parents = models.ManyToManyField("FlowVersion", symmetrical=False)

    @property
    def name_version(self) -> str:
        return f"{self.flow.name}@{self.version}"

    def copy_from(self, parent: FlowVersion):
        """Copies nodes and edges from a parent version"""

        # TODO @Architecture: not sure if FlowVersion is the best place to manage versioning

        # copy nodes
        child_node_by_parent_node_id = {}
        for parent_node in parent.nodes.all():
            child_node = parent_node.shallow_copy(flow=self)
            child_node_by_parent_node_id[parent_node.id] = child_node

        # copy node edges
        child_node_edges: list[FlowNodeEdge] = []
        for parent_edge in parent.node_edges.all():
            child_edge = parent_edge.shallow_copy(
                flow=self,
                dependent=child_node_by_parent_node_id[parent_edge.dependent_id],
                dependency=child_node_by_parent_node_id[parent_edge.dependency_id],
            )
            child_node_edges.append(child_edge)

        # copy artifact edges
        child_artifact_edges: list[FlowArtifactEdge] = []
        for parent_edge in parent.artifact_edges.all():
            child_edge = parent_edge.shallow_copy(
                flow=self,
                dependent=child_node_by_parent_node_id[parent_edge.dependent_id],
                dependency=parent_edge.dependency,
            )
            child_node_edges.append(child_edge)

        FlowNode.objects.bulk_create(child_node_by_parent_node_id.values())
        FlowNodeEdge.objects.bulk_create(child_node_edges)
        FlowArtifactEdge.objects.bulk_create(child_artifact_edges)

    class Meta:
        indexes = [
            models.Index(name="bench_flow_version_idx", fields=["version"]),
        ]
        constraints = [
            models.UniqueConstraint(
                name="bench_flow_version_flow_version_ak",
                fields=["flow", "version"],
            )
        ]


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
    config_arguments = models.JSONField(default=lambda: {})
    connected_artifacts = models.ManyToManyField("ArtifactVersion", through="FlowArtifactEdge")
    depends_on_nodes = models.ManyToManyField(
        "FlowNode",
        through="FlowNodeEdge",
        through_fields=("dependent", "dependency"),  # this node is the dependent
        related_name="dependent_nodes",
        symmetrical=False,
    )
    controller = models.ForeignKey("Controller", on_delete=models.RESTRICT, blank=True, null=True)

    def __str__(self):
        return f"{self.flow.name_version}/{self.name or self.id}"

    @property
    def is_committed(self) -> bool:
        return self.committed

    def shallow_copy(self, **kwargs) -> FlowNode:
        return FlowNode(
            name=self.name,
            created_at=self.created_at,
            function_id=self.function_id,
            config_arguments=copy.copy(self.config_arguments),
            **kwargs,
        )

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_flow_version_name_ak",
                fields=["flow", "name"],
            )
        ]


class FlowNodeEdge(UUIDModel):
    """
    An edge connecting two Flow Nodes in some way.
    """

    class ConnectionType(models.TextChoices):
        Argument = "argument"
        Input = "input"
        # "Output" is unnecessary because flow node connections are asymmetric.

    flow = models.ForeignKey(FlowVersion, on_delete=models.CASCADE, related_name="node_edges")
    connection_type = models.CharField(max_length=32, choices=ConnectionType.choices)
    connection_name_dependent = models.CharField(max_length=64)
    connection_name_dependency = models.CharField(max_length=64)
    dependent = models.ForeignKey(FlowNode, on_delete=models.CASCADE, related_name="+")
    dependency = models.ForeignKey(FlowNode, on_delete=models.CASCADE, related_name="+")

    def shallow_copy(self, **kwargs) -> FlowNodeEdge:
        return FlowNodeEdge(
            connection_type=self.connection_type,
            connection_name_dependent=self.connection_name_dependent,
            connection_name_dependency=self.connection_name_dependency,
            **kwargs,
        )


class FlowArtifactEdge(UUIDModel):
    """
    An edge connecting a flow node to an artifact in some way.
    """

    class ConnectionType(models.TextChoices):
        Argument = "argument"
        Input = "input"
        Output = "output"

    flow = models.ForeignKey(FlowVersion, on_delete=models.CASCADE, related_name="artifact_edges")
    connection_type = models.CharField(max_length=32, choices=ConnectionType.choices)
    connection_name = models.CharField(max_length=64)
    dependent = models.ForeignKey(FlowNode, on_delete=models.CASCADE)
    dependency = models.ForeignKey("ArtifactVersion", on_delete=models.RESTRICT)

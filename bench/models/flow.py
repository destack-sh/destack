from __future__ import annotations

import copy
import secrets
from typing import TYPE_CHECKING

from django.db import models, transaction

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel
from bench.models.versioning import VersionedBlob, VersionedCommit, VersionedRepository

if TYPE_CHECKING:
    from bench.models import Organization


class FlowManager(models.Manager):
    def create_flow_version_by_name(self, name: str, organization: Organization) -> FlowVersion:
        """Creates dataset version and corresponding dataset if it doesn't exist"""
        with transaction.atomic():
            flow, _ = Flow.objects.get_or_create(name=name, organization=organization)
            flow_version = FlowVersion.objects.create(flow=flow)
        return flow_version


class Flow(VersionedRepository, TaggableMixin, UUIDModel):
    """
    A hierarchical graph of nested Functions represented as Instructions connected by Edges.

    Flows are versioned. All versions are available in 'versions'.
    """

    type = models.CharField(max_length=64)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)

    organization: models.ForeignKey = models.ForeignKey(
        "bench.Organization", on_delete=models.CASCADE, related_name="flows"
    )

    objects = FlowManager()

    class Meta:
        indexes = [
            models.Index(name="bench_flow_name_idx", fields=["name"]),
        ]
        constraints = [
            models.UniqueConstraint(
                name="bench_flow_organization_name_ak", fields=["organization_id", "name"]
            )
        ]


def _generate_flow_version(nbytes: int = 3) -> str:
    return secrets.token_hex(nbytes)


class FlowVersion(VersionedCommit, TaggableMixin, UUIDModel):
    """
    A flow version is a specific (generally) immutable specification of a flow.
    """

    version = models.CharField(max_length=256, default=_generate_flow_version)
    flow = models.ForeignKey(Flow, on_delete=models.CASCADE, related_name="versions")
    parents = models.ManyToManyField("FlowVersion", symmetrical=False)
    root_instruction = models.ForeignKey(
        "FlowInstruction", on_delete=models.CASCADE, related_name="flow+"
    )

    def __str__(self) -> str:
        return self.name_version

    @property
    def organization(self):
        return self.flow.organization

    @property
    def name_version(self) -> str:
        return f"{self.flow.name}@{self.version}"

    def copy_from(self, parent: FlowVersion):
        """Copies instructions and edges from a parent version"""

        # TODO @Architecture: not sure if FlowVersion is the best place to manage versioning

        # copy instructions
        child_instruction_by_parent_instruction_id = {}
        for parent_instruction in parent.instructions.all():
            child_instruction = parent_instruction.shallow_copy(flow=self)
            child_instruction_by_parent_instruction_id[parent_instruction.id] = child_instruction

        # copy artifact edges
        child_artifact_edges: list[FlowArtifactEdge] = []
        for parent_edge in parent.artifact_edges.all():
            child_edge = parent_edge.shallow_copy(
                flow=self,
                dependent=child_instruction_by_parent_instruction_id[parent_edge.dependent_id],
                dependency=parent_edge.dependency,
            )
            child_instruction_edges.append(child_edge)

        FlowInstruction.objects.bulk_create(child_instruction_by_parent_instruction_id.values())
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


class FlowInstructionFlowEdge(models.Model):
    instruction = models.ForeignKey("FlowInstruction", on_delete=models.CASCADE)
    flow = models.ForeignKey(Flow, on_delete=models.CASCADE, related_name="+")


class FlowInstruction(UUIDModel, VersionedBlob):
    """
    An instruction is a curried Python function with high level arguments like datasets, models and flows.
    """

    flow = models.ForeignKey(FlowVersion, on_delete=models.CASCADE, related_name="instructions")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    parent = models.ForeignKey(
        "FlowInstruction", on_delete=models.CASCADE, null=True, related_name="children"
    )

    function_id = models.CharField(max_length=256)
    config_arguments = models.JSONField(default=dict)
    model_arguments = models.ManyToManyField("Model", through="FlowInstructionModelEdge")
    dataset_arguments = models.ManyToManyField("Dataset", through="FlowInstructionDatasetEdge")
    flow_arguments = models.ManyToManyField("Flow", through="FlowInstructionFlowEdge")

    def __str__(self):
        return f"{self.flow.name_version}/{self.name or self.id}"

    @property
    def first_instruction(self) -> FlowInstruction:
        """Gets the first instruction in this flow, errors if there is none"""
        raise NotImplementedError

    @property
    def last_instruction(self) -> FlowInstruction:
        """Gets the last instruction in this flow, errors if there is none"""
        raise NotImplementedError()

    def get_instruction_by_name(self, name: str) -> FlowInstruction:
        """Gets an instruction by name"""
        return self.children.get(name=name)

    def shallow_copy(self, **kwargs) -> FlowInstruction:
        return FlowInstruction(
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

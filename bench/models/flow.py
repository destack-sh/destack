from __future__ import annotations

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

    def __str__(self):
        return f"{self.organization.slug}/{self.name}"

    class Meta:
        indexes = [
            models.Index(name="bench_flow_name_idx", fields=["name"]),
        ]
        constraints = [
            # check that the name is unique within the project
            models.UniqueConstraint(
                name="bench_flow_project_name_ak", fields=["project_id", "name"]
            )
        ]


class FlowVersion(VersionedCommit, TaggableMixin, UUIDModel):
    """
    A flow version is a specific (generally) immutable specification of a flow.
    """

    flow = models.ForeignKey(Flow, on_delete=models.CASCADE, related_name="versions")
    version = models.CharField(max_length=256, null=True)
    parents = models.ManyToManyField("FlowVersion", symmetrical=False)
    root_instruction = models.ForeignKey(
        "FlowInstruction", on_delete=models.CASCADE, related_name="flow+"
    )

    def __str__(self) -> str:
        return f"{self.organization.slug}/{self.name_version}"

    @property
    def organization(self):
        return self.flow.organization

    @property
    def name_version(self) -> str:
        return f"{self.flow.name}@{self.version}"

    def copy_from(self, parent: FlowVersion):
        raise NotImplementedError

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


class FlowInstruction(UUIDModel, VersionedBlob):
    """
    An instruction is a curried Python function with high level arguments like datasets, models and flows.

    Instructions implement tasks which define the interface and guide instruction compilation.
    As an (async) Python function, instructions are defined as code (either in-place or as a built-in).
    Instructions may contain and use other instructions, forming an instruction tree.
    """

    flow = models.ForeignKey(FlowVersion, on_delete=models.CASCADE, related_name="instructions")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    parent = models.ForeignKey(
        "FlowInstruction", on_delete=models.CASCADE, null=True, related_name="children"
    )
    task = models.ForeignKey(
        "bench.Task", on_delete=models.CASCADE, null=True, related_name="implementations"
    )

    # either set code_id to built-in function id or set code
    code_id = models.CharField(blank=True, null=True, max_length=256)
    code = models.CharField(blank=True, null=True)

    # parameters to/from FlowInstructionParameter
    # arguments to/from FlowInstructionArgument

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

    class Meta:
        constraints = [
            # ensure name is unique inside flow version
            models.UniqueConstraint(
                name="bench_flow_instruction_flow_name_ak",
                fields=["flow", "name"],
            ),
            # ensure either code_id or code is set
            models.CheckConstraint(
                name="bench_flow_instruction_code_id_xor_code_ck",
                check=(
                    models.Q(code_id__isnull=False, code__isnull=True)
                    | models.Q(code_id__isnull=True, code__isnull=False)
                ),
            ),
        ]


class FlowInstructionParameter(UUIDModel):
    """
    A parameter is a named argument to a function which is bound by a FlowInstructionArgument.

    Parameters are typed using ?
    """

    instruction = models.ForeignKey(
        FlowInstruction, on_delete=models.CASCADE, related_name="parameters"
    )
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    type = models.CharField(max_length=64)

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_flow_instruction_parameter_ak",
                fields=["instruction", "name"],
            )
        ]


class FlowInstructionArgument(UUIDModel):
    """
    An argument is value binding an instruction parameter.
    An argument can be one of a model, a dataset, a flow or a plain JSON value.
    """

    instruction = models.ForeignKey(
        "FlowInstruction", on_delete=models.CASCADE, related_name="arguments"
    )
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    model = models.ForeignKey("Model", on_delete=models.CASCADE, null=True, blank=True)
    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE, null=True, blank=True)
    flow = models.ForeignKey("Flow", on_delete=models.CASCADE, null=True, blank=True)
    value = models.JSONField(null=True, blank=True)

    class Meta:
        # TODO @Robustness: ensure that only one argument type is set
        constraints = []

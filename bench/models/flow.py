from __future__ import annotations

from django.db import models

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class FlowManager(models.Manager):
    pass


class Flow(TaggableMixin, UUIDModel):
    """
    A tree of nested Instructions specifying how to do something with code, data & models.

    Flows are versioned. All versions are available in 'versions'.
    """

    type = models.CharField(max_length=64)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)

    root_instruction = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="flow+"
    )

    organization = models.ForeignKey("Organization", on_delete=models.CASCADE, related_name="flows")
    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="flows+")

    objects = FlowManager()

    def __str__(self) -> str:
        return f"{self.organization.slug}/{self.project.slug}/datasets/{self.name}@{self.id}"


class Instruction(UUIDModel):
    """
    An instruction is a curried Python function with high level arguments like datasets, models and flows.

    Instructions implement tasks which define the interface and guide instruction compilation.
    As an (async) Python function, instructions are defined as code (either in-place or as a built-in).
    Instructions may contain and use other instructions, forming an instruction tree.
    """

    flow = models.ForeignKey(Flow, on_delete=models.CASCADE, related_name="instructions")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    parent = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="children"
    )
    task = models.ForeignKey(
        "Task", on_delete=models.CASCADE, null=True, related_name="implementations"
    )

    # either set code_id to built-in function id or set code
    code_id = models.CharField(blank=True, null=True, max_length=256)
    code = models.TextField(blank=True, null=True)

    # parameters to/from InstructionParameter
    # arguments to/from InstructionArgument

    def __str__(self):
        return f"{self.flow}/{self.name or self.id}"

    @property
    def first_instruction(self) -> Instruction:
        """Gets the first instruction in this flow, errors if there is none"""
        raise NotImplementedError

    @property
    def last_instruction(self) -> Instruction:
        """Gets the last instruction in this flow, errors if there is none"""
        raise NotImplementedError()

    def get_instruction_by_name(self, name: str) -> Instruction:
        """Gets an instruction by name"""
        return self.children.get(name=name)

    class Meta:
        constraints = [
            # ensure name is unique inside flow version
            models.UniqueConstraint(
                name="bench_instruction_flow_name_ak",
                fields=["flow", "name"],
            ),
            # ensure either code_id or code is set
            models.CheckConstraint(
                name="bench_instruction_code_id_xor_code_ck",
                check=(
                    models.Q(code_id__isnull=False, code__isnull=True)
                    | models.Q(code_id__isnull=True, code__isnull=False)
                ),
            ),
        ]


class InstructionParameter(UUIDModel):
    """
    A parameter is a named argument to a function which is bound by a InstructionArgument.

    Parameters are typed using ?
    """

    instruction = models.ForeignKey(
        Instruction, on_delete=models.CASCADE, related_name="parameters"
    )
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    type = models.JSONField()

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_instruction_parameter_ak",
                fields=["instruction", "name"],
            )
        ]


class InstructionArgument(UUIDModel):
    """
    An argument is value binding an instruction parameter.
    An argument can be one of a model, a dataset, a flow or a plain JSON value.
    """

    instruction_bound = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="arguments"
    )
    instruction_free = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="+"
    )
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    model = models.ForeignKey("Model", on_delete=models.CASCADE, null=True, blank=True)
    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE, null=True, blank=True)
    flow = models.ForeignKey("Flow", on_delete=models.CASCADE, null=True, blank=True)
    value = models.JSONField(null=True, blank=True)

    class Meta:
        constraints = [
            # TODO @Robustness: ensure that only one argument value type is set
            # ensure that either instruction_bound or instruction_free is set
            models.CheckConstraint(
                name="bench_instruction_argument_bound_free_ck",
                check=(
                    models.Q(instruction_bound__isnull=False, instruction_free__isnull=True)
                    | models.Q(instruction_bound__isnull=True, instruction_free__isnull=False)
                ),
            ),
            # ensure that instruction bound/free can only be bound once per name
            # (two separate constraints because of the OR on nullable instruction_bound/instruction_free)
            models.UniqueConstraint(
                name="bench_instruction_argument_bound_name_ak",
                fields=["instruction_bound", "name"],
            ),
            models.UniqueConstraint(
                name="bench_instruction_argument_free_name_ak",
                fields=["instruction_free", "name"],
            ),
        ]

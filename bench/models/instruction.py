from __future__ import annotations

from typing import Callable

from django.db import models

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel


class InstructionScope(models.TextChoices):
    """
    The scope of instruction defines its semantics.

    Programs are top-level deployable instructions with only values as free parameters.
    Functions are reusable instructions for pure functions with any parameters & arguments.
    Generators are reusable instructions for pure generators with any parameters & arguments.
    """

    PROGRAM = "program", "Program"
    FUNCTION = "function", "Function"
    GENERATOR = "generator", "Generator"


class Instruction(TaggableMixin, UUIDModel):
    """
    Instructions specify how to do something using datasets, models and other instructions.
    Like in software, Instructions form a tree and are implemented as code with some syntactic sugar.

    Instructions define and implement tasks which define the interface and guide instruction compilation.
    As an (async) Python function, instructions are defined as code (either in-place or as a built-in).
    Instructions may contain and use other instructions, forming an instruction tree.
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    parent = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="children"
    )
    task = models.ForeignKey(
        "Task", on_delete=models.CASCADE, null=True, related_name="implementations"
    )

    scope: models.CharField = models.CharField(
        max_length=64, choices=InstructionScope.choices, default=InstructionScope.FUNCTION
    )
    # either set builtin id or set custom code
    builtin_id = models.CharField(blank=True, null=True, max_length=256)
    code = models.TextField(blank=True, null=True)

    # parameters to/from InstructionParameter
    # arguments to/from InstructionArgument

    def __str__(self):
        return f"{self.name}.instruct@{self.id.hex}"

    @property
    def anonymous(self) -> bool:
        """Whether this instruction is defined as anonymous code ir with a defined function (same name)"""
        if self.builtin_id is not None:
            # builtins are always directly callable
            return False
        else:
            # TODO @Robustness: check anonymous vs defined functions in a more general way
            return f"def {self.name}(" not in self.code

    class Meta:
        constraints = [
            # ensure either builtin_id or code is set
            models.CheckConstraint(
                name="bench_instruction_code_id_xor_code_ck",
                check=(models.Q(builtin_id__isnull=False) ^ models.Q(code__isnull=False)),
            ),
        ]


class InstructionParameterType(models.TextChoices):
    DATASET = "dataset"
    MODEL = "model"
    INSTRUCTION = "instruction"
    JSON = "json"

    @staticmethod
    def from_obj(obj) -> InstructionParameterType:
        from bench.backend.base import ModelHandle
        from bench.models import Dataset, Model
        from bench.utils.record import RecordBatch

        if isinstance(obj, (Dataset, RecordBatch)):
            return InstructionParameterType.DATASET
        elif isinstance(obj, (Model, ModelHandle)):
            return InstructionParameterType.MODEL
        elif isinstance(obj, (Instruction, Callable)):
            return InstructionParameterType.INSTRUCTION
        else:
            return InstructionParameterType.JSON


class InstructionParameter(UUIDModel):
    """
    A parameter is a named argument to a function which is bound by a InstructionArgument.

    Parameters are typed using ?
    """

    instruction = models.ForeignKey(
        Instruction, on_delete=models.CASCADE, related_name="parameters"
    )
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    type = models.CharField(max_length=64, choices=InstructionParameterType.choices)
    schema = models.JSONField(null=True)

    def __str__(self):
        return f"{self.name}:{self.type}.param@{self.id.hex}"

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
    An argument can be a model, a dataset, an instruction or a plain JSON value.
    """

    instruction_bound = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="arguments"
    )
    instruction_free = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="+"
    )
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    type = models.CharField(max_length=64, choices=InstructionParameterType.choices)
    model = models.ForeignKey("Model", on_delete=models.CASCADE, null=True, blank=True)
    model_settings = models.ForeignKey(
        "ModelInferenceSettings", on_delete=models.CASCADE, null=True, blank=True
    )
    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE, null=True, blank=True)
    dataset_view = models.ForeignKey("DatasetView", on_delete=models.CASCADE, null=True, blank=True)
    instruction = models.ForeignKey("Instruction", on_delete=models.CASCADE, null=True, blank=True)
    value = models.JSONField(null=True, blank=True)

    def __str__(self):
        return f"{self.name}:{self.type}.arg@{self.id.hex}"

    class Meta:
        constraints = [
            # TODO @Robustness: ensure that only one argument value type is set
            # ensure that either instruction_bound or instruction_free is set
            models.CheckConstraint(
                name="bench_instruction_argument_bound_free_ck",
                check=(
                    models.Q(instruction_bound__isnull=True)
                    ^ models.Q(instruction_free__isnull=True)
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

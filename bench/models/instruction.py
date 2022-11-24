from __future__ import annotations

import traceback
from contextlib import asynccontextmanager, contextmanager
from datetime import datetime, timezone
from typing import Any, Optional

from asgiref.sync import sync_to_async
from django.db import models, transaction

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel, UUIDTModel


class InstructionScope(models.TextChoices):
    """
    The scope of instruction defines its semantics.

    Modules are container for programs and functions.
    Programs are top-level deployable instructions with only values as free parameters.
    Functions are reusable instructions for pure functions with any parameters & arguments.
    """

    MODULE = "module", "Module"
    PROGRAM = "program", "Program"
    FUNCTION = "function", "Function"


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
    index = models.IntegerField(default=0)
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

    def add_parameter(
        self, name: str, type: InstructionParameterType, exists_ok: bool = False
    ) -> InstructionParameter:
        if exists_ok:
            parameter, created = InstructionParameter.objects.get_or_create(
                instruction=self, name=name, defaults={"type": type}
            )
            if not created:
                parameter.type = type
                parameter.save()
            return parameter
        else:
            return InstructionParameter.objects.create(instruction=self, name=name, type=type)

    async def aadd_parameter(
        self, name: str, type: InstructionParameterType, exists_ok: bool = False
    ) -> InstructionParameter:
        return await sync_to_async(self.add_parameter)(name, type, exists_ok)

    @transaction.atomic
    def bind_argument(
        self, name: str, value: Any, exists_ok: bool = False
    ) -> tuple[InstructionParameter, InstructionArgument]:
        if value is None:
            raise ValueError(f"cannot bind {self} argument {name} to None")
        argument_type = InstructionParameterType.from_obj(value)
        if argument_type == InstructionParameterType.DATASET:
            argument_kwargs = {"dataset": value}
        elif argument_type == InstructionParameterType.MODEL:
            argument_kwargs = {"model": value}
        elif argument_type == InstructionParameterType.JSON:
            argument_kwargs = {"value": value}
        else:
            raise RuntimeError(f"unsupported argument type {argument_type}")

        # get/create parameter and corresponding argument
        parameter = self.add_parameter(name, argument_type, exists_ok=True)
        argument, created = InstructionArgument.objects.get_or_create(
            instruction_bound=self, name=name, defaults=dict(type=argument_type, **argument_kwargs)
        )
        if not created and not exists_ok:
            raise RuntimeError(f"{self} argument {argument} already exists")
        elif not created:
            # update parameter type and kwargs
            argument.type = argument_type
            for key, value in argument_kwargs.items():
                setattr(argument, key, value)
            argument.save()

        return parameter, argument

    async def abind_argument(self, name: str, value: Any, exists_ok: bool = False):
        return await sync_to_async(self.bind_argument)(name=name, value=value, exists_ok=exists_ok)

    @property
    def anonymous(self) -> bool:
        """Whether this instruction is defined as anonymous code ir with a defined function (same name)"""
        if self.builtin_id is not None:
            # builtins are always directly callable
            return False
        elif self.code is not None:
            # TODO @Robustness: check anonymous vs defined functions in a more general way
            return f"def {self.name}(" not in self.code
        else:
            raise ValueError(f"instruction {self} must have either builtin_id or code")

    class Meta:
        constraints = [
            # ensure either builtin_id or code is set
            models.CheckConstraint(
                name="bench_instruction_code_id_xor_code_ck",
                check=(models.Q(builtin_id__isnull=False) ^ models.Q(code__isnull=False)),
            ),
            # ensure index into parent is unique
            models.UniqueConstraint(
                name="bench_instruction_parent_index_uk",
                fields=["parent", "index"],
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
        elif isinstance(obj, Instruction) or callable(obj):
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
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
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
    An argument is value bound to an instruction, usually to an instruction parameter.
    If there is no corresponding parameter the argument is an anonymous import.
    An argument can be a model, a dataset, an instruction or a plain JSON value.
    """

    instruction_bound = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="arguments"
    )
    instruction_free = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="+"
    )
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
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


class ExecutionStatus(models.TextChoices):
    Created = "created"
    Scheduled = "scheduled"
    Queued = "queued"
    Running = "running"
    Aborting = "aborting"
    # terminal statuses
    Aborted = "aborted"
    Failed = "failed"
    Completed = "completed"


TERMINAL_STATUSES = {ExecutionStatus.Aborted, ExecutionStatus.Failed, ExecutionStatus.Completed}
PENDING_STATUSES = set(ExecutionStatus) - TERMINAL_STATUSES


class Execution(UUIDTModel):
    """
    The execution of a hierarchical instruction.
    """

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    started_at = models.DateTimeField(
        blank=True, null=True, help_text="Time of transition to RUNNING status."
    )
    terminated_at = models.DateTimeField(
        blank=True, null=True, help_text="Time of transition to a terminal status."
    )
    status = models.CharField(
        max_length=32, choices=ExecutionStatus.choices, default=ExecutionStatus.Created
    )
    metadata = models.JSONField(null=True, blank=True)

    parent = models.ForeignKey(
        "Execution", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    instruction = models.ForeignKey(
        "Instruction",
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        related_name="executions",
    )
    model = models.ForeignKey(
        "Model", null=True, blank=True, on_delete=models.SET_NULL, related_name="executions"
    )
    model_inference = models.ForeignKey(
        "ModelInference",
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        related_name="executions",
    )

    def _set_transition_metadata(
        self, status: ExecutionStatus, transition_metadata: Optional[dict]
    ):
        if transition_metadata is None:
            return
        if self.metadata is None:
            self.metadata = {}
        self.metadata[status.value] = transition_metadata

    def update_status(self, status: ExecutionStatus, transition_metadata: Optional[dict] = None):
        self.status = status
        self._set_transition_metadata(status, transition_metadata)
        self.save()

    def start(
        self,
        status: ExecutionStatus = ExecutionStatus.Running,
        transition_metadata: Optional[dict] = None,
    ):
        """
        Marks this execution as started in the given status
        """
        self.started_at = datetime.utcnow().astimezone(tz=timezone.utc)
        self.status = status
        self._set_transition_metadata(status, transition_metadata)
        self.save()

    def terminate(
        self,
        status: ExecutionStatus = ExecutionStatus.Completed,
        transition_metadata: Optional[dict] = None,
    ):
        """
        Marks this execution as terminated in the given status
        """
        self.terminated_at = datetime.utcnow().astimezone(tz=timezone.utc)
        self.status = status
        self._set_transition_metadata(status, transition_metadata)
        self.save()

    @contextmanager
    def capture(self, start: bool = True, start_metadata: Optional[dict] = None):
        try:
            if start:
                self.start(transition_metadata=start_metadata)
            yield
            self.terminate()
        except Exception as e:
            stacktrace = traceback.format_stack()
            self.terminate(
                status=ExecutionStatus.Failed,
                transition_metadata={"error": str(e), "stacktrace": stacktrace},
            )
            raise

    @asynccontextmanager
    async def acapture(self, start: bool = True, start_metadata: Optional[dict] = None):
        try:
            if start:
                await sync_to_async(self.start)(transition_metadata=start_metadata)
            yield
            await sync_to_async(self.terminate)()
        except Exception as e:
            stacktrace = traceback.format_stack()
            await sync_to_async(self.terminate)(
                status=ExecutionStatus.Failed,
                transition_metadata={"error": str(e), "stacktrace": stacktrace},
            )
            raise

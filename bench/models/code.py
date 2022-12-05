from __future__ import annotations

import traceback
from contextlib import asynccontextmanager, contextmanager
from datetime import datetime, timezone
from typing import Any, Optional, cast
from uuid import UUID

from asgiref.sync import sync_to_async
from django.db import models, transaction
from django_choices_field import TextChoicesField

from bench.models.schema import SchemaElementField, SchemaField
from bench.models.symbol import SymbolContent, SymbolContentManager, SymbolDefinition, SymbolType
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel, UUIDTModel, is_jsonable
from bench.utils.schema import SchemaElement


class CodeManager(SymbolContentManager, models.Manager["Code"]):
    pass


class Code(SymbolContent):
    """
    Code specifies how to do something using datasets, models and other code.
    Code is just async Python code (either defined in-place or as a built-in).
    """

    input_schema = SchemaField("input")
    output_schema = SchemaElementField("output")
    # either set builtin id or set custom code
    builtin_id = models.CharField(blank=True, null=True, max_length=256)
    code = models.TextField(blank=True, null=True)
    code_function_name = models.CharField(blank=True, null=True, max_length=256)
    task = models.ForeignKey(
        "Task", on_delete=models.CASCADE, null=True, related_name="implementations"
    )
    # parameters to/from CodeParameter
    # arguments to/from CodeArgument

    def deepcopy(self, to: Code, refs: dict[UUID, SymbolDefinition | SymbolContent]):
        super().deepcopy(to, refs)
        # copy parameters
        for parameter in self.parameters.all():
            parameter.id = None
            parameter.code = to
            parameter.save()
        # copy arguments
        for argument in self.arguments.all():
            if argument.reference_id not in refs:
                continue
            argument.id = None
            argument.code = to
            argument.reference = cast(SymbolDefinition, refs[argument.reference_id])
            argument.save()

    def __str__(self):
        if self.builtin_id:
            content = f"builtin={self.builtin_id}"
        elif self.code_function_name and self.code:
            content = f"function={self.code_function_name},chars={len(self.code)},lines={len(self.code.splitlines())}"
        elif self.code:
            content = f"length={len(self.code)}"
        else:
            raise ValueError(f"code has no content: {self}")
        return f"{self.definition_str}({content},{self.input_schema}->{self.output_schema})"

    def add_parameter(
        self,
        name: str,
        type: CodeParameterType,
        exists_ok: bool = False,
        schema: Optional[SchemaElement] = None,
    ) -> CodeParameter:
        if exists_ok:
            parameter, created = CodeParameter.objects.get_or_create(
                code=self, name=name, defaults={"type": type, "schema": schema}
            )
            if not created:
                parameter.type = type
                parameter.schema = schema
                parameter.save()
            return parameter
        else:
            return CodeParameter.objects.create(code=self, name=name, type=type, schema=schema)

    async def aadd_parameter(
        self,
        name: str,
        type: CodeParameterType,
        exists_ok: bool = False,
        schema: Optional[dict] = None,
    ) -> CodeParameter:
        return await sync_to_async(self.add_parameter)(name, type, exists_ok, schema)

    @transaction.atomic
    def bind_argument(
        self, name: str, value: Any | SymbolDefinition, exists_ok: bool = False
    ) -> tuple[CodeParameter, CodeArgument]:
        if value is None:
            raise ValueError(f"cannot bind {self} argument {name} to None")
        # get/create parameter and corresponding argument
        argument_type = CodeParameterType.from_value(value)
        if argument_type == CodeParameterType.VALUE:
            value, reference = value, None
        else:
            value, reference = None, value

        parameter = self.add_parameter(name, argument_type, exists_ok=True)
        argument, created = CodeArgument.objects.get_or_create(
            code=self,
            name=name,
            defaults=dict(type=argument_type, value=value, reference=reference),
        )
        if not created and not exists_ok:
            raise RuntimeError(f"{self} argument {argument} already exists")
        elif not created:
            # update parameter type and kwargs
            argument.type = argument_type
            argument.value = value
            argument.reference = reference
            argument.save()

        return parameter, argument

    def bind_arguments(self, exists_ok: bool = False, **arguments: Any | SymbolDefinition):
        for name, value in arguments.items():
            self.bind_argument(name, value, exists_ok)

    async def abind_argument(
        self, name: str, value: Any | SymbolDefinition, exists_ok: bool = False
    ):
        return await sync_to_async(self.bind_argument)(name=name, value=value, exists_ok=exists_ok)

    @property
    def anonymous(self) -> bool:
        """Whether this code is defined as anonymous code ir with a defined function (same name)"""
        if self.builtin_id is not None:
            # builtins are always directly callable
            return False
        elif self.code is not None:
            return self.code_function_name is None
        else:
            raise ValueError(f"code {self} must have either builtin_id or code")

    objects = CodeManager()

    class Meta(SymbolContent.Meta):
        constraints = [
            # ensure either builtin_id or code is set
            models.CheckConstraint(
                name="bench_code_builtin_id_xor_code_ck",
                check=(models.Q(builtin_id__isnull=False) ^ models.Q(code__isnull=False)),
            ),
        ]


class CodeParameterType(models.TextChoices):
    DATA = "dataset"
    MODEL = "model"
    CODE = "code"
    VALUE = "value"

    @staticmethod
    def from_value(obj: Any | SymbolDefinition) -> CodeParameterType:
        if isinstance(obj, SymbolDefinition):
            if obj.type == SymbolType.DATASET or obj.type == SymbolType.DATASET_VIEW:
                return CodeParameterType.DATA
            elif obj.type == SymbolType.MODEL:
                return CodeParameterType.MODEL
            elif obj.type == SymbolType.CODE:
                return CodeParameterType.CODE
            else:
                raise ValueError(f"unexpected symbol type for code parameter: {obj}")
        elif is_jsonable(obj):
            return CodeParameterType.VALUE
        else:
            raise ValueError(f"unknown object {obj} to code parameter")


class CodeParameter(UUIDModel):
    """
    A parameter is a named argument to a function which is bound by a CodeArgument.
    Parameters are typed using SchemaElements.
    """

    code = models.ForeignKey(Code, on_delete=models.CASCADE, related_name="parameters")
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    type = TextChoicesField(choices_enum=CodeParameterType)
    schema = SchemaElementField(null=True, blank=True)  # models need not have a schema

    def __str__(self):
        return f"{self.code}/parameters/{self.name}(type={self.type})"

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_code_parameter_ak",
                fields=["code", "name"],
            )
        ]


class CodeArgument(UUIDModel):
    """
    An argument is bound value to some code, usually associated with a parameter.
    An argument can be a reference to a symbol or a specific value.

    Note: if there is no corresponding parameter the argument is an anonymous import. This
     doesn't seem perfect, but works for now.
    """

    code = models.ForeignKey("code", on_delete=models.CASCADE, null=True, related_name="arguments")
    code_free = models.ForeignKey("code", on_delete=models.CASCADE, null=True, related_name="+")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    type = models.CharField(max_length=64, choices=CodeParameterType.choices)
    reference = models.ForeignKey("SymbolDefinition", on_delete=models.CASCADE, null=True)
    value = models.JSONField(null=True, blank=True)

    def __str__(self):
        return f"{self.code}/arguments/{self.name}(type={self.type})"

    class Meta:
        constraints = [
            # TODO @Robustness: ensure that only one argument value type is set
            # ensure that either code or code_free is set
            models.CheckConstraint(
                name="bench_code_argument_bound_free_ck",
                check=(models.Q(code__isnull=True) ^ models.Q(code_free__isnull=True)),
            ),
            # ensure that code bound/free can only be bound once per name
            # (two separate constraints because of the OR on nullable code/code_free)
            models.UniqueConstraint(
                name="bench_code_argument_bound_name_ak",
                fields=["code", "name"],
            ),
            models.UniqueConstraint(
                name="bench_code_argument_free_name_ak",
                fields=["code_free", "name"],
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
    The execution of a hierarchical code.
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
    code = models.ForeignKey(
        "code",
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

from __future__ import annotations

import asyncio
import enum
import traceback
import uuid
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Callable, ClassVar, Coroutine, Optional
from uuid import UUID

import PIL.Image

from bench.language.parse import ModuleIndex
from bench.language.type import (
    Build,
    Code,
    Dataset,
    InterpSymbol,
    LiteralValue,
    Model,
    Module,
    Record,
    SymbolType,
    Task,
    Type,
    TypeNode,
    Value,
    XBlock,
)
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType
from bench.utils.record import RecordBatch
from bench.utils.utils import required_field
from bench.utils.uuidt import UUIDT

AsyncCodeCallable = Callable[..., Coroutine]
SyncCodeCallable = Callable[..., Any]


@dataclass(repr=False)
class ModuleInstance:
    module: Module
    index: ModuleIndex


@dataclass(repr=False)
class SymbolInstance:
    build: Optional[Build] = None

    @property
    def symbol_type(self):
        return SYMBOL_TYPE_BY_INSTANCE_CLASS[self.__class__]

    @property
    def py_handle(self) -> Any:
        raise NotImplementedError


@dataclass(repr=False)
class TaskInstance(SymbolInstance, Task):
    implementation: CodeInstance = required_field()

    @property
    def py_handle(self) -> Any:
        return self.implementation.py_handle


@dataclass(repr=False)
class TypeInstance(SymbolInstance, Type):
    py_type: Any = required_field()

    @property
    def py_handle(self) -> Any:
        return self.py_type


@dataclass(repr=False)
class DatasetInstance(SymbolInstance, Dataset):
    records_batch: RecordBatch = required_field()

    @property
    def py_handle(self) -> RecordBatch:
        return self.records_batch


@dataclass(repr=False)
class ValueInstance(SymbolInstance, Value):
    @property
    def py_handle(self):
        return self.value


@dataclass(repr=False)
class ModelInstance(SymbolInstance, Model):
    inference: ModelInference = required_field()

    @property
    def py_handle(self):
        return self.inference


@dataclass(repr=False)
class CodeInstance(SymbolInstance, Code):
    task: Optional[TaskInstance] = None
    transformed_code: str = required_field()
    code_callable: SyncCodeCallable | AsyncCodeCallable = required_field()
    is_async: bool = required_field()

    @property
    def py_handle(self) -> SyncCodeCallable | AsyncCodeCallable:
        return self.code_callable


SYMBOL_TYPE_BY_INSTANCE_CLASS = {
    TaskInstance: SymbolType.TASK,
    TypeInstance: SymbolType.TYPE,
    DatasetInstance: SymbolType.DATA,
    ValueInstance: SymbolType.VALUE,
    ModelInstance: SymbolType.MODEL,
    CodeInstance: SymbolType.CODE,
}

#
# Executions
#


@dataclass(slots=True)
class ExecutionFrame:
    id: UUID
    module_id: UUID
    build: Optional[Build]
    task: Optional[TaskInstance]
    code: Optional[CodeInstance]
    model: Optional[ModelInstance]
    root: Optional[ExecutionFrame]
    parent: Optional[ExecutionFrame]
    inference_id: Optional[UUID]
    entered_at: datetime
    exited_at: Optional[datetime]
    inputs: Optional[dict[str, LiteralValue]]
    outputs: Optional[LiteralValue]
    error: Optional[Exception]
    queue_position: Optional[int]

    def __str__(self):
        # get str of all non-null fields
        fields_strs = [
            f"module={self.module_id}",
            f"build={self.build}" if self.build else None,
            f"task={self.task}" if self.task else None,
            f"code={self.code}" if self.code else None,
            f"model={self.model}" if self.model else None,
            f"root={self.root.id}" if self.root else None,
            f"parent={self.parent.id}" if self.parent else None,
            f"inference_context={self.inference_id}" if self.inference_id else None,
            f"entered={self.entered_at}",
            f"exited={self.exited_at}" if self.exited_at else None,
            f"inputs={summarize_args(self.inputs)}",
            f"outputs={summarize_args(self.outputs)}" if self.outputs else None,
            f"error={self.error}" if self.error else None,
            f"queue_position={self.queue_position}" if self.queue_position else None,
        ]
        fields_str = [s for s in fields_strs if s]
        return f"id={self.id} ({', '.join(fields_str)})"

    def __repr__(self):
        return f"<ExecutionFrame {self}>"


@dataclass(slots=True)
class ErrorData:
    """Wire-able representation of an exception."""

    type: str
    message: str
    traceback: list[str]


@dataclass(slots=True)
class ExecutionFrameData:
    """Wire-able representation of an execution frame."""

    id: UUID
    module_id: UUID
    build_id: Optional[UUID]
    task_id: Optional[UUID]
    code_id: Optional[UUID]
    model_id: Optional[UUID]
    root_id: Optional[UUID]
    parent_id: Optional[UUID]
    inference_id: Optional[UUID]
    entered_at: datetime
    exited_at: Optional[datetime]
    inputs: dict[str, Any]
    outputs: Optional[Any]
    error: Optional[ErrorData]
    queue_position: Optional[int]
    # additional context data not in ExecutionFrame
    project_id: UUID
    tracing_level: Optional[ExecutionTracingLevel]
    deployment_id: UUID
    trigger_type: Optional[ExecutionTriggerType]
    trigger_id: Optional[UUID]

    @staticmethod
    def from_frame(
        frame: ExecutionFrame,
        *,
        project_id: UUID,
        tracing_level: ExecutionTracingLevel,
        deployment_id: UUID,
        trigger_type: Optional[ExecutionTriggerType] = None,
        trigger_id: Optional[UUID] = None,
    ) -> ExecutionFrameData:
        if frame.error:
            error_data = ErrorData(
                type=type(frame.error).__name__,
                message=str(frame.error),
                traceback=traceback.format_exception(
                    type(frame.error), frame.error, frame.error.__traceback__
                ),
            )
        else:
            error_data = None
        return ExecutionFrameData(
            id=frame.id,
            module_id=frame.module_id,
            build_id=frame.build.id if frame.build else None,
            task_id=frame.task.id if frame.task else None,
            code_id=frame.code.id if frame.code else None,
            model_id=frame.model.id if frame.model else None,
            root_id=frame.root.id if frame.root else None,
            parent_id=frame.parent.id if frame.parent else None,
            entered_at=frame.entered_at,
            exited_at=frame.exited_at,
            inputs=frame.inputs,
            outputs=frame.outputs,
            inference_id=frame.inference_id,
            error=error_data,
            queue_position=frame.queue_position,
            project_id=project_id,
            tracing_level=tracing_level,
            deployment_id=deployment_id,
            trigger_type=trigger_type,
            trigger_id=trigger_id,
        )


#
# Building
#


class BuildCandidateStatus(enum.StrEnum):
    Planned = "planned"
    Building = "building"
    Evaluating = "evaluating"
    CompletedWon = "completed_won"
    CompletedAbandoned = "completed_abandoned"
    Cancelled = "cancelled"


@dataclass(slots=True)
class BuildCandidateData:
    id: UUID
    build_id: UUID
    status: BuildCandidateStatus
    name: str
    evaluation_id: Optional[UUID]
    job_id: Optional[UUID]
    file_id: Optional[UUID]
    order_key: str
    # additional context
    project_id: UUID
    project_version_id: UUID


#
# Evaluation
#


class EvaluationMetric(enum.StrEnum):
    # Summary (global and build specific)
    Clarity = "clarity"  # [0, 1]
    Difficulty = "difficulty"  # [0, inf)
    Performance = "performance"  # [0, 1]
    Speed = "speed"  # [0, inf) (inverse of estimated run duration)
    # Clarity (global)
    InstructionPerplexity = "instruction_perplexity"  # [0, 1]
    InstructionAgreement = "instruction_agreement"  # [0, 1]
    InstructionOverlap = "instruction_overlap"  # [0, 1]
    # Difficulty (global)
    NodesCount = "nodes_count"  # [0, inf)
    StepsCount = "steps_count"  # [0, inf)
    # Difficulty (build specific?)
    InferencesCount = "inferences_count"  # [0, inf)
    TokensCount = "tokens_count"  # [0, inf)
    # Performance (build specific)
    TypeValidity = "type_validity"  # [0, 1]
    InstructionSatisfaction = "instruction_satisfaction"  # [0, 1]
    FeedbackCorrelation = "feedback_correlation"  # [-1, 1]
    # Speed (build specific)
    EstimatedRunDuration = "estimated_run_duration"  # [0, inf)
    AverageRunDuration = "average_run_duration"  # [0, inf)


class EvaluationKind(enum.StrEnum):
    EVALUATION = "evaluation"
    LINT = "lint"


class EvaluationScope(enum.StrEnum):
    INSTRUCTION = "node"
    BUILD = "build"
    MODULE = "module"


@dataclass(repr=False, slots=True)
class EvaluationResult:
    kind: EvaluationKind
    scope: EvaluationScope
    aggregated_metrics: dict[str, float]
    self_metrics: Optional[dict[str, float]] = None
    system: Optional[InterpSymbol | TypeNode | Record] = None
    build: Optional[Build] = None
    build_candidate: Optional[Any] = None  # can't refer to BuildCandidate here
    id: UUID = field(default_factory=uuid.uuid4)
    children: list["EvaluationResult"] = field(default_factory=list)

    def __str__(self):
        metrics_str = ", ".join(
            f"{k}: {self.aggregated_metrics[k]:0.02f}" for k, v in self.aggregated_metrics.items()
        )
        return f"{self.kind} {self.scope} {metrics_str}"

    def __repr__(self):
        return f"<Evaluation {self.system} {self}>"


@dataclass(slots=True)
class EvaluationResultData:
    id: UUID
    kind: EvaluationKind
    scope: EvaluationScope
    aggregated_metrics: dict[str, float]
    self_metrics: Optional[dict[str, float]]
    statement_id: Optional[UUID]
    type_node_id: Optional[UUID]
    record_id: Optional[UUID]
    build_id: Optional[UUID]
    build_candidate_id: Optional[UUID]
    parent_id: Optional[UUID]
    # additional context
    project_id: UUID
    project_version_id: UUID
    job_id: Optional[UUID]

    @staticmethod
    def from_result(
        result: EvaluationResult,
        *,
        project_id: UUID,
        project_version_id: UUID,
        job_id: Optional[UUID],
        parent_id: Optional[UUID] = None,
    ) -> list[EvaluationResultData]:
        """Flattens an EvaluationResult tree into a list of EvaluationResultData (recursively)."""
        result_data = EvaluationResultData(
            id=result.id,
            kind=result.kind,
            scope=result.scope,
            aggregated_metrics=result.aggregated_metrics,
            self_metrics=result.self_metrics,
            statement_id=result.system.id if isinstance(result.system, InterpSymbol) else None,
            type_node_id=result.system.id if isinstance(result.system, TypeNode) else None,
            record_id=result.system.id if isinstance(result.system, Record) else None,
            build_id=result.build.id if result.build else None,
            build_candidate_id=result.build_candidate.id if result.build_candidate else None,
            project_id=project_id,
            project_version_id=project_version_id,
            job_id=job_id,
            parent_id=parent_id,
        )
        results_data = [result_data]
        for child in result.children:
            results_data += EvaluationResultData.from_result(
                child,
                project_id=project_id,
                project_version_id=project_version_id,
                job_id=job_id,
                parent_id=result.id,
            )
        return results_data


#
# Model inference
#

# Ideally, endpoint settings should be 1) extensible and 2) types in the std lib.
# For now, we just use internal dataclasses. :TypeSafeSettings


class IncapableError(NotImplementedError):
    pass


@dataclass
class TextGenerationSettings:
    temperature: float
    max_tokens: int
    top_p: Optional[float]
    stop: Optional[list[str]] = field(default_factory=list)
    logit_bias: Optional[dict[str, float]] = field(default_factory=dict)


@dataclass
class EmbeddingSettings:
    pass


@dataclass
class ImageGenerationSettings:
    seed: int
    steps: int
    width: int
    height: int
    cfg_scale: float


class Modality(enum.StrEnum):
    """Core modality capabilities of a model."""

    GenerateText = "generate_text"  # any -> text
    GenerateImage = "generate_image"  # any -> image
    Embed = "embed"  # any -> embedding


class ModelInference:
    """Generic model with an endpoint for each core modality."""

    async def generate_text(
        self, input: list[XBlock], settings: TextGenerationSettings
    ) -> str | list[str]:
        raise IncapableError()

    async def generate_image(
        self, input: list[XBlock], settings: ImageGenerationSettings
    ) -> PIL.Image | list[PIL.Image]:
        raise IncapableError()

    async def embed(self, input: list[XBlock], settings: EmbeddingSettings) -> list[float]:
        raise IncapableError()


BASE_SETTINGS_BY_MODALITY = {
    Modality.GenerateText: TextGenerationSettings,
    Modality.GenerateImage: ImageGenerationSettings,
    Modality.Embed: EmbeddingSettings,
}


def summarize_args(arguments: Any) -> str:
    """
    Summarize the names (if available) and types of arguments.
    """
    if isinstance(arguments, dict):
        return ", ".join(f"{name}={type(value).__name__}" for name, value in arguments.items())
    elif isinstance(arguments, (list, tuple, set)):
        return ", ".join(type(value).__name__ for value in arguments)
    else:
        return type(arguments).__name__


BuildMap = Callable[[InterpSymbol], Optional[InterpSymbol]]

#
# Jobs
#


class JobType(enum.StrEnum):
    INTERP = "interp"
    GENERATE = "generate"
    BUILD = "build"
    EVALUATE = "evaluate"
    LINT = "lint"
    RUN = "run"


class JobStatus(enum.StrEnum):
    Queued = "queued"
    Running = "running"
    Completed = "completed"
    Cancelling = "cancelling"
    Cancelled = "cancelled"
    Failed = "failed"


# lower is higher
JOB_DEFAULT_PRIORITY = {
    JobType.INTERP: 0,
    JobType.RUN: 0,
    JobType.GENERATE: 1,
    JobType.BUILD: 2,
    JobType.EVALUATE: 3,
    JobType.LINT: 4,
}


@dataclass(repr=False)
class Job:
    type: ClassVar[JobType]
    project_id: UUID
    project_version_id: UUID
    deployment_id: Optional[UUID]
    id: UUID = field(default_factory=UUIDT)
    status: JobStatus = JobStatus.Queued
    started_at: Optional[datetime] = None
    terminated_at: Optional[datetime] = None
    task: Optional[asyncio.Task] = None
    terminated: asyncio.Event = field(default_factory=asyncio.Event)

    def __str__(self):
        return f"{self.type} {self.id} ({self.status})"

    def __repr__(self):
        return f"<Job {self}>"

    def __lt__(self, other):
        return self.default_priority < other.default_priority

    @property
    def success(self) -> bool:
        raise NotImplementedError

    @property
    def default_priority(self) -> int:
        return JOB_DEFAULT_PRIORITY[self.type]


@dataclass(repr=False)
class JobData:
    id: UUID
    type: JobType
    status: JobStatus
    started_at: Optional[datetime]
    terminated_at: Optional[datetime]
    # additional context
    project_id: UUID
    project_version_id: UUID
    deployment_id: Optional[UUID]

    @staticmethod
    def from_job(
        job: Job, project_id: UUID, project_version_id: UUID, deployment_id: Optional[UUID]
    ) -> JobData:
        return JobData(
            type=job.type,
            id=job.id,
            status=job.status,
            started_at=job.started_at,
            terminated_at=job.terminated_at,
            project_id=project_id,
            project_version_id=project_version_id,
            deployment_id=deployment_id,
        )

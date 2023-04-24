from __future__ import annotations

import enum
import json
import traceback
from collections import OrderedDict
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Callable, Coroutine, Optional
from uuid import UUID, uuid5

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
    XBlock,
)
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType
from bench.utils.record import RecordBatch
from bench.utils.utils import required_field

#
# Instances
#

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
    task: Optional[Task]
    code: Optional[Code]
    model: Optional[Model]
    root: Optional[ExecutionFrame]
    parent: Optional[ExecutionFrame]
    entered_at: datetime
    exited_at: Optional[datetime]
    cached_generated_at: Optional[datetime]
    cached_duration: Optional[float]
    inputs: Optional[dict[str, LiteralValue]]
    outputs: Optional[LiteralValue]
    error: Optional[Exception]
    queue_position: Optional[int]
    children: list[ExecutionFrame] = field(default_factory=list)

    @property
    def duration(self) -> float:
        if self.exited_at is None:
            return 0
        return (self.exited_at - self.entered_at).total_seconds()

    @property
    def duration_with_cache(self) -> float:
        if self.exited_at is None:
            return 0
        return self.duration + (self.cached_duration or 0)

    def walk_descendants(self):
        yield self
        for child in self.children:
            yield from child.walk_descendants()

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
        ]
        fields_str = [s for s in fields_strs if s]
        return f"id={self.id} ({', '.join(fields_str)})"

    def __repr__(self):
        return f"<ExecutionFrame {self}>"


@dataclass(slots=True)
class Inference:
    """The cached inference struct"""

    generated_at: datetime
    duration: float
    result: Any

    def to_json_str(self) -> str:
        inference_json = {
            "generated_at": self.generated_at.isoformat(),
            "duration": self.duration,
            "result": self.result,
        }
        return json.dumps(inference_json)

    @classmethod
    def from_json_str(cls, json_str: str):
        data = json.loads(json_str)
        return cls(
            generated_at=datetime.fromisoformat(data["generated_at"]),
            duration=data["duration"],
            result=data["result"],
        )


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
    entered_at: datetime
    exited_at: Optional[datetime]
    cached_generated_at: Optional[datetime]
    cached_duration: Optional[float]
    inputs: dict[str, Any]
    outputs: Optional[Any]
    error: Optional[ErrorData]
    queue_position: Optional[int]
    # additional context data not in ExecutionFrame
    project_id: UUID
    tracing_level: Optional[ExecutionTracingLevel]
    deployment_id: UUID
    worker_id: UUID
    trigger_type: Optional[ExecutionTriggerType]
    trigger_id: Optional[UUID]

    @staticmethod
    def from_frame(
        frame: ExecutionFrame,
        *,
        project_id: UUID,
        tracing_level: ExecutionTracingLevel,
        deployment_id: UUID,
        worker_id: UUID,
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
            cached_generated_at=frame.cached_generated_at,
            cached_duration=frame.cached_duration,
            inputs=frame.inputs,
            outputs=frame.outputs,
            error=error_data,
            queue_position=frame.queue_position,
            project_id=project_id,
            tracing_level=tracing_level,
            deployment_id=deployment_id,
            worker_id=worker_id,
            trigger_type=trigger_type,
            trigger_id=trigger_id,
        )


#
# Jobs
#


@dataclass(slots=True)
class JobData:
    id: UUID
    type: str
    status: str
    project_id: UUID
    project_version_id: UUID
    deployment_id: Optional[UUID]
    worker_id: UUID
    started_at: Optional[datetime]
    terminated_at: Optional[datetime]


#
# Building
#


class BuildScope(enum.Enum):
    SELECTED = "selected"
    REACTIVE = "reactive"
    ALL = "all"


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
    instruct_model_id: Optional[UUID]
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
    """
    Standard evaluation metrics. Custom metrics will be allowed later (probably with a prefix).
    """

    # Summary (global and build specific)
    Clarity = "clarity"  # [0, 1]
    Difficulty = "difficulty"  # [0, inf)
    Performance = "performance"  # [0, 1]
    Speed = "speed"  # [0, inf) (estimated run duration)
    # Clarity (global)
    InstructionPerplexity = "instruction_perplexity"  # [0, 1]
    InstructionAgreement = "instruction_agreement"  # [0, 1]
    InstructionOverlap = "instruction_overlap"  # [0, 1]
    # Difficulty (global)
    InstructionCount = "instruction_count"  # [0, inf)
    InstructionComplexity = "instruction_complexity"  # [0, inf)
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
class EvaluationPlan:
    system: Task
    eval_model: Model
    datasets: list[Dataset] = field(default_factory=list)
    id: UUID = field(init=False)

    def __post_init__(self):
        self.id = uuid5(self.system_id, f"evaluation_plan:{self.system_id}")

    def __str__(self):
        return f"evals for {self.system}"

    def __repr__(self):
        return f"<EvaluationPlan {str(self)}>"

    @property
    def system_id(self) -> UUID:
        return self.system.id


@dataclass(repr=False, slots=True)
class EvaluationResult:
    kind: EvaluationKind
    scope: EvaluationScope
    aggregated_metrics: dict[str, float]
    self_metrics: Optional[dict[str, float]] = None
    system: Optional[InterpSymbol | TypeNode | Record] = None
    build: Optional[Build] = None
    build_candidate: Optional[Any] = None  # can't refer to BuildCandidate here
    plan: Optional[EvaluationPlan] = None
    children: list["EvaluationResult"] = field(default_factory=list)

    def __str__(self):
        metrics_str = ", ".join(
            f"{k}: {self.aggregated_metrics[k]:0.02f}" for k, v in self.aggregated_metrics.items()
        )
        return f"{self.kind} {self.scope} {self.system_id} {metrics_str}"

    def __repr__(self):
        return f"<Evaluation {self.system} {self}>"

    @property
    def system_id(self) -> Optional[UUID]:
        return self.system.id if self.system else None

    def make_id(self, module_id: UUID) -> UUID:
        """
        Make a deterministic ID for a specific evaluation
        Useful to associate other data (e.g. build candidates) with evaluations
        Don't change this
        """
        environment_id = self.build_candidate.id if self.build_candidate else module_id
        return uuid5(
            environment_id, f"evaluation:{self.kind}:{self.scope}:{environment_id}:{self.system_id}"
        )

    def walk(self):
        yield self
        for child in self.children:
            yield from child.walk()


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

    def __str__(self):
        metrics_str = ", ".join(
            f"{k}: {self.aggregated_metrics[k]:0.02f}" for k, v in self.aggregated_metrics.items()
        )
        return f"{self.kind} {self.scope} {metrics_str}"

    def __repr__(self):
        return f"<Evaluation {self.statement_id or self.record_id or self.type_node_id} {self}>"

    @property
    def system_id(self) -> Optional[UUID]:
        return self.statement_id or self.record_id or self.type_node_id

    @property
    def environment_id(self) -> UUID:
        if self.build_candidate_id:
            return self.build_candidate_id
        else:
            return self.project_version_id

    @staticmethod
    def from_result(
        result: EvaluationResult,
        *,
        project_id: UUID,
        project_version_id: UUID,
        job_id: Optional[UUID],
    ) -> list[EvaluationResultData]:
        """Flattens an EvaluationResult tree into a list of EvaluationResultData (recursively)."""
        results_data: dict[UUID, EvaluationResultData] = OrderedDict()
        for result in result.walk():
            statement_id = result.system.id if isinstance(result.system, InterpSymbol) else None
            type_node_id = (
                result.system.id
                if isinstance(result.system, TypeNode)
                and not isinstance(result.system, InterpSymbol)
                else None
            )
            record_id = result.system.id if isinstance(result.system, Record) else None
            result_data = EvaluationResultData(
                id=result.make_id(project_version_id),
                kind=result.kind,
                scope=result.scope,
                aggregated_metrics=result.aggregated_metrics,
                self_metrics=result.self_metrics,
                statement_id=statement_id,
                type_node_id=type_node_id,
                record_id=record_id,
                build_id=result.build.id if result.build else None,
                build_candidate_id=result.build_candidate.id if result.build_candidate else None,
                project_id=project_id,
                project_version_id=project_version_id,
                job_id=job_id,
                parent_id=None,  # set in second pass
            )
            if result.scope == EvaluationScope.INSTRUCTION and result_data.system_id is None:
                raise ValueError(f"missing system_id for {result_data}")
            results_data[result_data.id] = result_data

        # assign parent ids
        for result in result.walk():
            for child in result.children:
                child_id = child.make_id(project_version_id)
                results_data[child_id].parent_id = result.id

        return list(results_data.values())


#
# Model inference
#

# Ideally, endpoint settings should be 1) extensible and 2) types in the std lib.
# For now, we just use internal dataclasses. :TypeSafeSettings


class IncapableError(NotImplementedError):
    pass


class Modality(enum.StrEnum):
    """Core modality capabilities of a model."""

    GenerateText = "generate_text"  # any -> text
    GenerateImage = "generate_image"  # any -> image
    GenerateAudio = "generate_audio"  # any -> audio
    Embed = "embed"  # any -> embedding
    Struct = "struct"  # any -> struct(ture prediction)


@dataclass
class TextGenerationSettings:
    temperature: float
    max_tokens: int
    top_p: Optional[float]
    stop: Optional[list[str]] = field(default_factory=list)
    logit_bias: Optional[dict[str, float]] = field(default_factory=dict)


@dataclass
class ImageGenerationSettings:
    seed: int
    steps: int
    width: int
    height: int
    cfg_scale: float


@dataclass
class AudioGenerationSettings:
    pass


@dataclass
class EmbeddingSettings:
    pass


@dataclass
class StructSettings:
    pass


SETTINGS_CLS_BY_MODALITY = {
    Modality.GenerateText: TextGenerationSettings,
    Modality.GenerateImage: ImageGenerationSettings,
    Modality.GenerateAudio: AudioGenerationSettings,
    Modality.Embed: EmbeddingSettings,
}


class ModelInference:
    """Generic model with an endpoint for each core modality."""

    async def generate_text(self, input: list[XBlock], settings: TextGenerationSettings) -> str:
        raise IncapableError()

    async def generate_image(
        self, input: list[XBlock], settings: ImageGenerationSettings
    ) -> PIL.Image:
        raise IncapableError()

    async def generate_audio(self, input: list[XBlock], settings: AudioGenerationSettings) -> bytes:
        raise IncapableError()

    async def embed(self, input: list[XBlock], settings: EmbeddingSettings) -> list[float]:
        raise IncapableError()

    async def struct(self, input: list[XBlock], settings: StructSettings) -> Any:
        raise IncapableError()


BASE_SETTINGS_BY_MODALITY = {
    Modality.GenerateText: TextGenerationSettings,
    Modality.GenerateImage: ImageGenerationSettings,
    Modality.GenerateAudio: AudioGenerationSettings,
    Modality.Embed: EmbeddingSettings,
    Modality.Struct: StructSettings,
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
# Workers
#


class WorkerType(enum.StrEnum):
    LANGUAGE = "LANGUAGE"
    COMMUNITY = "COMMUNITY"
    DEDICATED = "DEDICATED"

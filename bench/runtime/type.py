from __future__ import annotations

import enum
import sys
import traceback
from collections import OrderedDict
from dataclasses import dataclass, field
from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, cast
from uuid import UUID, uuid5

from bench.language.type import (
    Build,
    Code,
    InterpSymbol,
    LiteralValue,
    Model,
    Record,
    SymbolType,
    Task,
    TypeNode,
)
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType
from bench.utils.utils import to_pyidentifier_multi

if TYPE_CHECKING:
    from bench.runtime.instance import CodeInstance


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
class PyFrameData:
    filename: str
    lineno: int
    name: str
    locals: dict[str, Any] = None
    line: str = None

    @staticmethod
    def from_traceback(frame: traceback.FrameSummary):
        return PyFrameData(
            filename=frame.filename,
            lineno=frame.lineno,
            name=frame.name,
            locals=frame.locals,
            line=frame.line,
        )

    @staticmethod
    def from_stack(stack: traceback.StackSummary) -> list[PyFrameData]:
        return [PyFrameData.from_traceback(frame) for frame in stack]

    @staticmethod
    def clean(stack: list[PyFrameData], from_code: "CodeInstance") -> list[PyFrameData]:
        from bench.runtime.instance import CodeInstance

        session = from_code.session
        code_instances = [
            cast(CodeInstance, instance)
            for instance in session.instances.values()
            if instance.symbol_type == SymbolType.CODE
        ]
        code_instances_by_method_name: dict[str, CodeInstance] = {
            instance.transform.method_name: instance for instance in code_instances
        }

        transform = from_code.transform
        found_start = False
        cleaned_stack = []
        for frame in stack:
            if not found_start:
                # impute bench source info into instantiated code callables
                code = code_instances_by_method_name.get(frame.name)
                if code is not None:
                    if code == from_code:
                        found_start = True
                    elif not found_start:
                        continue  # ignore
                    if from_code.source is not None:
                        frame.filename = to_pyidentifier_multi(
                            from_code.source.file.path, from_code.source.name
                        )
                    frame.name = from_code.name
                    frame.line = transform.transformed_code.splitlines()[frame.lineno - 1]
                    frame.lineno = frame.lineno - transform.start_offset
                    frame.locals = frame.locals or {}
                    for ident, var in code.context.items():
                        if ident not in frame.locals:
                            frame.locals[ident] = repr(session.instances[var.id])
            if found_start:
                # trim file path for python modules
                python_version = f"{sys.version_info.major}.{sys.version_info.minor}"
                if python_version in frame.filename:
                    frame.filename = frame.filename.split(python_version)[-1][1:]  # skip slash
                cleaned_stack.append(frame)
        return cleaned_stack


@dataclass(slots=True)
class RunErrorData:
    """Wire-able representation of an exception."""

    type: str
    message: str
    symbol: Optional[str]
    traceback: list[PyFrameData]


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
    error: Optional[RunErrorData]
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
            if frame.code is None:
                raise ValueError(f"error outside code: {frame}")
            stack_summary = traceback.StackSummary.extract(
                traceback.walk_tb(frame.error.__traceback__), capture_locals=True
            )
            stack = PyFrameData.from_stack(stack_summary)
            stack = PyFrameData.clean(stack, frame.code)
            error_data = RunErrorData(
                type=type(frame.error).__name__,
                symbol=str(frame.code),
                message=str(frame.error),
                traceback=stack,
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
# Workers
#


class WorkerType(enum.StrEnum):
    LANGUAGE = "LANGUAGE"
    COMMUNITY = "COMMUNITY"
    DEDICATED = "DEDICATED"

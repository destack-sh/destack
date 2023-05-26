import enum
from typing import cast

import structlog

from bench.language import ModuleIndex
from bench.language.type import Code, Data, Task, Type, TypeTag
from bench.runtime.instruct import Instruction, InstructionOp

logger = structlog.get_logger(__name__)


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


class MetricType(enum.StrEnum):
    PERCENTAGE = "percentage"
    COUNT = "count"


ALL_METRICS = set(EvaluationMetric)
COUNT_METRICS = {metric for metric in ALL_METRICS if metric.endswith("_count")}
PERCENTAGE_METRICS = ALL_METRICS - COUNT_METRICS

HIGHER_IS_BETTER = {
    EvaluationMetric.Performance,
    EvaluationMetric.Clarity,
    # Speed is measured in duration, so lower is better
    EvaluationMetric.TypeValidity,
    EvaluationMetric.InstructionSatisfaction,
    EvaluationMetric.FeedbackCorrelation,
    EvaluationMetric.InstructionAgreement,
    EvaluationMetric.InstructionOverlap,
}
LOWER_IS_BETTER = ALL_METRICS - HIGHER_IS_BETTER


def get_summary_metrics(metrics: dict[str, float]) -> dict[EvaluationMetric, float]:
    summary_metrics = {}

    # clarity
    if (
        EvaluationMetric.InstructionPerplexity in metrics
        and EvaluationMetric.InstructionAgreement in metrics
        and EvaluationMetric.InstructionOverlap in metrics
    ):
        perplexity = metrics[EvaluationMetric.InstructionPerplexity]
        agreement = metrics[EvaluationMetric.InstructionAgreement]
        overlap = metrics[EvaluationMetric.InstructionOverlap]
        clarity = (1 - min(perplexity, 1.0)) * agreement * (1 - overlap)
        summary_metrics[EvaluationMetric.Clarity] = min(clarity, 0.99)

    # difficulty
    if (
        EvaluationMetric.InstructionCount in metrics
        and EvaluationMetric.InstructionComplexity in metrics
    ):
        count = metrics[EvaluationMetric.InstructionCount]
        complexity = metrics[EvaluationMetric.InstructionComplexity]
        difficulty = count * complexity
        summary_metrics[EvaluationMetric.Difficulty] = difficulty

    # performance
    if (
        EvaluationMetric.TypeValidity in metrics
        or EvaluationMetric.InstructionSatisfaction in metrics
    ):
        performance = (metrics.get(EvaluationMetric.TypeValidity, 1)) * (
            (1.0 - 0.6 * metrics.get(EvaluationMetric.InstructionSatisfaction, 1))
        )
        # perfection is unattainable (... and 100 is suspicious)
        summary_metrics[EvaluationMetric.Performance] = min(performance, 0.99)

    # speed
    if EvaluationMetric.AverageRunDuration in metrics:
        speed = metrics[EvaluationMetric.AverageRunDuration]
        summary_metrics[EvaluationMetric.Speed] = speed

    return summary_metrics


async def lint_instruction(instruction: Instruction) -> dict[str, float]:
    """Lints a single instruction."""

    # TODO @Incomplete: compute model clarity metrics
    # TODO @Incomplete: compute model difficulty metrics?

    instruction_agreement = 1.0
    instruction_perplexity = 0.0
    instruction_overlap = 0.0

    # below are some heuristics for measuring 'clarity' (i.e. non-perplexity/confusion)
    # TODO @Incomplete: some clarity heuristics should just/also be warnings/errors

    HALF_CONFUSION = 0.25
    FULL_CONFUSION = 0.5

    if instruction.op in (
        InstructionOp.TaskDefinition,
        InstructionOp.ExpectationDefinition,
        InstructionOp.TypeDefinition,
        InstructionOp.DataDefinition,
    ):
        # natural definitions should have names and ideally descriptions
        no_name = (instruction.node.name or "").strip() == ""
        no_description = (instruction.node.description or "").strip() == ""
        if no_name:
            instruction_perplexity += FULL_CONFUSION
        # TODO @Broken: 'inline' function type nodes (which cannot have descriptions) are docked for clarity
        if no_description:
            instruction_perplexity += HALF_CONFUSION

    if instruction.op == InstructionOp.TypeDefinition:
        # types should not be 'anything'
        type = cast(Type, instruction.node)
        if type.tag == TypeTag.ANY:
            instruction_perplexity += FULL_CONFUSION

    if instruction.op == InstructionOp.TaskDefinition:
        # task definitions (not steps) should have types
        task = cast(Task, instruction.node)
        if not task.type.outputs:  # output type at least?
            instruction_perplexity += FULL_CONFUSION

    if instruction.op in (InstructionOp.SampleData, InstructionOp.DataDefinition):
        # sample data / data defs should have at least 2 samples
        data = cast(Data, instruction.node)
        if len(data.records) < 1:
            instruction_perplexity += HALF_CONFUSION
        elif len(data.records) < 2:
            instruction_perplexity += FULL_CONFUSION
        # sample data / data defs should have types
        if len(data.type.type_nodes or []) == 0:
            instruction_perplexity += FULL_CONFUSION

    if instruction.op in (InstructionOp.SampleCode, InstructionOp.CheckCode):
        # sample / check code should have input & output type
        code = cast(Code, instruction.node)
        if not code.type.inputs:
            instruction_perplexity += FULL_CONFUSION
        if not code.type.outputs:
            instruction_perplexity += FULL_CONFUSION

    self_metrics = {
        EvaluationMetric.InstructionCount: 1,
        EvaluationMetric.InstructionComplexity: 1,
        EvaluationMetric.InstructionAgreement: instruction_agreement,
        EvaluationMetric.InstructionOverlap: instruction_overlap,
        EvaluationMetric.InstructionPerplexity: instruction_perplexity,
    }
    self_metrics.update(get_summary_metrics(self_metrics))
    return self_metrics


async def lint(idx: ModuleIndex):
    """Lints an entire module."""
    raise NotImplementedError

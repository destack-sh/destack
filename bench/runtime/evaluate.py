import enum
import uuid
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Optional

import structlog

from bench.language.type import Build, InterpSymbol, XKind
from bench.runtime.instruct import InstructionSourceGenerate, anonymous_dataset
from bench.runtime.run import RunError, run
from bench.runtime.type import TaskInstance

logger = structlog.get_logger(__name__)


class EvaluationMetric(enum.StrEnum):
    # Summary metrics
    Clarity = "clarity"
    Performance = "performance"
    Sophistication = "sophistication"  # == speed?
    # Clarity related (shared across builds?)
    InstructionPerplexity = "instruction_perplexity"
    InstructionAgreement = "instruction_agreement"
    InstructionOverlap = "instruction_overlap"
    # Performance related
    TypeValidity = "type_validity"
    ExpectationSatisfaction = "expectation_satisfaction"
    FeedbackCorrelation = "feedback_correlation"
    # Complexity related
    EstimatedRunDuration = "estimated_run_duration"
    InferencesCount = "inferences_count"
    NodesCount = "nodes_count"
    StepsCount = "steps_count"
    TokensCount = "tokens_count"


class MetricType(enum.StrEnum):
    PERCENTAGE = "percentage"
    COUNT = "count"


ALL_METRICS = set(EvaluationMetric)
COUNT_METRICS = {metric for metric in ALL_METRICS if metric.endswith("_count")}
PERCENTAGE_METRICS = ALL_METRICS - COUNT_METRICS

HIGHER_IS_BETTER = {
    EvaluationMetric.Performance,
    EvaluationMetric.Clarity,
    EvaluationMetric.TypeValidity,
    EvaluationMetric.ExpectationSatisfaction,
    EvaluationMetric.FeedbackCorrelation,
    EvaluationMetric.InstructionAgreement,
    EvaluationMetric.InstructionOverlap,
}
LOWER_IS_BETTER = ALL_METRICS - HIGHER_IS_BETTER


@dataclass(repr=False, slots=True)
class EvaluationResult:
    symbol: Optional[InterpSymbol]
    build: Build
    metrics: dict[str, float]
    id: uuid.UUID = field(default_factory=uuid.uuid4)
    children: list["EvaluationResult"] = field(default_factory=list)

    def __str__(self):
        return ", ".join(
            f"{k}: {self.metrics[k]:0.02f}" for k in EvaluationMetric if k in self.metrics
        )

    def __repr__(self):
        return f"<Evaluation {self.symbol} {self}>"


def aggregate_evaluations(
    evaluations: list[EvaluationResult], weights: dict[uuid.UUID | str, float] = None
) -> EvaluationResult:
    """Combines multiple evaluations into one."""
    weights = weights or defaultdict(lambda: 1.0)
    # sum the counts
    summed_counts = {}
    for metric in COUNT_METRICS & summed_counts.keys():
        summed_counts[metric] = sum(
            evaluation.metrics[metric] * weights.get(evaluation.id, weights[metric])
            for evaluation in evaluations
        )
    # average the percentages (?)
    averaged_percentages = defaultdict(float)
    for evaluation in evaluations:
        for metric, value in evaluation.metrics.items():
            averaged_percentages[metric] += value * weights.get(evaluation.id, weights[metric])
    for metric in PERCENTAGE_METRICS & averaged_percentages.keys():
        averaged_percentages[metric] /= len(evaluations)

    # check that the build is the same
    build_ids = {evaluation.build.id for evaluation in evaluations}
    if len(build_ids) > 1:
        raise ValueError("cannot aggregate evaluations from different builds")

    return EvaluationResult(
        symbol=None,
        build=evaluations[0].build,
        metrics={**summed_counts, **averaged_percentages},
        children=evaluations,
    )


def compare_evaluations(
    a: EvaluationResult, b: EvaluationResult, weights: dict[str, float]
) -> float:
    """
    If a is better than b, return a positive number. Otherwise, return a negative number.
    If a and b are equal, return 0.

    Only metrics in `weights` are considered.
    """
    diff = 0
    for metric, weight in weights.items():
        a_value = a.metrics[metric]
        b_value = b.metrics[metric]
        if metric in HIGHER_IS_BETTER:
            diff += (a_value - b_value) * weight
        elif metric in LOWER_IS_BETTER:
            diff += (b_value - a_value) * weight
    return diff


async def evaluate_task(task: TaskInstance, build: Build, n_samples: int) -> EvaluationResult:
    """Evaluates a task implementation against the instructions."""
    log = logger.bind(task=task, build=build)
    tokens_count = sum(
        [len(xblock) for xblock in task.implementation.xblocks if xblock.kind != XKind.Settings]
    )
    count_metrics = {
        # only 1 always for now :TaskGrouping
        EvaluationMetric.NodesCount: 1,
        EvaluationMetric.StepsCount: 1,
        # this only works for strings
        EvaluationMetric.TokensCount: tokens_count,
    }

    # generates samples to test
    inputs = await InstructionSourceGenerate(task.type.input, count=n_samples, seed=1337)()
    outputs = anonymous_dataset(task.type.output, n_samples)
    n_successful_runs = 0
    for i, sample in enumerate(inputs.records):
        try:
            outputs.records[i].data = await run(task.implementation, sample.data)
            n_successful_runs += 1
        except RunError:
            log.debug("evaluate.run.failed", sample=sample, excinfo=True)
            continue

    performance_metrics = {
        EvaluationMetric.TypeValidity: n_successful_runs / n_samples,
        # TODO @Incomplete: evaluate against expectations (all, implicit or otherwise)
        EvaluationMetric.ExpectationSatisfaction: 1.0,
        EvaluationMetric.FeedbackCorrelation: 1.0,
    }

    # TODO @Broken: compute proper summary metrics
    summary_metrics = {
        EvaluationMetric.Clarity: 1.0,
        EvaluationMetric.Performance: 1.0,
        EvaluationMetric.Sophistication: 1.0,
    }
    return EvaluationResult(
        symbol=task,
        build=build,
        metrics={**count_metrics, **performance_metrics, **summary_metrics},
    )

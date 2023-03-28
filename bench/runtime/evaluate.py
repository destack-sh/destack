import asyncio
import enum
import uuid
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Optional

import structlog

from bench.language import ModuleIndex
from bench.language.type import (
    Build,
    InterpSymbol,
    Model,
    Record,
    TypeNode,
    XKind,
    flatten_func_type,
)
from bench.runtime.instruct import (
    SampleSourceGenerator,
    anonymous_dataset,
    instruction_tree_from_module,
)
from bench.runtime.run import run
from bench.runtime.type import TaskInstance
from bench.utils.func import dict_minus

logger = structlog.get_logger(__name__)


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
    EvaluationMetric.InstructionSatisfaction,
    EvaluationMetric.FeedbackCorrelation,
    EvaluationMetric.InstructionAgreement,
    EvaluationMetric.InstructionOverlap,
}
LOWER_IS_BETTER = ALL_METRICS - HIGHER_IS_BETTER


@dataclass(repr=False, slots=True)
class EvaluationResult:
    aggregated_metrics: dict[str, float]
    self_metrics: Optional[dict[str, float]] = None
    system: Optional[InterpSymbol | TypeNode | Record] = None
    build: Optional[Build] = None
    id: uuid.UUID = field(default_factory=uuid.uuid4)
    children: list["EvaluationResult"] = field(default_factory=list)

    def __str__(self):
        return ", ".join(
            f"{k}: {self.aggregated_metrics[k]:0.02f}"
            for k in EvaluationMetric
            if k in self.aggregated_metrics
        )

    def __repr__(self):
        return f"<Evaluation {self.system} {self}>"


def aggregate_evaluations(
    evaluations: list[EvaluationResult], weights: dict[uuid.UUID | str, float] = None
) -> EvaluationResult:
    """Combines multiple evaluations into one."""
    aggregated_metrics = aggregate_metrics(evaluations, weights)

    # check that the build is the same
    build_ids = {evaluation.build.id if evaluation.build else None for evaluation in evaluations}
    if len(build_ids) > 1:
        raise ValueError(f"cannot aggregate evaluations from different builds: {build_ids}")

    return EvaluationResult(
        system=None,
        build=evaluations[0].build if len(build_ids) == 1 else None,
        self_metrics=None,
        aggregated_metrics=aggregated_metrics,
        children=evaluations,
    )


def aggregate_metrics(
    evaluations: list[EvaluationResult], weights: dict[uuid.UUID | str, float] = None
) -> dict[str, float]:
    weights = weights or defaultdict(lambda: 1.0)
    # sum the counts
    summed_counts = {}
    for metric in COUNT_METRICS & summed_counts.keys():
        summed_counts[metric] = sum(
            evaluation.self_metrics[metric] * weights.get(evaluation.id, weights[metric])
            for evaluation in evaluations
        )
    # average the percentages (?)
    averaged_percentages = defaultdict(float)
    for evaluation in evaluations:
        for metric, value in evaluation.aggregated_metrics.items():
            averaged_percentages[metric] += value * weights.get(evaluation.id, weights[metric])
    for metric in PERCENTAGE_METRICS & averaged_percentages.keys():
        averaged_percentages[metric] /= len(evaluations)
    aggregated_metrics = {**summed_counts, **averaged_percentages}
    return aggregated_metrics


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
        a_value = a.aggregated_metrics[metric]
        b_value = b.aggregated_metrics[metric]
        if metric in HIGHER_IS_BETTER:
            diff += (a_value - b_value) * weight
        elif metric in LOWER_IS_BETTER:
            diff += (b_value - a_value) * weight
    return diff


async def evaluate_task(
    task: TaskInstance, eval_model: Model, build: Build, n_samples: int
) -> EvaluationResult:
    """Evaluates a task implementation against the instructions."""
    log = logger.bind(task=task, build=build)
    # technically this is characters count, not tokens count
    # we'll want proper token counts soon to properly optimize for the available context

    tokens_count = sum(
        [len(xblock) for xblock in task.implementation.xblocks if xblock.kind != XKind.Settings]
    )
    count_metrics = {
        EvaluationMetric.NodesCount: 1,
        # only 1 always for now :TaskGrouping
        EvaluationMetric.StepsCount: 1,
        EvaluationMetric.InferencesCount: 1,
        # this only works for strings
        EvaluationMetric.TokensCount: tokens_count,
    }

    # generates samples to test
    inputs = await SampleSourceGenerator(
        flatten_func_type(task.type), model=eval_model, count=n_samples, seed=1337
    )()
    results = await asyncio.gather(
        *(
            run(task.implementation, dict_minus(sample.data, {"output"}))
            for sample in inputs.records
        ),
        return_exceptions=True,
    )
    outputs = anonymous_dataset(task.type.output, n_samples)
    n_successful_runs = 0
    for i, result in enumerate(results):
        if isinstance(result, Exception):
            log.warning("evaluate.run.failed", result=result)
            continue
        outputs.records[i].data = result
        n_successful_runs += 1

    performance_metrics = {
        EvaluationMetric.TypeValidity: n_successful_runs / n_samples,
        # TODO @Incomplete: compute instruction satisfaction
        EvaluationMetric.InstructionSatisfaction: 1.0,
        EvaluationMetric.FeedbackCorrelation: 1.0,
    }

    # TODO @Broken: compute proper summary metrics
    summary_metrics = {
        EvaluationMetric.Performance: 0.9,
        EvaluationMetric.Difficulty: 0.14,
    }
    return EvaluationResult(
        system=task,
        build=build,
        self_metrics=None,
        aggregated_metrics={**count_metrics, **performance_metrics, **summary_metrics},
    )


async def lint(idx: ModuleIndex) -> EvaluationResult:
    """Lints an entire module."""
    tree = instruction_tree_from_module(idx)
    evaluations: dict[uuid.UUID, EvaluationResult] = {}

    # evaluate nodes individually
    for node in tree.walk():
        self_metrics = {EvaluationMetric.NodesCount: 1}
        evaluation = EvaluationResult(
            system=node,
            build=None,
            self_metrics=self_metrics,
            aggregated_metrics={**self_metrics},
        )
        evaluations[node.id] = evaluation

    # aggregate evaluations bottom-up (post-order)
    for node in tree.walk_postorder():
        child_evaluations = [evaluations[child.id] for child in node.children]
        evaluations[node.id].aggregated_metrics = aggregate_metrics(child_evaluations)

    root_evaluations = [evaluations[root.id] for root in tree.roots]
    root_evaluation = aggregate_evaluations(root_evaluations)
    return root_evaluation

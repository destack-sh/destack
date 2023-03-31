import asyncio
import enum
import random
import uuid
from collections import defaultdict
from typing import Any

import structlog

from bench.language import ModuleIndex
from bench.language.type import Build, Model, XKind, flatten_func_type
from bench.runtime.instruct import (
    Instruction,
    SampleSourceGenerator,
    anonymous_dataset,
    instruction_tree_from_module,
)
from bench.runtime.run import run
from bench.runtime.tracing import in_memory_traces
from bench.runtime.type import (
    EvaluationKind,
    EvaluationMetric,
    EvaluationResult,
    EvaluationScope,
    TaskInstance,
)
from bench.utils.func import dict_minus

logger = structlog.get_logger(__name__)


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


def aggregate_metrics(
    evaluations: list[EvaluationResult], weights: dict[uuid.UUID | str, float] = None
) -> dict[str, float]:
    weights = weights or defaultdict(lambda: 1.0)
    # sum the counts
    summed_counts = {}
    for metric in COUNT_METRICS & summed_counts.keys():
        summed_counts[metric] = sum(
            evaluation.aggregated_metrics[metric] * weights.get(evaluation.id, weights[metric])
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
    Compares two evaluations using the weights.
    """
    a_score = score_evaluation(a, weights)
    b_score = score_evaluation(b, weights)
    return a_score - b_score


def score_evaluation(a: EvaluationResult, weights: dict[str, float]) -> float:
    """
    Returns a score for the evaluation, based on the weights.
    """
    score = 0
    for metric, weight in weights.items():
        value = a.aggregated_metrics[metric]
        if metric in HIGHER_IS_BETTER:
            score += value * weight
        elif metric in LOWER_IS_BETTER:
            score += (1 - value) * weight
    return score


async def evaluate_task(
    task: TaskInstance,
    eval_model: Model,
    build: Build,
    n_samples: int,
    build_candidate: Any = None,
) -> EvaluationResult:
    """Evaluates a task implementation against the instructions."""
    log = logger.bind(task=task, build=build)
    # technically this is characters count, not tokens count
    # we'll want proper token counts soon to properly optimize for the available context

    tokens_count = sum(
        [len(xblock) for xblock in task.implementation.xblocks if xblock.kind != XKind.Settings]
    )
    count_metrics = {
        EvaluationMetric.InstructionCount: 1,
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
    with in_memory_traces() as traces:
        runs = (
            run(task.implementation, dict_minus(sample.data, {"output"}))
            for sample in inputs.records
        )
        results = await asyncio.gather(*runs, return_exceptions=True)
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

    # TODO @Incomplete: compute proper summary metrics
    summary_metrics = {
        EvaluationMetric.Performance: performance_metrics[EvaluationMetric.TypeValidity],
        EvaluationMetric.Speed: random.random() * 1000,
    }
    return EvaluationResult(
        kind=EvaluationKind.EVALUATION,
        scope=EvaluationScope.INSTRUCTION,
        system=task,
        build=build,
        build_candidate=build_candidate,
        self_metrics=None,
        aggregated_metrics={**count_metrics, **performance_metrics, **summary_metrics},
    )


async def lint_instruction(instruction: Instruction) -> dict[str, float]:
    """Lints a single instruction."""
    # TODO @Incomplete: compute proper lint metrics
    self_metrics = {EvaluationMetric.InstructionCount: 1, EvaluationMetric.Clarity: 1.0}
    self_metrics[EvaluationMetric.Difficulty] = self_metrics[EvaluationMetric.InstructionCount]
    return self_metrics


async def lint(idx: ModuleIndex) -> EvaluationResult:
    """Lints an entire module."""
    tree = instruction_tree_from_module(idx, exclude_generated=True)
    evaluations: dict[uuid.UUID, EvaluationResult] = {}

    # evaluate nodes individually
    instructions = list(tree.walk_postorder())
    instruction_self_metrics = await asyncio.gather(
        *[lint_instruction(node) for node in instructions]
    )
    for self_metrics, instruction in zip(instruction_self_metrics, instructions):
        evaluation = EvaluationResult(
            kind=EvaluationKind.LINT,
            scope=EvaluationScope.INSTRUCTION,
            system=instruction,
            build=None,
            self_metrics=self_metrics,
            aggregated_metrics={**self_metrics},
        )
        evaluations[instruction.id] = evaluation

        # aggregate evaluations (incl. self)
        # this will break when we get cycles :InstructionCircles
        child_evaluations = [evaluations[child.id] for child in instruction.children]
        evaluations[instruction.id].aggregated_metrics = aggregate_metrics(
            [evaluation, *child_evaluations]
        )

    root_evaluations = [evaluations[root.id] for root in tree.roots]
    root_evaluation = EvaluationResult(
        kind=EvaluationKind.LINT,
        scope=EvaluationScope.MODULE,
        system=None,
        build=None,
        aggregated_metrics=aggregate_metrics(root_evaluations),
    )
    return root_evaluation

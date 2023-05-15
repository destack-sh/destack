import asyncio
import enum
import uuid
from collections import defaultdict
from typing import cast

import structlog

from bench.language import ModuleIndex
from bench.language.type import Code, Data, Task, Type, TypeTag
from bench.runtime.instruct import Instruction, InstructionOp, instruction_tree_from_module
from bench.runtime.type import EvaluationKind, EvaluationMetric, EvaluationResult, EvaluationScope

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
    # Speed is measured in duration, so lower is better
    EvaluationMetric.TypeValidity,
    EvaluationMetric.InstructionSatisfaction,
    EvaluationMetric.FeedbackCorrelation,
    EvaluationMetric.InstructionAgreement,
    EvaluationMetric.InstructionOverlap,
}
LOWER_IS_BETTER = ALL_METRICS - HIGHER_IS_BETTER


def aggregate_metrics(
    evaluations: list[EvaluationResult], weights: dict[str, float] = None
) -> dict[str, float]:
    weights = weights or defaultdict(lambda: 1.0)

    # sum the counts
    summed_counts = {}
    for metric in COUNT_METRICS & summed_counts.keys():
        summed_counts[metric] = sum(
            evaluation.aggregated_metrics[metric] * weights[metric] for evaluation in evaluations
        )

    # average the percentages (?)
    averaged_percentages = defaultdict(float)
    for evaluation in evaluations:
        for metric, value in evaluation.aggregated_metrics.items():
            averaged_percentages[metric] += value * weights[metric]
    for metric in PERCENTAGE_METRICS & averaged_percentages.keys():
        averaged_percentages[metric] /= len(evaluations)
    aggregated_metrics = {**summed_counts, **averaged_percentages}

    # recompute summary metrics
    aggregated_metrics.update(get_summary_metrics(aggregated_metrics))

    return aggregated_metrics


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


def aggregate_metrics_by_system(evaluations: list[EvaluationResult]) -> list[EvaluationResult]:
    evaluations_by_node = defaultdict(list)
    for evaluation in evaluations:
        evaluations_by_node[evaluation.system.id].append(evaluation)

    aggregated: list[EvaluationResult] = []
    for node_id, evaluations in evaluations_by_node.items():
        all_children = {
            child.id: child for evaluation in evaluations for child in evaluation.type_nodes
        }
        evaluation = EvaluationResult(
            kind=evaluations[0].kind,
            scope=evaluations[0].scope,
            system=evaluations[0].system,
            build=evaluations[0].build,
            aggregated_metrics=aggregate_metrics(evaluations),
            children=list(all_children.values()),
        )
        aggregated.append(evaluation)
    return aggregated


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


async def lint(idx: ModuleIndex) -> EvaluationResult:
    """Lints an entire module."""
    tree = instruction_tree_from_module(idx, exclude_generated=True)
    evaluations: dict[uuid.UUID, EvaluationResult] = {}

    # evaluate nodes individually
    lintable_instructions = [
        node
        for node in tree.walk()
        if node.op not in (InstructionOp.Pseudo, InstructionOp.BuildDefinition)
    ]
    instruction_self_metrics = await asyncio.gather(
        *[lint_instruction(node) for node in lintable_instructions]
    )
    for self_metrics, instruction in zip(instruction_self_metrics, lintable_instructions):
        evaluation = EvaluationResult(
            kind=EvaluationKind.LINT,
            scope=EvaluationScope.INSTRUCTION,
            system=instruction.node,
            build=None,
            self_metrics=self_metrics,
            aggregated_metrics={**self_metrics},
        )
        evaluations[instruction.id] = evaluation

        # aggregate evaluations (incl. self)
        # this will break when we get cycles :InstructionCircles
        child_evaluations = [evaluations[child.id] for child in instruction.children]
        evaluation.aggregated_metrics = aggregate_metrics([evaluation, *child_evaluations])
        evaluation.children = [c for c in child_evaluations if c.system.id != instruction.node.id]

    root_evaluations = [evaluations[root.id] for root in tree.roots if root.id in evaluations]
    root_evaluation = EvaluationResult(
        kind=EvaluationKind.LINT,
        scope=EvaluationScope.MODULE,
        system=None,
        build=None,
        aggregated_metrics=aggregate_metrics(root_evaluations),
        children=root_evaluations,
    )
    return root_evaluation

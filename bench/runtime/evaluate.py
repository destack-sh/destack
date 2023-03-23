import enum
import uuid
from collections import defaultdict
from dataclasses import dataclass, field
from itertools import chain
from typing import Optional

from bench.language.type import InterpSymbol
from bench.runtime.type import TaskInstance


class SummaryMetric(enum.StrEnum):
    Performance = "performance"
    Clarity = "clarity"
    Complexity = "complexity"


class BaseMetric(enum.StrEnum):
    # Performance related
    TypeCorrectness = "type_correctness"
    ExpectationSatisfaction = "expectation_satisfaction"
    FeedbackCorrelation = "feedback_correlation"
    # Clarity related
    InstructionPerplexity = "instruction_perplexity"
    InstructionAgreement = "instruction_agreement"
    InstructionOverlap = "instruction_overlap"
    # Complexity related
    ExecutionDuration = "execution_duration"  # need to consider caching
    NodesCount = "nodes_count"
    StepsCount = "steps_count"
    TokensCount = "tokens_count"


ALL_METRICS = set(chain(SummaryMetric, BaseMetric))

HIGHER_IS_BETTER = {
    SummaryMetric.Performance,
    SummaryMetric.Clarity,
    BaseMetric.TypeCorrectness,
    BaseMetric.ExpectationSatisfaction,
    BaseMetric.FeedbackCorrelation,
    BaseMetric.InstructionAgreement,
    BaseMetric.InstructionOverlap,
}
LOWER_IS_BETTER = {
    SummaryMetric.Complexity,
    BaseMetric.InstructionPerplexity,
    BaseMetric.ExecutionDuration,
    BaseMetric.NodesCount,
    BaseMetric.StepsCount,
    BaseMetric.TokensCount,
}


@dataclass
class Evaluation:
    symbol: Optional[InterpSymbol]
    metrics: dict[str, float]
    id: uuid.UUID = field(default_factory=uuid.uuid4)

    def __str__(self):
        return ", ".join(f"{k}: {v:0.02f}" for k, v in self.metrics.items())

    def __repr__(self):
        return f"<Evaluation {self.symbol} {self}>"

    @property
    def summary_metrics(self):
        return {metric: self.metrics[metric] for metric in SummaryMetric}

    @property
    def base_metrics(self):
        return {metric: self.metrics[metric] for metric in BaseMetric}


def compare_evaluations(a: Evaluation, b: Evaluation) -> bool:
    """If a is better than b, return True."""
    return b  # TODO @Broken: actually compare


async def evaluate_task(task: TaskInstance) -> Evaluation:
    # TODO @Incomplete: implement real evaluation
    return Evaluation(
        symbol=task,
        # zero everything
        metrics={metric: 0.0 for metric in chain(SummaryMetric, BaseMetric)},
    )


def aggregate_evaluations(
    evaluations: list[Evaluation], weights: dict[uuid.UUID, float] = None
) -> Evaluation:
    """Combines multiple evaluations into one."""
    weights = weights or defaultdict(lambda: 1.0)
    summed_metrics = defaultdict(float)
    for evaluation in evaluations:
        for metric, value in evaluation.metrics.items():
            summed_metrics[metric] += value * weights[evaluation.id]
    # just average for now
    averaged_metrics = {
        metric: value / len(evaluations) for metric, value in summed_metrics.items()
    }
    return Evaluation(
        symbol=None,
        metrics=averaged_metrics,
    )

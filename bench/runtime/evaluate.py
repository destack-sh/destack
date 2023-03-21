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
    ModelSuccessRate = "model_success_rate"
    TypeCorrectness = "type_correctness"
    ExpectationSatisfaction = "expectation_satisfaction"
    FeedbackCorrelation = "feedback_correlation"
    # Clarity related
    InstructionPerplexity = "instruction_perplexity"
    InstructionAgreement = "instruction_agreement"
    InstructionOverlap = "instruction_overlap"
    # Complexity related
    ExecutionDuration = "execution_duration"
    NodesCount = "nodes_count"
    StepsCount = "steps_count"
    TokensCount = "tokens_count"


@dataclass
class Evaluation:
    symbol: Optional[InterpSymbol]
    metrics: dict[str, float]
    id: uuid.UUID = field(default_factory=uuid.uuid4)

    @property
    def summary_metrics(self):
        return {metric: self.metrics[metric] for metric in SummaryMetric}

    @property
    def base_metrics(self):
        return {metric: self.metrics[metric] for metric in BaseMetric}


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

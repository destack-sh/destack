import enum
from dataclasses import dataclass

from bench.runtime.type import TaskInstance


class SummaryMetric(enum.StrEnum):
    Performance = "performance"
    Clarity = "clarity"
    Complexity = "complexity"


class BaseMetric(enum.StrEnum):
    ExecutionDuration = "execution_duration"
    NodesCount = "nodes_count"
    StepsCount = "steps_count"
    InstructionPerplexity = "instruction_perplexity"
    InstructionAgreement = "instruction_agreement"
    InstructionOverlap = "instruction_overlap"


@dataclass
class Evaluation:
    aggregate_metrics: dict[SummaryMetric, float]


async def evaluate(task: TaskInstance) -> Evaluation:
    raise NotImplementedError

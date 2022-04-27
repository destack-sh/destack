from typing import Optional

from bench.function.base import Record, Transform
from bench.models import Dataset


class NamedEntityDatasetImputer(Transform):
    def __init__(self, dataset: Dataset):
        self.dataset = dataset

    def __call__(self, record: Record) -> Record:
        pass


def upper_case(record: Record) -> Record:
    return {**record, "text": record["text"].upper()}


def augment_spelling(record: Record) -> Record:
    return record


def check_text_length(
    record: Record, min_length: int = 0, max_length: Optional[int] = None
) -> bool:
    return min_length <= len(record["text"]) <= (max_length or float("inf"))

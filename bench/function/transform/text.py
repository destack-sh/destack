import abc
from typing import Optional

from bench.dataset.base import DatasetHandler
from bench.function.base import Predicate, Record, Transform, functions
from bench.utils.spec import DatasetType


class TextTransform(Transform, abc.ABC):
    pass


@functions.register("bench.text.named_entity_imputer")
class NamedEntityImputer(Transform):
    config_spec = {"dataset": DatasetType(record_spec={})}
    input_spec = {"text": str, "entities": dict}
    output_spec = {"text": str, "entities": dict}

    def __init__(self, dataset: DatasetHandler):
        self.dataset = dataset

    def __call__(self, record: Record) -> Record:
        return record


@functions.register("bench.text.upper_case", impl=Transform)
def upper_case(text: str) -> str:
    """Transforms text to uppercase"""
    return text.upper()


@functions.register("bench.text.swap_characters")
class SwapCharacters(Transform):
    def __init__(self, max_distance: int = 2):
        self.max_distance = max_distance

    def __call__(self, record: Record) -> Record:
        text: str = record["text"]
        # TODO @Feature: support random state in functions
        return {**record, "text": text}


@functions.register("bench.text.check_length")
class CheckTextLength(Predicate):
    """Checks whether text length is in the specified bounds"""

    def __init__(self, min_length: int = 0, max_length: Optional[int] = None):
        self.min_length = min_length
        self.max_length = max_length

    def __call__(self, record: Record) -> bool:
        text: str = record["text"]
        return self.min_length <= len(text) <= (self.max_length or float("inf"))

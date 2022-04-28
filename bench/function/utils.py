from typing import Optional

from bench.function.base import FunctionBase, Predicate, Transform, functions
from bench.utils.record import Record


class Router(FunctionBase):
    input_spec = None
    output_spec = None

    def __init__(
        self, predicate: Predicate, function_a: FunctionBase, function_b: FunctionBase
    ):
        self.predicate = predicate
        self.function_a = function_a
        self.function_b = function_b


def filter(record: Record, predicate: Predicate) -> Optional[Record]:
    if predicate(record):
        return record
    else:
        return None


@functions.register("identity", impl=Transform)
def identity(record: Record) -> Record:
    return record

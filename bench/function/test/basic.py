from typing import Optional

from bench.function.base import SingleRecordTransform, Test, functions
from bench.utils.record import Record


@functions.register("bench.test.comparison_static")
class TestStaticComparison(SingleRecordTransform, Test):
    def __init__(self, operator: str, value: float, key: Optional[str] = None):
        super().__init__(result_key="result")
        self.operator = operator
        self.value = value
        self.key = key

    def _evaluate(self, record: Record) -> bool:
        if self.key:
            record = record[self.key]  # type: ignore

        if self.operator == "eq":
            return record == self.value
        elif self.operator == "neq":
            return record != self.value
        elif self.operator == "gt":
            return record > self.value  # type: ignore
        elif self.operator == "gte":
            return record >= self.value  # type: ignore
        elif self.operator == "lt":
            return record < self.value  # type: ignore
        elif self.operator == "lte":
            return record <= self.value  # type: ignore
        else:
            raise ValueError(f"unexpected comparison operator: {self.operator}")

    def transform(self, record: Record) -> Record:
        result = bool(self._evaluate(record))
        return {"result": result}

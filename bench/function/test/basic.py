import enum
from typing import Optional, cast

from bench.function.base import SingleRecordTransform, Test, functions
from bench.utils.func import dict_to_ordered
from bench.utils.record import Record
from bench.utils.spec import convert_to_record_spec


class ComparisonOperator(enum.Enum):
    Eq = "eq"
    Neq = "neq"
    Gt = "gt"
    Gte = "gte"
    Lt = "lt"
    Lte = "lte"


class TestConstantComparison(SingleRecordTransform, Test):
    output_spec = dict_to_ordered({"*": convert_to_record_spec({"result": bool})})

    def __init__(self, operator: ComparisonOperator, value: float, key: Optional[str] = None):
        super().__init__(result_key="result")
        self.operator = operator
        self.value = value
        self.key = key

    def _evaluate(self, record: Record) -> bool:
        if self.key:
            record = record[self.key]
        record = cast(float, record)

        if self.operator == ComparisonOperator.Eq:
            return record == self.value
        elif self.operator == ComparisonOperator.Neq:
            return record != self.value
        elif self.operator == ComparisonOperator.Gt:
            return record > self.value
        elif self.operator == ComparisonOperator.Gte:
            return record >= self.value
        elif self.operator == ComparisonOperator.Lt:
            return record < self.value
        elif self.operator == ComparisonOperator.Lte:
            return record <= self.value
        else:
            raise ValueError(f"unexpected comparison operator: {self.operator}")

    def transform(self, record: Record) -> Record:
        result = bool(self._evaluate(record))
        return {"result": result}


@functions.register("bench.test.compare_constant_int")
class TestConstantComparisonInt(TestConstantComparison):
    input_spec = {"*": convert_to_record_spec(int)}

    def __init__(self, operator: ComparisonOperator, value: int, key: Optional[str] = None):
        super().__init__(operator, value, key)


@functions.register("bench.test.compare_constant_float")
class TestConstantComparisonFloat(TestConstantComparison):
    input_spec = {"*": convert_to_record_spec(float)}

    def __init__(self, operator: ComparisonOperator, value: float, key: Optional[str] = None):
        super().__init__(operator, value, key)

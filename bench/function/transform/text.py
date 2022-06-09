from bench.function.base import SingleRecordTransform, functions
from bench.utils.record import Record


@functions.register("bench.text.upper_case")
class UpperCaseTextTransform(SingleRecordTransform):
    def transform(self, record: Record) -> Record:
        return {"text": record["text"].upper()}

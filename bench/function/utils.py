from collections import OrderedDict
from typing import Optional, Union, cast

from bench.function.base import FunctionMetadata, RecordTransform, SingleRecordTransform, functions
from bench.model.base import ModelHandler
from bench.utils.record import Record, RecordBatch
from bench.utils.spec import BLANK_RECORD_SPEC, convert_to_record_spec


@functions.register("bench.identity")
class IdentityRecordTransform(RecordTransform):
    metadata = FunctionMetadata("Identity", "Returns the input unchanged")

    input_spec = {"*": BLANK_RECORD_SPEC}
    output_spec = OrderedDict([("*", BLANK_RECORD_SPEC)])

    def transform(self, record: Record) -> Record:
        return record

    def transform_batch(self, records: RecordBatch) -> RecordBatch:
        return records


@functions.register("bench.model")
class ModelRecordTransform(RecordTransform):
    metadata = FunctionMetadata("Model", "Runs the model", tags=["model"])

    input_spec = {"*": BLANK_RECORD_SPEC}
    output_spec = OrderedDict([("*", BLANK_RECORD_SPEC)])

    def __init__(self, model: ModelHandler):
        self.model = model
        self.input_spec = {"*": self.model.spec.input_spec}
        self.output_spec = OrderedDict([("*", self.model.spec.output_spec)])

    def transform(self, record: Record) -> Union[Record, RecordBatch]:
        return self.model.run(record)

    def transform_batch(self, records: RecordBatch) -> RecordBatch:
        return self.model.run_batch(records)


@functions.register("bench.count")
class CountRecordTransform(SingleRecordTransform):
    metadata = FunctionMetadata("Count", "Counts elements in an array-like")

    input_spec = {"*": BLANK_RECORD_SPEC}
    output_spec = OrderedDict([("*", convert_to_record_spec(int))])

    def __init__(self, key: Optional[str] = None):
        self.key = key

    def transform(self, record: Record) -> Union[Record, RecordBatch]:
        if self.key is not None:
            record = cast(dict, record)[self.key]

        return len(record)

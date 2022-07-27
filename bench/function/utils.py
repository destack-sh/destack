from collections import OrderedDict
from typing import Union

from bench.function.base import RecordTransform, functions
from bench.model.base import ModelHandler
from bench.utils.record import Record, RecordBatch
from bench.utils.spec import BLANK_RECORD_SPEC


@functions.register("bench.identity")
class IdentityRecordTransform(RecordTransform):
    input_spec = {"*": BLANK_RECORD_SPEC}
    output_spec = OrderedDict([("*", BLANK_RECORD_SPEC)])

    def transform(self, record: Record) -> Record:
        return record

    def transform_batch(self, records: RecordBatch) -> RecordBatch:
        return records


@functions.register("bench.model")
class ModelRecordTransform(RecordTransform):
    input_spec = {"*": BLANK_RECORD_SPEC}
    output_spec = OrderedDict([("*", BLANK_RECORD_SPEC)])

    def __init__(self, model: ModelHandler):
        self.model = model
        self.input_spec = {"*": self.model.spec.input_spec}
        self.output_spec = OrderedDict([("*", self.model.spec.output_spec)])

    def transform(self, record: Record) -> Union[Record, RecordBatch]:
        return self.model.predict(record)

    def transform_batch(self, records: RecordBatch) -> RecordBatch:
        return self.model.predict_batch(records)

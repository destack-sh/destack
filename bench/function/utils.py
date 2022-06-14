from bench.function.base import RecordTransform, functions
from bench.model.base import ModelHandler
from bench.utils.record import Record, RecordBatch


@functions.register("bench.identity")
class IdentityRecordTransform(RecordTransform):
    def transform(self, record: Record) -> Record:
        return record

    def transform_batch(self, records: RecordBatch) -> RecordBatch:
        return records


@functions.register("bench.model")
class ModelRecordTransform(RecordTransform):
    def __init__(self, model: ModelHandler):
        self.model = model

    def transform(self, record: Record) -> Record:
        output = self.model.predict(record)
        if isinstance(output, RecordBatch):
            raise ValueError("model record transform cannot handle record->batch transform")
        return output

    def transform_batch(self, records: RecordBatch) -> RecordBatch:
        return self.model.predict_batch(records)

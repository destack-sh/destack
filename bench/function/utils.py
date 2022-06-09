from bench.function.base import RecordTransform, functions
from bench.utils.record import Record, RecordBatch


@functions.register("bench.identity")
class IdentityRecordTransform(RecordTransform):
    def transform(self, record: Record) -> Record:
        return record

    def transform_batch(self, records: RecordBatch) -> RecordBatch:
        return records

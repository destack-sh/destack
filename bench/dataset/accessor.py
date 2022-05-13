from bench.models import Dataset
from bench.utils.record import Record, RecordBatch


class DatasetAccessor:
    def __init__(self, dataset: Dataset):
        self.dataset = dataset

    def commit(self):
        pass

    def checkout(self):
        pass

    def append(self, record: Record):
        raise NotImplementedError

    def extend(self, records: RecordBatch):
        raise NotImplementedError

    def update(self, index: int, record: Record):
        raise NotImplementedError

    def delete(self, index: int):
        raise NotImplementedError

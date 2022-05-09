import typing
from typing import Iterator, List, Union
from uuid import UUID

from django.db import transaction

from bench.dataset.base import DatasetReader, DatasetWriter
from bench.models import Dataset
from bench.models import Record as DbRecord
from bench.utils.record import Record, RecordBatch
from bench.utils.spec import DatasetSpec, DatasetType, FieldType, RecordSpec


# TODO @Architecture: what does DbDataset do? how does it relate to actual Dataset/DatasetVersion
#  Clearly, DbDataset is special since it talks directly to our DB.
#  However, we probably want to leave some/many aspects of DatasetVersion management
#  to a higher level interface.
class DbDataset(DatasetReader, DatasetWriter):
    def __init__(
        self,
        artifact_id: UUID,
        version: str,
        spec: Union[None, DatasetType, DatasetSpec] = None,
    ):
        super().__init__(version=version, spec=spec)

        self._dataset: Dataset = Dataset.objects.filter()

    def set_spec(self, spec: RecordSpec):
        raise NotImplementedError

    def append(self, record: Record):
        db_record = DbRecord(index=len(self), data=record)
        db_record.save()

    def extend(self, records: RecordBatch):
        # TODO @Performance: batch DbDataset.extend insert
        with transaction.atomic():
            for record in records:
                self.append(record)

    def update(self, index: int, record: Record):
        raise NotImplementedError

    def delete(self, index: int):
        raise NotImplementedError

    @typing.overload
    def __getitem__(self, index: int) -> Record:
        ...

    @typing.overload
    def __getitem__(self, index: slice) -> RecordBatch:
        ...

    @typing.overload
    def __getitem__(self, index: str) -> List[FieldType]:
        ...

    def __getitem__(
        self, index: Union[int, slice, str]
    ) -> Union[Record, RecordBatch, List[FieldType]]:
        raise NotImplementedError

    def __iter__(self) -> Iterator[Record]:
        raise NotImplementedError

    def __len__(self) -> int:
        raise NotImplementedError

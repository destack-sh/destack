import typing
from typing import Iterator, Union
from uuid import UUID

from django.db import transaction

from bench.dataset.base import DatasetReader, DatasetWriter
from bench.models import DatasetVersion
from bench.models import Record as DbRecord
from bench.models.record import (
    RecordTree,
    append_to_tree,
    get_record,
    get_records_field,
    get_records_slice,
    iter_record_tree,
)
from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.spec import DatasetSpec, DatasetType, FieldType, RecordSpec


# TODO @Architecture: what does DbDataset do? how does it relate to actual Dataset/DatasetVersion
#  Clearly, DbDataset is special since it talks directly to our DB.
#  Also, DbDataset should not deal with anything but DbRecord.data (e.g. its metadata).
#  Generally, we probably want some higher level interface orchestrating our DB access.
class DbDataset(DatasetReader, DatasetWriter):
    def __init__(
        self,
        artifact_id: UUID,
        version: str,
        spec: Union[None, DatasetType, DatasetSpec] = None,
    ):
        super().__init__(version=version, spec=spec)

        self._dataset: DatasetVersion = DatasetVersion.objects.filter(
            artifact_id=artifact_id, version=version
        ).get()

    @property
    def root_tree(self) -> RecordTree:
        if self._dataset.records_root is None:
            raise ValueError(f"dataset has no records root {self._dataset}")
        return self._dataset.records_root

    def set_spec(self, spec: RecordSpec):
        raise NotImplementedError

    def append(self, record: Record):
        db_record = DbRecord(data=record)
        append_to_tree(self.root_tree, db_record)

    def extend(self, records: RecordBatch):
        # TODO @Performance: batch DbDataset.extend insert
        with transaction.atomic():
            for record in records:
                self.append(record)

    def update(self, index: int, record: Record):
        db_record = get_record(self.root_tree, index)
        db_record.data = record
        db_record.save()

    def delete(self, index: int):
        raise NotImplementedError

    @typing.overload
    def __getitem__(self, index: int) -> Record:
        ...

    @typing.overload
    def __getitem__(self, index: slice) -> RecordBatch:
        ...

    @typing.overload
    def __getitem__(self, index: str) -> list[FieldType]:
        ...

    def __getitem__(
        self, index: Union[int, slice, str]
    ) -> Union[Record, RecordBatch, list[FieldType]]:
        if isinstance(index, int):
            db_record = get_record(self.root_tree, index)
            return db_record.data
        elif isinstance(index, slice):
            db_records = get_records_slice(
                self.root_tree, index.start, index.stop or len(self)
            )
            return RecordList([record.data for record in db_records])
        elif isinstance(index, str):
            return get_records_field(self.root_tree, index)
        else:
            raise ValueError(f"unexpected index type: {index}")

    def __iter__(self) -> Iterator[Record]:
        return iter_record_tree(self.root_tree)

    def __len__(self) -> int:
        return self.root_tree.max_index

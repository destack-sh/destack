import typing
from typing import Iterator, Optional, Union
from uuid import UUID

from bench.dataset.accessor import DatasetAccessor, DatasetRecord
from bench.dataset.base import DatasetHandlerMetadata, DatasetReader, DatasetWriter, datasets
from bench.models import DatasetVersion
from bench.models.record import clear_record_tree, delete_record
from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.spec import DatasetType, FieldValue, RecordSpec


@datasets.register("bench.db")
class DbDataset(DatasetReader, DatasetWriter):
    """Our primary implementation of datasets that delegates to DatasetAccessor."""

    metadata = DatasetHandlerMetadata(
        name="In-DB dataset",
        description="Versioned in-DB datasets scaling to Ms of records",
        tags=["local"],
    )

    def __init__(
        self,
        artifact_id: UUID,
        version: str,
        spec: Optional[DatasetType] = None,
    ):
        self._dataset: DatasetVersion = DatasetVersion.objects.filter(
            artifact_id=artifact_id, version=version
        ).get()
        self._ds_accessor = DatasetAccessor(self._dataset)

        super().__init__(artifact_id=artifact_id, version=version, spec=spec or self._dataset.spec)

    def set_spec(self, spec: RecordSpec):
        # nothing special needs to be done
        pass

    def append(self, record: Record) -> int:
        return self._ds_accessor.append(DatasetRecord.make(record))

    def extend(self, records: typing.Iterable[Record]) -> tuple[int, int]:
        return self._ds_accessor.extend(DatasetRecord.make(record) for record in records)

    def update(self, index: int, record: Record):
        self.db_update(index, DatasetRecord.make(record))

    def delete(self, index: int):
        delete_record(self.root, index)

    def clear(self):
        clear_record_tree(self.root)

    @typing.overload
    def __getitem__(self, index: int) -> Record:
        ...

    @typing.overload
    def __getitem__(self, index: slice) -> RecordBatch:
        ...

    @typing.overload
    def __getitem__(self, index: str) -> list[FieldValue]:
        ...

    def __getitem__(
        self, index: Union[int, slice, str]
    ) -> Union[Record, RecordBatch, list[FieldValue]]:
        if isinstance(index, int):
            return self._ds_accessor.get_record(index).data
        elif isinstance(index, slice):
            ds_records = self._ds_accessor.get_records_slice(index.start, index.stop)
            return RecordList([record.data for record in ds_records])
        elif isinstance(index, str):
            return self._ds_accessor.get_records_data_field(index)
        else:
            raise ValueError(f"unexpected index type: {index}")

    def __iter__(self) -> Iterator[Record]:
        for ds_record in self._ds_accessor:
            yield ds_record.data

    def __len__(self) -> int:
        return len(self._ds_accessor)

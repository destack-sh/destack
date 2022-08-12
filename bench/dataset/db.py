import typing
from typing import Iterator, Optional, Union
from uuid import UUID

from django.db import connection

from bench.artifact.base import ArtifactVersionHandler
from bench.dataset.base import DatasetReader, DatasetWriter, datasets
from bench.models import DatasetVersion
from bench.models import Record as DbRecord
from bench.models.record import (
    RecordTree,
    RecordTreeReference,
    append_record,
    append_records,
    clear_record_tree,
    delete_record,
    get_record,
    get_records_field,
    get_records_slice,
    iter_record_tree,
    replace_record,
)
from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.spec import DatasetType, FieldValue, RecordSpec


# TODO @Architecture: what does DbDataset do? how does it relate to actual Dataset/DatasetVersion
#  Clearly, DbDataset is special since it talks directly to our DB.
#  Also, DbDataset should not deal with anything but DbRecord.data (e.g. its metadata).
#  Generally, we probably want some higher level interface orchestrating our DB access.
#  See DatasetAccessor for an attempt at abstracting this and further discussion.
@datasets.register("bench.db")
class DbDataset(DatasetReader, DatasetWriter, ArtifactVersionHandler):
    def __init__(
        self,
        artifact_id: UUID,
        version: str,
        spec: Optional[DatasetType] = None,
    ):
        self._dataset: DatasetVersion = DatasetVersion.objects.filter(
            artifact_id=artifact_id, version=version
        ).get()

        # TODO @Cleanup: move/guard record tree creation on write access?
        if self._dataset.record_tree_root is None:
            new_root = RecordTree()
            new_root.save()
            self._dataset.record_tree_root = new_root
            self._dataset.save()

        super().__init__(version=version, spec=spec or self._dataset.spec)

    @property
    def root(self) -> RecordTree:
        if self._dataset.record_tree_root is None:
            raise ValueError(f"dataset has no records root {self._dataset}")
        return self._dataset.record_tree_root

    def set_spec(self, spec: RecordSpec):
        # nothing special needs to be done
        pass

    def append(self, record: Record) -> int:
        # TODO @Storage: use content hashes to avoid creating duplicates?
        db_record = DbRecord.objects.create(data=record)
        return append_record(self.root, db_record)

    def extend(self, records: typing.Iterable[Record]) -> tuple[int, int]:
        db_records = [DbRecord(data=record) for record in records]
        DbRecord.objects.bulk_create(db_records)
        return append_records(self.root, db_records)

    def update(self, index: int, record: Record):
        db_record = get_record(self.root, index)
        # TODO @Storage: use content hashes to avoid creating duplicates?
        # "copy" record, replace data with new record data
        db_record.id = None
        db_record._state.adding = True
        db_record.data = record
        db_record.save()
        # replace current record reference with new record reference
        replace_record(self.root, index, new_record=db_record)

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
            db_record = get_record(self.root, index)
            return db_record.data
        elif isinstance(index, slice):
            stop = index.stop if index.stop is not None else 0
            db_records = get_records_slice(self.root, index.start, stop)
            return RecordList([record.data for record in db_records])
        elif isinstance(index, str):
            return get_records_field(self.root, index)
        else:
            raise ValueError(f"unexpected index type: {index}")

    def __iter__(self) -> Iterator[Record]:
        for db_record in iter_record_tree(self.root):
            yield db_record.data

    def __len__(self) -> int:
        return self.root.max_index + 1

    def commit(self):
        # mark this tree and all subtrees (recursive) as committed
        has_subtrees = (
            RecordTreeReference.objects.filter(tree=self.root).exclude(subtree=None).exists()
        )
        if has_subtrees:
            raise NotImplementedError("committing trees with subtrees is not supported yet")

        self.root.committed = True
        self.root.save()

    def checkout(self, version: str) -> "ArtifactVersionHandler":
        new_dataset: DatasetVersion = DatasetVersion.objects.filter(
            artifact_id=self._dataset.artifact.id, version=version
        ).get()
        if new_dataset.record_tree_root is not None:
            raise RuntimeError(f"new dataset {new_dataset} already has record root")

        # create new tree root for new dataset
        new_root = RecordTree.objects.create(committed=False)
        new_dataset.record_tree_root = new_root
        new_dataset.save()

        # copy root tree references for new tree
        with connection.cursor() as cursor:
            cursor.execute(
                "INSERT INTO bench_recordtreereference (tree_id, [index], record_id, subtree_id)"
                " SELECT %s, [index], record_id, subtree_id"
                " FROM bench_recordtreereference WHERE tree_id = %s",
                params=[str(new_root.id), str(new_root.id)],
            )

        return DbDataset(artifact_id=new_dataset.artifact.id, version=version, spec=self.spec)

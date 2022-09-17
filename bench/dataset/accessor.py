import dataclasses
from typing import Any, Iterable, Iterator, Optional, Sequence, Union
from uuid import UUID

import structlog.stdlib
from django.db import connection, transaction
from django.db.models import QuerySet
from fsspec import AbstractFileSystem

import bench.models.record as db_record
from bench.dataset.base import DatasetHandler, DatasetReader, DatasetWriter, load_dataset
from bench.models import ArtifactVersion, Dataset, DatasetVersion
from bench.models.dataset import DatasetMetadata, DatasetMetadataSerializer, DatasetViewData
from bench.models.record import DbRecord, DbRecordTree, DbRecordTreeReference
from bench.models.utils import proxies
from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.spec import BLANK_RECORD_SPEC, FieldValue, RecordSpec, is_blank_spec

logger = structlog.stdlib.get_logger()


@dataclasses.dataclass
class DatasetRecord:
    __slots__ = ["data", "metadata", "index", "id"]

    data: Record
    metadata: Optional[dict[str, Any]]
    index: Optional[int]
    id: Optional[UUID]

    @staticmethod
    def make(data: Record, metadata: dict[str, Any] = None, index: int = None, id: UUID = None):
        return DatasetRecord(data=data, metadata=metadata, index=index, id=id)

    @staticmethod
    def from_db_record(record: DbRecord, index: int = None) -> "DatasetRecord":
        return DatasetRecord(data=record.data, metadata=record.metadata, id=record.id, index=index)


@dataclasses.dataclass
class DatasetSearch:
    text_like: Optional[str] = None


AnyDatasetRecord = Union[DatasetRecord, DbRecord]


class DatasetAccessor:
    """Manages all access to datasets managed by us, including any indices."""

    def __init__(self, dataset: DatasetVersion):
        if dataset.handler_id != "bench.db":
            raise ValueError("primary dataset accessor only works with our datasets")

        self.dataset = dataset

    @property
    def root(self) -> DbRecordTree:
        # auto-create record tree root if we don't have one yet
        # TODO @Cleanup: move/guard record tree creation on write access?
        if self.dataset.record_tree_root is None:
            new_root = DbRecordTree()
            new_root.save()
            self.dataset.record_tree_root = new_root
            self.dataset.save()
        return self.dataset.record_tree_root

    def search_records(
        self, search: DatasetSearch, limit: int, offset: int
    ) -> Sequence[AnyDatasetRecord]:
        db_query = DbRecord.objects.filter(data__search=search.text_like)
        db_records = db_query[offset : offset + limit]
        # TODO @Feature: set correct index for dataset records within search
        # TODO @Feature: apply limit & offset
        return [DatasetRecord.from_db_record(record) for record in db_records]

    def get_record(self, index: int) -> AnyDatasetRecord:
        return DatasetRecord.from_db_record(db_record.get_record(self.root, index), index)

    def get_records_slice(
        self, start: int, stop: Optional[int] = None
    ) -> Sequence[AnyDatasetRecord]:
        stop = stop if stop is not None else 0
        db_records = db_record.get_records_slice(self.root, start, stop)
        ds_records = []
        for i, record in enumerate(db_records):
            ds_records.append(DatasetRecord.from_db_record(record, start + i))
        return ds_records

    def get_records_data_field(self, key: str) -> list[FieldValue]:
        return db_record.get_records_field(self.root, key)

    def __iter__(self) -> Iterator[AnyDatasetRecord]:
        for record in db_record.iter_record_tree(self.root):
            yield record

    def append(self, record: DatasetRecord) -> int:
        return db_record.append_record(
            self.root, DbRecord.objects.create(data=record.data, metadata=record.metadata)
        )

    def extend(self, records: Iterable[DatasetRecord]) -> tuple[int, int]:
        db_records = [DbRecord(data=record.data, metadata=record.metadata) for record in records]
        DbRecord.objects.bulk_create(db_records)
        return db_record.append_records(self.root, db_records)

    def update(self, index: int, record: DatasetRecord):
        # Note that this always creates a new record even if we just want to change some metadata.
        # We might want another endpoint to just update metadata in-place (for realz) later.
        db_record.update_record(self.root, index, data=record.data, metadata=record.metadata)

    def delete(self, index: int):
        db_record.delete_record(self.root, index)

    def __len__(self) -> int:
        return self.root.max_index + 1

    @staticmethod
    def checkout_from_parents(dataset: DatasetVersion):
        parents: QuerySet[DatasetVersion] = proxies(dataset.parents.all(), DatasetVersion)
        if parents:
            # this should be caught in DatasetVersionSerializer validation
            if len(parents) != 1:
                raise RuntimeError("creating versions with multiple parents is not supported yet")

            # note: parent metadata is copied in ArtifactVersionViewSet.create prior to actual create
            # create a new version of the dataset state based on the parent
            # TODO @Architecture: creating new version logic should be elsewhere (DatasetAccessor?)
            #  because it is a shared concern and needs to drill down into (partial) sub-datasets.
            parent = parents[0]
            DatasetAccessor(parent).checkout(dataset.version)
        return DatasetAccessor(dataset)

    def commit(self):
        has_subtrees = (
            DbRecordTreeReference.objects.filter(tree=self.root).exclude(subtree=None).exists()
        )
        if has_subtrees:
            raise NotImplementedError("committing trees with subtrees is not supported yet")
        if self.dataset.committed:
            raise ValueError(f"cannot commit dataset, already committed: {self.dataset}")

        self.root.committed = True
        self.root.save()
        self.dataset.committed = True
        self.dataset.save()

    def checkout(self, version: str):
        new_dataset: DatasetVersion = DatasetVersion.objects.filter(
            artifact_id=self.dataset.artifact.id, version=version
        ).get()
        if new_dataset.record_tree_root is not None:
            raise RuntimeError(f"new dataset {new_dataset} already has record root")

        # create new tree root for new dataset
        new_root = DbRecordTree.objects.create(committed=False)
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


# TODO @Performance: internalize_dataset should be a background job
def internalize_dataset(dataset: DatasetVersion, batch_size: int = 512):
    # converts a dataset into our in-DB indexed dataset format
    if dataset.handler_id == "bench.db":
        # already internalized
        return

    logger.info("internalize_dataset", dataset=dataset)
    source_reader = get_dataset_version_reader(dataset)
    with transaction.atomic():
        dataset.handler_id = "bench.db"
        target_ds = DatasetAccessor(dataset)

        source_len = len(source_reader)
        for i in range(0, source_len, batch_size):
            logger.info("internalize_dataset", dataset=dataset, i=i, batch_size=batch_size)
            end = min(i + batch_size, source_len)
            ds_records = [DatasetRecord.make(record) for record in source_reader[i:end]]
            target_ds.extend(ds_records)

        dataset.save()
    logger.info("internalize_dataset_done", dataset=dataset)


def _to_handler_args(version: DatasetVersion) -> dict:
    return {
        "arguments": version.config_arguments,
        "artifact_id": version.artifact.id,
        "handler_id": version.handler_id,
        "storage_uri": version.storage_uri,
        "version": version.version,
        "spec": version.spec,
    }


def get_dataset_version_handler(version: DatasetVersion) -> DatasetHandler:
    return load_dataset(**_to_handler_args(version))


def get_dataset_version_reader(version: DatasetVersion) -> DatasetReader:
    dataset = load_dataset(**_to_handler_args(version))
    if not isinstance(dataset, DatasetReader):
        raise ValueError(f"dataset cannot be read: {dataset}")
    return dataset


def get_dataset_version_writer(version: DatasetVersion) -> DatasetWriter:
    dataset = load_dataset(**_to_handler_args(version))
    if not isinstance(dataset, DatasetWriter):
        raise ValueError(f"dataset cannot be written: {dataset}")
    return dataset


def records_to_batch(records: Union[Record, RecordBatch]) -> RecordBatch:
    if isinstance(records, RecordBatch):
        return records
    else:
        return RecordList([records])


def read_dataset_version(
    version: DatasetVersion, view_data: Optional[DatasetViewData] = None
) -> RecordBatch:
    reader = get_dataset_version_reader(version)
    if view_data is None:
        return reader[0 : len(reader)]
    else:
        return reader[view_data.apply(0) : view_data.apply(len(reader))]


def update_dataset_spec(
    dataset: DatasetVersion, record_spec: Optional[RecordSpec], overwrite: bool = True
):
    if record_spec is None or is_blank_spec(record_spec.type):
        return
    if overwrite or (dataset.record_spec is None or is_blank_spec(dataset.record_spec.type)):
        updated_metadata = dataclasses.replace(dataset.metadata_typed, record_spec=record_spec)
        dataset.metadata = DatasetMetadataSerializer(updated_metadata).data
        dataset.save()


def write_to_dataset(
    name: str,
    version: str,
    records: Union[Record, RecordBatch],
    record_spec: Optional[RecordSpec] = None,
    append=True,
) -> tuple[ArtifactVersion, tuple[int, int]]:
    dataset = Dataset.objects.get_or_create_dataset_version(
        name,
        version,
        metadata=DatasetMetadata(
            handler_id="bench.db", record_spec=record_spec or BLANK_RECORD_SPEC
        ),
    )
    update_dataset_spec(dataset, record_spec)
    view_slice = write_to_dataset_version(dataset, records, append=append)
    return dataset, view_slice


def write_to_dataset_version(
    version: DatasetVersion,
    records: Union[Record, RecordBatch],
    record_spec: Optional[RecordSpec] = None,
    append: bool = True,
) -> tuple[int, int]:
    writer = get_dataset_version_writer(version)
    update_dataset_spec(version, record_spec)
    if not append:
        writer.clear()
    batch = records_to_batch(records)
    return writer.extend(batch)


def get_file_system(storage_uri: str) -> tuple[AbstractFileSystem, str]:
    """
    Parses the given storage uri into the corresponding file system & path
    @param storage_uri: The storage URI
    @return: A tuple of [fs, path]
    """
    # TODO @Feature: parse storage_uri into fsspec's fs & path for ArtifactHandler
    #  Note: how will we get auth information in here?
    raise NotImplementedError

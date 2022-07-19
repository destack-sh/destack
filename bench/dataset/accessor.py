from typing import Optional, Union

import structlog.stdlib

from bench.dataset.base import (
    DatasetHandler,
    DatasetReader,
    DatasetWriter,
    get_dataset_reader,
    get_dataset_writer,
    load_dataset,
)
from bench.models import ArtifactVersion, Dataset, DatasetVersion
from bench.models.dataset import DatasetMetadata, DatasetViewData
from bench.utils.record import Record, RecordBatch, RecordList

logger = structlog.stdlib.get_logger()


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


def _to_handler_opts(version: DatasetVersion) -> dict:
    return {
        **version.config_arguments,
        "handler_id": version.handler_id,
        "storage_uri": version.storage_uri,
        "version": version.version,
        "spec": version.spec,
        "artifact_id": version.artifact.id,
    }


def get_dataset_version_handler(version: DatasetVersion) -> DatasetHandler:
    return load_dataset(**_to_handler_opts(version))


def get_dataset_version_reader(version: DatasetVersion) -> DatasetReader:
    return get_dataset_reader(**_to_handler_opts(version))


def get_dataset_version_writer(version: DatasetVersion) -> DatasetWriter:
    return get_dataset_writer(**_to_handler_opts(version))


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


def write_to_dataset(
    name: str, version: str, records: Union[Record, RecordBatch], append=True
) -> tuple[ArtifactVersion, tuple[int, int]]:
    dataset = Dataset.objects.get_or_create_dataset_version(
        name, version, metadata=DatasetMetadata.default_db()
    )
    view_slice = write_to_dataset_version(dataset, records, append=append)
    return dataset, view_slice


def write_to_dataset_version(
    version: DatasetVersion, records: Union[Record, RecordBatch], append: bool = True
) -> tuple[int, int]:
    writer = get_dataset_version_writer(version)
    if not append:
        writer.clear()
    batch = records_to_batch(records)
    return writer.extend(batch)

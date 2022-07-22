import dataclasses
from typing import Optional, Union

import structlog.stdlib
from fsspec import AbstractFileSystem

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
from bench.utils.spec import BLANK_RECORD_SPEC, RecordSpec

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
        "arguments": {**version.config_arguments, "artifact_id": version.artifact.id},
        "handler_id": version.handler_id,
        "storage_uri": version.storage_uri,
        "version": version.version,
        "spec": version.spec,
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


def update_dataset_spec_if_unset(dataset: DatasetVersion, record_spec: Optional[RecordSpec]):
    if dataset.record_spec is None and record_spec is not None:
        dataset.metadata = DatasetMetadata(
            **dataclasses.asdict(dataset.metadata), record_spec=record_spec
        )
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
    update_dataset_spec_if_unset(dataset, record_spec)
    view_slice = write_to_dataset_version(dataset, records, append=append)
    return dataset, view_slice


def write_to_dataset_version(
    version: DatasetVersion,
    records: Union[Record, RecordBatch],
    record_spec: Optional[RecordSpec] = None,
    append: bool = True,
) -> tuple[int, int]:
    writer = get_dataset_version_writer(version)
    update_dataset_spec_if_unset(version, record_spec)
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

from bench.dataset.base import (
    DatasetHandler,
    DatasetReader,
    DatasetWriter,
    get_dataset_reader,
    get_dataset_writer,
    load_dataset,
)
from bench.models import ArtifactVersion, Dataset, DatasetVersion
from bench.models.dataset import DatasetMetadata
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


def write_to_dataset(version: DatasetVersion, records: RecordBatch, append: bool = True):
    writer = get_dataset_version_writer(version)
    if not append:
        writer.clear()
    writer.extend(records)


def read_dataset(version: DatasetVersion) -> RecordBatch:
    reader = get_dataset_version_reader(version)
    return reader[0 : len(reader)]


def convert_records_to_dataset(name: str, data: RecordBatch) -> ArtifactVersion:
    dataset = Dataset.objects.create_dataset_version_by_name(
        name=name, metadata=DatasetMetadata.default_db()
    )
    write_to_dataset(dataset, data)
    return dataset

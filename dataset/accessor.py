from bench.dataset.base import DatasetReader, DatasetWriter, get_dataset_reader, get_dataset_writer
from bench.models import DatasetVersion


def _to_handler_opts(version: DatasetVersion) -> dict:
    return {
        **version.config_arguments,
        "handler_id": version.handler_id,
        "storage_uri": version.storage_uri,
        "version": version.version,
        "spec": version.spec,
        "artifact_id": version.artifact.id,
    }


def get_dataset_version_reader(version: DatasetVersion) -> DatasetReader:
    return get_dataset_reader(**_to_handler_opts(version))


def get_dataset_version_writer(version: DatasetVersion) -> DatasetWriter:
    return get_dataset_writer(**_to_handler_opts(version))

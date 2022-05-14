from bench.dataset.base import get_dataset_reader, get_dataset_writer
from bench.models import DatasetVersion


def _to_handler_opts(version: DatasetVersion) -> dict:
    return {
        "handler_id": version.handler_id,
        "storage_uri": version.storage_uri,
        "version": version.version,
        "arguments": version.config_arguments,
        "spec": version.spec,
    }


def copy_dataset_version(source: DatasetVersion, target: DatasetVersion):
    # TODO @Performance: batch copy (with configurable batch size)
    source_reader = get_dataset_reader(**_to_handler_opts(source))
    target_writer = get_dataset_writer(**_to_handler_opts(target))

    target_writer.clear()
    for record in source_reader:
        target_writer.append(record)

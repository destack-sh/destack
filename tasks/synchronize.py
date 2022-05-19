from bench.dataset.accessor import get_dataset_version_reader, get_dataset_version_writer
from bench.models import DatasetVersion


def copy_dataset_version(source: DatasetVersion, target: DatasetVersion):
    # TODO @Performance: batch copy (with configurable batch size)
    source_reader = get_dataset_version_reader(source)
    target_writer = get_dataset_version_writer(target)

    target_writer.clear()
    for record in source_reader:
        target_writer.append(record)

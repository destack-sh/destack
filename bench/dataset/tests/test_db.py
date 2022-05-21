import copy

import pytest

from bench.dataset.base import DatasetReader
from bench.dataset.db import DbDataset
from bench.models import Dataset
from bench.models.dataset import DatasetMetadata, DatasetVersion
from bench.utils.spec import RecordSpec


@pytest.fixture()
def dataset_version() -> DatasetVersion:
    dataset_version = Dataset.objects.create_dataset_version_by_name(
        name="test",
        version="0",
        metadata=DatasetMetadata(
            handler_id="db",
            config_arguments={},
            record_spec=RecordSpec(name="", description="", type={}),
        ),
    )
    return dataset_version


@pytest.fixture()
def dataset(dataset_version: DatasetVersion) -> DbDataset:
    dataset = DbDataset(artifact_id=dataset_version.artifact.id, version=dataset_version.version)
    return dataset


def _assert_slices_are_equal(dataset: DatasetReader, records: list):
    assert list(dataset) == records
    for i in range(0, len(records)):
        for j in range(i, len(records)):
            assert dataset[i:j] == records[i:j], f"slices [{i}:{j}] are equal"


@pytest.mark.django_db
def test_append_get(dataset: DbDataset):
    records: list[dict] = []
    # append and get individual items
    for i in range(0, 8):
        records.append({"text": str(i), "int": i})
        dataset.append(copy.deepcopy(records[i]))

    # compare all consecutive slices
    _assert_slices_are_equal(dataset, records)


@pytest.mark.django_db
def test_extend_update(dataset: DbDataset):
    records: list[dict] = []
    for i in range(0, 8):
        records.append({"text": str(i), "int": i})
    dataset.extend(copy.deepcopy(records))

    # update
    for i in range(len(records)):
        records[i]["text"] = f"updated {i}"
        records[i]["updated"] = True
        dataset.update(i, copy.deepcopy(records[i]))

    # compare all consecutive slices
    _assert_slices_are_equal(dataset, records)

import pytest

from bench.dataset.db import DbDataset
from bench.models import Dataset
from bench.models.dataset import DatasetMetadata
from bench.utils.spec import RecordSpec


@pytest.mark.django_db
def test_append():
    dataset_version = Dataset.datasets.create_dataset_version_by_name(
        name="test",
        version="0",
        metadata=DatasetMetadata(
            handler_id="db",
            config_arguments={},
            record_spec=RecordSpec(name="", description="", type={}),
        ),
    )
    dataset = DbDataset(artifact_id=dataset_version.artifact.id, version=dataset_version.version)

    dataset.append({"test": 0})
    assert dataset[0] == {"test": 0}

    dataset.append({"test": 1})
    assert dataset[1] == {"test": 1}

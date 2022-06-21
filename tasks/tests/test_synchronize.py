import pytest

from bench.dataset.accessor import get_dataset_version_reader
from bench.models import Dataset
from bench.models.dataset import DatasetMetadata
from bench.utils.spec import RecordSpec
from tasks.synchronize import copy_dataset_version


@pytest.mark.django_db
def test_copy_dataset_version():
    source = Dataset.objects.create_dataset_version(
        "hfhub/rotten_tomatoes",
        metadata=DatasetMetadata(
            handler_id="bench.huggingface.hub",
            config_arguments={"dataset_name": "rotten_tomatoes"},
            record_spec=RecordSpec(name="", description="", type={}),
        ),
    )
    target = Dataset.objects.create_dataset_version(
        "rotten_tomatoes",
        metadata=DatasetMetadata(
            handler_id="bench.db",
            config_arguments={},
            record_spec=RecordSpec(name="", description="", type={}),
        ),
    )
    copy_dataset_version(source, target)
    source_reader = get_dataset_version_reader(source)
    target_reader = get_dataset_version_reader(target)
    assert list(source_reader) == list(target_reader), "source and target are equal"

import json
from typing import Any, Optional

from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.dataset.accessor import DatasetHandler, DatasetRecord
from bench.models import Dataset, DatasetVersion, Organization
from bench.models.versioning import get_head


class Command(BaseCommand):
    help = "Updates dataset from local JSON files"

    def add_arguments(self, parser: CommandParser):
        parser.add_argument("name", type=str)
        parser.add_argument("--path", type=str, required=True)
        parser.add_argument("--organization", type=str, default="local")

    @transaction.atomic
    def handle(self, *args, **options):
        dataset_version = update_dataset(options["organization"], options["name"], options["path"])
        self.stdout.write(
            self.style.SUCCESS(f"Updated dataset with new version: {dataset_version}")
        )


def update_dataset(
    organization_name: str,
    dataset_name: str,
    path: Optional[str] = None,
    records: Optional[list[Any]] = None,
) -> DatasetVersion:
    """Updates a dataset with the given records or path to JSON file"""
    organization = Organization.objects.get(slug=organization_name)
    dataset, created = Dataset.objects.get_or_create(organization=organization, name=dataset_name)
    if created:
        dataset_version = Dataset.objects.create_dataset_version(
            name=dataset_name, organization=organization
        )
    else:
        previous_version = get_head(dataset)
        dataset_version = Dataset.objects.create_dataset_version(
            name=dataset_name, organization=organization
        )
        dataset_version.parents.set([previous_version])

    # read json array from path
    if records is None:
        with open(path, "r") as f:
            records = json.load(f)

    # assume new_data_records must be an array of data-only records
    dataset_version.clear()
    dataset_version.extend(DatasetRecord(data=data) for data in records)
    return dataset_version

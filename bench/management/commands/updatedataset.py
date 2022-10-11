import json

from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.dataset.accessor import DatasetAccessor, DatasetRecord
from bench.models import Dataset, Organization
from bench.models.versioning import get_head


class Command(BaseCommand):
    help = "Updates dataset from local JSON files"

    def add_arguments(self, parser: CommandParser):
        parser.add_argument("name", type=str)
        parser.add_argument("--path", type=str, required=True)
        parser.add_argument("--organization", type=str, default="local")

    @transaction.atomic
    def handle(self, *args, **options):
        organization = Organization.objects.get(slug=options["organization"])
        dataset, created = Dataset.objects.get_or_create(
            organization=organization, name=options["name"]
        )
        if created:
            dataset_version = Dataset.objects.create_dataset_version(
                name=options["name"], organization=organization
            )
        else:
            previous_version = get_head(dataset)
            dataset_version = Dataset.objects.create_dataset_version(
                name=options["name"], organization=organization
            )
            dataset_version.parents.set([previous_version])

        # read json array from path
        with open(options["path"], "r") as f:
            new_data_records = json.load(f)
        # assume new_data_records must be an array of data-only records
        dataset_records = [DatasetRecord.make(data=data) for data in new_data_records]
        DatasetAccessor(dataset_version).extend(dataset_records)
        self.stdout.write(
            self.style.SUCCESS(f"Updated dataset with new version: {dataset_version}")
        )

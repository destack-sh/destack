from django.core.management.base import BaseCommand, CommandParser
from django.db import transaction

from bench.dataset.accessor import internalize_dataset
from bench.models import Dataset
from bench.models.dataset import DatasetMetadata
from bench.models.user import User
from bench.utils.spec import ClassLabelType, FieldSpec, RecordSpec, ValueType


class Command(BaseCommand):
    help = "Sets up dev environment with sample data"

    def add_arguments(self, parser: CommandParser):
        pass

    @transaction.atomic
    def handle(self, *args, **options):
        organization, team, user = User.objects.bootstrap(
            email="test@symbolx.com",
            password="password",
            first_name="Yatima",
            organization_name="Localhost, inc.",
            organization_kwargs={"slug": "local"},
            is_staff=True,
        )
        self.stdout.write(self.style.SUCCESS(f"Created bootstrap user: {user}"))

        ds_metadata = DatasetMetadata(
            handler_id="bench.huggingface.hub",
            config_arguments={"dataset_name": "emotion"},
            record_spec=RecordSpec(
                name="message",
                description="a randomly sampled and annotated tweet",
                type={
                    "text": FieldSpec(name="text", description="", type=ValueType(dtype="str")),
                    "label": FieldSpec(
                        name="label",
                        description="",
                        type=ClassLabelType(
                            num_classes=6,
                            names=["sadness", "joy", "love", "anger", "fear", "surprise"],
                        ),
                    ),
                },
            ),
        )
        dataset_version = Dataset.objects.create_dataset_version(
            name="emotion", organization=organization, metadata=ds_metadata
        )
        # internalize from huggingface hub
        internalize_dataset(dataset_version)

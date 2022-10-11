import os

from django.core.management.base import BaseCommand, CommandParser
from django.db import transaction

from bench.dataset.accessor import internalize_dataset
from bench.models import Dataset
from bench.models.dataset import DatasetMetadata
from bench.models.model import Model, ModelMetadata
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
        dataset = Dataset.objects.create_dataset_version(
            name="emotion", organization=organization, metadata=ds_metadata
        )
        # internalize from huggingface hub
        internalize_dataset(dataset)
        self.stdout.write(self.style.SUCCESS(f"Created dataset: {dataset}"))

        spacy_model_metadata = ModelMetadata(
            handler_id="bench.spacy.bundled", config_arguments={"model": "en_core_web_md"}
        )
        spacy_model = Model.objects.create_model_version(
            name="spacy.en_core_web_md", organization=organization, metadata=spacy_model_metadata
        )
        self.stdout.write(self.style.SUCCESS(f"Created model: {spacy_model}"))

        openai_api_key = os.environ.get("OPENAI_API_KEY")
        if openai_api_key:
            for openai_model in ("text-curie-002", "text-davinci-002"):
                openai_model_metadata = ModelMetadata(
                    handler_id="bench.openai.text_generation",
                    config_arguments={"model": openai_model, "api_key": openai_api_key},
                )
                openai_model = Model.objects.create_model_version(
                    name=f"openai.{openai_model}",
                    organization=organization,
                    metadata=openai_model_metadata,
                )
                self.stdout.write(self.style.SUCCESS(f"Created model: {openai_model}"))
        else:
            self.stdout.write(self.style.NOTICE("Skipped creating model: OPENAI_API_KEY not set"))

import os

from django.core.management.base import BaseCommand, CommandParser
from django.db import transaction

from bench.models.model import Model
from bench.models.user import User


class Command(BaseCommand):
    help = "Sets up dev environment with sample data"

    def add_arguments(self, parser: CommandParser):
        pass

    @transaction.atomic
    def handle(self, *args, **options):
        organization, user = User.objects.bootstrap(
            email="test@symbolx.com",
            password="password",
            first_name="Yatima",
            organization_name="Localhost, inc.",
            organization_kwargs={"slug": "local"},
            is_staff=True,
        )
        self.stdout.write(self.style.SUCCESS(f"Created bootstrap user: {user}"))

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

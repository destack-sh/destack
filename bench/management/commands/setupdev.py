import datetime

from django.core.management.base import BaseCommand, CommandParser
from django.db import transaction

from bench.models import Model, Organization, Project
from bench.models.model import ModelInferenceSettings, ProviderKey
from bench.models.project import ProjectType, ProjectVersion
from bench.models.user import User

TEST_USER_EMAIL = "test@symbolx.com"
TEST_ORGANIZATION_SLUG = "symbolx"

providers = [
    {
        "name": "OpenAI",
        "slug": "openai",
        "models": [
            "text-davinci-002",
            "text-curie-001",
            "text-babbage-001",
            "text-ada-001",
            "code-davinci-002",
            "code-cushman-001",
        ],
    },
    {
        "name": "Goose AI",
        "slug": "gooseai",
        "models": [
            "fairseq-13b",
            "fairseq-6b-7b",
            "gpt-j-20b",
            "gpt-j-6b",
        ],
    },
]


class Command(BaseCommand):
    help = "Sets up dev environment with sample data"

    def add_arguments(self, parser: CommandParser):
        pass

    @transaction.atomic
    def handle(self, *args, **options):
        # create test organization and user if they don't exist
        if not User.objects.filter(email=TEST_USER_EMAIL).exists():
            organization, user = User.objects.bootstrap(
                email=TEST_USER_EMAIL,
                password="password",
                first_name="Yatima",
                organization_name="SymbolX AG.",
                organization_kwargs={"slug": TEST_ORGANIZATION_SLUG},
                is_staff=True,
            )
            self.stdout.write(self.style.SUCCESS(f"Created bootstrap user: {user}"))
        # else:
        #     organization = Organization.objects.get(slug=TEST_ORGANIZATION_SLUG)
        #     user = User.objects.get(email=TEST_USER_EMAIL)

        # create provider models
        self.create_default_providers()

    def create_default_providers(self):
        for provider in providers:
            organization = Organization.objects.filter(slug=provider["slug"]).first()
            if organization is None:
                organization = Organization.objects.create(
                    name=provider["name"], slug=provider["slug"]
                )
                library = Project.objects.create_project(
                    organization,
                    f"{provider['name']} standard library",
                    "stdlib",
                    type=ProjectType.LIBRARY,
                )
                self.stdout.write(self.style.SUCCESS(f"Created provider: {organization}"))
            else:
                library = Project.objects.filter(organization=organization, slug="backends").first()

            # version with current month format like 2022.11
            version_id = datetime.datetime.now().strftime("%Y.%m")
            library_v: ProjectVersion = library.head  # just advance head
            if library_v.name == version_id:
                # skip if version already exists
                continue

            # add models to library
            for model_id in provider["models"]:
                provider_key = ProviderKey[provider["slug"].upper()]  # type: ignore
                model = Model.objects.create(
                    name=model_id,
                    provider=provider_key,
                    default_settings=ModelInferenceSettings.objects.create(),
                )
                library_v.create_file(name=model_id).create_definition(model)
            library_v.commit(version_id)

            self.stdout.write(
                self.style.SUCCESS(
                    f"Created provider library {library_v} with models: {provider['models']}"
                )
            )

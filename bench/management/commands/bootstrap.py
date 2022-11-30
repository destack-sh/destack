import datetime

import structlog
from django.core.management import BaseCommand
from django.db import transaction

from bench.management.commands.load import load_symbols
from bench.models import Model, ModelInferenceSettings, Organization, Project
from bench.models.model import ProviderKey
from bench.models.project import ProjectType, ProjectVersion

logger = structlog.get_logger(__name__)

providers = [
    {
        "name": "OpenAI",
        "slug": "openai",
        "models": [
            "text-davinci-003",
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
    help = "Initializes the database"

    @transaction.atomic
    def handle(self, *args, **options):
        create_model_providers()
        create_symbolx_stdlib("bench/demo/stdlib.py")


@transaction.atomic
def create_symbolx_stdlib(path: str) -> None:
    stdlib = get_or_create_stdlib("SymbolX", "symbolx")
    version_id = datetime.datetime.now().strftime("%Y.%m.%d")
    stdlib_v: ProjectVersion = stdlib.head_
    if stdlib_v.name == version_id:
        # skip if version already exists
        logger.info(f"Skip updating library {stdlib_v} to {version_id} (already exists)")
        return

    stdlib_v = stdlib.create_version(version_id, parent=stdlib_v)
    load_symbols(stdlib_v, path)

    logger.info(f"Created library {stdlib_v} from {path}")


@transaction.atomic
def create_model_providers():
    for provider in providers:
        stdlib = get_or_create_stdlib(provider["name"], provider["slug"])
        stdlib_v: ProjectVersion = stdlib.head_
        # version with date format like 2022.11.29
        version_id = datetime.datetime.now().strftime("%Y.%m.%d")
        if stdlib_v.name == version_id:
            # skip if version already exists
            logger.info(f"Skip updating library {stdlib_v} to {version_id} (already exists)")
            continue

        # add models to library
        for model_id in provider["models"]:
            provider_key = ProviderKey[provider["slug"].upper()]  # type: ignore
            model = Model.objects.create(
                external_name=model_id,
                provider=provider_key,
                default_settings=ModelInferenceSettings.objects.create(),
            )
            model_file = stdlib_v.create_file(name=model_id)
            model_file.create_definition(model_id, model)
        stdlib_v.commit(version_id)

        logger.info(f"Created provider library {stdlib_v} with models: {provider['models']}")


def get_or_create_stdlib(organization_name: str, organization_slug: str) -> Project:
    organization = Organization.objects.filter(slug=organization_slug).first()
    if organization is None:
        organization = Organization.objects.create(name=organization_name, slug=organization_slug)
        library = Project.objects.create_project(
            organization,
            f"{organization_name} standard library",
            "stdlib",
            type=ProjectType.LIBRARY,
        )
        logger.info(f"Created provider: {organization}")
    else:
        library = Project.objects.get(organization=organization, slug="stdlib")
    return library

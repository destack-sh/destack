import datetime
from dataclasses import dataclass
from pathlib import Path

import structlog
from django.core.management import BaseCommand
from django.db import transaction

from bench.backend.mapper import write
from bench.language import lex, parse
from bench.language.lex import SourceFile
from bench.models import Model, ModelInferenceSettings, Organization, Project
from bench.models.model import ProviderKey
from bench.models.project import FileType, ProjectType, ProjectVersion

logger = structlog.get_logger(__name__)


@dataclass
class Provider:
    name: str
    slug: str
    models: list[str]


providers: list[Provider] = [
    Provider(
        name="OpenAI",
        slug="openai",
        models=[
            "text-davinci-003",
            "text-davinci-002",
            "text-curie-001",
            "text-babbage-001",
            "text-ada-001",
            "code-davinci-002",
            "code-cushman-001",
        ],
    ),
    Provider("Goose AI", "gooseai", ["fairseq-13b", "fairseq-6b-7b", "gpt-j-20b", "gpt-j-6b"]),
]


class Command(BaseCommand):
    help = "Initializes the database"

    def add_arguments(self, parser):
        # overwrite flag
        parser.add_argument(
            "--force",
            action="store_true",
            help="overwrite existing data",
        )

    @transaction.atomic
    def handle(self, force: bool, *args, **options):
        create_model_providers()
        create_symbolx_stdlib("bench/demo/stdlib.bench", overwrite=force)


@transaction.atomic
def create_symbolx_stdlib(path: str, overwrite: bool) -> None:
    stdlib = get_or_create_stdlib("SymbolX", "symbolx")

    # get last modified date from file at path
    last_modified = datetime.datetime.fromtimestamp(Path(path).stat().st_mtime)
    # format version name as YYYY.MM.DD-ts
    version_id = last_modified.strftime("%Y.%m.%d") + "-" + str(int(last_modified.timestamp()))
    stdlib_v: ProjectVersion = stdlib.head_
    exists = stdlib_v.name == version_id
    if exists and not overwrite:
        # skip if version already exists
        logger.info(f"Skip updating library {stdlib_v} to {version_id} (already exists)")
        return
    if exists:
        logger.warn(f"Overwriting library {stdlib_v} at {version_id}")

    stdlib_v = stdlib.create_version(name=version_id, parent=stdlib_v)
    stdlib_v.reset()
    source_file = SourceFile(path=path, content=Path(path).read_text())
    language_files = parse(lex(source_file))
    write(language_files, stdlib_v)

    # advance head
    stdlib_v.commit(name=version_id)
    stdlib.head = stdlib_v
    stdlib.save()

    logger.info(f"Created library {stdlib_v} from {path}")


@transaction.atomic
def create_model_providers():
    for provider in providers:
        stdlib = get_or_create_stdlib(provider.name, provider.slug)
        stdlib_v: ProjectVersion = stdlib.head_
        # version with date format like 2022.11.29
        version_id = datetime.datetime.now().strftime("%Y.%m.%d")
        if stdlib_v.name == version_id:
            # skip if version already exists
            logger.info(f"Skip updating library {stdlib_v} to {version_id} (already exists)")
            continue
        stdlib_v = stdlib.create_version(name=version_id, parent=stdlib_v)
        stdlib_v.reset()

        # add models to library
        models_file = stdlib_v.create_file(name="text", type=FileType.INSTRUCT)
        for model_id in provider.models:
            provider_key = ProviderKey[provider.slug.upper()]
            model = Model.objects.create(
                external_name=model_id,
                provider=provider_key,
                default_settings=ModelInferenceSettings.objects.create(),
            )
            models_file.define_symbol(name=model_id, content=model)

        # advance head
        stdlib_v.commit(version_id)
        stdlib.head = stdlib_v
        stdlib.save()

        logger.info(f"Created provider library {stdlib_v} with models: {provider.models}")


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

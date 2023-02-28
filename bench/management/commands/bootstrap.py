import datetime
from dataclasses import dataclass
from pathlib import Path

import structlog
from django.core.management import BaseCommand
from django.db import transaction

from bench.language import lex, parse, wire
from bench.language.lex import SourceFile
from bench.language.type import StatementType, SymbolType
from bench.models import Organization, Project, Statement
from bench.models.mapper import lookup_in_db_module, write_module
from bench.models.project import ProjectType, ProjectVersion, ProjectVisibility
from bench.runtime.execute import ProviderKey
from bench.utils.fractional import generate_n_keys_between

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
    Provider(
        "Cohere", "cohere", ["xlarge", "medium", "command-xlarge-beta", "command-xlarge-nightly"]
    ),
    Provider(
        "Forefront", "forefront", ["codegen-16b", "gpt-j-6b", "gpt-neox-20b", "codegen-16b-nl"]
    ),
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
        create_symbolx_std("bench/bench/std.bench", overwrite=force)


@transaction.atomic
def create_symbolx_std(path: str, overwrite: bool) -> None:
    std = get_or_create_std("SymbolX", "symbolx")

    # get last modified date from file at path
    last_modified = datetime.datetime.fromtimestamp(Path(path).stat().st_mtime)
    # format version name as YYYY.MM.DD-ts
    version_id = last_modified.strftime("%Y.%m.%d") + "-" + str(int(last_modified.timestamp()))
    std_v: ProjectVersion = std.head_
    exists = std_v.name == version_id
    if exists and not overwrite:
        # skip if version already exists
        logger.info(f"Skip updating library {std_v} to {version_id} (already exists)")
        return
    if exists:
        logger.warn(f"Overwriting library {std_v} at {version_id}")
    std_v = std.create_version(name=version_id, parent=std_v)
    std_v.reset()
    source_file = SourceFile(path=path, content=Path(path).read_text())
    module, _ = parse(lex(source_file), lookup_in_module=lookup_in_db_module, on_error="raise")
    wire_module = wire.rmap_module(module)
    write_module(wire_module.files, std_v)

    # advance head
    std_v.commit(name=version_id)
    std.head = std_v
    std.save()

    logger.info(f"Created library {std_v} from {path}")


@transaction.atomic
def create_model_providers():
    for provider in providers:
        std = get_or_create_std(provider.name, provider.slug)
        std_v: ProjectVersion = std.head_
        # version with date format like 2022.11.29
        version_id = datetime.datetime.now().strftime("%Y.%m.%d")
        if std_v.name == version_id:
            # skip if version already exists
            logger.info(f"Skip updating library {std_v} to {version_id} (already exists)")
            continue
        std_v = std.create_version(name=version_id, parent=std_v)
        std_v.reset()

        # add models to library
        # TODO @Cleanup: use bench string instead of DB models to bootstrap model providers
        #  (not yet possible since models can't be expressed in bench yet)
        models_file = std_v.create_file(name="text")
        order_keys = generate_n_keys_between(None, None, len(provider.models))
        for order_key, model_id in zip(order_keys, provider.models):
            provider_key = ProviderKey[provider.slug.upper()]
            Statement.objects.create(
                project_version=std_v,
                file=models_file,
                parent=None,
                order_key=order_key,
                type=StatementType.DEFINITION,
                symbol_type=SymbolType.MODEL,
                name=model_id,
                provider=provider_key,
                external_name=model_id,
            )

        # advance head
        std_v.commit(version_id)
        std.head = std_v
        std.save()

        logger.info(f"Created provider library {std_v} with models: {provider.models}")


def get_or_create_std(organization_name: str, organization_slug: str) -> Project:
    organization = Organization.objects.filter(owner_slug_id=organization_slug).first()
    if organization is None:
        organization = Organization.objects.create_organization(
            name=organization_name, slug=organization_slug
        )
        library = Project.objects.create_project(
            owner=organization,
            name=f"{organization_name} standard library",
            slug="std",
            type=ProjectType.LIBRARY,
            visibility=ProjectVisibility.PUBLIC,
            create_onboarding_files=False,
        )
        logger.info(f"Created provider: {organization}")
    else:
        library = Project.objects.get(organization=organization, slug="std")
    return library

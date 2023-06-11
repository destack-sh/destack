import datetime
from dataclasses import dataclass, field

import structlog
from django.core.management import BaseCommand
from django.db import transaction

from bench.bench import StatementType, SymbolType
from bench.models import Organization, Project, Statement
from bench.models.project import ProjectType, ProjectVersion, ProjectVisibility
from bench.utils.fractional import generate_n_keys_between

logger = structlog.get_logger(__name__)


@dataclass
class Provider:
    name: str
    slug: str
    text_models: list[str | tuple[str, str]] = field(default_factory=list)
    image_models: list[str | tuple[str, str]] = field(default_factory=list)
    audio_models: list[str | tuple[str, str]] = field(default_factory=list)


providers: list[Provider] = [
    Provider(
        name="OpenAI",
        slug="openai",
        text_models=[
            ("gpt4", "gpt-4"),
            ("gpt3", "gpt-3.5-turbo"),
            "text-ada-001",
            "whisper",
        ],
    ),
    Provider(
        name="Anthropic",
        slug="anthropic",
        text_models=[("claude-instant", "claude-instant-v1.1"), ("claude", "claude-v1.3")],
    ),
    Provider(
        name="Hugging Face",
        slug="huggingface",
        text_models=[("starcoder", "bigcode/starcoder")],
        image_models=[("stable-diffusion", "runwayml/stable-diffusion-v1-5")],
    ),
    Provider(name="Stability AI", slug="stabilityai", text_models=[]),
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
    get_or_create_std("SymbolX", "symbolx")
    # no std yet


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
        std_v = std.create_new_blank_head(name=version_id, parent=std_v)

        # add models to library
        # TODO @Cleanup: use bench string instead of DB models to bootstrap model providers
        #  (not yet possible since models can't be expressed in bench yet)
        for type, models in (("text", provider.text_models),):
            models_file = std_v.create_file(name=type)
            order_keys = generate_n_keys_between(None, None, len(models))
            for order_key, model_id in zip(order_keys, models):
                if isinstance(model_id, tuple):
                    model_id, external_name = model_id
                else:
                    external_name = model_id
                Statement.objects.create(
                    project_version=std_v,
                    file=models_file,
                    parent=None,
                    order_key=order_key,
                    type=StatementType.SYMBOL,
                    symbol_type=SymbolType.MODEL,
                    name=model_id,
                    external_name=external_name,
                )

        # advance head
        std_v.commit(version_id)
        std.head = std_v
        std.save()

        logger.info(f"Created provider library {std_v} with models: {provider.text_models}")


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

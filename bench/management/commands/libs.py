import structlog
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench import models
from bench.bench import wire
from bench.bench.libs import DEFAULT_MODULES
from bench.models.packer import upsert_module

logger = structlog.get_logger(__name__)


class Command(BaseCommand):
    help = "Manages generated Bench libraries"

    def add_arguments(self, parser: CommandParser):
        # action
        parser.add_argument("action", type=str, help="Action")
        # module name or 'all'
        parser.add_argument("module", type=str, help="Module")

    @transaction.atomic
    def handle(self, module: str, action: str, *args, **options):
        create_libs_if_not_exists()

        if module == "all":
            modules = DEFAULT_MODULES.keys()
        else:
            modules = [module]

        if action == "upsert":
            for module in modules:
                _upsert_module(module)
        else:
            raise ValueError(f"unknown action: {action}")


def create_libs_if_not_exists():  # probably should put this elsewhere
    for org_name, org_slug in [
        ("SymbolX", "symbolx"),
        ("OpenAI", "openai"),
        ("Anthropic", "anthropic"),
    ]:
        if not models.Organization.objects.filter(owner_slug_id=org_slug).exists():
            models.Organization.objects.create_organization(name=org_name, slug=org_slug)
            logger.info("bootstrap_organization", organization=org_slug)


@transaction.atomic
def _upsert_module(module_name: str, sanity_check: bool = True):
    logger.info("lib.upsert", module=module_name)
    module = DEFAULT_MODULES[module_name]
    owner, name = module_name.split(".")
    try:
        project = models.Project.objects.get_by_slug(owner, name)
    except models.Project.DoesNotExist:
        owner = models.OwnerSlug.objects.get(slug=owner).owner
        project = models.Project.objects.create_project(
            owner=owner,
            slug=name,
            name=name,
            visibility=models.ProjectVisibility.PUBLIC,
            head_version_id=module.id,
        )

    new_module = wire.pack_module(module)
    applied_mutations = upsert_module(project.head, new_module, apply_deletes=False)
    for mut in applied_mutations:
        logger.info("apply", mutation=mut)
    logger.info("lib.upsert.done", module=module_name, mutations=len(applied_mutations))

    if sanity_check:
        # do it again and asset that no mutations are applied
        new_module = wire.pack_module(module)
        applied_mutations = upsert_module(project.head, new_module, apply_deletes=False)
        assert len(applied_mutations) == 0, f"sanity check failed: {applied_mutations}"

import os

import structlog
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench import models
from bench.language import wire
from bench.language.libs import DEFAULT_MODULES
from bench.language.mutate import diff_modules
from bench.models import packer
from bench.models.packer import DEFAULT_PACK_FILTER

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
        create_orgs_if_not_exist()

        if module == "all":
            modules = DEFAULT_MODULES.keys()
        else:
            modules = [module]

        if action == "upsert":
            for module in modules:
                _upsert_module(module, os.environ["VERSION"])
        else:
            raise ValueError(f"unknown action: {action}")


def create_orgs_if_not_exist():  # probably should put this elsewhere
    for org_name, org_slug in [
        ("SymbolX", "symbolx"),
        ("OpenAI", "openai"),
        ("Anthropic", "anthropic"),
    ]:
        if not models.Organization.objects.filter(owner_slug_id=org_slug).exists():
            models.Organization.objects.create_organization(name=org_name, slug=org_slug)
            logger.info("bootstrap_organization", organization=org_slug)


@transaction.atomic
def _upsert_module(module_name: str, version: str):
    """ """
    log = logger.bind(module=module_name, version=version)
    log.info("lib.upsert")
    module = DEFAULT_MODULES[module_name]
    owner, name = module_name.split(".")
    try:
        project = models.Project.objects.get_by_slug(owner, name)

        # delete existing project version if it exists
        existing_project_v = project.versions.filter(tag=version).first()
        if existing_project_v is not None:
            log.info("lib.upsert.delete", project_v=existing_project_v)
            existing_project_v.delete()

        # create new project version
        project_v = models.ProjectVersion.objects.create(
            id=module.id, project=project, name=version, tag=version
        )
        project.head = project_v
        project.save()
    except models.Project.DoesNotExist:
        owner = models.OwnerSlug.objects.get(slug=owner).owner
        project = models.Project.objects.create_project(
            id=module.ck,
            owner=owner,
            slug=name,
            name=name,
            visibility=models.ProjectVisibility.PUBLIC,
            head_version_id=module.id,
        )
        project_v = project.head
        project_v.tag = version
        project_v.name = version
        project_v.save()

    blank_module = packer.pack_module(project_v, filter=DEFAULT_PACK_FILTER)
    blank_module_tree = wire.ModuleTree(blank_module.nodes)
    new_module = wire.pack_module(module)
    mutations = diff_modules(blank_module, new_module)
    packer.write_mutations(project_v, blank_module_tree, mutations, wait_for_os=False)

    project_v.commit()

    log.info("lib.upsert.done", nodes=len(new_module.nodes))

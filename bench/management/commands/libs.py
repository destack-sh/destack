import os
from pathlib import Path

import structlog
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench import models
from bench.language import wire
from bench.language.const import INTERP_NODE_TYPES
from bench.language.edit import diff_modules
from bench.language.libs import DEFAULT_MODULES
from bench.language.module import NodeTree
from bench.models import packer
from bench.models.packer import DEFAULT_PACK_FILTER
from bench.utils.utils import DEBUG, LOCAL, TEST

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
                _upsert_module(module, os.environ["VERSION"], sanity_check=DEBUG or TEST or LOCAL)
        elif action == "dump":
            for module in modules:
                _dump_module(module)
        else:
            raise ValueError(f"unknown action: {action}")


def create_orgs_if_not_exist():  # probably should put this elsewhere
    for org_name, org_slug in [
        ("SymbolX", "symbolx"),
        ("OpenAI", "openai"),
        ("Anthropic", "anthropic"),
        ("Deepgram", "deepgram"),
    ]:
        if not models.Organization.objects.filter(owner_slug_id=org_slug).exists():
            models.Organization.objects.create_organization(name=org_name, slug=org_slug)
            logger.info("bootstrap_organization", organization=org_slug)


@transaction.atomic
def _upsert_module(module_name: str, version: str, sanity_check: bool):
    """
    Replace the module
    """
    log = logger.bind(module=module_name, version=version)
    log.info("lib.upsert")
    module = DEFAULT_MODULES[module_name]
    owner, name = module_name.split(".")
    try:
        project = models.Project.objects.get_by_slug(owner, name)

        # delete existing project version (with same tag) if it exists
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

        # link to parent
        parent = project.versions.filter(tag__lt=version).order_by("-tag").first()
        parents = [parent] if parent is not None else []
        project_v.parents.set(parents)
    except models.Project.DoesNotExist:
        # create new project
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

    blank_module_data = packer.pack_module_host(project_v, filter=DEFAULT_PACK_FILTER)
    wire.unpack_module(blank_module_data.nodes, session=None)
    new_module = wire.pack_module_inline(module, exclude=set())
    edits = diff_modules(blank_module_data, new_module, project_id=project.id)
    packer.write_host_db_edits(project_v, NodeTree(blank_module_data.nodes), edits, validate=False)
    project_v.commit()

    log.info("lib.upsert.done", nodes=len(new_module.nodes))


_LIB_DUMP_DIR = "/tmp/bench_libs"


def _get_module_dump_path(module_name: str):
    return os.path.join(_LIB_DUMP_DIR, f"{module_name}.bench")


def _dump_module(module_name: str):
    module = DEFAULT_MODULES[module_name]
    module_data = wire.pack_module_inline(module, exclude=INTERP_NODE_TYPES)
    module_bytes = wire.serialize_module(module_data)
    module_path = _get_module_dump_path(module_name)
    os.makedirs(os.path.dirname(module_path), exist_ok=True)
    Path(module_path).write_bytes(module_bytes)
